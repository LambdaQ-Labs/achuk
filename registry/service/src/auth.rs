//! Accounts for the Achuk registry: signup, login, token-based auth.
//!
//! Two entry paths share one token: the CLI sends `Authorization: Bearer
//! <token>` on publish; the web UI stores the same token in a signed-ish
//! httponly cookie. Passwords are Argon2id-hashed; tokens are 32 random
//! bytes, hex-encoded, unique per user.

use argon2::password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use axum::extract::State;
use axum::http::{header, HeaderMap, StatusCode};
use axum::response::{IntoResponse, Redirect};
use axum::Form;
use rand::RngCore;
use serde::Deserialize;
use sqlx::Row;

use crate::AppState;

pub async fn init_schema(pool: &sqlx::PgPool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS users (
            id BIGSERIAL PRIMARY KEY,
            username TEXT UNIQUE NOT NULL,
            email TEXT UNIQUE NOT NULL,
            pass_hash TEXT NOT NULL,
            token TEXT UNIQUE NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT now()
        )",
    )
    .execute(pool)
    .await
    .expect("users schema");
    // Attribute packages to their publisher (nullable for pre-auth rows).
    let _ = sqlx::query("ALTER TABLE packages ADD COLUMN IF NOT EXISTS owner TEXT")
        .execute(pool)
        .await;
}

fn gen_token() -> String {
    let mut b = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hash_password(pw: &str) -> Result<String, String> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(pw.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| e.to_string())
}

fn verify_password(pw: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(pw.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

fn valid_username(u: &str) -> bool {
    (3..=32).contains(&u.len())
        && u.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Resolve the caller to a username via Bearer token (CLI) or the `at`
/// cookie (web). Returns None if unauthenticated.
pub async fn current_user(st: &AppState, headers: &HeaderMap) -> Option<String> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
        .or_else(|| {
            headers
                .get(header::COOKIE)
                .and_then(|v| v.to_str().ok())
                .and_then(|c| {
                    c.split(';')
                        .find_map(|kv| kv.trim().strip_prefix("at="))
                        .map(|s| s.to_string())
                })
        })?;
    let row = sqlx::query("SELECT username FROM users WHERE token = $1")
        .bind(&token)
        .fetch_optional(&st.pool)
        .await
        .ok()??;
    Some(row.get::<String, _>("username"))
}

#[derive(Deserialize)]
pub struct SignupForm {
    pub username: String,
    pub email: String,
    pub password: String,
}

/// `POST /signup` (form) — create an account, set the cookie, land on home.
pub async fn signup(
    State(st): State<AppState>,
    Form(f): Form<SignupForm>,
) -> impl IntoResponse {
    if !valid_username(&f.username) {
        return page_msg("Sign up", "Username must be 3–32 chars: letters, numbers, - or _.").into_response();
    }
    if f.password.len() < 8 {
        return page_msg("Sign up", "Password must be at least 8 characters.").into_response();
    }
    if !f.email.contains('@') {
        return page_msg("Sign up", "Enter a valid email.").into_response();
    }
    let hash = match hash_password(&f.password) {
        Ok(h) => h,
        Err(_) => return page_msg("Sign up", "Could not create the account. Try again.").into_response(),
    };
    let token = gen_token();
    let res = sqlx::query(
        "INSERT INTO users (username, email, pass_hash, token) VALUES ($1,$2,$3,$4)",
    )
    .bind(&f.username)
    .bind(&f.email)
    .bind(&hash)
    .bind(&token)
    .execute(&st.pool)
    .await;
    match res {
        Ok(_) => set_cookie_redirect(&token, "/"),
        Err(_) => page_msg("Sign up", "That username or email is already taken.").into_response(),
    }
}

#[derive(Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

/// `POST /login` (form) — verify, set the cookie, land on home.
pub async fn login(
    State(st): State<AppState>,
    Form(f): Form<LoginForm>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT pass_hash, token FROM users WHERE username = $1")
        .bind(&f.username)
        .fetch_optional(&st.pool)
        .await
        .ok()
        .flatten();
    match row {
        Some(r) if verify_password(&f.password, &r.get::<String, _>("pass_hash")) => {
            set_cookie_redirect(&r.get::<String, _>("token"), "/")
        }
        _ => page_msg("Log in", "Wrong username or password.").into_response(),
    }
}

/// `POST /login-cli` (JSON-ish form) — returns the raw token for `achuk login`.
pub async fn login_cli(
    State(st): State<AppState>,
    Form(f): Form<LoginForm>,
) -> impl IntoResponse {
    let row = sqlx::query("SELECT pass_hash, token FROM users WHERE username = $1")
        .bind(&f.username)
        .fetch_optional(&st.pool)
        .await
        .ok()
        .flatten();
    match row {
        Some(r) if verify_password(&f.password, &r.get::<String, _>("pass_hash")) => {
            (StatusCode::OK, r.get::<String, _>("token")).into_response()
        }
        _ => (StatusCode::UNAUTHORIZED, "invalid credentials".to_string()).into_response(),
    }
}

pub async fn logout() -> impl IntoResponse {
    (
        [(
            header::SET_COOKIE,
            "at=; Path=/; HttpOnly; Max-Age=0; SameSite=Lax".to_string(),
        )],
        Redirect::to("/"),
    )
}

fn set_cookie_redirect(token: &str, to: &str) -> axum::response::Response {
    (
        [(
            header::SET_COOKIE,
            format!("at={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age=31536000"),
        )],
        Redirect::to(to),
    )
        .into_response()
}

fn page_msg(title: &str, msg: &str) -> axum::response::Html<String> {
    crate::ui::shell_page(
        title,
        &format!(
            r#"<h1>{}</h1><p class="sub">{}</p><p><a href="/login">Log in</a> · <a href="/signup">Sign up</a></p>"#,
            crate::ui::esc(title),
            crate::ui::esc(msg)
        ),
    )
}

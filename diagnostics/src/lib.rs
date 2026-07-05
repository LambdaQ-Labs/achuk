//! achuk-diagnostics — the structured-error protocol (WS-D).
//!
//! The struct is the source of truth; prose is a rendering. Agents consume
//! `Diagnostic` JSON, apply the top-ranked patch, and re-check — no prose
//! parsing, ever.
//!
//! Spec: docs/p2-spec.md §2.5 / master-plan WS-D.

use achuk_core::Hash;
use serde::{Deserialize, Serialize};

/// Where an error lives: a definition (by content hash) + a span within
/// its rendered form. Never file+line — files are projections.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Loc {
    pub hash: Hash,
    /// Byte span within the rendered definition.
    pub span: (u32, u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    TypeMismatch,
    UnknownSymbol,
    ArityMismatch,
    EffectViolation,
    CapabilityMissing,
    ContractViolation,
    Syntax,
}

/// A machine-applicable fix suggestion, ranked (1 = best).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Patch {
    pub rank: u8,
    /// Human-readable description of the edit.
    pub edit: String,
    /// Optional machine form: replacement rendered-source for the span.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replacement: Option<String>,
}

/// The diagnostic. Everything an agent needs to act, no prose parsing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub loc: Loc,
    /// Stable machine code, e.g. "E-TYPE-0007".
    pub code: String,
    pub category: Category,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub got: Option<String>,
    /// The smallest statement of what must change for this error to clear.
    pub minimal_constraint: String,
    /// Ranked fixes, best first.
    pub patches: Vec<Patch>,
}

impl Diagnostic {
    /// Prose rendering — for humans. Derived, never authoritative.
    pub fn render(&self) -> String {
        let mut s = String::new();
        match (&self.expected, &self.got) {
            (Some(e), Some(g)) => s.push_str(&format!(
                "{} at {}: expected `{}`, got `{}`. ",
                self.code, self.loc.hash, e, g
            )),
            _ => s.push_str(&format!("{} at {}: ", self.code, self.loc.hash)),
        }
        s.push_str(&self.minimal_constraint);
        if let Some(p) = self.best_patch() {
            s.push_str(&format!(" Suggested fix: {}", p.edit));
        }
        s
    }

    /// The best patch, if any — what an agent applies first.
    pub fn best_patch(&self) -> Option<&Patch> {
        self.patches.iter().min_by_key(|p| p.rank)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Diagnostic {
        Diagnostic {
            loc: Loc {
                hash: Hash("7b2e".repeat(16)),
                span: (4, 12),
            },
            code: "E-TYPE-0007".into(),
            category: Category::TypeMismatch,
            expected: Some("Result Ledger TransferErr".into()),
            got: Some("Ledger".into()),
            minimal_constraint: "branch must return Result; wrap in Ok".into(),
            patches: vec![
                Patch {
                    rank: 2,
                    edit: "change return type to `Ledger`".into(),
                    replacement: None,
                },
                Patch {
                    rank: 1,
                    edit: "wrap `Ledger.of {...}` in `Ok (...)`".into(),
                    replacement: Some("Ok (Ledger.of { from: left, to: to.balance + amt })".into()),
                },
            ],
        }
    }

    #[test]
    fn json_roundtrip_is_lossless() {
        let d = sample();
        let json = serde_json::to_string(&d).unwrap();
        let back: Diagnostic = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }

    #[test]
    fn best_patch_is_rank_one_regardless_of_order() {
        let d = sample();
        assert_eq!(d.best_patch().unwrap().rank, 1);
        assert!(d.best_patch().unwrap().replacement.is_some());
    }

    #[test]
    fn render_is_derived_prose() {
        let r = sample().render();
        assert!(r.contains("E-TYPE-0007"));
        assert!(r.contains("expected `Result Ledger TransferErr`"));
        assert!(r.contains("Suggested fix: wrap"));
    }

    #[test]
    fn category_serializes_snake_case() {
        let json = serde_json::to_string(&Category::TypeMismatch).unwrap();
        assert_eq!(json, "\"type_mismatch\"");
    }
}

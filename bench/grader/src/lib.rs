//! claw-bench-grader — the deterministic grader (WS-J).
//!
//! Grading is a pure function of (task, produced state). No model in the
//! loop, reproducible, CI-runnable. Multi-signal: compile-shaped checks ∧
//! hallucination detection ∧ forbidden rules. Tests/contract execution
//! plug in as the compiler comes online — the schema carries them now.
//!
//! Spec: docs/benchmark-harness.md §3.

use claw_cdb::Cdb;
use claw_core::Def;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------
// Task schema (docs/benchmark-harness.md §2.2)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Category {
    FromScratch,
    Translate,
    RepoFeature,
    Contract,
    Effect,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeSpec {
    /// Must the produced code typecheck?
    #[serde(default = "default_true")]
    pub compile: bool,
    /// Test oracles (paths to .claw test specs; executed once the compiler lands).
    #[serde(default)]
    pub tests: Vec<String>,
    /// Contract assertions that must hold.
    #[serde(default)]
    pub contracts: Vec<String>,
    /// Rules the produced code must not trip (e.g. "hallucinated-symbol").
    #[serde(default)]
    pub forbidden: Vec<String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub category: Category,
    pub prompt: String,
    pub grade: GradeSpec,
    /// Path to the reference solution (not shown to the model).
    #[serde(default)]
    pub reference: Option<String>,
}

// ---------------------------------------------------------------------
// Grade result (docs/benchmark-harness.md §3)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GradeResult {
    pub task_id: String,
    pub compiled: bool,
    pub tests_passed: (u32, u32),
    pub contracts_held: (u32, u32),
    pub forbidden_hit: Vec<String>,
    /// Symbols the produced code references that do not exist in the CDB —
    /// the headline metric the constraint server must drive to ~0.
    pub hallucinated_symbols: Vec<String>,
    pub pass: bool,
    pub retries_used: u32,
    pub tokens: u64,
}

/// Grade produced definitions against a task, in the context of a CDB.
///
/// Hallucination detection: (a) any `Ref(hash)` the CDB doesn't contain,
/// (b) any free variable that no bound name resolves — both are references
/// to code that does not exist.
pub fn grade(
    task: &Task,
    produced: &[Def],
    cdb: &Cdb,
    retries_used: u32,
    tokens: u64,
) -> anyhow::Result<GradeResult> {
    let mut hallucinated: Vec<String> = Vec::new();

    let known_names: std::collections::BTreeSet<String> =
        cdb.symbols()?.into_iter().map(|(n, _)| n).collect();

    for def in produced {
        for h in def.deps() {
            if !cdb.contains_hash(&h)? {
                hallucinated.push(format!("ref:{h}"));
            }
        }
        for v in def.expr.free_vars() {
            if !known_names.contains(&v) {
                hallucinated.push(format!("name:{v}"));
            }
        }
    }
    hallucinated.sort();
    hallucinated.dedup();

    // "Compiled" for the prototype = no dangling references. The real
    // typecheck replaces this predicate when the compiler comes online;
    // the interface stays fixed.
    let compiled = hallucinated.is_empty();

    let mut forbidden_hit = Vec::new();
    if task
        .grade
        .forbidden
        .iter()
        .any(|f| f == "hallucinated-symbol")
        && !hallucinated.is_empty()
    {
        forbidden_hit.push("hallucinated-symbol".to_string());
    }

    // Tests/contracts: executable once the compiler lands. Until then a
    // task with test oracles reports 0/N — visibly ungraded, never
    // silently passed (docs/benchmark-harness.md §7: no silent truncation).
    let tests_total = task.grade.tests.len() as u32;
    let contracts_total = task.grade.contracts.len() as u32;
    let tests_passed = (0, tests_total);
    let contracts_held = (0, contracts_total);

    let pass = (!task.grade.compile || compiled)
        && tests_passed.0 == tests_total
        && contracts_held.0 == contracts_total
        && forbidden_hit.is_empty();

    Ok(GradeResult {
        task_id: task.id.clone(),
        compiled,
        tests_passed,
        contracts_held,
        forbidden_hit,
        hallucinated_symbols: hallucinated,
        pass,
        retries_used,
        tokens,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use claw_core::{Expr, Hash, Lit, Type};

    fn named(n: &str) -> Type {
        Type::Named(n.into())
    }

    fn simple_task(forbidden: Vec<String>) -> Task {
        Task {
            id: "t-001".into(),
            category: Category::FromScratch,
            prompt: "produce a Nat".into(),
            grade: GradeSpec {
                compile: true,
                tests: vec![],
                contracts: vec![],
                forbidden,
            },
            reference: None,
        }
    }

    #[test]
    fn clean_production_passes() {
        let mut cdb = Cdb::in_memory().unwrap();
        let dep = Def::new(Expr::Lit(Lit::Int(1)), named("Nat"));
        let dep_h = cdb.put(&dep).unwrap();
        cdb.bind("one", &dep_h).unwrap();

        let produced = Def::new(
            Expr::App {
                func: Box::new(Expr::Ref(dep_h)),
                args: vec![],
            },
            named("Nat"),
        );
        let r = grade(&simple_task(vec![]), &[produced], &cdb, 0, 100).unwrap();
        assert!(r.compiled);
        assert!(r.hallucinated_symbols.is_empty());
        assert!(r.pass);
    }

    #[test]
    fn dangling_ref_is_hallucination_and_fails() {
        let cdb = Cdb::in_memory().unwrap();
        let ghost = Hash("ab".repeat(32));
        let produced = Def::new(
            Expr::App {
                func: Box::new(Expr::Ref(ghost)),
                args: vec![],
            },
            named("Nat"),
        );
        let r = grade(
            &simple_task(vec!["hallucinated-symbol".into()]),
            &[produced],
            &cdb,
            2,
            500,
        )
        .unwrap();
        assert!(!r.compiled);
        assert_eq!(r.hallucinated_symbols.len(), 1);
        assert!(r.hallucinated_symbols[0].starts_with("ref:"));
        assert_eq!(r.forbidden_hit, vec!["hallucinated-symbol"]);
        assert!(!r.pass);
    }

    #[test]
    fn unresolved_free_name_is_hallucination() {
        // the `generate_nonce()` case: a name nothing binds
        let cdb = Cdb::in_memory().unwrap();
        let produced = Def::new(
            Expr::App {
                func: Box::new(Expr::Var("generate_nonce".into())),
                args: vec![],
            },
            named("Bytes"),
        );
        let r = grade(&simple_task(vec![]), &[produced], &cdb, 0, 50).unwrap();
        assert_eq!(r.hallucinated_symbols, vec!["name:generate_nonce"]);
        assert!(!r.pass);
    }

    #[test]
    fn ungraded_tests_never_silently_pass() {
        let mut task = simple_task(vec![]);
        task.grade.tests = vec!["tests/spec.claw".into()];
        let cdb = Cdb::in_memory().unwrap();
        let produced = Def::new(Expr::Lit(Lit::Int(9)), named("Nat"));
        let r = grade(&task, &[produced], &cdb, 0, 10).unwrap();
        assert_eq!(r.tests_passed, (0, 1));
        assert!(!r.pass, "tasks with unexecuted oracles must not pass");
    }

    #[test]
    fn task_schema_roundtrips_from_json() {
        let json = r#"{
            "id": "wallet-transfer-001",
            "category": "repo-feature",
            "prompt": "Implement transfer respecting the Ledger invariant.",
            "grade": {
                "compile": true,
                "tests": ["tests/transfer_spec.claw"],
                "contracts": ["from'.balance == from.balance - amt"],
                "forbidden": ["unsafe", "hallucinated-symbol"]
            },
            "reference": "solutions/wallet-transfer-001.claw"
        }"#;
        let t: Task = serde_json::from_str(json).unwrap();
        assert_eq!(t.id, "wallet-transfer-001");
        assert!(matches!(t.category, Category::RepoFeature));
        assert_eq!(t.grade.contracts.len(), 1);
    }
}

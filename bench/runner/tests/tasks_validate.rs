//! Every task file in bench/tasks must parse against the schema and its
//! scope must build into a CDB. A task that drifts from the schema fails
//! CI here — loudly, not at run time.

use achuk_bench_grader::Task;
use std::path::PathBuf;

fn tasks_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../tasks")
}

#[test]
fn all_task_files_parse_and_scopes_build() {
    let dir = tasks_dir();
    let mut count = 0;
    for entry in std::fs::read_dir(&dir).expect("bench/tasks must exist") {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let raw = std::fs::read_to_string(&path).unwrap();
        let task: Task = serde_json::from_str(&raw)
            .unwrap_or_else(|e| panic!("{} does not match schema: {e}", path.display()));
        let cdb = task
            .build_scope_cdb()
            .unwrap_or_else(|e| panic!("{}: scope failed to build: {e}", path.display()));
        assert_eq!(
            cdb.symbols().unwrap().len(),
            task.scope.len(),
            "{}: every scope entry must produce a distinct bound symbol",
            path.display()
        );
        count += 1;
    }
    assert!(count >= 15, "expected at least 15 tasks, found {count}");
}

#[test]
fn task_ids_are_unique() {
    let dir = tasks_dir();
    let mut ids = std::collections::BTreeSet::new();
    for entry in std::fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let task: Task = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(ids.insert(task.id.clone()), "duplicate task id {}", task.id);
    }
}

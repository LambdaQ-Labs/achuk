//! claw-bench — benchmark runner CLI (WS-J).
//!
//! Usage:
//!   claw-bench run --arm A0 --tasks bench/tasks [--retries 3] [--json out.json]
//!
//! Model endpoint via env: CLAW_MODEL_URL, CLAW_MODEL_NAME, CLAW_MODEL_KEY.

use claw_bench_grader::Task;
use claw_bench_runner::{aggregate, run_task, Arm, HttpGenerator, RunConfig};
use std::path::PathBuf;

fn main() {
    if let Err(e) = real_main() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn real_main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) != Some("run") {
        anyhow::bail!(
            "usage: claw-bench run --arm A0|A1 --tasks <dir> [--retries N] [--json <out>]"
        );
    }

    let mut arm = Arm::A0;
    let mut tasks_dir = PathBuf::from("bench/tasks");
    let mut retries: u32 = 3;
    let mut json_out: Option<PathBuf> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--arm" => {
                arm = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--arm needs a value"))?
                    .parse()?;
                i += 2;
            }
            "--tasks" => {
                tasks_dir = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--tasks needs a value"))?
                    .into();
                i += 2;
            }
            "--retries" => {
                retries = args
                    .get(i + 1)
                    .ok_or_else(|| anyhow::anyhow!("--retries needs a value"))?
                    .parse()?;
                i += 2;
            }
            "--json" => {
                json_out = Some(
                    args.get(i + 1)
                        .ok_or_else(|| anyhow::anyhow!("--json needs a value"))?
                        .into(),
                );
                i += 2;
            }
            other => anyhow::bail!("unknown flag `{other}`"),
        }
    }

    // Load tasks
    let mut tasks: Vec<Task> = Vec::new();
    for entry in std::fs::read_dir(&tasks_dir)? {
        let path = entry?.path();
        if path.extension().and_then(|e| e.to_str()) == Some("json") {
            let raw = std::fs::read_to_string(&path)?;
            match serde_json::from_str::<Task>(&raw) {
                Ok(t) => tasks.push(t),
                Err(e) => eprintln!("skipping {}: {e}", path.display()), // loud, not silent
            }
        }
    }
    tasks.sort_by(|a, b| a.id.cmp(&b.id));
    anyhow::ensure!(
        !tasks.is_empty(),
        "no tasks found in {}",
        tasks_dir.display()
    );
    eprintln!(
        "running {} task(s), arm {:?}, retries {}",
        tasks.len(),
        arm,
        retries
    );

    let cfg = RunConfig {
        arm,
        max_retries: retries,
    };
    let mut results = Vec::new();
    let mut errored: Vec<(String, String)> = Vec::new();

    for task in &tasks {
        // fresh generator per task: no cross-task context bleed
        let mut generator = HttpGenerator::from_env()?;
        match run_task(task, &cfg, &mut generator) {
            Ok(r) => {
                eprintln!("  {} compiled={} pass={}", task.id, r.compiled, r.pass);
                results.push(r);
            }
            Err(e) => {
                eprintln!("  {} ERROR: {e}", task.id);
                errored.push((task.id.clone(), e.to_string()));
            }
        }
    }

    let report = aggregate(arm, results);
    println!("{}", report.render_table());
    if !errored.is_empty() {
        // no silent truncation: errored tasks are reported, not dropped
        println!("{} task(s) errored (not graded):", errored.len());
        for (id, e) in &errored {
            println!("  {id}: {e}");
        }
    }
    if let Some(path) = json_out {
        std::fs::write(&path, serde_json::to_string_pretty(&report)?)?;
        eprintln!("report written to {}", path.display());
    }
    Ok(())
}

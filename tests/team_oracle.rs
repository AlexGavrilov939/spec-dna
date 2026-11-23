//! The self-test of the tasks of the model-team measurement (ADR 0029).
//!
//! `bench/team/selftest.py` runs the real runner with a stub `claude`, with no
//! model and no cost. For each task: the start state fails the hidden tests, the
//! reference and a second correct solution pass each check, a wrong solution
//! passes the visible tests and fails the hidden ones, and a stub that hunts for
//! the hidden tests finds none.

use std::path::PathBuf;
use std::process::Command;

fn python3_exists() -> bool {
    Command::new("python3")
        .arg("--version")
        .output()
        .is_ok_and(|o| o.status.success())
}

#[test]
fn team_tasks_pass_their_self_test() {
    if !python3_exists() {
        eprintln!("python3 is absent: the self-test of the team tasks is skipped");
        return;
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out = Command::new("python3")
        .args(["-I", "bench/team/selftest.py"])
        .current_dir(&root)
        .env("SPEC_DNA_BIN", env!("CARGO_BIN_EXE_spec-dna"))
        .output()
        .expect("python3 starts");
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(
        out.status.success(),
        "self-test failed:\n{text}\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // 12 tasks, 6 modes each.
    assert_eq!(
        text.matches("\nok   ").count() + usize::from(text.starts_with("ok   ")),
        72
    );
    assert!(text.contains("selftest: passed"));
}

#[test]
fn each_task_has_the_files_of_the_form() {
    let tasks = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bench/team/tasks");
    let mut count = 0;
    for entry in std::fs::read_dir(&tasks).expect("tasks folder") {
        let dir = entry.expect("entry").path();
        for name in [
            "task.txt",
            "meta.json",
            "fixture",
            "oracle",
            "reference",
            "wrong",
            "alt",
        ] {
            assert!(dir.join(name).exists(), "{} lacks {name}", dir.display());
        }
        count += 1;
    }
    assert_eq!(count, 12);
}

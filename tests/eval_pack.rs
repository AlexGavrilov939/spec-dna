//! Tests of `spec-dna eval pack` (ADR 0016, 0.7.0 slice 5). The command
//! writes the shipped evals and their small fixtures to `<[evals] dir>/pack/`.
//! A test that needs the whole pack of 55 evals is ignored until the evals of
//! the slice (tasks t12 to t17) are in the binary. The other tests read the
//! files that the binary writes, whatever their number, and check the rules
//! that hold for each of them. No test starts an agent.

mod common;
#[path = "eval_pack_pilot/mod.rs"]
mod pilot;

use common::{Repo, VERSION};
use std::fs;
use std::path::Path;

const SKILLS: [&str; 11] = [
    "brainstorm",
    "specify",
    "clarify",
    "write-plan",
    "execute",
    "tdd",
    "review",
    "verify",
    "finish",
    "debug",
    "post-mortem",
];

fn project(evals: &str) -> Repo {
    let r = Repo::with_config(&format!("\n{evals}\n[[roots]]\npath = \"docs\"\n"));
    r.write("docs/a.md", "# A\n\nText.\n");
    r.write(
        "evals/check.sh",
        "file_has() {\n    grep -q -F -- \"$2\" \"$1\"\n}\n",
    );
    r.commit("start");
    r
}

fn pack(r: &Repo) -> (i32, String, String) {
    r.run(&["eval", "pack"])
}

/// The names of the files `<dir>/pack/*.json`, sorted.
fn packed(r: &Repo) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(r.dir.join("evals/pack"))
        .map(|d| {
            d.map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
                .filter(|n| n.ends_with(".json"))
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// The first file under a folder, in the order of the names.
fn first_file(dir: &Path) -> std::path::PathBuf {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    let first = entries.remove(0);
    if first.is_dir() {
        first_file(&first)
    } else {
        first
    }
}

fn files_under(dir: &Path) -> usize {
    let mut n = 0;
    if let Ok(read) = fs::read_dir(dir) {
        for e in read {
            let p = e.unwrap().path();
            n += if p.is_dir() { files_under(&p) } else { 1 };
        }
    }
    n
}

#[test]
fn eval_pack_writes_the_evals_and_their_fixtures_below_the_pack_folder() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let names = packed(&r);
    assert!(!names.is_empty(), "the binary holds no eval: {out}");
    for name in &names {
        let id = name.strip_suffix(".json").unwrap();
        let text = r.read(&format!("evals/pack/{name}"));
        assert!(text.contains(&format!("\"id\": \"{id}\"")), "{name}");
        // Each file has the key `skill`, and the id is `<skill>-<n>`.
        let skill = id.rsplit_once('-').unwrap().0;
        assert!(SKILLS.contains(&skill), "{id}");
        assert!(text.contains(&format!("\"skill\": \"{skill}\"")), "{name}");
        assert!(
            text.contains("\"runs\": 4"),
            "{name}: the pack runs 4 times"
        );
        assert!(
            text.contains(&format!("\"fixture\": \"evals/pack/fixtures/{id}\"")),
            "{name}"
        );
        assert!(
            files_under(&r.dir.join(format!("evals/pack/fixtures/{id}"))) > 0,
            "{id}: no fixture file"
        );
    }
    assert!(
        out.contains(&format!("wrote evals/pack/{}", names[0])),
        "{out}"
    );
    assert!(out.contains("eval pack: "), "{out}");
}

#[test]
fn each_file_of_the_pack_is_valid_for_the_runner() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let skills: Vec<String> = packed(&r)
        .iter()
        .map(|n| n.rsplit_once('-').unwrap().0.to_string())
        .collect();
    assert!(!skills.is_empty());
    for skill in skills {
        // A dry run reads each file with the rules of SD506.
        let (code, out, err) = r.run(&[
            "eval",
            "run",
            "--ab",
            &skill,
            "--dry-run",
            "--max-total-usd",
            "1000",
        ]);
        assert_eq!(code, 0, "{skill}: {out}{err}");
    }
}

#[test]
fn the_evals_of_the_pack_are_not_read_by_eval_run_and_sd506() {
    let r = project("[evals]\n");
    let (code, ..) = pack(&r);
    assert_eq!(code, 0);
    r.write(
        "evals/example.json",
        "{\"id\":\"example\",\"task\":\"x\",\"fixture\":\"evals/pack/fixtures/tdd-1\",\"max-budget-usd\":1,\"checks\":[{\"name\":\"a\",\"run\":\"true\"}]}\n",
    );
    r.commit("pack");
    let (_, out, err) = r.run(&["eval", "run", "--dry-run"]);
    assert!(
        out.contains("1 evals"),
        "only example.json is an eval: {out}{err}"
    );
    let (_, out, _) = r.run(&["check", "--all"]);
    assert!(!out.contains("SD506"), "{out}");
}

#[test]
fn eval_pack_keeps_a_file_that_exists_and_names_it() {
    let r = project("[evals]\n");
    pack(&r);
    let name = packed(&r)[0].clone();
    let path = format!("evals/pack/{name}");
    r.write(&path, "{\"my\":\"edit\"}\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(r.read(&path), "{\"my\":\"edit\"}\n", "the file stays");
    assert!(out.contains(&format!("kept {path}")), "{out}");
    // A file that is absent is written again.
    fs::remove_file(r.dir.join(&path)).unwrap();
    let (code, out, _) = pack(&r);
    assert_eq!(code, 0);
    assert!(out.contains(&format!("wrote {path}")), "{out}");
    // A kept fixture file stays too.
    let id = name.strip_suffix(".json").unwrap();
    let fixture = first_file(&r.dir.join(format!("evals/pack/fixtures/{id}")));
    fs::write(&fixture, "mine\n").unwrap();
    let (code, ..) = pack(&r);
    assert_eq!(code, 0);
    assert_eq!(fs::read_to_string(&fixture).unwrap(), "mine\n");
}

#[test]
fn eval_pack_json_lists_the_written_and_the_kept_files() {
    let r = project("[evals]\n");
    let (code, out, err) = r.run(&["eval", "pack", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.starts_with("{\"written\":["), "{out}");
    assert!(out.contains("evals/pack/"), "{out}");
    assert!(out.contains("\"kept\":[]"), "{out}");
    let (code, out, _) = r.run(&["eval", "pack", "--json"]);
    assert_eq!(code, 0);
    assert!(out.contains("\"written\":[]"), "{out}");
}

#[test]
fn eval_pack_needs_the_evals_table_and_takes_no_argument() {
    let r = project("");
    let (code, _, err) = pack(&r);
    assert_eq!(code, 3, "{err}");
    assert!(err.contains("[evals]"), "{err}");
    assert!(!r.dir.join("evals/pack").exists());
    let r = project("[evals]\n");
    let (code, _, err) = r.run(&["eval", "pack", "tdd"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn eval_pack_uses_the_folder_of_the_table_and_the_fixture_path_follows_it() {
    let r = Repo::with_config("\n[evals]\ndir = \"qa/evals\"\n\n[[roots]]\npath = \"docs\"\n");
    r.write("docs/a.md", "# A\n\nText.\n");
    r.write("qa/evals/check.sh", "true() { :; }\n");
    r.commit("start");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let any = fs::read_dir(r.dir.join("qa/evals/pack"))
        .unwrap()
        .filter_map(Result::ok)
        .find(|e| e.file_name().to_string_lossy().ends_with(".json"))
        .unwrap();
    let text = fs::read_to_string(any.path()).unwrap();
    assert!(
        text.contains("\"fixture\": \"qa/evals/pack/fixtures/"),
        "{text}"
    );
}

#[test]
fn a_check_that_runs_spec_dna_check_names_the_codes_that_it_reads() {
    // The arms differ in the generated files, so the exit code of `check`
    // alone would differ with the arm. A check reads the codes.
    let r = project("[evals]\n");
    pack(&r);
    assert!(!packed(&r).is_empty(), "the binary holds no eval");
    for name in packed(&r) {
        let text = r.read(&format!("evals/pack/{name}"));
        for line in text.lines().filter(|l| l.contains("spec-dna check")) {
            assert!(line.contains("SD"), "{name}: a check names no code: {line}");
        }
    }
}

#[test]
fn the_fixture_names_no_consumer_project_and_pins_the_version_of_the_binary() {
    let r = project("[evals]\n");
    pack(&r);
    assert!(!packed(&r).is_empty(), "the binary holds no eval");
    for name in packed(&r) {
        let id = name.strip_suffix(".json").unwrap();
        let toml = r
            .dir
            .join(format!("evals/pack/fixtures/{id}/spec-dna.toml"));
        let text = fs::read_to_string(toml).unwrap_or_else(|e| panic!("{id}: {e}"));
        assert!(
            text.starts_with(&format!("spec-dna = \"{VERSION}\"")),
            "{id}: {text}"
        );
    }
}

// The tests that need the 55 evals of the pack.

#[test]
#[ignore = "part B: t12-t17"]
fn the_pack_has_5_evals_for_each_of_the_11_skills() {
    let r = project("[evals]\n");
    let (code, out, err) = pack(&r);
    assert_eq!(code, 0, "{out}{err}");
    let names = packed(&r);
    assert_eq!(names.len(), 55, "{names:?}");
    for skill in SKILLS {
        for n in 1..=5 {
            assert!(names.contains(&format!("{skill}-{n}.json")), "{skill}-{n}");
        }
    }
    // Each eval is valid for SD506 and for the runner.
    for skill in SKILLS {
        let (code, out, err) = r.run(&[
            "eval",
            "run",
            "--ab",
            skill,
            "--dry-run",
            "--max-total-usd",
            "1000",
        ]);
        assert_eq!(code, 0, "{skill}: {out}{err}");
        assert!(out.contains("5 evals, 40 runs"), "{skill}: {out}");
    }
}

#[test]
#[ignore = "part B: t12-t17"]
fn a_dry_run_of_a_skill_and_of_the_agents_line_each_print_40_runs_and_an_estimate() {
    let r = project("[evals]\n");
    pack(&r);
    for target in ["execute", "agents-line"] {
        let (code, out, err) = r.run(&[
            "eval",
            "run",
            "--ab",
            target,
            "--dry-run",
            "--max-total-usd",
            "1000",
        ]);
        assert_eq!(code, 0, "{target}: {out}{err}");
        assert!(
            out.contains(&format!(
                "eval run --ab {target} (dry run): 5 evals, 40 runs, estimate "
            )),
            "{target}: {out}"
        );
    }
    // The estimate of the full run is about 440 runs for 55 evals.
    let (code, out, _) = r.run(&[
        "eval",
        "run",
        "--ab",
        "all",
        "--dry-run",
        "--max-total-usd",
        "1000",
    ]);
    assert_eq!(code, 0);
    assert!(out.contains("55 evals, 440 runs"), "{out}");
}

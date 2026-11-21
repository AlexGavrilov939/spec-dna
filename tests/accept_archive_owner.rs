//! The archive file of a change (design 0.8.0, D3 item 3) belongs to the change
//! that its field `change` names. The name of the file decides nothing:
//! `fix-2.toml` is the archive of the change `fix-2`, and it is also the file 2
//! of the change `fix`. `check` looks the archive up by the field. The fixtures
//! use example names only.

mod common;

use common::*;

const REQ: &str = "docs/requirements/valves.md";
const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **overflow**: When a bed overflows, the controller SHALL close the valve of that bed.\n";

fn config() -> String {
    "profile = \"standard\"\n\n[change]\npolicy = \"three-class\"\nacceptance-for = \"all\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n".to_string()
}

fn folder(slug: &str) -> String {
    format!("docs/intent/{slug}")
}

fn archive(slug: &str) -> String {
    format!(".spec-dna/accepted/{slug}.toml")
}

/// The files of one change that has the code file `code`.
fn change_files(r: &Repo, slug: &str, key: &str, code: &str) {
    let dir = folder(slug);
    r.write(
        &format!("{dir}/intent.md"),
        "---\nstatus: accepted\nclass: routine\n---\n# A fix\n\n## Intent\n\nClose the valve.\n\n## Why now\n\nA valve stayed open.\n\n## Out of scope\n\n- An alarm.\n",
    );
    r.write(
        &format!("{dir}/spec.md"),
        &format!("---\nstatus: approved\n---\n# A fix\n\n## Delta\n\n- ADD requirements/valves#{key}: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n## Concerns\n\n- [x] A slow sensor. Resolution: 60 seconds.\n\n## Open questions\n\n- [x] Does the valve reopen? Answer: no.\n"),
    );
    r.write(
        &format!("{dir}/plan.md"),
        &format!("---\nstatus: approved\n---\n# A fix\n\n## Files\n\n- `{code}`\n\n## Order\n\n- [x] t1: Close the valve. Files: `{code}`. Verify: the test passes.\n\n## Risks\n\n- A slow sensor.\n\n## Proof\n\nThe test covers `requirements/valves#{key}`.\n"),
    );
}

/// The branch adds the changes `fix` and `fix-2`, with a skeleton record each.
fn built() -> Repo {
    let r = Repo::with_config(&config());
    r.write(REQ, VALVES);
    r.write("src/a.py", "a = 1\n");
    r.write("src/b.py", "b = 1\n");
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    change_files(&r, "fix", "key-a", "src/a.py");
    change_files(&r, "fix-2", "key-b", "src/b.py");
    r.commit("plans");
    r.write("src/a.py", "a = 2\n");
    r.write("src/b.py", "b = 2\n");
    r.commit("code");
    for slug in ["fix", "fix-2"] {
        let (code, out, err) = r.run(&["accept", slug, "--session", "s1"]);
        assert_eq!(code, 0, "{out}{err}");
    }
    r.commit("records");
    r
}

/// `finish` by hand: the folder is deleted, and the archive holds the record.
fn finish_by_hand(r: &Repo, slug: &str, with_archive: bool) {
    let record = r.read(&format!("{}/acceptance.toml", folder(slug)));
    r.git(&["rm", "-q", "-r", &folder(slug)]);
    if with_archive {
        r.write(&archive(slug), &record);
    }
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| l.contains(&format!(" {code} ")))
}

#[test]
fn two_changes_finished_in_one_diff_have_each_its_own_archive_file() {
    let r = built();
    finish_by_hand(&r, "fix", true);
    finish_by_hand(&r, "fix-2", true);
    r.commit("finish both");
    let (_, out, err) = r.run(&["check"]);
    // The archive `fix-2.toml` is the archive of `fix-2`, and `fix` has `fix.toml`.
    assert!(!has(&out, "SD935"), "{out}{err}");
    assert!(!has(&out, "SD927"), "{out}{err}");
}

#[test]
fn a_change_deleted_with_no_finish_is_not_hidden_by_the_archive_of_another() {
    // `fix` is deleted with no `finish`; `fix-2` has its archive `fix-2.toml`.
    let r = built();
    finish_by_hand(&r, "fix", false);
    finish_by_hand(&r, "fix-2", true);
    r.commit("finish one");
    let (_, out, err) = r.run(&["check"]);
    assert!(has(&out, "SD927"), "{out}{err}");
    assert!(!has(&out, "SD935"), "{out}{err}");
    // The other order: `fix-2` is deleted with no `finish`, and `fix` has its archive.
    let r = built();
    finish_by_hand(&r, "fix", true);
    finish_by_hand(&r, "fix-2", false);
    r.commit("finish one");
    let (_, out, err) = r.run(&["check"]);
    assert!(has(&out, "SD927"), "{out}{err}");
    assert!(!has(&out, "SD935"), "{out}{err}");
}

//! Integration tests of `spec-dna accept <slug>` and `accept <slug> --brief`
//! (design 0.8.0, D3 item 5): the skeleton of the acceptance record, the brief,
//! the refusals and the exit codes. The rules of a valid record are unit tests
//! of `src/record.rs`, because the crate is a binary.
//!
//! Each fixture is a pull request: the base commit is on `main` with
//! `origin/HEAD`, and the branch `feature` adds the folder of the change and its
//! code. The fixtures use example names only.

mod common;

use common::*;
use sha2::{Digest, Sha256};

const SLUG: &str = "valve-timeout";
const INTENT: &str = "docs/intent/valve-timeout/intent.md";
const SPEC: &str = "docs/intent/valve-timeout/spec.md";
const PLAN: &str = "docs/intent/valve-timeout/plan.md";
const RECORD: &str = "docs/intent/valve-timeout/acceptance.toml";
const CODE: &str = "src/valves.py";
const TEST: &str = "tests/test_valves.py";
const REQ: &str = "docs/requirements/valves.md";

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **overflow**: When a bed overflows, the controller SHALL close the valve of that bed.\n";
const CODE_OLD: &str = "def close_valve(bed):\n    bed.valve_open = False\n";
const CODE_NEW: &str = "def close_valve(bed):\n    bed.valve_open = False\n\n\ndef on_sensor_timeout(bed):\n    close_valve(bed)\n";
const TEST_OLD: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n";
const TEST_NEW: &str = "# covers: requirements/valves#overflow\ndef test_overflow():\n    ...\n\n\n# covers: requirements/valves#sensor-timeout\ndef test_sensor_timeout():\n    ...\n";

const INTENT_TEXT: &str = "---\nstatus: accepted\nclass: routine\n---\n# Valve timeout\n\n## Intent\n\nClose the valve when a sensor times out.\n\n## Why now\n\nA valve stayed open.\n\n## Out of scope\n\n- An alarm.\n";
const SPEC_TEXT: &str = "---\nstatus: approved\n---\n# Valve timeout\n\n## Delta\n\n- ADD requirements/valves#sensor-timeout: When a sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed.\n- ADD requirements/valves#sensor-alarm: When a sensor gives no value for 60 seconds, the controller SHALL raise an alarm.\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n## Concerns\n\n- [x] A slow sensor. Resolution: 60 seconds.\n\n## Open questions\n\n- [x] Does the valve reopen? Answer: no.\n";
const PLAN_TEXT: &str = "---\nstatus: approved\n---\n# Valve timeout\n\n## Files\n\n- `src/valves.py`\n- `tests/test_valves.py`\n\n## Order\n\n- [x] t1: Close the valve. Files: `src/valves.py`. Verify: the test passes.\n\n## Risks\n\n- A slow sensor.\n\n## Proof\n\nThe test covers `requirements/valves#sensor-timeout`.\n";

fn config(profile: &str, change: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n"
    )
}

/// The base commit on `main`, `origin/HEAD` on it, and the branch `feature`.
fn start(config_text: &str) -> Repo {
    let r = Repo::with_config(config_text);
    r.write(".gitignore", "reports/\n");
    r.write(REQ, VALVES);
    r.write(CODE, CODE_OLD);
    r.write(TEST, TEST_OLD);
    r.write("src/other.py", "y = 1\n");
    r.write("README.md", "# Project\n");
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

/// The pull request of a change: the plan in one commit, the code in the next.
/// The tree is clean.
fn built_with(config_text: &str) -> Repo {
    let r = start(config_text);
    r.write(INTENT, INTENT_TEXT);
    r.write(SPEC, SPEC_TEXT);
    r.write(PLAN, PLAN_TEXT);
    r.commit("plan");
    r.write(CODE, CODE_NEW);
    r.write(TEST, TEST_NEW);
    r.commit("code");
    r
}

fn built() -> Repo {
    built_with(&config("standard", ""))
}

fn accept(r: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut a = vec!["accept"];
    a.extend_from_slice(args);
    r.run(&a)
}

/// The brief of the change, with the exit code 0.
fn brief(r: &Repo, extra: &[&str]) -> String {
    let mut a = vec![SLUG, "--brief"];
    a.extend_from_slice(extra);
    let (code, out, err) = accept(r, &a);
    assert_eq!(code, 0, "{out}{err}");
    out
}

fn last_line(text: &str) -> &str {
    text.lines().last().unwrap()
}

/// The first 8 bytes of SHA-256 as 16 hexadecimal digits.
fn sha16(bytes: &[u8]) -> String {
    Sha256::digest(bytes)[..8]
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A top-level value `key = "value"` of the record text.
fn field(text: &str, key: &str) -> String {
    let line = text
        .lines()
        .find(|l| l.starts_with(&format!("{key} = ")))
        .unwrap_or_else(|| panic!("no `{key}` in\n{text}"));
    line.split_once(" = ")
        .unwrap()
        .1
        .trim_matches('"')
        .to_string()
}

fn write_record(r: &Repo) -> String {
    let (code, out, err) = accept(r, &[SLUG, "--session", "s1"]);
    assert_eq!(code, 0, "{out}{err}");
    r.read(RECORD)
}

#[test]
fn the_brief_twice_on_one_tree_prints_the_same_bytes() {
    let r = built();
    assert_eq!(brief(&r, &[]), brief(&r, &[]));
}

#[test]
fn the_brief_has_the_first_line_the_files_whole_the_fields_and_its_own_hash() {
    let r = built();
    let b = brief(&r, &[]);
    assert!(
        b.starts_with(&format!("spec-dna-brief 1 {SLUG} acceptor\n")),
        "{b}"
    );
    // The fixed text of the format, about 3 KB, comes before the files.
    let fixed_end = b.find("=== intent.md ===").unwrap();
    assert!(
        (2000..=4500).contains(&fixed_end),
        "the fixed text has {fixed_end} bytes"
    );
    // The 3 files whole, in this order.
    let at = |s: &str| b.find(s).unwrap_or_else(|| panic!("no {s}"));
    assert!(at("=== intent.md ===") < at("=== spec.md ==="));
    assert!(at("=== spec.md ===") < at("=== plan.md ==="));
    for (name, text) in [
        ("intent.md", INTENT_TEXT),
        ("spec.md", SPEC_TEXT),
        ("plan.md", PLAN_TEXT),
    ] {
        assert!(b.contains(&format!("=== {name} ===\n{text}")), "{name}");
    }
    // scope, paths and tree as the record writes them.
    assert!(
        b.contains("=== scope ===\n- src/valves.py\n- tests/test_valves.py\n=== paths ===\n- src/valves.py\n- tests/test_valves.py\n=== tree ===\n"),
        "{b}"
    );
    // No invariants, no deviations and no owner roles yet.
    for name in ["invariants", "deviations", "roles"] {
        assert!(!b.contains(&format!("=== {name} ===")), "{name}");
    }
    // The last line is the hash of the text before it, computed again here.
    let last = last_line(&b);
    let at_last = b.rfind(last).unwrap();
    assert_eq!(last, format!("brief {}", sha16(&b.as_bytes()[..at_last])));
    assert!(b.ends_with('\n') && !b.ends_with("\n\n"));
    // The same tree gives the tree of the record.
    let rec = write_record(&r);
    let tree = field(&rec, "tree");
    assert!(b.contains(&format!("=== tree ===\n{tree}\n")));
}

#[test]
fn an_edit_of_the_plan_changes_the_last_line() {
    let r = built();
    let before = brief(&r, &[]);
    r.write(
        PLAN,
        &PLAN_TEXT.replace("A slow sensor.", "A slow sensor, again."),
    );
    r.commit("plan edit");
    let after = brief(&r, &[]);
    assert_ne!(last_line(&before), last_line(&after));
    assert!(after.contains("A slow sensor, again."));
    // An edit of the spacing counts too: the hash covers the bytes.
    r.write(
        PLAN,
        &format!(
            "{}\n",
            PLAN_TEXT.replace("A slow sensor.", "A slow sensor, again.")
        ),
    );
    r.commit("trailing blank line");
    assert_ne!(last_line(&after), last_line(&brief(&r, &[])));
}

#[test]
fn the_role_changes_the_first_line_and_the_hash() {
    let r = built();
    let plain = brief(&r, &[]);
    let role = brief(&r, &["--role", "security-reviewer"]);
    assert!(
        role.starts_with(&format!("spec-dna-brief 1 {SLUG} security-reviewer\n")),
        "{role}"
    );
    assert_ne!(last_line(&plain), last_line(&role));
    // The rest of the text is the same.
    let rest = |t: &str| {
        t.split_once('\n')
            .unwrap()
            .1
            .rsplit_once("brief ")
            .unwrap()
            .0
            .to_string()
    };
    assert_eq!(rest(&plain), rest(&role));
    // The role `acceptor` is the role that `--role` leaves out.
    assert_eq!(brief(&r, &["--role", "acceptor"]), plain);
}

#[test]
fn the_brief_reads_the_committed_files() {
    let r = built();
    let committed = brief(&r, &[]);
    // A file with an edit that HEAD does not have is a refusal, not another text.
    r.write(PLAN, &format!("{PLAN_TEXT}\nMore.\n"));
    let (code, out, err) = accept(&r, &[SLUG, "--brief"]);
    assert_eq!(code, 1, "{out}{err}");
    r.git(&["checkout", "--", PLAN]);
    assert_eq!(brief(&r, &[]), committed);
}

#[test]
fn the_skeleton_lists_each_requirement_with_its_proof_state_and_pending() {
    let r = built();
    let (code, out, err) = accept(&r, &[SLUG, "--session", "6c0e2f1a"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(&format!("wrote {RECORD}")), "{out}");
    assert!(out.lines().any(|l| l == "acceptor: 1"), "{out}");
    let t = r.read(RECORD);
    assert_eq!(field(&t, "format"), "1");
    assert_eq!(field(&t, "change"), SLUG);
    assert!(t.contains("scope = [\"src/valves.py\", \"tests/test_valves.py\"]\n"));
    assert!(t.contains("paths = [\"src/valves.py\", \"tests/test_valves.py\"]\n"));
    assert_eq!(field(&t, "tree").len(), 16);
    assert_eq!(field(&t, "result"), "pending");
    assert!(t.contains("acknowledged-by = []\n"));
    // One acceptor entry with `pending` and the claims.
    assert!(t.contains("[[acceptor]]\nrole = \"acceptor\"\nbrief = \"pending\"\nsession = \"6c0e2f1a\"\nagent = \"unknown\"\nresult = \"pending\"\n"), "{t}");
    // One proof for each requirement of the delta. The tag proves one of them
    // in no report, so its state is `tagged`; the other has no tag.
    assert!(t.contains("[[proof]]\nrequirement = \"requirements/valves#sensor-alarm\"\nstate = \"none\"\ncases = []\n"), "{t}");
    assert!(t.contains("[[proof]]\nrequirement = \"requirements/valves#sensor-timeout\"\nstate = \"tagged\"\ncases = []\n"), "{t}");
    assert_eq!(t.matches("[[proof]]").count(), 2);
    // The folder is the only place that changed, and nothing is committed.
    assert_eq!(
        r.git_out(&["status", "--porcelain"]),
        format!("?? {RECORD}")
    );
    assert!(r.git_out(&["log", "-1", "--format=%s"]) == "code");
}

#[test]
fn the_tree_of_the_record_is_the_hash_of_the_listed_paths() {
    let r = built();
    let t = write_record(&r);
    let oid = |p: &str| r.git_out(&["rev-parse", &format!("HEAD:{p}")]);
    let text = format!(
        "spec-dna-tree 1\n100644 {}\t{CODE}\n100644 {}\t{TEST}\n",
        oid(CODE),
        oid(TEST)
    );
    assert_eq!(field(&t, "tree"), sha16(text.as_bytes()));
}

#[test]
fn a_proof_with_a_fresh_report_is_passed_once_and_names_its_case() {
    let evidence = "[evidence]\nreports = [\"reports/*\"]\n";
    let r = built_with(&format!("{}\n{evidence}", config("standard", "")));
    // The config keys of one table: write `[evidence]` before `[[roots]]` is not
    // needed, because a table can follow the arrays of tables.
    r.write(
        "reports/junit.xml",
        "<?xml version=\"1.0\" encoding=\"utf-8\"?><testsuites><testsuite name=\"pytest\" errors=\"0\" failures=\"0\" skipped=\"0\" tests=\"2\" time=\"0.1\"><testcase classname=\"tests.test_valves\" name=\"test_overflow\" time=\"0.01\"/><testcase classname=\"tests.test_valves\" name=\"test_sensor_timeout\" time=\"0.01\"/></testsuite></testsuites>",
    );
    let (code, out, err) = r.run(&["evidence", "stamp"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = write_record(&r);
    assert!(
        t.contains("requirement = \"requirements/valves#sensor-timeout\"\nstate = \"passed-once\"\ncases = [\""),
        "{t}"
    );
    assert!(t.contains("test_sensor_timeout"), "{t}");
    // A requirement that no tag names stays `none`.
    assert!(
        t.contains("requirement = \"requirements/valves#sensor-alarm\"\nstate = \"none\""),
        "{t}"
    );
}

#[test]
fn a_requirement_in_the_tree_has_its_state_from_the_tree() {
    // The requirement is in the tree already (a change with no delta): its
    // `verified-by` gives the state `verified`.
    let r = start(&config("standard", ""));
    r.write(INTENT, INTENT_TEXT);
    r.write(
        SPEC,
        "---\nstatus: approved\n---\n# Valve timeout\n\n## Delta\n\n- CHANGE requirements/valves#overflow@0000000000000000: When a bed overflows, the controller SHALL close the valve of that bed in 5 seconds.\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n## Concerns\n\n- [x] None.\n\n## Open questions\n\n- [x] None.\n",
    );
    r.write(PLAN, PLAN_TEXT);
    r.commit("plan");
    r.write(CODE, CODE_NEW);
    r.commit("code");
    let t = write_record(&r);
    // The target resolves, and the test file has a tag, so `tagged`.
    assert!(
        t.contains("[[proof]]\nrequirement = \"requirements/valves#overflow\"\nstate = \"tagged\""),
        "{t}"
    );
}

#[test]
fn the_scope_has_the_plan_and_the_shared_paths_and_the_paths_are_code_in_scope() {
    let r = built_with(&config(
        "standard",
        "shared-paths = [\"deps/*.lock\", \"docs/*.md\"]\n",
    ));
    r.write("deps/a.lock", "1\n");
    r.write("src/other.py", "y = 2\n");
    r.write("README.md", "# Project, edited\n");
    r.write("docs/notes.md", "# Notes\n");
    r.commit("more");
    let t = write_record(&r);
    assert!(
        t.contains("scope = [\"deps/*.lock\", \"docs/*.md\", \"src/valves.py\", \"tests/test_valves.py\"]\n"),
        "{t}"
    );
    // The lock file is in the scope and is code. `src/other.py` is code outside
    // the scope, and the Markdown files are no code.
    assert!(
        t.contains("paths = [\"deps/a.lock\", \"src/valves.py\", \"tests/test_valves.py\"]\n"),
        "{t}"
    );
}

#[test]
fn a_deleted_code_path_stays_in_paths_with_an_absent_line() {
    let r = built();
    std::fs::remove_file(r.dir.join(CODE)).unwrap();
    r.commit("delete the code");
    let t = write_record(&r);
    assert!(
        t.contains("paths = [\"src/valves.py\", \"tests/test_valves.py\"]\n"),
        "{t}"
    );
    let oid = r.git_out(&["rev-parse", &format!("HEAD:{TEST}")]);
    let text = format!("spec-dna-tree 1\nabsent\t{CODE}\n100644 {oid}\t{TEST}\n");
    assert_eq!(field(&t, "tree"), sha16(text.as_bytes()));
}

#[test]
fn another_open_change_does_not_change_the_paths() {
    let r = built();
    let alone = {
        let t = write_record(&r);
        std::fs::remove_file(r.dir.join(RECORD)).unwrap();
        t
    };
    // The change `other` edits `src/other.py` in the same diff.
    r.write(
        "docs/intent/other/intent.md",
        &INTENT_TEXT.replace("Valve timeout", "Other"),
    );
    r.write(
        "docs/intent/other/plan.md",
        &PLAN_TEXT
            .replace("src/valves.py", "src/other.py")
            .replace("tests/test_valves.py", "tests/test_other.py"),
    );
    r.write("src/other.py", "y = 3\n");
    r.write("tests/test_other.py", "# nothing\n");
    r.commit("another change");
    let with_other = write_record(&r);
    assert_eq!(alone, with_other);
    assert_eq!(field(&with_other, "tree"), field(&alone, "tree"));
}

#[test]
fn after_the_code_merged_the_paths_are_the_paths_since_the_commit_of_the_plan() {
    let r = built();
    // The base is the head now: the diff against the base is empty.
    origin_head(&r.dir, "HEAD");
    let t = write_record(&r);
    assert!(
        t.contains("paths = [\"src/valves.py\", \"tests/test_valves.py\"]\n"),
        "{t}"
    );
    assert_eq!(field(&t, "tree").len(), 16);
}

#[test]
fn a_second_run_adds_an_acceptor_and_keeps_every_other_byte() {
    let r = built();
    write_record(&r);
    // The first acceptor writes its work by hand, with a comment.
    let mut t = r.read(RECORD);
    t = format!(
        "# my note\n{t}\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n"
    );
    r.write(RECORD, &t);
    let (code, out, err) = accept(
        &r,
        &[SLUG, "--role", "security-reviewer", "--session", "s2"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.lines().any(|l| l == "acceptor: 2"), "{out}");
    let after = r.read(RECORD);
    assert!(after.starts_with("# my note\n"));
    assert!(
        after.contains(
            "[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n"
        )
    );
    assert!(after.contains("[[acceptor]]\nrole = \"security-reviewer\"\nbrief = \"pending\"\nsession = \"s2\"\nagent = \"unknown\"\nresult = \"pending\"\n"));
    assert_eq!(after.matches("[[acceptor]]").count(), 2);
    // Each byte of the old text is still there, in order.
    let (head, tail) = t.split_once("[[proof]]").unwrap();
    assert!(after.starts_with(head));
    assert!(after.contains(tail.trim_end()));
    // A third run gives the number 3.
    let (_, out, _) = accept(&r, &[SLUG, "--session", "s3"]);
    assert!(out.lines().any(|l| l == "acceptor: 3"), "{out}");
}

#[test]
fn a_second_run_after_a_code_edit_exits_1() {
    for committed in [true, false] {
        let r = built();
        write_record(&r);
        let first = r.read(RECORD);
        r.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
        if committed {
            r.commit("edit the code");
        }
        let (code, out, err) = accept(&r, &[SLUG, "--session", "s2"]);
        assert_eq!(code, 1, "committed {committed}: {out}{err}");
        if committed {
            assert!(
                err.contains("changed since the record")
                    && err.contains("delete the record and accept again"),
                "{err}"
            );
        } else {
            // An edit that HEAD does not have is a refusal before the compare.
            assert!(err.contains("commit or remove"), "{err}");
        }
        assert_eq!(r.read(RECORD), first, "the refusal changes nothing");
    }
}

#[test]
fn a_second_run_after_a_markdown_edit_is_not_refused() {
    let r = built();
    write_record(&r);
    r.write("README.md", "# Project, edited\n");
    r.commit("docs");
    let (code, out, err) = accept(&r, &[SLUG, "--session", "s2"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_session_that_is_absent_or_empty_is_unknown() {
    for args in [
        vec![SLUG],
        vec![SLUG, "--session", ""],
        vec![SLUG, "--session", "  "],
    ] {
        let r = built();
        let (code, out, err) = accept(&r, &args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        assert!(
            r.read(RECORD)
                .contains("session = \"unknown\"\nagent = \"unknown\"\n"),
            "{args:?}"
        );
    }
    // A value with a quote and a backslash is written as TOML.
    let r = built();
    let (code, _, err) = accept(&r, &[SLUG, "--session", "a\"b\\c"]);
    assert_eq!(code, 0, "{err}");
    assert!(r.read(RECORD).contains("session = \"a\\\"b\\\\c\"\n"));
}

#[test]
fn an_edit_that_head_does_not_have_is_a_refusal_in_both_forms() {
    type Edit = fn(&Repo);
    let edits: [(&str, Edit); 7] = [
        ("plan", |r| r.write(PLAN, &format!("{PLAN_TEXT}\nMore.\n"))),
        ("spec", |r| r.write(SPEC, &format!("{SPEC_TEXT}\nMore.\n"))),
        ("intent", |r| {
            r.write(INTENT, &format!("{INTENT_TEXT}\nMore.\n"))
        }),
        ("config", |r| {
            let t = r.read("spec-dna.toml");
            r.write("spec-dna.toml", &format!("{t}\n# edit\n"));
        }),
        ("staged plan", |r| {
            r.write(PLAN, &format!("{PLAN_TEXT}\nMore.\n"));
            r.git(&["add", PLAN]);
            // The working tree has the committed text again.
            r.write(PLAN, PLAN_TEXT);
        }),
        ("untracked file in the folder", |r| {
            r.write("docs/intent/valve-timeout/notes.txt", "x\n");
        }),
        ("deleted file", |r| {
            std::fs::remove_file(r.dir.join(SPEC)).unwrap();
        }),
    ];
    for (name, edit) in edits {
        for form in [vec![SLUG, "--brief"], vec![SLUG]] {
            let r = built();
            edit(&r);
            let before = r.git_out(&["status", "--porcelain"]);
            let (code, out, err) = accept(&r, &form);
            assert_eq!(code, 1, "{name} {form:?}: {out}{err}");
            assert!(
                err.contains("commit the work, then accept"),
                "{name}: {err}"
            );
            assert!(out.is_empty(), "{name}: {out}");
            assert_eq!(r.git_out(&["status", "--porcelain"]), before, "{name}");
            assert!(!r.dir.join(RECORD).exists(), "{name}");
        }
    }
}

#[test]
fn a_code_path_in_scope_with_an_edit_that_head_does_not_have_is_a_refusal_in_both_forms() {
    type Edit = fn(&Repo);
    let edits: [(&str, Edit); 5] = [
        ("untracked file", |r| r.write("src/scratch.py", "z = 1\n")),
        ("unstaged edit", |r| {
            r.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
        }),
        ("staged edit", |r| {
            r.write(CODE, &format!("{CODE_NEW}\n\ndef more():\n    pass\n"));
            r.git(&["add", CODE]);
            r.write(CODE, CODE_NEW);
        }),
        ("deleted file", |r| {
            std::fs::remove_file(r.dir.join(TEST)).unwrap();
        }),
        ("untracked file in a new folder", |r| {
            r.write("src/new/scratch.py", "z = 1\n");
        }),
    ];
    for (name, edit) in edits {
        for form in [vec![SLUG, "--brief"], vec![SLUG]] {
            let r = built_with(&config("standard", ""));
            // The plan lists `src/valves.py` and `tests/test_valves.py` only, so
            // a scratch file is in scope through a glob.
            r.write(
                PLAN,
                &PLAN_TEXT.replace(
                    "- `tests/test_valves.py`\n",
                    "- `tests/test_valves.py`\n- `src/**`\n",
                ),
            );
            r.commit("a wider scope");
            edit(&r);
            let before = r.git_out(&["status", "--porcelain"]);
            let (code, out, err) = accept(&r, &form);
            assert_eq!(code, 1, "{name} {form:?}: {out}{err}");
            assert!(err.contains("commit or remove"), "{name}: {err}");
            assert!(out.is_empty(), "{name}: {out}");
            assert_eq!(r.git_out(&["status", "--porcelain"]), before, "{name}");
            assert!(!r.dir.join(RECORD).exists(), "{name}");
        }
    }
}

#[test]
fn an_untracked_file_out_of_scope_or_not_code_is_no_refusal() {
    let r = built();
    // `src/scratch.py` is code, but the plan lists exact paths and not this one.
    r.write("src/scratch.py", "z = 1\n");
    let (code, out, err) = accept(&r, &[SLUG, "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
    // A Markdown file is not code.
    let r = built();
    r.write("notes.md", "# Notes\n");
    let (code, out, err) = accept(&r, &[SLUG, "--brief"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn an_uncommitted_record_alone_is_no_refusal() {
    let r = built();
    write_record(&r);
    // The record is untracked, and it is the only edit.
    assert_eq!(
        r.git_out(&["status", "--porcelain"]),
        format!("?? {RECORD}")
    );
    let (code, _, err) = accept(&r, &[SLUG, "--brief"]);
    assert_eq!(code, 0, "{err}");
    let (code, _, err) = accept(&r, &[SLUG, "--session", "s2"]);
    assert_eq!(code, 0, "{err}");
    // An edit of the record that is tracked and changed is no refusal either.
    r.commit("record");
    r.write(RECORD, &format!("{}\n# edit\n", r.read(RECORD)));
    let (code, _, err) = accept(&r, &[SLUG, "--brief"]);
    assert_eq!(code, 0, "{err}");
    // The brief does not read the record: the same text with or without it.
    let with_record = brief(&r, &[]);
    r.git(&["rm", "-q", "-f", RECORD]);
    r.commit("remove the record");
    assert_eq!(brief(&r, &[]), with_record);
}

#[test]
fn the_exit_codes() {
    let r = built();
    // 5: no folder for the slug.
    let (code, _, err) = accept(&r, &["no-such-change"]);
    assert_eq!(code, 5, "{err}");
    let (code, _, _) = accept(&r, &["no-such-change", "--brief"]);
    assert_eq!(code, 5);
    // 2: usage.
    for args in [
        vec![],
        vec![SLUG, "extra"],
        vec![SLUG, "--role", "Bad Role"],
        vec![SLUG, "--role", ""],
        vec![SLUG, "--role", "1abc"],
        vec![SLUG, "--brief", "--session", "s1"],
        vec!["a/b"],
        vec![".."],
    ] {
        let (code, _, err) = accept(&r, &args);
        assert_eq!(code, 2, "{args:?}: {err}");
    }
    // The options belong to `accept`.
    for args in [
        vec!["status", "--brief"],
        vec!["status", "--role", "x"],
        vec!["status", "--session", "x"],
    ] {
        let (code, _, err) = r.run(&args);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(err.contains("is an option of `accept`"), "{err}");
    }
    // 3: the pin names another version.
    let toml = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &toml.replace(env!("CARGO_PKG_VERSION"), "0.0.1"),
    );
    r.commit("pin");
    let (code, _, err) = accept(&r, &[SLUG]);
    assert_eq!(code, 3, "{err}");
    r.write("spec-dna.toml", &toml);
    r.commit("pin back");
    // 0: both forms.
    assert_eq!(accept(&r, &[SLUG, "--brief"]).0, 0);
    assert_eq!(accept(&r, &[SLUG]).0, 0);
}

#[test]
fn a_change_with_no_plan_or_no_intent_is_a_refusal() {
    let r = start(&config("standard", ""));
    r.write(INTENT, INTENT_TEXT);
    r.commit("intent only");
    let (code, _, err) = accept(&r, &[SLUG]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("has no `plan.md`"), "{err}");
    let (code, _, _) = accept(&r, &[SLUG, "--brief"]);
    assert_eq!(code, 1);
    let r = start(&config("standard", ""));
    r.write(PLAN, PLAN_TEXT);
    r.commit("plan only");
    let (code, _, err) = accept(&r, &[SLUG]);
    assert_eq!(code, 1, "{err}");
    assert!(err.contains("has no `intent.md`"), "{err}");
}

#[test]
fn the_command_works_in_each_profile() {
    for profile in ["lite", "standard", "strict"] {
        let r = built_with(&config(profile, ""));
        let b = brief(&r, &[]);
        assert!(b.starts_with("spec-dna-brief 1 "), "{profile}");
        let (code, out, err) = accept(&r, &[SLUG, "--session", "s1"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        assert!(r.read(RECORD).contains("[[acceptor]]"), "{profile}");
    }
}

#[test]
fn the_json_forms() {
    let r = built();
    let (code, out, err) = accept(&r, &[SLUG, "--brief", "--json"]);
    assert_eq!(code, 0, "{err}");
    let plain = brief(&r, &[]);
    assert!(
        out.starts_with(&format!(
            "{{\"slug\":\"{SLUG}\",\"role\":\"acceptor\",\"brief\":\""
        )),
        "{out}"
    );
    let hash = last_line(&plain).strip_prefix("brief ").unwrap();
    assert!(out.contains(&format!("\"brief\":\"{hash}\"")), "{out}");
    let (code, out, err) = accept(&r, &[SLUG, "--session", "s1", "--json"]);
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        out.trim_end(),
        format!(
            "{{\"slug\":\"{SLUG}\",\"path\":\"{RECORD}\",\"acceptor\":1,\"role\":\"acceptor\",\"session\":\"s1\",\"created\":true}}"
        )
    );
    let (_, out, _) = accept(&r, &[SLUG, "--session", "s2", "--json"]);
    assert!(
        out.contains("\"acceptor\":2") && out.contains("\"created\":false"),
        "{out}"
    );
}

#[test]
fn the_help_lists_the_command_and_its_options() {
    let r = built();
    let (code, out, _) = r.run(&["accept", "--help"]);
    assert_eq!(code, 0);
    for word in [
        "--brief",
        "--role",
        "--session",
        "acceptance.toml",
        "spec-dna-brief 1",
    ] {
        assert!(out.contains(word), "{word}: {out}");
    }
    let (_, all, _) = r.run(&["--help"]);
    assert!(all.contains("accept <slug>"), "{all}");
}

#[test]
fn a_skeleton_over_the_size_limit_is_refused_and_names_the_cause_and_the_fix() {
    // 500 code paths of 40 bytes each make a `paths` line over 16,384 bytes.
    let r = built_with(&config("standard", "shared-paths = [\"gen/*.txt\"]\n"));
    for n in 0..500 {
        r.write(&format!("gen/a_module_with_a_long_name_{n:04}.txt"), "x\n");
    }
    r.commit("many files");
    let (code, out, err) = accept(&r, &[SLUG, "--session", "s1"]);
    assert_eq!(code, 1, "{out}{err}");
    let all = format!("{out}{err}");
    assert!(all.contains("16384") || all.contains("16,384"), "{all}");
    assert!(
        all.contains("too many paths") && all.contains("Split the change"),
        "{all}"
    );
    // No file is written.
    assert!(!r.dir.join(RECORD).exists());
}

/// The `Files` section of a real plan has prose around its code spans. A word
/// in a code span (`accept`, `next`) or a phrase (`eval run --ab`) is no path,
/// so it stays out of `scope`. A bare name that exists (`Makefile`) stays in.
#[test]
fn the_scope_leaves_out_the_code_spans_that_are_no_paths() {
    let plan = "---\nstatus: approved\n---\n# Valve timeout\n\n## Files\n\n\
- `src/valves.py`, `src/other.py` The record, its rules, its writer, and the command with the brief.\n\
- `src/stage.rs`, `src/harness.rs` The item `accept` of `next`, the agent `acceptor`, the skill and the hook.\n\
- `src/check.rs`, `src/diag.rs` The codes, the group `acceptance`, the keys and the CLI.\n\
- `src/init.rs` The key that `init` writes, and the step of `finish` and `check`.\n\
- `schema/spec-dna.schema.json` The keys.\n\
- `tests/*.py`, `tests/golden/**` The tests and the golden files.\n\
- `.claude/agents/ownership.md` The file rows.\n\
- `src/eval_ab.rs` The arm `without` of `eval run --ab`.\n\
- `Makefile`, `Dockerfile`, `Cargo.toml` Two bare names.\n\
\n## Order\n\n- [x] t1: Close the valve. Files: `src/valves.py`. Verify: the test passes.\n\
\n## Risks\n\n- A slow sensor.\n\n## Proof\n\nThe test covers `requirements/valves#sensor-timeout`.\n";
    let r = start(&config("standard", ""));
    r.write("Makefile", "all:\n");
    r.commit("a makefile");
    r.write(INTENT, INTENT_TEXT);
    r.write(SPEC, SPEC_TEXT);
    r.write(PLAN, plan);
    r.commit("plan");
    r.write(CODE, CODE_NEW);
    r.commit("code");
    let t = write_record(&r);
    let scope = t
        .lines()
        .find(|l| l.starts_with("scope = "))
        .unwrap_or_else(|| panic!("{t}"));
    for word in [
        "\"accept\"",
        "\"acceptance\"",
        "\"acceptor\"",
        "\"check\"",
        "\"finish\"",
        "\"init\"",
        "\"next\"",
        "\"without\"",
        "eval run --ab",
        "\"Dockerfile\"",
    ] {
        assert!(!scope.contains(word), "{word} in {scope}");
    }
    for path in [
        "\".claude/agents/ownership.md\"",
        "\"tests/*.py\"",
        "\"tests/golden/**\"",
        "\"schema/spec-dna.schema.json\"",
        "\"src/valves.py\"",
        "\"Cargo.toml\"",
        "\"Makefile\"",
    ] {
        assert!(scope.contains(path), "{path} not in {scope}");
    }
}

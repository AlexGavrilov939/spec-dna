//! Tests of the fixes of the review of step 3 of 0.6.0: the write of `finish`
//! through a symbolic link, a nested `REMOVE`, SD914 for an interface in
//! `check`, an id that ends in `.md`, the place and the number of an `ADD`,
//! and a target document with an edit that is not committed.
//!
//! Each fixture has a `main` branch with the base commit, a
//! `refs/remotes/origin/HEAD` that names it, and a `feature` branch. The base
//! commit holds the folder of the change `dose`. A line of the delta can name
//! the hash of a requirement as `{h:<id>#<key>}`.

mod common;

use common::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";
const HTTP: &str = "---\nstatus: active\n---\n# HTTP\n\n## Requirements\n\n- **ping**: The service SHALL answer a ping.\n";
const NESTED: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **a**: The controller SHALL close the valve in 5 seconds.\n  - **a1**: The controller SHALL report the close.\n- **z**: The controller SHALL open the valve in 5 seconds.\n";

const FOLDER: &str = "docs/intent/dose";
const SPEC: &str = "docs/intent/dose/spec.md";
const INTENT: &str = "docs/intent/dose/intent.md";
const PLAN: &str = "docs/intent/dose/plan.md";
const VALVES_PATH: &str = "docs/requirements/valves.md";

struct Fx<'a> {
    /// The delta lines of the spec.
    delta: &'a [&'a str],
    /// Text that the spec has after the `Delta` section.
    spec_extra: &'a str,
    /// The text of `requirements/valves`.
    valves: &'a str,
}

impl Default for Fx<'_> {
    fn default() -> Self {
        Fx {
            delta: &[],
            spec_extra: "",
            valves: VALVES,
        }
    }
}

fn hash(r: &Repo, id: &str) -> String {
    let (code, out, err) = r.run(&["show", id, "--json"]);
    assert_eq!(code, 0, "{id}: {out}{err}");
    let rest = out.split("\"hash\":\"").nth(1).expect("a hash");
    rest.split('"').next().unwrap().to_string()
}

/// Replaces each `{h:<id>#<key>}` with the hash of the requirement.
fn expand(r: &Repo, line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(i) = rest.find("{h:") {
        out.push_str(&rest[..i]);
        let tail = &rest[i + 3..];
        let end = tail.find('}').expect("a closing brace");
        out.push_str(&hash(r, &tail[..end]));
        rest = &tail[end + 1..];
    }
    out.push_str(rest);
    out
}

impl Fx<'_> {
    fn config() -> String {
        "profile = \"standard\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.contracts]\ntypes = [\"interface\"]\n\n[layers.decisions]\ntypes = [\"decision\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\", \"contracts\"]\nkeep = \"until-finished\"\n".to_string()
    }

    /// The base commit on `main`, `origin/HEAD` on it, and the branch
    /// `feature`. `setup` runs before the delta is written, so the hashes see
    /// the files that it adds.
    fn start(&self, setup: &dyn Fn(&Repo)) -> Repo {
        let r = Repo::with_config(&Fx::config());
        r.write(VALVES_PATH, self.valves);
        r.write("docs/contracts/http.md", HTTP);
        r.write("src/dose.py", "x = 1\n");
        r.write("tests/test_other.py", "# nothing\n");
        setup(&r);
        r.write(
            INTENT,
            "---\nstatus: accepted\nclass: higher-risk\n---\n# Dose\n\n## Intent\n\nThe dose is too high.\n",
        );
        r.write(
            PLAN,
            "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n- `tests/test_valves.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
        );
        let mut body = String::new();
        if !self.delta.is_empty() {
            body.push_str("## Delta\n\n");
            for l in self.delta {
                body.push_str(&format!("{}\n", expand(&r, l)));
            }
        }
        body.push_str(self.spec_extra);
        r.write(
            SPEC,
            &format!("---\nstatus: approved\n---\n# Spec\n\n{body}"),
        );
        r.commit("base");
        origin_head(&r.dir, "main");
        r.git(&["checkout", "-q", "-b", "feature"]);
        r
    }
}

fn fin(r: &Repo, args: &[&str]) -> (i32, String, String) {
    let mut v = vec!["finish", "dose"];
    v.extend(args);
    r.run(&v)
}

/// Each file of the working tree, with its bytes. A symbolic link gives its
/// target text. The folder `.git` is not part of it.
fn tree(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for e in fs::read_dir(dir).unwrap() {
            let p = e.unwrap().path();
            if p.file_name().is_some_and(|n| n == ".git") {
                continue;
            }
            let kind = fs::symlink_metadata(&p).unwrap().file_type();
            let rel = p.strip_prefix(root).unwrap().to_string_lossy().into_owned();
            if kind.is_symlink() {
                let target = fs::read_link(&p).unwrap();
                out.insert(
                    format!("{rel} -> link"),
                    target.to_string_lossy().into_owned().into_bytes(),
                );
            } else if kind.is_dir() {
                walk(root, &p, out);
            } else {
                out.insert(rel, fs::read(&p).unwrap());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn has(out: &str, code: &str) -> bool {
    out.lines().any(|l| {
        l.contains(&format!(": error {code} ")) || l.contains(&format!(": warning {code} "))
    })
}

fn staged(r: &Repo) -> Vec<String> {
    r.git_out(&["diff", "--cached", "--name-status"])
        .lines()
        .map(|l| l.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// The text of a finished run, for a message check.
fn said(out: &str, err: &str) -> String {
    format!("{out}{err}")
}

// Finding 1 and 4: a path with a symbolic link

#[cfg(unix)]
mod links {
    use super::*;
    use std::os::unix::fs::symlink;

    /// A folder outside the repository, and a fixture whose tracked link
    /// `docs/requirements/out` names it.
    fn through_a_folder(delta: &[&str], inside: &[(&str, &str)]) -> (Repo, Repo) {
        let outside = Repo::empty();
        for (p, t) in inside {
            outside.write(p, t);
        }
        let target = outside.dir.clone();
        let fx = Fx {
            delta,
            ..Fx::default()
        };
        let r =
            fx.start(&|r: &Repo| symlink(&target, r.dir.join("docs/requirements/out")).unwrap());
        (r, outside)
    }

    fn refused(r: &Repo, outside: &Repo, what: &str, message: &str) {
        let before = tree(&r.dir);
        let outer = tree(&outside.dir);
        for args in [&[][..], &["--dry-run"][..]] {
            let (code, out, err) = fin(r, args);
            let all = said(&out, &err);
            assert_eq!(code, 1, "{what} {args:?}: {all}");
            assert!(all.contains(message), "{what}: {all}");
        }
        assert_eq!(tree(&r.dir), before, "{what}: the repository changed");
        assert_eq!(tree(&outside.dir), outer, "{what}: the outside changed");
        assert!(staged(r).is_empty(), "{what}: {:?}", staged(r));
    }

    #[test]
    fn an_add_through_a_linked_folder_writes_nothing_outside() {
        let (r, outside) = through_a_folder(
            &["- ADD requirements/out/x#k: The controller SHALL flush the line."],
            &[],
        );
        r.write("tests/test_valves.py", "# covers: requirements/out/x#k\n");
        refused(&r, &outside, "new document", "symbolic link");
        assert!(!outside.dir.join("x.md").exists());
    }

    #[test]
    fn a_change_add_and_remove_through_a_linked_folder_are_refused() {
        let y = "---\nstatus: active\n---\n# Y\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";
        for (what, line) in [
            (
                "change",
                "- CHANGE requirements/out/y#close@{h:requirements/valves#close}: The controller SHALL close the valve in 4 seconds.",
            ),
            (
                "remove",
                "- REMOVE requirements/out/y#open@{h:requirements/valves#open}",
            ),
            (
                "add to an existing document",
                "- ADD requirements/out/y#flush: The controller SHALL flush the line.",
            ),
        ] {
            let (r, outside) = through_a_folder(&[line], &[("y.md", y)]);
            r.write(
                "tests/test_valves.py",
                "# covers: requirements/out/y#close\n# covers: requirements/out/y#flush\n",
            );
            // The corpus does not read a document behind a linked folder, so
            // these lines stop at step 3 with "names no document" or "exists".
            refused(&r, &outside, what, "SD914");
        }
    }

    #[test]
    fn a_change_remove_and_add_on_a_linked_document_file_keep_the_link() {
        for line in [
            "- CHANGE requirements/linked#close@{h:requirements/valves#close}: The controller SHALL close the valve in 4 seconds.",
            "- REMOVE requirements/linked#open@{h:requirements/valves#open}",
            "- ADD requirements/linked#flush: The controller SHALL flush the line.",
        ] {
            let outside = Repo::empty();
            outside.write("real.md", VALVES);
            let target = outside.dir.join("real.md");
            let fx = Fx {
                delta: &[line],
                ..Fx::default()
            };
            let r = fx.start(&|r: &Repo| {
                symlink(&target, r.dir.join("docs/requirements/linked.md")).unwrap()
            });
            r.write(
                "tests/test_valves.py",
                "# covers: requirements/linked#close\n# covers: requirements/linked#flush\n",
            );
            let before = tree(&r.dir);
            let outer = tree(&outside.dir);
            let (code, out, err) = fin(&r, &[]);
            let all = said(&out, &err);
            assert_eq!(code, 1, "{line}: {all}");
            assert!(all.contains("symbolic link"), "{line}: {all}");
            assert!(all.contains("docs/requirements/linked.md"), "{line}: {all}");
            assert_eq!(tree(&r.dir), before, "{line}");
            assert_eq!(tree(&outside.dir), outer, "{line}");
            assert!(
                fs::symlink_metadata(r.dir.join("docs/requirements/linked.md"))
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
        }
    }

    #[test]
    fn a_link_in_the_folder_of_the_change_stops_finish() {
        let outside = Repo::empty();
        outside.write("data.txt", "data\n");
        let target = outside.dir.join("data.txt");
        let r = Fx::default().start(&|r: &Repo| {
            fs::create_dir_all(r.dir.join(FOLDER)).unwrap();
            symlink(&target, r.dir.join(FOLDER).join("data.txt")).unwrap()
        });
        let before = tree(&r.dir);
        let (code, out, err) = fin(&r, &[]);
        let all = said(&out, &err);
        assert_eq!(code, 1, "{all}");
        assert!(
            all.contains("symbolic link") && all.contains("data.txt"),
            "{all}"
        );
        assert_eq!(tree(&r.dir), before);
        assert!(staged(&r).is_empty());
    }
}

// Finding 2: a nested requirement

#[test]
fn a_remove_of_an_item_with_a_nested_requirement_names_the_nested_key() {
    let fx = Fx {
        delta: &["- REMOVE requirements/valves#a@{h:requirements/valves#a}"],
        valves: NESTED,
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    let before = tree(&r.dir);
    let (code, out, err) = fin(&r, &[]);
    let all = said(&out, &err);
    assert_eq!(code, 1, "{all}");
    assert!(all.contains("a1"), "{all}");
    assert!(all.contains("step 4"), "{all}");
    assert_eq!(tree(&r.dir), before);
    assert!(staged(&r).is_empty());
}

#[test]
fn a_remove_of_the_item_and_of_its_nested_requirement_is_accepted() {
    let fx = Fx {
        delta: &[
            "- REMOVE requirements/valves#a@{h:requirements/valves#a}",
            "- REMOVE requirements/valves#a1@{h:requirements/valves#a1}",
        ],
        valves: NESTED,
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read(VALVES_PATH),
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **z**: The controller SHALL open the valve in 5 seconds.\n"
    );
}

// Finding 3: SD914 for an interface that the diff does not change

fn deleted_with_the_delta_applied(http_changes: bool) -> (i32, String) {
    let fx = Fx {
        delta: &[
            "- CHANGE requirements/valves#close@{h:requirements/valves#close}: The controller SHALL close the valve in 4 seconds.",
        ],
        spec_extra: "\n## Interfaces\n\n- CHANGE contracts/http: The ping answer has a body.\n",
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    // The pull request applies the delta by hand and deletes the folder.
    r.write(
        VALVES_PATH,
        &VALVES.replace("in 5 seconds.\n- **open", "in 4 seconds.\n- **open"),
    );
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\n",
    );
    if http_changes {
        r.write(
            "docs/contracts/http.md",
            &HTTP.replace("# HTTP\n", "# HTTP\n\nThe service speaks plain HTTP.\n"),
        );
    }
    fs::remove_dir_all(r.dir.join(FOLDER)).unwrap();
    r.commit("finish by hand");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    (code, out)
}

#[test]
fn check_gives_sd914_for_an_interface_that_a_deleting_diff_does_not_change() {
    let (code, out) = deleted_with_the_delta_applied(false);
    assert!(
        has(&out, "SD914") && out.contains("contracts/http"),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn check_gives_no_sd914_if_the_diff_changes_the_interface() {
    let (code, out) = deleted_with_the_delta_applied(true);
    assert!(!has(&out, "SD914"), "{out}");
    assert_eq!(code, 0, "{out}");
}

// Finding 7: an id that ends in `.md`

#[test]
fn an_id_that_ends_in_md_gives_sd914_in_finish_and_in_check() {
    let fx = Fx {
        delta: &["- ADD requirements/x.md#k: The controller SHALL flush the line."],
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    r.write("tests/test_valves.py", "# covers: requirements/x.md#k\n");
    let before = tree(&r.dir);
    let (code, out, err) = fin(&r, &[]);
    let all = said(&out, &err);
    assert_eq!(code, 1, "{all}");
    assert!(
        all.contains("SD914") && all.contains("without `.md`"),
        "{all}"
    );
    assert_eq!(tree(&r.dir), before);
    assert!(!r.dir.join("docs/requirements/x.md.md").exists());
    // The spec is in the base, so the diagnostic is old and a warning.
    let (_, out, _) = r.run(&["check", "--all", "--base", "main"]);
    assert!(has(&out, "SD914") && out.contains("without `.md`"), "{out}");
}

// Finding 8: the place and the number of an `ADD`, and a dirty target

#[test]
fn an_add_after_a_nested_last_requirement_is_a_sibling_of_the_top_level_item() {
    let valves = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **a**: The controller SHALL close the valve in 5 seconds.\n- **b**: The controller SHALL open the valve in 5 seconds.\n  - **b1**: The controller SHALL report the open.\n\n## Notes\n\nText.\n";
    let fx = Fx {
        delta: &["- ADD requirements/valves#flush: The controller SHALL flush the line."],
        valves,
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#flush\n",
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read(VALVES_PATH),
        valves.replace(
            "report the open.\n",
            "report the open.\n- **flush**: The controller SHALL flush the line.\n"
        )
    );
}

#[test]
fn a_target_document_with_an_edit_that_is_not_committed_stops_finish() {
    let fx = Fx {
        delta: &["- ADD requirements/valves#flush: The controller SHALL flush the line."],
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#flush\n",
    );
    // An edit of the author that is not part of the delta.
    r.write(
        VALVES_PATH,
        &VALVES.replace("open the valve in 5", "open the valve in 6"),
    );
    let before = tree(&r.dir);
    for args in [&[][..], &["--dry-run"][..]] {
        let (code, out, err) = fin(&r, args);
        let all = said(&out, &err);
        assert_eq!(code, 1, "{args:?}: {all}");
        assert!(
            all.contains(VALVES_PATH) && all.contains("not committed"),
            "{all}"
        );
    }
    assert_eq!(tree(&r.dir), before);
    assert!(staged(&r).is_empty());
}

#[test]
fn a_delta_that_the_author_applied_by_hand_and_did_not_commit_does_not_stop_finish() {
    let fx = Fx {
        delta: &[
            "- CHANGE requirements/valves#close@{h:requirements/valves#close}: The controller SHALL close the valve in 4 seconds.",
        ],
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#close\n",
    );
    r.write(
        VALVES_PATH,
        &VALVES.replace("close the valve in 5", "close the valve in 4"),
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_number_of_an_add_follows_the_list_after_the_removals() {
    let ordered = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n1. **close**: The controller SHALL close the valve in 5 seconds.\n2. **open**: The controller SHALL open the valve in 5 seconds.\n";
    let fx = Fx {
        delta: &[
            "- REMOVE requirements/valves#open@{h:requirements/valves#open}",
            "- ADD requirements/valves#flush: The controller SHALL flush the line.",
        ],
        valves: ordered,
        ..Fx::default()
    };
    let r = fx.start(&|_| {});
    r.write(
        "tests/test_valves.py",
        "# covers: requirements/valves#flush\n",
    );
    let (code, out, err) = fin(&r, &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        r.read(VALVES_PATH),
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n1. **close**: The controller SHALL close the valve in 5 seconds.\n2. **flush**: The controller SHALL flush the line.\n"
    );
}

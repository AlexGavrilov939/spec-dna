//! SD606 and a layer folder that the last change leaves empty (0.7.1, item 6).
//! `finish` deletes the folder of a change. If it was the last one, git no
//! longer has the folder of the layer (`docs/intent`), but the layer exists and
//! the next change goes there. A document that names the layer folder is not
//! drift. A folder that held other files, or a path that is no layer folder,
//! still is.

mod common;

use common::*;

const CONFIG: &str = "profile = \"standard\"\n\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n";

const INTENT: &str =
    "---\nstatus: accepted\nclass: routine\n---\n# Dose\n\n## Intent\n\nThe dose is too high.\n";
const PLAN: &str = "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- None.\n\n## Order\n\n- [x] t1: Read the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n";

fn valves(cite: &str) -> String {
    format!(
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n\nThe code host guards {cite}.\n"
    )
}

fn project(cite: &str, extra: &[(&str, &str)]) -> Repo {
    let r = Repo::with_config(CONFIG);
    r.write("docs/requirements/valves.md", &valves(cite));
    r.write("docs/intent/dose/intent.md", INTENT);
    r.write("docs/intent/dose/plan.md", PLAN);
    for (p, t) in extra {
        r.write(p, t);
    }
    r.commit("base");
    r
}

fn sd606(r: &Repo) -> String {
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    out.lines()
        .filter(|l| l.contains("SD606"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_last_change_folder_leaves_the_layer_folder_cited_without_drift() {
    let r = project("`docs/intent/`", &[]);
    r.git(&["rm", "-q", "-r", "docs/intent/dose"]);
    assert!(!r.dir.join("docs/intent").exists());
    let out = sd606(&r);
    assert_eq!(out, "", "{out}");
    // The same without the trailing slash.
    let r = project("`docs/intent`", &[]);
    r.git(&["rm", "-q", "-r", "docs/intent/dose"]);
    assert_eq!(sd606(&r), "");
}

#[test]
fn the_deleted_change_folder_itself_is_still_drift() {
    let r = project("`docs/intent/dose`", &[]);
    r.git(&["rm", "-q", "-r", "docs/intent/dose"]);
    let out = sd606(&r);
    assert!(out.contains("`docs/intent/dose` was removed"), "{out}");
}

#[test]
fn a_folder_that_held_another_file_is_still_drift() {
    let r = project("`docs/intent/`", &[("docs/intent/OWNERS", "owner\n")]);
    r.git(&["rm", "-q", "-r", "docs/intent"]);
    let out = sd606(&r);
    assert!(out.contains("`docs/intent` was removed"), "{out}");
}

#[test]
fn a_folder_that_is_no_layer_folder_is_still_drift() {
    let r = project(
        "`src/old/`",
        &[("src/old/a.py", "x = 1\n"), ("src/old/b.py", "y = 2\n")],
    );
    r.git(&["rm", "-q", "-r", "src/old"]);
    let out = sd606(&r);
    assert!(out.contains("`src/old` was removed"), "{out}");
}

#[test]
fn a_subfolder_and_a_plan_only_folder_of_a_change_do_not_give_drift() {
    // An image in a subfolder of the change folder goes with the change.
    let r = project(
        "`docs/intent/`",
        &[("docs/intent/dose/assets/flow.png", "png\n")],
    );
    r.git(&["rm", "-q", "-r", "docs/intent/dose"]);
    assert_eq!(sd606(&r), "");
    // A change folder that holds a plan only is a change folder too.
    let r = Repo::with_config(CONFIG);
    r.write("docs/requirements/valves.md", &valves("`docs/intent/`"));
    r.write("docs/intent/dose/plan.md", PLAN);
    r.commit("base");
    r.git(&["rm", "-q", "-r", "docs/intent/dose"]);
    assert_eq!(sd606(&r), "");
}

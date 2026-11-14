//! Integration tests of the data of a change (step 3, slice S1): the fields
//! `class`, `areas` and `from` of an intent, SD302 for a `class` that is not
//! one, the line `class: routine` of the 3 intent templates, and the cache of
//! the parsed data. The binary has no library, so the tests of the parsed list
//! items, of the byte ranges and of the delta lines are unit tests in
//! `src/parse.rs`, `src/index.rs` and `src/change.rs`.

mod common;

use common::*;

const CONFIG: &str = "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

fn repo() -> Repo {
    Repo::with_config(CONFIG)
}

/// The text of `check --all --json` after the files are staged.
fn check(r: &Repo) -> (i32, String) {
    r.git(&["add", "-A"]);
    let (code, out, _) = r.run(&["check", "--all", "--json"]);
    (code, out)
}

/// The diagnostics and the documents of the JSON text of `check`, without the
/// summary, which counts the parsed and the cached documents.
fn result(json: &str) -> &str {
    &json[json.find("\"diagnostics\":").unwrap()..]
}

/// The codes in the JSON text of `check`, in the order of the text.
fn codes(json: &str) -> Vec<String> {
    json.split("\"code\":\"")
        .skip(1)
        .map(|s| s.split('"').next().unwrap().to_string())
        .collect()
}

fn intent(fm: &str) -> String {
    format!("---\nstatus: draft\n{fm}---\n# Drip\n\n## Intent\n\nSlow the drip.\n")
}

#[test]
fn new_intent_writes_class_routine_for_each_of_the_3_templates() {
    for args in [
        &["new", "intent", "drip"][..],
        &["new", "intent", "drip", "--template", "prd"][..],
        &["new", "intent", "drip", "--template", "rfc"][..],
        &["new", "intent", "drip", "--template", "intent"][..],
    ] {
        let r = repo();
        let (code, out, err) = r.run(args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        let text = r.read("docs/intent/drip/intent.md");
        assert!(
            text.starts_with("---\nstatus: draft\nclass: routine\n---\n# Drip\n"),
            "{args:?}: {text}"
        );
        // The new document has no problem with its front matter.
        let (_, json) = check(&r);
        assert!(!codes(&json).contains(&"SD302".to_string()), "{json}");
    }
}

#[test]
fn the_other_templates_have_no_class() {
    let r = repo();
    for args in [
        &["new", "intent", "drip"][..],
        &["new", "spec", "drip"][..],
        &["new", "plan", "drip"][..],
        &["new", "requirements", "valves"][..],
        &["new", "decision", "pump"][..],
        &["new", "incident", "leak"][..],
    ] {
        assert_eq!(r.run(args).0, 0, "{args:?}");
    }
    for path in [
        "docs/intent/drip/spec.md",
        "docs/intent/drip/plan.md",
        "docs/requirements/valves.md",
        "docs/decisions/pump.md",
        "docs/incidents/leak.md",
    ] {
        assert!(!r.read(path).contains("class:"), "{path}");
    }
}

#[test]
fn a_project_template_of_an_intent_keeps_its_own_front_matter() {
    let r = repo();
    r.write(
        ".spec-dna/templates/hr.md",
        "---\ntype: intent\nclass: higher-risk\n---\n# T\n\n## Intent\n\n{{Text.}}\n",
    );
    r.write(
        ".spec-dna/templates/plain.md",
        "---\ntype: intent\n---\n# T\n\n## Intent\n\n{{Text.}}\n",
    );
    assert_eq!(r.run(&["new", "intent", "a", "--template", "hr"]).0, 0);
    assert_eq!(r.run(&["new", "intent", "b", "--template", "plain"]).0, 0);
    assert!(
        r.read("docs/intent/a/intent.md")
            .starts_with("---\nclass: higher-risk\nstatus: draft\n---\n# A\n")
    );
    // A template with no `class` stays without it: an absent class is routine.
    assert!(
        r.read("docs/intent/b/intent.md")
            .starts_with("---\nstatus: draft\n---\n# B\n")
    );
}

#[test]
fn class_values_a_table() {
    let valid = [
        "class: routine",
        "class: higher-risk",
        "class: \"higher-risk\"",
        "class: 'routine'",
        "class:",
        "",
    ];
    for fm in valid {
        let r = repo();
        r.write(
            "docs/intent/drip/intent.md",
            &intent(&if fm.is_empty() {
                String::new()
            } else {
                format!("{fm}\n")
            }),
        );
        let (code, json) = check(&r);
        assert_eq!(code, 0, "{fm:?}: {json}");
        assert!(!codes(&json).contains(&"SD302".to_string()), "{fm:?}");
    }
    let invalid = [
        "class: small-fix",
        "class: Routine",
        "class: high-risk",
        "class: urgent",
        "class: [routine]",
        "class: routine higher-risk",
    ];
    for fm in invalid {
        let r = repo();
        r.write("docs/intent/drip/intent.md", &intent(&format!("{fm}\n")));
        let (code, json) = check(&r);
        assert_eq!(code, 1, "{fm}: {json}");
        assert_eq!(codes(&json), ["SD302"], "{fm}: {json}");
        assert!(json.contains("`class` must be"), "{json}");
        assert!(json.contains("\"line\":3"), "{json}");
    }
    let r = repo();
    r.write("docs/intent/drip/intent.md", &intent("class: small-fix\n"));
    assert!(check(&r).1.contains("a small fix has no intent"));
}

#[test]
fn areas_and_from_of_an_intent_give_no_diagnostic() {
    let r = repo();
    r.write(
        "docs/intent/drip/intent.md",
        &intent("class: higher-risk\nareas: [valves, sensors]\nfrom: incidents/flood\n"),
    );
    let (code, json) = check(&r);
    assert_eq!(code, 0, "{json}");
    assert!(codes(&json).is_empty(), "{json}");
}

#[test]
fn the_fields_of_a_change_in_another_document_mean_nothing() {
    // A `from` or a `class` in a document that is not an intent has another
    // meaning, so spec-dna does not check it.
    let r = repo();
    r.write(
        "docs/research/mail.md",
        "---\nfrom: [a, b]\nclass: small-fix\nareas: [x]\n---\n# Mail\n\nText.\n",
    );
    r.write(
        "docs/intent/drip/plan.md",
        "---\nstatus: draft\nclass: small-fix\n---\n# Plan\n\n## Files\n\n- `a.py`\n",
    );
    let (code, json) = check(&r);
    assert_eq!(code, 0, "{json}");
    assert!(!codes(&json).contains(&"SD302".to_string()), "{json}");
}

#[test]
fn a_change_folder_gives_the_same_result_with_a_cold_and_a_warm_cache() {
    let r = repo();
    r.write(
        "docs/intent/drip/intent.md",
        &intent("class: higher-risk\n"),
    );
    r.write(
        "docs/intent/drip/spec.md",
        "---\nstatus: draft\n---\n# Spec\n\n## Delta\n\n- ADD requirements/valves#close: The valve SHALL close.\n- not a delta line\n\n## Concerns\n\n- [ ] open\n",
    );
    r.write(
        "docs/intent/drip/plan.md",
        "---\nstatus: draft\n---\n# Plan\n\n## Files\n\n- `src/valve.py`\n",
    );
    r.write(
        "docs/requirements/valves.md",
        "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **open**: The valve SHALL open. <!-- aliases: valves#old -->\n",
    );
    let cold = check(&r);
    let cache = r.dir.join(".git/spec-dna/index-v1.bin");
    assert!(cache.exists(), "the first run writes the cache");
    let warm = check(&r);
    assert_eq!(cold.0, warm.0);
    assert_eq!(result(&cold.1), result(&warm.1));
    assert!(warm.1.contains("\"cached\":4"), "{}", warm.1);
    // A cache of an older parser is not used, and the run replaces it.
    let bytes = std::fs::read(&cache).unwrap();
    let mut old = bytes.clone();
    old[12..16].copy_from_slice(&11u32.to_le_bytes());
    std::fs::write(&cache, &old).unwrap();
    let rebuilt = check(&r);
    assert_eq!(result(&rebuilt.1), result(&cold.1));
    assert!(rebuilt.1.contains("\"parsed\":4"), "{}", rebuilt.1);
    let new = std::fs::read(&cache).unwrap();
    assert_eq!(new, bytes, "the run writes the cache again");
    assert_ne!(u32::from_le_bytes(new[12..16].try_into().unwrap()), 11);
}

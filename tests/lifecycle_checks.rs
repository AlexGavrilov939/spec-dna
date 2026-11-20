//! Integration tests of the lifecycle checks that read one tree (step 3,
//! slice S3): SD905 (delta part), SD912, SD914, SD919 and SD920 (absent part).
//! Also the exemption of SD105 for an `ADD` line, the silence of SD101 and
//! SD102 in the locked text of a decision, and `from` as a reference.
//!
//! The fixture has a custom graph. The `intent` layer moves into
//! `requirements` only, so a target in `contracts` or `decisions` is in a layer
//! that the delta cannot reach. `incidents` moves into `intent` and has no
//! `linked-from`, so a reference from an intent to an incident gives SD901.

mod common;

use common::*;

const VALVES: &str = "---\nstatus: active\n---\n# Valves\n\n## Requirements\n\n- **close**: The controller SHALL close the valve in 5 seconds.\n- **open**: The controller SHALL open the valve in 5 seconds.\n";

const HTTP: &str = "---\nstatus: active\n---\n# Http\n\n## Requirements\n\n- **status**: The interface SHALL return 200 for a valid call.\n";

const DECISION: &str =
    "---\nstatus: accepted\n---\n# Use a timer\n\n## Decision Outcome\n\nUse a timer.\n";

/// The configuration of a fixture. `change` has more keys of `[change]`,
/// `intent` more keys of `[layers.intent]`, and `incidents` more keys of
/// `[layers.incidents]`.
fn config(profile: &str, change: &str, intent: &str, incidents: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\n{change}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.contracts]\ntypes = [\"interface\"]\n\n[layers.decisions]\ntypes = [\"decision\"]\n\n[layers.incidents]\ntypes = [\"incident\"]\nmoves-to = [\"intent\"]\n{incidents}\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\"]\nkeep = \"until-finished\"\n{intent}"
    )
}

/// A fixture with the documents `requirements/valves` and `contracts/http`.
fn repo_with(cfg: &str) -> Repo {
    let r = Repo::with_config(cfg);
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/contracts/http.md", HTTP);
    r
}

fn repo(profile: &str) -> Repo {
    repo_with(&config(profile, "", "max-started = 1\n", ""))
}

/// The change `slug`: its intent, and a spec with `body` after the title.
fn change(r: &Repo, slug: &str, intent: &str, spec: &str, body: &str) {
    r.write(
        &format!("docs/intent/{slug}/intent.md"),
        &format!("---\nstatus: {intent}\n---\n# Change\n\n## Intent\n\nText.\n"),
    );
    r.write(
        &format!("docs/intent/{slug}/spec.md"),
        &format!("---\nstatus: {spec}\n---\n# Spec\n\n{body}"),
    );
}

fn delta(lines: &[&str]) -> String {
    format!("## Delta\n\n{}\n", lines.join("\n"))
}

/// The hash that `show` prints for a requirement.
fn hash(r: &Repo, id: &str) -> String {
    let (code, out, err) = r.run(&["show", id, "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    let rest = out.split("\"hash\":\"").nth(1).expect("a hash");
    rest.split('"').next().unwrap().to_string()
}

/// `check --all`: the exit code and the text.
fn check(r: &Repo) -> (i32, String) {
    let (code, out, _) = r.run(&["check", "--all"]);
    (code, out)
}

/// The severity words of the diagnostics of `code` in the text of `check`.
fn severities(out: &str, code: &str) -> Vec<&'static str> {
    let mut v = Vec::new();
    for line in out.lines() {
        if line.contains(&format!(": error {code} ")) {
            v.push("error");
        } else if line.contains(&format!(": warning {code} ")) {
            v.push("warning");
        }
    }
    v
}

/// The messages of the diagnostics of `code`.
fn messages<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    out.lines()
        .filter(|l| l.contains(&format!(" {code} ")) && !l.starts_with(code))
        .collect()
}

const PROFILES: [&str; 3] = ["lite", "standard", "strict"];

/// Asserts the severity of one diagnostic in each profile. `want` has the
/// severity for `lite`, `standard` and `strict` (`None`: no diagnostic).
fn matrix(code: &str, want: [Option<&str>; 3], setup: impl Fn(&Repo)) {
    for (profile, want) in PROFILES.iter().zip(want) {
        let r = repo(profile);
        setup(&r);
        let (_, out) = check(&r);
        let got = severities(&out, code);
        match want {
            Some(w) => assert_eq!(got, [w], "{code} in {profile}: {out}"),
            None => assert!(got.is_empty(), "{code} in {profile}: {out}"),
        }
    }
}

/// Asserts that a fixture gives no diagnostic of `code` in any profile.
fn never(code: &str, setup: impl Fn(&Repo)) {
    matrix(code, [None, None, None], setup);
}

// SD914

#[test]
fn an_invalid_delta_line_is_a_warning_with_no_base() {
    // With no base, no spec is touched: K3 caps the severity at warning.
    let setup = |r: &Repo| {
        change(
            r,
            "dose",
            "draft",
            "draft",
            &delta(&["- Update the valves."]),
        );
    };
    matrix(
        "SD914",
        [Some("warning"), Some("warning"), Some("warning")],
        setup,
    );
    let r = repo("standard");
    setup(&r);
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    assert!(
        messages(&out, "SD914")[0].contains(
            "the delta line is not valid: a delta line starts with ADD, CHANGE or REMOVE"
        ),
        "{out}"
    );
    assert!(err.contains("note: no base for the change checks"), "{err}");
}

#[test]
fn a_valid_delta_gives_no_sd914_in_any_profile() {
    let r = repo("standard");
    let h = hash(&r, "requirements/valves#close");
    never("SD914", |r| {
        change(
            r,
            "dose",
            "accepted",
            "draft",
            &delta(&[
                &format!(
                    "- CHANGE requirements/valves#close@{h}: The controller SHALL close the valve in 3 seconds."
                ),
                // A new key, and a new document in a layer that the intent reaches.
                "- ADD requirements/valves#flush: The controller SHALL flush the line.",
            ]),
        );
    });
    never("SD914", |r| {
        change(
            r,
            "dose",
            "accepted",
            "draft",
            &delta(&["- ADD requirements/pumps#start: The pump SHALL start."]),
        );
    });
}

#[test]
fn each_problem_of_a_delta_line_gives_sd914() {
    let r = repo("standard");
    let h = hash(&r, "requirements/valves#close");
    let cases: [(String, &str); 9] = [
        (
            format!("- CHANGE requirements/nothing#close@{h}: Text."),
            "`requirements/nothing` names no document",
        ),
        (
            format!("- REMOVE requirements/nothing#close@{h}"),
            "`requirements/nothing` names no document",
        ),
        (
            format!("- CHANGE requirements/valves#nothing@{h}: Text."),
            "`requirements/valves` has no requirement `nothing`",
        ),
        (
            "- ADD requirements/valves#close: Another text.".to_string(),
            "has the key `close` already",
        ),
        (
            "- CHANGE requirements/valves#close@0123456789abcdef: Text.".to_string(),
            "the hash `0123456789abcdef` is not the hash of the requirement now",
        ),
        (
            "- REMOVE requirements/valves#open@0123456789abcdef".to_string(),
            "the hash `0123456789abcdef` is not the hash of the requirement now",
        ),
        (
            "- CHANGE contracts/http#status@0123456789abcdef: Text.".to_string(),
            "is in the layer `contracts`, and the `moves-to` list of the layer `intent` does not have it",
        ),
        (
            "- ADD contracts/new#status: The interface SHALL answer.".to_string(),
            "the new document `contracts/new` is in the layer `contracts`",
        ),
        (
            "- ADD decisions/timer#x: Text.".to_string(),
            "a `decision` document",
        ),
    ];
    r.write("docs/decisions/timer.md", DECISION);
    for (line, why) in cases {
        change(&r, "dose", "draft", "draft", &delta(&[&line]));
        let (_, out) = check(&r);
        let got = messages(&out, "SD914");
        assert_eq!(got.len(), 1, "{line}: {out}");
        assert!(got[0].contains(why), "{line}: {why}: {out}");
    }
}

#[test]
fn a_document_that_holds_no_requirement_is_not_a_target() {
    let r = repo("standard");
    r.write("docs/decisions/timer.md", DECISION);
    change(
        &r,
        "dose",
        "draft",
        "draft",
        &delta(&["- CHANGE decisions/timer#x@0123456789abcdef: Text."]),
    );
    let (_, out) = check(&r);
    let got = messages(&out, "SD914");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].contains("`decisions/timer` is a `decision` document; a delta line needs a document of the type `requirements`, `interface` or `rule`"),
        "{out}"
    );
}

#[test]
fn a_line_that_is_already_applied_gives_no_sd914() {
    // The head has the text of the line, so the hash of the line is old.
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        &delta(&[
            "- CHANGE requirements/valves#close@0123456789abcdef: The controller SHALL close the valve in 5 seconds.",
            "- ADD requirements/valves#open: The controller SHALL open the valve in 5 seconds.",
        ]),
    );
    let (_, out) = check(&r);
    assert!(severities(&out, "SD914").is_empty(), "{out}");
}

#[test]
fn a_remove_line_is_applied_when_the_base_has_the_key_and_the_head_has_not() {
    let r = repo("standard");
    let pumps = VALVES.replace("Valves", "Pumps");
    r.write("docs/requirements/pumps.md", &pumps);
    let h = hash(&r, "requirements/pumps#open");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        &delta(&[&format!("- REMOVE requirements/pumps#open@{h}")]),
    );
    r.commit("base");
    // The key is there and the hash matches: no problem.
    let (_, out) = check(&r);
    assert!(severities(&out, "SD914").is_empty(), "{out}");
    // The author removes the key. The base has it, so the line is applied.
    let open = "- **open**: The controller SHALL open the valve in 5 seconds.\n";
    r.write("docs/requirements/pumps.md", &pumps.replace(open, ""));
    let (_, out) = check(&r);
    assert!(severities(&out, "SD914").is_empty(), "{out}");
    // A key that neither the base nor the head has is a problem.
    r.write(
        "docs/intent/dose/spec.md",
        &format!(
            "---\nstatus: approved\n---\n# Spec\n\n{}",
            delta(&[&format!("- REMOVE requirements/pumps#gone@{h}")])
        ),
    );
    let (_, out) = check(&r);
    assert_eq!(severities(&out, "SD914"), ["warning"], "{out}");
    assert!(
        messages(&out, "SD914")[0].contains("`requirements/pumps` has no requirement `gone`"),
        "{out}"
    );
}

#[test]
fn a_closed_change_gives_none_of_the_spec_checks() {
    never("SD914", |r| {
        change(r, "dose", "closed", "draft", &delta(&["- garbage"]));
    });
    never("SD919", |r| {
        change(
            r,
            "dose",
            "closed",
            "approved",
            "## Delta\n\n- ADD requirements/valves#x: Text.\n\n## Concerns\n\n- [ ] open\n",
        );
    });
}

#[test]
fn an_item_of_interfaces_names_an_interface_document() {
    let ok = "## Interfaces\n\n- CHANGE contracts/http: The status code changes.\n";
    never("SD914", |r| change(r, "dose", "draft", "draft", ok));
    let cases = [
        (
            "- CHANGE contracts/nothing: Text.",
            "`contracts/nothing` names no interface document",
        ),
        (
            "- CHANGE requirements/valves: Text.",
            "is a `requirements` document, not an interface",
        ),
        (
            "- contracts/http",
            "an item of `Interfaces` is `CHANGE <interface id>: <text>`",
        ),
        (
            "- CHANGE contracts/http",
            "an item of `Interfaces` is `CHANGE <interface id>: <text>`",
        ),
    ];
    for (line, why) in cases {
        let r = repo("standard");
        change(
            &r,
            "dose",
            "draft",
            "draft",
            &format!("## Interfaces\n\n{line}\n"),
        );
        let (_, out) = check(&r);
        let got = messages(&out, "SD914");
        assert_eq!(got.len(), 1, "{line}: {out}");
        assert!(got[0].contains(why), "{line}: {out}");
    }
}

/// A repository with a base commit on `main` and a branch `feature`. The
/// changes `dose` and `flow` each have a delta line with a wrong hash, and the
/// branch edits the spec of `dose` only.
fn with_base(profile: &str) -> Repo {
    let r = repo(profile);
    r.write(
        "docs/requirements/pumps.md",
        &VALVES.replace("Valves", "Pumps"),
    );
    let bad = |doc: &str| delta(&[&format!("- CHANGE {doc}#close@0123456789abcdef: Text.")]);
    // The intent of `dose` is `accepted`, so the branch can edit its spec
    // without SD910.
    change(&r, "dose", "accepted", "draft", &bad("requirements/valves"));
    change(&r, "flow", "draft", "draft", &bad("requirements/pumps"));
    r.git(&["add", "-A"]);
    r.git(&["commit", "-q", "-m", "base"]);
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write(
        "docs/intent/dose/spec.md",
        &format!(
            "---\nstatus: draft\n---\n# Spec\n\n{}\nA note.\n",
            bad("requirements/valves")
        ),
    );
    r.commit("touch dose");
    r
}

#[test]
fn sd914_is_an_error_only_for_a_spec_that_the_diff_touches() {
    for (profile, touched, other) in [
        ("lite", "warning", "warning"),
        ("standard", "error", "warning"),
        ("strict", "error", "warning"),
    ] {
        let r = with_base(profile);
        let (code, out, _) = r.run(&["check", "--all", "--base", "main"]);
        let lines = messages(&out, "SD914");
        assert_eq!(lines.len(), 2, "{profile}: {out}");
        let find = |path: &str| {
            let l = lines.iter().find(|l| l.contains(path)).expect("a line");
            if l.contains(": error SD914") {
                "error"
            } else {
                "warning"
            }
        };
        assert_eq!(
            find("docs/intent/dose/spec.md"),
            touched,
            "{profile}: {out}"
        );
        assert_eq!(find("docs/intent/flow/spec.md"), other, "{profile}: {out}");
        assert_eq!(code, i32::from(touched == "error"), "{profile}: {out}");
    }
}

#[test]
fn sd914_follows_the_fingerprint_of_the_gate() {
    // Without `--all`, `check` reports the new diagnostics only. The
    // fingerprint is the code, the document and the target. A new hash for the
    // same target is the same diagnostic, so it is not new.
    let r = with_base("standard");
    r.write(
        "docs/intent/dose/spec.md",
        &format!(
            "---\nstatus: draft\n---\n# Spec\n\n{}",
            delta(&["- CHANGE requirements/valves#close@fedcba9876543210: Text."])
        ),
    );
    r.commit("new line");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "the diagnostic has the same fingerprint: {out}");
    // Another target is a new diagnostic, and the spec is touched.
    let r = with_base("standard");
    r.write(
        "docs/intent/dose/spec.md",
        &format!(
            "---\nstatus: draft\n---\n# Spec\n\n{}",
            delta(&["- CHANGE requirements/valves#open@0123456789abcdef: Text."])
        ),
    );
    r.commit("new target");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 1, "{out}");
    assert_eq!(messages(&out, "SD914").len(), 1, "{out}");
}

// SD919

#[test]
fn an_open_concern_of_an_approved_spec_gives_sd919() {
    let body = |box_: &str| {
        format!(
            "{}\n## Concerns\n\n- {box_} Who owns the valve timer?\n- [x] Which unit? Seconds.\n",
            delta(&["- ADD requirements/valves#flush: The controller SHALL flush the line."])
        )
    };
    matrix(
        "SD919",
        [Some("warning"), Some("error"), Some("error")],
        |r| change(r, "dose", "accepted", "approved", &body("[ ]")),
    );
    // A resolved concern, a draft spec, and a bullet with no box give nothing.
    never("SD919", |r| {
        change(r, "dose", "accepted", "approved", &body("[x]"))
    });
    never("SD919", |r| {
        change(r, "dose", "accepted", "draft", &body("[ ]"))
    });
    never("SD919", |r| {
        change(r, "dose", "accepted", "approved", &body(""))
    });
}

#[test]
fn sd919_fails_check_in_standard() {
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        "## Delta\n\n- ADD requirements/valves#flush: The controller SHALL flush the line.\n\n## Concerns\n\n- [ ] Who owns the valve timer?\n",
    );
    let (code, out) = check(&r);
    assert_eq!(code, 1, "{out}");
    assert!(
        out.contains(
            "docs/intent/dose/spec.md:12: error SD919 concern-open: the spec is `approved`, and the concern `Who owns the valve timer?` is open"
        ),
        "{out}"
    );
}

// SD920

#[test]
fn a_decision_that_is_absent_gives_sd920_when_the_spec_is_approved() {
    let body = |id: &str| {
        format!(
            "{}\n## Decisions\n\n- [[{id}]]: use a timer\n",
            delta(&["- ADD requirements/valves#flush: The controller SHALL flush the line."])
        )
    };
    matrix(
        "SD920",
        [Some("warning"), Some("error"), Some("error")],
        |r| change(r, "dose", "accepted", "approved", &body("decisions/timer")),
    );
    // The decision exists, or the spec is a draft: nothing.
    never("SD920", |r| {
        r.write("docs/decisions/timer.md", DECISION);
        change(r, "dose", "accepted", "approved", &body("decisions/timer"));
    });
    never("SD920", |r| {
        change(r, "dose", "accepted", "draft", &body("decisions/timer"))
    });
}

// SD912

#[test]
fn more_started_changes_than_max_started_give_sd912() {
    let setup = |r: &Repo| {
        change(r, "dose", "accepted", "draft", &delta(&[]));
        change(r, "flow", "accepted", "draft", &delta(&[]));
    };
    matrix("SD912", [None, Some("warning"), Some("error")], setup);
    // One accepted intent, or one that is draft or closed: at the limit.
    never("SD912", |r| {
        change(r, "dose", "accepted", "draft", &delta(&[]));
        change(r, "flow", "draft", "draft", &delta(&[]));
        change(r, "pump", "closed", "draft", &delta(&[]));
    });
    let r = repo("standard");
    setup(&r);
    let (_, out) = check(&r);
    let got = messages(&out, "SD912");
    assert!(
        got[0].starts_with("spec-dna.toml:1: warning SD912 started-limit: 2 changes have an `accepted` intent, more than `max-started = 1` of the layer `intent`"),
        "{out}"
    );
}

#[test]
fn a_layer_with_no_max_started_has_no_sd912() {
    let r = repo_with(&config("strict", "", "", ""));
    change(&r, "dose", "accepted", "draft", &delta(&[]));
    change(&r, "flow", "accepted", "draft", &delta(&[]));
    change(&r, "pump", "accepted", "draft", &delta(&[]));
    let (_, out) = check(&r);
    assert!(severities(&out, "SD912").is_empty(), "{out}");
}

// SD905

#[test]
fn a_delta_over_max_fanout_gives_sd905() {
    let cfg = |p: &str| config(p, "max-fanout = 1\n", "max-started = 1\n", "");
    let two = delta(&[
        "- ADD requirements/valves#flush: The controller SHALL flush the line.",
        "- ADD requirements/pumps#start: The pump SHALL start.",
    ]);
    let one = delta(&[
        "- ADD requirements/valves#flush: The controller SHALL flush the line.",
        "- ADD requirements/valves#drain: The controller SHALL drain the line.",
    ]);
    for (profile, want) in [
        ("lite", None),
        ("standard", Some("warning")),
        ("strict", Some("warning")),
    ] {
        let r = repo_with(&cfg(profile));
        change(&r, "dose", "draft", "draft", &two);
        let (_, out) = check(&r);
        match want {
            Some(w) => assert_eq!(severities(&out, "SD905"), [w], "{profile}: {out}"),
            None => assert!(severities(&out, "SD905").is_empty(), "{profile}: {out}"),
        }
        // Two keys of one document are one document.
        let r = repo_with(&cfg(profile));
        change(&r, "dose", "draft", "draft", &one);
        let (_, out) = check(&r);
        assert!(severities(&out, "SD905").is_empty(), "{profile}: {out}");
    }
}

// The policy `none`, and the profiles that keep the codes.

#[test]
fn the_policy_none_runs_none_of_these_checks() {
    let r = Repo::with_config(
        "profile = \"strict\"\n[change]\npolicy = \"none\"\nmax-fanout = 1\n[rules]\nevidence = \"off\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.write("docs/requirements/valves.md", VALVES);
    for slug in ["dose", "flow"] {
        change(
            &r,
            slug,
            "accepted",
            "approved",
            "## Delta\n\n- garbage\n- ADD requirements/valves#a: Text.\n- ADD requirements/pumps#a: Text.\n\n## Concerns\n\n- [ ] open\n\n## Decisions\n\n- [[decisions/nothing]]\n",
        );
    }
    let (_, out) = check(&r);
    for code in ["SD905", "SD912", "SD914", "SD919", "SD920"] {
        assert!(!out.contains(&format!(" {code} ")), "{code}: {out}");
    }
    // A rule cannot turn them on.
    let r2 = Repo::with_config(
        "[change]\npolicy = \"none\"\n[rules]\nchanges = \"error\"\nSD914 = \"error\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    change(
        &r2,
        "dose",
        "accepted",
        "approved",
        "## Delta\n\n- garbage\n",
    );
    let (code, out) = check(&r2);
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD914"), "{out}");
}

#[test]
fn no_change_table_means_no_lifecycle_check() {
    let r =
        Repo::with_config("profile = \"strict\"\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n");
    r.write("docs/requirements/valves.md", VALVES);
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        "## Delta\n\n- garbage\n",
    );
    let (_, out) = check(&r);
    assert!(!out.contains("SD914"), "{out}");
}

#[test]
fn a_rule_sets_the_severity_of_a_lifecycle_code() {
    let r = repo_with(&config("standard", "", "max-started = 1\n", ""));
    let text = std::fs::read_to_string(r.dir.join("spec-dna.toml")).unwrap();
    r.write(
        "spec-dna.toml",
        &text.replace("[rules]\n", "[rules]\nSD919 = \"off\"\nSD912 = \"error\"\n"),
    );
    for slug in ["dose", "flow"] {
        change(
            &r,
            slug,
            "accepted",
            "approved",
            "## Concerns\n\n- [ ] open\n\n## Delta\n\n- ADD requirements/valves#a: Text.\n",
        );
    }
    let (code, out) = check(&r);
    assert_eq!(code, 1, "{out}");
    assert!(severities(&out, "SD919").is_empty(), "{out}");
    assert_eq!(severities(&out, "SD912"), ["error"], "{out}");
}

// SD105 and the `covers:` tag of an `ADD` line

#[test]
fn a_covers_tag_of_an_add_line_gives_no_sd105() {
    let tag = "# covers: requirements/valves#flush\n";
    let add = delta(&["- ADD requirements/valves#flush: The controller SHALL flush the line."]);
    for profile in PROFILES {
        // An accepted change: the tag is silent.
        let r = repo(profile);
        change(&r, "dose", "accepted", "draft", &add);
        r.write("tests/valves_test.py", tag);
        let (_, out) = check(&r);
        assert!(severities(&out, "SD105").is_empty(), "{profile}: {out}");
        // A draft or closed intent does not start the change, so the tag is
        // an error again. So is a tag of another key.
        for status in ["draft", "closed"] {
            let r = repo(profile);
            change(&r, "dose", status, "draft", &add);
            r.write("tests/valves_test.py", tag);
            let (_, out) = check(&r);
            let want = if profile == "lite" {
                "warning"
            } else {
                "error"
            };
            assert_eq!(
                severities(&out, "SD105"),
                [want],
                "{profile} {status}: {out}"
            );
        }
        let r = repo(profile);
        change(&r, "dose", "accepted", "draft", &add);
        r.write(
            "tests/valves_test.py",
            "# covers: requirements/valves#drain\n",
        );
        let (_, out) = check(&r);
        assert_eq!(severities(&out, "SD105").len(), 1, "{profile}: {out}");
    }
}

#[test]
fn a_covers_tag_of_an_add_line_for_a_new_document_gives_no_sd105() {
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "draft",
        &delta(&["- ADD requirements/pumps#start: The pump SHALL start."]),
    );
    r.write(
        "tests/pumps_test.py",
        "# covers: requirements/pumps#start\n",
    );
    let (_, out) = check(&r);
    assert!(severities(&out, "SD105").is_empty(), "{out}");
}

// SD101 and SD102 in the locked text of a decision

fn decision(sections: &str, status: &str) -> String {
    format!(
        "---\nstatus: {status}\n---\n# Use a timer\n\n## Decision Outcome\n\nUse a timer.\n\n{sections}"
    )
}

#[test]
fn a_broken_reference_in_the_locked_text_of_a_decision_gives_no_sd101() {
    let id = "See [[intent/gone/spec]] and [[requirements/valves#nothing]].";
    let link = "See [the plan](../intent/gone/plan.md).";
    for profile in PROFILES {
        let r = repo(profile);
        // Locked text: the outcome. Nothing is reported.
        r.write(
            "docs/decisions/timer.md",
            &decision("", "accepted")
                .replace("Use a timer.\n\n", &format!("Use a timer. {id} {link}\n\n")),
        );
        let (_, out) = check(&r);
        assert!(severities(&out, "SD101").is_empty(), "{profile}: {out}");
        assert!(severities(&out, "SD102").is_empty(), "{profile}: {out}");
        // An amendment section and a live section can change, so
        // the broken reference there gives SD101 and SD102.
        for section in ["Amendments", "Status"] {
            let r = repo(profile);
            r.write(
                "docs/decisions/timer.md",
                &decision(&format!("## {section}\n\n{id} {link}\n"), "accepted"),
            );
            let (_, out) = check(&r);
            let want = if profile == "lite" {
                "warning"
            } else {
                "error"
            };
            assert_eq!(
                severities(&out, "SD101"),
                [want, want],
                "{profile} {section}: {out}"
            );
            assert_eq!(
                severities(&out, "SD102"),
                [want],
                "{profile} {section}: {out}"
            );
        }
        // A decision that is not accepted has no locked text.
        let r = repo(profile);
        r.write(
            "docs/decisions/timer.md",
            &decision("", "proposed")
                .replace("Use a timer.\n\n", &format!("Use a timer. {id} {link}\n\n")),
        );
        let (_, out) = check(&r);
        assert_eq!(severities(&out, "SD101").len(), 2, "{profile}: {out}");
        assert_eq!(severities(&out, "SD102").len(), 1, "{profile}: {out}");
    }
}

#[test]
fn a_broken_depends_on_in_the_front_matter_of_an_accepted_decision_still_gives_sd101() {
    // SD405 does not lock the front matter, so the author can fix it.
    let r = repo("standard");
    r.write(
        "docs/decisions/timer.md",
        &DECISION.replace(
            "status: accepted\n",
            "status: accepted\ndepends-on: [intent/gone/spec]\n",
        ),
    );
    let (_, out) = check(&r);
    assert_eq!(severities(&out, "SD101"), ["error"], "{out}");
}

// `from` is a reference

fn incident() -> &'static str {
    "---\nstatus: active\n---\n# Flood\n\n## Summary\n\nThe bed flooded.\n\n## Root cause\n\nSee [[requirements/valves]].\n"
}

#[test]
fn the_from_field_of_an_intent_is_a_reference_for_sd101() {
    for (profile, want) in [
        ("lite", "warning"),
        ("standard", "error"),
        ("strict", "error"),
    ] {
        let r = repo(profile);
        r.write(
            "docs/intent/dose/intent.md",
            "---\nstatus: draft\nfrom: incidents/nothing\n---\n# Change\n\n## Intent\n\nText.\n",
        );
        let (_, out) = check(&r);
        assert_eq!(severities(&out, "SD101"), [want], "{profile}: {out}");
        let line = messages(&out, "SD101")[0];
        assert!(
            line.contains("docs/intent/dose/intent.md:3:")
                && line.contains("`incidents/nothing` names no document"),
            "{out}"
        );
        // An incident that exists: no SD101.
        r.write("docs/incidents/nothing.md", incident());
        let (_, out) = check(&r);
        assert!(severities(&out, "SD101").is_empty(), "{profile}: {out}");
    }
}

#[test]
fn the_from_field_of_an_intent_is_a_reference_for_sd901() {
    let intent = "---\nstatus: draft\nfrom: incidents/flood\n---\n# Change\n\n## Intent\n\nText.\n";
    // The intent is more stable than an incident, and the incident layer does
    // not list it in `linked-from`.
    for (profile, want) in [
        ("lite", None),
        ("standard", Some("warning")),
        ("strict", Some("warning")),
    ] {
        let r = repo(profile);
        r.write("docs/incidents/flood.md", incident());
        r.write("docs/intent/dose/intent.md", intent);
        let (_, out) = check(&r);
        match want {
            Some(w) => assert_eq!(severities(&out, "SD901"), [w], "{profile}: {out}"),
            None => assert!(severities(&out, "SD901").is_empty(), "{profile}: {out}"),
        }
    }
    // With `linked-from`, the reference is allowed.
    let r = repo_with(&config(
        "standard",
        "",
        "max-started = 1\n",
        "linked-from = [\"intent\"]\n",
    ));
    r.write("docs/incidents/flood.md", incident());
    r.write("docs/intent/dose/intent.md", intent);
    let (_, out) = check(&r);
    assert!(severities(&out, "SD901").is_empty(), "{out}");
}

#[test]
fn from_in_a_document_that_is_not_an_intent_is_not_a_reference() {
    // `from` in a document that is not an intent means nothing.
    let r = repo("standard");
    r.write(
        "docs/decisions/timer.md",
        &DECISION.replace(
            "status: accepted\n",
            "status: accepted\nfrom: nothing/here\n",
        ),
    );
    let (_, out) = check(&r);
    assert!(severities(&out, "SD101").is_empty(), "{out}");
}

// SD914: the forms of `Interfaces`

const EVENTS: &str = "---\nstatus: active\n---\n# Events\n\n## Requirements\n\n- **emit**: The interface SHALL emit one event for a change.\n";

#[test]
fn interfaces_accept_none_and_add_of_a_new_interface() {
    // `none` states "no interface", in each form of the `none` rule.
    for item in [
        "- none",
        "- None.",
        "- none: no contract changes",
        "- [ ] None.",
        "- [x] none",
    ] {
        never("SD914", |r| {
            change(
                r,
                "dose",
                "draft",
                "draft",
                &format!("## Interfaces\n\n{item}\n"),
            );
        });
    }
    // `ADD` names a document that is absent, at the place of an interface.
    never("SD914", |r| {
        change(
            r,
            "dose",
            "draft",
            "draft",
            "## Interfaces\n\n- ADD contracts/events: add the event contract.\n",
        );
    });
    // `ADD` of an interface that exists is also valid.
    never("SD914", |r| {
        change(
            r,
            "dose",
            "draft",
            "draft",
            "## Interfaces\n\n- ADD contracts/http: add a field.\n- CHANGE contracts/http: change a field.\n",
        );
    });
    // The place of the id gives another type.
    let r = repo("standard");
    change(
        &r,
        "dose",
        "draft",
        "draft",
        "## Interfaces\n\n- ADD requirements/events: add it.\n",
    );
    let (_, out) = check(&r);
    let got = messages(&out, "SD914");
    assert_eq!(got.len(), 1, "{out}");
    assert!(got[0].contains("not `interface`"), "{out}");
    assert!(got[0].contains("`requirements/events`"), "{out}");
}

#[test]
fn a_none_item_that_is_not_alone_or_not_the_word_is_not_none() {
    // `none` states "nothing" only as the one item of the section. The
    // item of 0.6.1 gave SD914 as a bad form, and it still does.
    for item in [
        "- none\n- CHANGE contracts/http: Text.",
        "- None of the contracts change.",
        "- nonexistent",
        "- `none`",
    ] {
        let r = repo("standard");
        change(
            &r,
            "dose",
            "draft",
            "draft",
            &format!("## Interfaces\n\n{item}\n"),
        );
        let (_, out) = check(&r);
        let got = messages(&out, "SD914");
        assert_eq!(got.len(), 1, "{item}: {out}");
        assert!(
            got[0].contains("an item of `Interfaces` is `CHANGE <interface id>: <text>`"),
            "{item}: {out}"
        );
    }
}

#[test]
fn an_add_item_needs_its_document_when_the_diff_deletes_the_folder() {
    let setup = |with_document: bool| {
        let r = repo("standard");
        change(
            &r,
            "dose",
            "accepted",
            "approved",
            "## Interfaces\n\n- ADD contracts/events: add the event contract.\n",
        );
        r.commit("base");
        r.git(&["checkout", "-q", "-b", "feature"]);
        std::fs::remove_dir_all(r.dir.join("docs/intent/dose")).unwrap();
        if with_document {
            r.write("docs/contracts/events.md", EVENTS);
        }
        r.commit("finish dose");
        r
    };
    // No document and no delta line that adds it.
    let r = setup(false);
    let (_, out, _) = r.run(&["check", "--all", "--base", "main"]);
    let got = messages(&out, "SD914");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].contains(
            "the spec adds the interface `contracts/events`, and the tree has no such document; write it in this pull request, or add its requirements with `ADD contracts/events#<key>` lines"
        ),
        "{out}"
    );
    // The document is in the diff.
    let r = setup(true);
    let (_, out, _) = r.run(&["check", "--all", "--base", "main"]);
    assert!(severities(&out, "SD914").is_empty(), "{out}");
}

// SD925

#[test]
fn an_item_with_no_box_in_concerns_gives_sd925() {
    matrix("SD925", [None, Some("warning"), Some("error")], |r| {
        change(
            r,
            "dose",
            "accepted",
            "approved",
            "## Concerns\n\n- The cache can grow without a limit.\n",
        )
    });
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        &format!(
            "{}\n## Concerns\n\n- The cache can grow without a limit.\n",
            delta(&["- ADD requirements/valves#flush: The controller SHALL flush the line."])
        ),
    );
    let (code, out) = check(&r);
    assert_eq!(code, 0, "a warning does not fail check: {out}");
    let got = messages(&out, "SD925");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].starts_with("docs/intent/dose/spec.md:12: warning SD925 checkbox-missing: "),
        "{out}"
    );
    assert!(
        got[0].contains(
            "an item of `Concerns` has no task box, so SD919 cannot see if it is open; write `- [ ]` for an open concern or `- [x]` for a resolved one"
        ),
        "{out}"
    );
    // The spec has no other diagnostic for the item.
    assert!(severities(&out, "SD919").is_empty(), "{out}");
}

#[test]
fn an_item_with_no_box_in_open_questions_names_sd922() {
    matrix("SD925", [None, Some("warning"), Some("error")], |r| {
        change(
            r,
            "dose",
            "accepted",
            "approved",
            "## Open questions\n\n- Who owns the timer?\n",
        )
    });
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        "## Open questions\n\n- Who owns the timer?\n",
    );
    let (_, out) = check(&r);
    let got = messages(&out, "SD925");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].contains(
            "an item of `Open questions` has no task box, so SD922 cannot see if it is open"
        ),
        "{out}"
    );
}

#[test]
fn sd925_reads_a_spec_of_each_status_except_a_closed_change() {
    let body = "## Concerns\n\n- No box.\n";
    for spec in ["draft", "approved"] {
        matrix("SD925", [None, Some("warning"), Some("error")], |r| {
            change(r, "dose", "draft", spec, body)
        });
    }
    never("SD925", |r| change(r, "dose", "closed", "approved", body));
}

#[test]
fn an_item_with_a_box_gives_no_sd925() {
    never("SD925", |r| {
        change(
            r,
            "dose",
            "accepted",
            "draft",
            "## Concerns\n\n- [x] Resolved.\n- [ ] Open.\n\n## Open questions\n\n- [x] Answered. Yes.\n- [ ] Open.\n",
        )
    });
}

#[test]
fn a_none_item_states_nothing_in_concerns_and_open_questions() {
    // The box does not matter: one item that starts with `none` states
    // "nothing", so no SD919, SD922 or SD925 reads it.
    for item in [
        "- [ ] None.",
        "- None.",
        "- [x] none",
        "- none",
        "- None: no risk.",
    ] {
        let setup = |r: &Repo| {
            change(
                r,
                "dose",
                "accepted",
                "approved",
                &format!("## Concerns\n\n{item}\n\n## Open questions\n\n{item}\n"),
            )
        };
        never("SD925", setup);
        never("SD919", setup);
        never("SD922", setup);
    }
    // The 0.6.1 result for a real concern that starts with the word `None`.
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        "## Concerns\n\n- [ ] None of the old files stay.\n",
    );
    let (_, out) = check(&r);
    assert_eq!(severities(&out, "SD919"), ["error"], "{out}");
    // A `none` item with other items is an item like the others.
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        "## Open questions\n\n- [ ] None.\n- [ ] Who owns it?\n",
    );
    let (_, out) = check(&r);
    assert_eq!(severities(&out, "SD922"), ["error", "error"], "{out}");
}

#[test]
fn an_add_item_with_a_bad_id_gives_sd914() {
    // A glob, `..` and an absolute path are no document id. The id would
    // widen the scope of SD913 if it passed.
    for id in [
        "contracts/*",
        "contracts/a?",
        "contracts/[ab]",
        "contracts/../x",
        "/abs",
    ] {
        let r = repo("standard");
        change(
            &r,
            "dose",
            "draft",
            "draft",
            &format!("## Interfaces\n\n- ADD {id}: anything.\n"),
        );
        let (_, out) = check(&r);
        let got = messages(&out, "SD914");
        assert_eq!(got.len(), 1, "{id}: {out}");
        assert!(
            got[0].contains(&format!("`{id}` is not a document id")),
            "{id}: {out}"
        );
    }
}

#[test]
fn a_place_with_no_type_is_not_named_as_requirements() {
    let r = repo("standard");
    change(
        &r,
        "dose",
        "draft",
        "draft",
        "## Interfaces\n\n- ADD foo/bar: anything.\n",
    );
    let (_, out) = check(&r);
    let got = messages(&out, "SD914");
    assert_eq!(got.len(), 1, "{out}");
    assert!(
        got[0].contains("the place of `foo/bar` gives no type, not `interface`"),
        "{out}"
    );
}

#[test]
fn a_document_of_another_type_does_not_satisfy_an_add_item_after_the_folder_is_deleted() {
    let r = repo("standard");
    change(
        &r,
        "dose",
        "accepted",
        "approved",
        "## Interfaces\n\n- ADD contracts/new: add the contract.\n",
    );
    r.commit("base");
    r.git(&["checkout", "-q", "-b", "feature"]);
    std::fs::remove_dir_all(r.dir.join("docs/intent/dose")).unwrap();
    r.write(
        "docs/contracts/new.md",
        &EVENTS.replace("status: active\n", "status: active\ntype: requirements\n"),
    );
    r.commit("finish dose");
    let (_, out, _) = r.run(&["check", "--all", "--base", "main"]);
    let got = messages(&out, "SD914");
    assert!(
        got.iter()
            .any(|l| l.contains("`contracts/new` is a `requirements` document, not an interface")),
        "{out}"
    );
}

// The budget of a plan (design 0.7.0, section 15)

/// A draft plan of `n` tasks of about 310 characters each.
fn plan_of(n: usize) -> String {
    let tasks: String = (1..=n)
        .map(|i| format!("- [ ] t{i}: Write the failing test of the close time of valve {i}, then close the valve within 2 seconds and log the time of the close in the report of the run. Files: `src/valve_{i}.py`, `tests/test_valve_{i}.py`. Verify: `python3 -m pytest tests/test_valve_{i}.py` passes, and the new test failed before the code.\n"))
        .collect();
    format!(
        "---\nstatus: draft\n---\n# Faster close\n\n## Files\n\n- `src/`\n- `tests/`\n\n## Order\n\n{tasks}\n## Risks\n\n- A slow valve.\n\n## Proof\n\nThe tests pass.\n"
    )
}

/// A fixture with a draft intent, the plan `plan`, and the extra tables `more`.
fn repo_with_plan(plan: &str, more: &str) -> Repo {
    let r = repo_with(&format!(
        "{}\n{more}",
        config("standard", "", "max-started = 1\n", "")
    ));
    r.write(
        "docs/intent/faster-close/intent.md",
        "---\nstatus: draft\nclass: routine\n---\n# Faster close\n\n## Intent\n\nText.\n",
    );
    r.write("docs/intent/faster-close/plan.md", plan);
    r
}

/// The codes SD701 to SD704 of the text of `check`.
fn budget_codes(out: &str) -> Vec<&'static str> {
    ["SD701", "SD702", "SD703", "SD704"]
        .into_iter()
        .filter(|c| out.contains(&format!(" {c} ")))
        .collect()
}

#[test]
fn a_plan_of_40_tasks_gives_no_budget_diagnostic() {
    let r = repo_with_plan(&plan_of(40), "");
    let (code, out) = check(&r);
    assert!(budget_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn budgets_plan_gives_the_plan_a_budget() {
    let more = "[budgets.plan]\nsoft = 3000\nhard = 9000\n";
    // About 4,000 characters: over the soft budget.
    let r = repo_with_plan(&plan_of(12), more);
    let (code, out) = check(&r);
    assert_eq!(budget_codes(&out), ["SD701"], "{out}");
    assert_eq!(code, 0, "{out}");
    // About 12,500 characters: over the hard budget of a new document.
    let r = repo_with_plan(&plan_of(40), more);
    let (code, out) = check(&r);
    assert!(budget_codes(&out).contains(&"SD702"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn a_zero_budget_turns_the_budget_of_a_type_off() {
    let intent = format!(
        "---\nstatus: draft\nclass: routine\n---\n# Faster close\n\n## Intent\n\n{}\n",
        "The valve closes too slowly, and the bed overflows.\n\n".repeat(300)
    );
    let r = repo_with_plan(&plan_of(1), "");
    r.write("docs/intent/faster-close/intent.md", &intent);
    let (code, out) = check(&r);
    assert!(budget_codes(&out).contains(&"SD702"), "{out}");
    assert_eq!(code, 1, "{out}");
    let r = repo_with_plan(&plan_of(1), "[budgets.intent]\nsoft = 0\nhard = 0\n");
    r.write("docs/intent/faster-close/intent.md", &intent);
    let (code, out) = check(&r);
    assert!(budget_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_budgets_plan_table_with_one_field_takes_the_old_default_for_the_other() {
    // Only `soft`: `hard` is the default of 0.6.1, 12,000. A plan of about
    // 12,500 characters is over both.
    let r = repo_with_plan(&plan_of(40), "[budgets.plan]\nsoft = 3000\n");
    let (code, out) = check(&r);
    assert_eq!(budget_codes(&out), ["SD702"], "{out}");
    assert_eq!(code, 1, "{out}");
    // Only `soft`, above the old `hard`: `hard` follows `soft`, with no
    // configuration error.
    let r = repo_with_plan(&plan_of(40), "[budgets.plan]\nsoft = 15000\n");
    let (code, out) = check(&r);
    assert!(budget_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
    // Only `hard`: `soft` is the default of 0.6.1, 6,000.
    let r = repo_with_plan(&plan_of(12), "[budgets.plan]\nhard = 20000\n");
    let (code, out) = check(&r);
    assert!(budget_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
    let r = repo_with_plan(&plan_of(25), "[budgets.plan]\nhard = 20000\n");
    let (code, out) = check(&r);
    assert_eq!(budget_codes(&out), ["SD701"], "{out}");
    assert_eq!(code, 0, "{out}");
    // Only `hard`, below the old `soft`: no configuration error.
    let r = repo_with_plan(&plan_of(1), "[budgets.plan]\nhard = 2000\n");
    let (code, out) = check(&r);
    assert!(budget_codes(&out).is_empty(), "{out}");
    assert_eq!(code, 0, "{out}");
}

// SD913: the files of the tool, and `[change] shared-paths` (0.7.1, B3 and B8)

/// A branch with an approved plan whose `Files` is `src/dose.py`. `change` has
/// more keys of `[change]`. The base `main` has the plan, and `feature` is the
/// branch. A test edits files on `feature`.
fn scoped(change: &str) -> Repo {
    let r = repo_with(&config(
        "standard",
        &format!("gate = \"warn\"\n{change}"),
        "",
        "",
    ));
    r.write(
        "docs/intent/dose/intent.md",
        "---\nstatus: accepted\nclass: routine\n---\n# Dose\n\n## Intent\n\nThe dose is too high.\n",
    );
    r.write(
        "docs/intent/dose/plan.md",
        "---\nstatus: approved\n---\n# Plan\n\n## Files\n\n- `src/dose.py`\n\n## Order\n\n- Edit the code.\n\n## Risks\n\n- None.\n\n## Proof\n\n- A test.\n",
    );
    r.write("src/dose.py", "x = 1\n");
    r.commit("base");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r
}

const QUESTIONS: &str = "---\ntype: research\n---\n# Questions\n";

/// The paths of the SD913 lines of `check` on the branch.
fn outside(r: &Repo) -> Vec<String> {
    let (_, out, _) = r.run(&["check"]);
    out.lines()
        .filter(|l| l.contains(" SD913 "))
        .map(|l| l.split(':').next().unwrap().to_string())
        .collect()
}

#[test]
fn sd913_skips_the_files_that_the_tool_writes() {
    let r = scoped("");
    r.write("src/dose.py", "x = 2\n");
    for p in [
        "AGENTS.md",
        "CLAUDE.md",
        ".claude/skills/spec-dna/SKILL.md",
        ".pre-commit-config.yaml",
        ".husky/pre-commit",
        "tools/spec-dna-pin.toml",
    ] {
        r.write(p, "text\n");
    }
    let cfg = r.read("spec-dna.toml");
    r.write("spec-dna.toml", &format!("{cfg}\n# A comment.\n"));
    assert_eq!(outside(&r), Vec::<String>::new());
    // A file that is no file of the tool is still outside.
    r.write("src/other.py", "y = 1\n");
    assert_eq!(outside(&r), ["src/other.py"]);
}

#[test]
fn sd913_skips_a_path_of_shared_paths() {
    // The edit of `src/dose.py` ties the diff to the change.
    let r = scoped("");
    r.write("src/dose.py", "x = 2\n");
    r.write("docs/questions.md", QUESTIONS);
    assert_eq!(outside(&r), ["docs/questions.md"]);
    let r = scoped("shared-paths = [\"docs/questions.md\", \"notes/**\"]\n");
    r.write("src/dose.py", "x = 2\n");
    r.write("docs/questions.md", QUESTIONS);
    r.write("notes/a/b.txt", "text\n");
    r.write("src/other.py", "y = 1\n");
    assert_eq!(outside(&r), ["src/other.py"]);
}

// The invariants and the deviations (design 0.8.0, D6 and D7, slice S5)

const RULE_DOC: &str = "---\nstatus: active\napplies-to: [\"**\"]\n---\n# Safety\n\n## Rules\n\n- **valve-closed**: The controller SHALL close each valve on a fault.\n- **alarm-sounds**: The controller SHALL sound the alarm on a fault.\n";

fn inv_config(profile: &str, keys: &str) -> String {
    format!(
        "profile = \"{profile}\"\n\n[change]\npolicy = \"three-class\"\n{keys}\n[rules]\nSD608 = \"off\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.rules]\ntypes = [\"rule\"]\nmoves-to = [\"requirements\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\nmoves-to = [\"requirements\", \"rules\"]\nkeep = \"until-finished\"\n"
    )
}

fn inv_repo(profile: &str, keys: &str) -> Repo {
    let r = Repo::with_config(&inv_config(profile, keys));
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/rules/safety.md", RULE_DOC);
    r.write("README.md", "# Project\n");
    r
}

const INV_DELTA: &str =
    "- ADD requirements/valves#timeout: The controller SHALL close the valve after a timeout.";

/// The change `dose` with a spec that has the delta `delta` and the section
/// `Invariants` with `items`.
fn inv_spec(r: &Repo, status: &str, delta: &str, items: &str) {
    r.write(
        "docs/intent/dose/intent.md",
        "---\nstatus: accepted\nclass: routine\n---\n# Change\n\n## Intent\n\nText.\n",
    );
    r.write(
        "docs/intent/dose/spec.md",
        &format!(
            "---\nstatus: {status}\n---\n# Spec\n\n## Delta\n\n{delta}\n\n## Invariants\n\n{items}\n"
        ),
    );
}

fn pin(r: &Repo, key: &str) -> String {
    format!(
        "rules/safety#{key}@{}",
        hash(r, &format!("rules/safety#{key}"))
    )
}

/// A pull request: the base on `main`, then the branch `feature`. The base commit
/// pins 0.8.0, as a project that adopted 0.8.0 has it, and a later commit moves
/// the pin to the version of the binary.
fn on_branch(r: &Repo) {
    let cfg = r.read("spec-dna.toml");
    r.write(
        "spec-dna.toml",
        &cfg.replacen(
            &format!("spec-dna = \"{VERSION}\""),
            "spec-dna = \"0.8.0\"",
            1,
        ),
    );
    r.commit("base");
    r.write("spec-dna.toml", &cfg);
    r.write("README.md", "# Project, new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
}

fn codes_of(out: &str, code: &str) -> usize {
    out.lines()
        .filter(|l| l.contains(&format!(" {code} ")))
        .count()
}

#[test]
fn an_item_of_a_requirements_document_gives_sd936() {
    let r = inv_repo("standard", "invariants = true\n");
    let h = hash(&r, "requirements/valves#close");
    inv_spec(
        &r,
        "draft",
        INV_DELTA,
        &format!("- requirements/valves#close@{h}"),
    );
    let (code, out) = check(&r);
    assert_eq!(code, 1, "{out}");
    assert_eq!(severities(&out, "SD936"), ["error"], "{out}");
    let m = messages(&out, "SD936");
    assert!(m[0].contains("not a rule"), "{}", m[0]);
    assert!(m[0].contains("docs/intent/dose/spec.md:"), "{}", m[0]);
}

#[test]
fn a_bad_item_a_missing_rule_and_a_missing_key_give_sd936() {
    let r = inv_repo("standard", "invariants = true\n");
    let h = hash(&r, "rules/safety#valve-closed");
    let items = format!(
        "- The valve stays closed.\n- rules/safety#nothing@{h}\n- rules/absent#valve-closed@{h}\n- rules/safety#valve-closed"
    );
    inv_spec(&r, "draft", INV_DELTA, &items);
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD936"), 4, "{out}");
    assert_eq!(codes_of(&out, "SD937"), 0, "{out}");
}

#[test]
fn the_item_none_is_valid_alone_and_not_with_a_pin() {
    let r = inv_repo("standard", "invariants = true\n");
    inv_spec(&r, "draft", INV_DELTA, "- none");
    let (code, out) = check(&r);
    assert_eq!(codes_of(&out, "SD936"), 0, "{out}");
    assert_eq!(code, 0, "{out}");
    let p = pin(&r, "valve-closed");
    inv_spec(&r, "draft", INV_DELTA, &format!("- none\n- {p}"));
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD936"), 1, "{out}");
    assert!(messages(&out, "SD936")[0].contains("stands alone"), "{out}");
}

#[test]
fn a_good_pin_gives_no_sd936_and_no_sd937_also_with_crlf_line_ends() {
    let r = inv_repo("standard", "invariants = true\n");
    let p = pin(&r, "valve-closed");
    inv_spec(&r, "approved", INV_DELTA, &format!("- {p}"));
    let (code, out) = check(&r);
    assert_eq!(code, 0, "{out}");
    let spec = r.read("docs/intent/dose/spec.md").replace('\n', "\r\n");
    r.write("docs/intent/dose/spec.md", &spec);
    let (code, out) = check(&r);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_rule_that_the_same_delta_adds_or_removes_is_no_invariant_before_the_approval() {
    let r = inv_repo("standard", "invariants = true\n");
    let h = hash(&r, "rules/safety#valve-closed");
    // REMOVE: the rule exists, so only the delta can name it.
    let delta = format!("- REMOVE rules/safety#valve-closed@{h}");
    inv_spec(
        &r,
        "draft",
        &delta,
        &format!("- rules/safety#valve-closed@{h}"),
    );
    let (_, out) = check(&r);
    let m = messages(&out, "SD936");
    assert_eq!(m.len(), 1, "{out}");
    assert!(
        m[0].contains("removes the rule") && m[0].contains("remove the item"),
        "{}",
        m[0]
    );
    // ADD: the rule is new, so it has no hash to pin. The text says the cause.
    let delta = "- ADD rules/safety#new-rule: The controller SHALL log each fault.";
    inv_spec(&r, "draft", delta, &format!("- rules/safety#new-rule@{h}"));
    let (_, out) = check(&r);
    let m = messages(&out, "SD936");
    assert_eq!(m.len(), 1, "{out}");
    assert!(
        m[0].contains("adds the rule") && m[0].contains("remove the item"),
        "{}",
        m[0]
    );
    assert_eq!(codes_of(&out, "SD937"), 0, "{out}");
}

#[test]
fn a_change_line_for_the_rule_is_allowed_and_the_item_compares_with_the_line() {
    let r = inv_repo("standard", "invariants = true\n");
    let old = hash(&r, "rules/safety#valve-closed");
    let delta = format!(
        "- CHANGE rules/safety#valve-closed@{old}: The controller SHALL close each valve within 2 seconds of a fault."
    );
    inv_spec(
        &r,
        "approved",
        &delta,
        &format!("- rules/safety#valve-closed@{old}"),
    );
    let (code, out) = check(&r);
    assert_eq!(code, 0, "{out}");
    // The rule has new text now (the state after `finish`): the item still
    // equals the line, so no SD937.
    let new_rule = RULE_DOC.replace(
        "close each valve on a fault",
        "close each valve within 2 seconds of a fault",
    );
    r.write("docs/rules/safety.md", &new_rule);
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD937"), 0, "{out}");
    // Another hash than the line: SD937. It names the hash of the line.
    let other = hash(&r, "rules/safety#alarm-sounds");
    inv_spec(
        &r,
        "approved",
        &delta,
        &format!("- rules/safety#valve-closed@{other}"),
    );
    let (_, out) = check(&r);
    let m = messages(&out, "SD937");
    assert_eq!(m.len(), 1, "{out}");
    assert!(
        m[0].contains("CHANGE line") && m[0].contains(&old),
        "{}",
        m[0]
    );
}

#[test]
fn an_old_hash_is_sd937_error_for_a_touched_spec_and_a_warning_for_an_untouched_one() {
    let r = inv_repo("standard", "invariants = true\n");
    let p = pin(&r, "valve-closed");
    inv_spec(&r, "approved", INV_DELTA, &format!("- {p}"));
    on_branch(&r);
    // The rule changes on the branch: the spec is not in the diff.
    let new_rule = RULE_DOC.replace(
        "close each valve on a fault.",
        "close each valve within 2 seconds of a fault.",
    );
    r.write("docs/rules/safety.md", &new_rule);
    r.commit("the rule changes");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(severities(&out, "SD937"), ["warning"], "{out}");
    assert_eq!(code, 0, "{out}");
    // The spec is in the diff: an error, with the new hash in the message.
    let spec = r
        .read("docs/intent/dose/spec.md")
        .replace("## Invariants", "## Invariants\n");
    r.write("docs/intent/dose/spec.md", &spec);
    r.commit("touch the spec");
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(severities(&out, "SD937"), ["error"], "{out}");
    assert_eq!(code, 1, "{out}");
    let now = hash(&r, "rules/safety#valve-closed");
    assert!(messages(&out, "SD937")[0].contains(&now), "{out}");
}

#[test]
fn the_codes_need_the_key_and_the_profile() {
    let setup = |r: &Repo| {
        let items = "- requirements/valves#close@0123456789abcdef\n- not a pin".to_string();
        inv_spec(r, "draft", INV_DELTA, &items);
    };
    let fresh = |cfg: &str| {
        let r = Repo::with_config(cfg);
        r.write("docs/requirements/valves.md", VALVES);
        r.write("docs/rules/safety.md", RULE_DOC);
        setup(&r);
        r
    };
    // No key: no SD936 and no SD937 in any profile, also with a group rule.
    for profile in ["lite", "standard", "strict"] {
        for rules in ["", "acceptance = \"error\"\n"] {
            let cfg = inv_config(profile, "").replace("[rules]\n", &format!("[rules]\n{rules}"));
            let (_, out) = check(&fresh(&cfg));
            assert_eq!(
                codes_of(&out, "SD936") + codes_of(&out, "SD937"),
                0,
                "{profile} {out}"
            );
        }
    }
    // The key in `lite` gives nothing, as the profile is off.
    let (_, out) = check(&fresh(&inv_config("lite", "invariants = true\n")));
    assert_eq!(codes_of(&out, "SD936"), 0, "{out}");
    // The key in `lite` and a rule for the code: it runs.
    let cfg = inv_config("lite", "invariants = true\n")
        .replace("[rules]\n", "[rules]\nSD936 = \"warning\"\n");
    let (_, out) = check(&fresh(&cfg));
    assert_eq!(severities(&out, "SD936"), ["warning", "warning"], "{out}");
    // `standard` and `strict` with the key: errors.
    for profile in ["standard", "strict"] {
        let (_, out) = check(&fresh(&inv_config(profile, "invariants = true\n")));
        assert_eq!(
            severities(&out, "SD936"),
            ["error", "error"],
            "{profile} {out}"
        );
    }
}

#[test]
fn a_spec_that_a_commit_added_at_pin_0_7_1_keeps_its_free_text_and_a_new_spec_does_not() {
    let free = "- The controller keeps the valve closed.\n- Another note.";
    let keyed = inv_config("standard", "invariants = true\n");
    // The spec comes on `main` under the pin 0.7.1, and a later commit pins the
    // version of the binary (a project that upgraded and set the key).
    let r = Repo::new();
    r.write("spec-dna.toml", &format!("spec-dna = \"0.7.1\"\n{keyed}"));
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/rules/safety.md", RULE_DOC);
    r.write("README.md", "# Project\n");
    inv_spec(&r, "approved", INV_DELTA, free);
    r.commit("spec at 0.7.1");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{keyed}"),
    );
    r.write("README.md", "# Project, new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/a.py", "x = 1\n");
    r.commit("code");
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(codes_of(&out, "SD936"), 0, "{out}");
    // A spec that the branch adds is always read.
    let r = inv_repo("standard", "invariants = true\n");
    on_branch(&r);
    inv_spec(&r, "draft", INV_DELTA, free);
    r.commit("spec");
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(codes_of(&out, "SD936"), 2, "{out}");
}

/// The test merge of a code host: `main` moves on with a commit to another
/// path, and `HEAD` is a merge commit of the branch.
fn in_test_merge(r: &Repo) {
    r.git(&["checkout", "-q", "main"]);
    let n = r.git_out(&["rev-list", "--count", "main"]);
    r.write("README.md", &format!("# Project, main moves on {n}\n"));
    r.commit("main moves on");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "--detach", "main"]);
    r.git(&["merge", "-q", "--no-ff", "-m", "test merge", "feature"]);
}

#[test]
fn the_invariant_codes_give_the_same_result_on_the_test_merge_and_after_main_merged_in() {
    let r = inv_repo("standard", "invariants = true\n");
    on_branch(&r);
    inv_spec(
        &r,
        "draft",
        INV_DELTA,
        &format!("- {}", pin(&r, "valve-closed")),
    );
    r.commit("an honest spec");
    in_test_merge(&r);
    let (code, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(code, 0, "{out}");
    // A defect: the same on the test merge.
    r.git(&["checkout", "-q", "feature"]);
    inv_spec(
        &r,
        "draft",
        INV_DELTA,
        "- rules/safety#valve-closed@0123456789abcdef",
    );
    r.commit("a stale pin");
    in_test_merge(&r);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(severities(&out, "SD937"), ["error"], "{out}");
    // `main` merged into the branch, with a commit to another path.
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "-m", "merge main", "main"]);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(severities(&out, "SD937"), ["error"], "{out}");
}

// SD939 and SD940

const PLAN_DEV: &str = "## Files\n\n- `src/dose.py`\n\n## Order\n\n- [ ] t1: Code. Files: `src/dose.py`. Verify: the test passes.\n\n## Risks\n\n- None.\n\n## Proof\n\nThe test.\n";

fn dev_plan(r: &Repo, items: &str) {
    r.write(
        "docs/intent/dose/plan.md",
        &format!("---\nstatus: draft\n---\n# Plan\n\n{PLAN_DEV}\n## Deviations\n\n{items}\n"),
    );
}

#[test]
fn a_bad_deviation_item_gives_sd939() {
    for bad in [
        "- d1: text",
        "- d1 (other): text",
        "- d1 (plan) text",
        "- d1 (plan):",
        "- d1 (plan): a\n- d1 (plan): b",
    ] {
        let r = inv_repo("standard", "");
        inv_spec(&r, "approved", INV_DELTA, "- none");
        dev_plan(&r, bad);
        let (code, out) = check(&r);
        assert_eq!(codes_of(&out, "SD939"), 1, "{bad}: {out}");
        assert_eq!(severities(&out, "SD939"), ["error"], "{bad}: {out}");
        assert_eq!(code, 1, "{bad}: {out}");
    }
    let r = inv_repo("standard", "");
    dev_plan(
        &r,
        "- d1 (plan): The check moved.\n- d2 (plan): The order changed.",
    );
    let (code, out) = check(&r);
    assert_eq!(
        codes_of(&out, "SD939") + codes_of(&out, "SD940"),
        0,
        "{out}"
    );
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_spec_deviation_needs_a_delta_line_or_a_follow_up_folder() {
    let r = inv_repo("standard", "");
    inv_spec(&r, "approved", INV_DELTA, "- none");
    // No label.
    dev_plan(&r, "- d1 (spec): The valve closes after 2 reads.");
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD940"), 1, "{out}");
    assert_eq!(severities(&out, "SD940"), ["error"], "{out}");
    // A delta line of the spec: fine. A delta line that the spec has not: SD940.
    dev_plan(
        &r,
        "- d1 (spec): The valve closes after 2 reads. Delta: ADD requirements/valves#timeout",
    );
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD940"), 0, "{out}");
    dev_plan(
        &r,
        "- d1 (spec): The valve closes after 2 reads. Delta: CHANGE requirements/valves#timeout",
    );
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD940"), 1, "{out}");
    assert!(
        messages(&out, "SD940")[0].contains("no line of the delta"),
        "{out}"
    );
    dev_plan(
        &r,
        "- d1 (spec): x. Delta: UPDATE requirements/valves#timeout",
    );
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD940"), 1, "{out}");
    // A change folder of the tree: fine. A folder that does not exist: SD940.
    r.write(
        "docs/intent/later/intent.md",
        "---\nstatus: draft\nclass: routine\n---\n# Later\n\n## Intent\n\nText.\n",
    );
    dev_plan(&r, "- d1 (spec): x. Follow-up: later");
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD940"), 0, "{out}");
    dev_plan(&r, "- d1 (spec): x. Follow-up: nothing-here");
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD940"), 1, "{out}");
    assert!(
        messages(&out, "SD940")[0].contains("`Follow-up: nothing-here`"),
        "{out}"
    );
    // A `plan` item needs neither.
    dev_plan(&r, "- d1 (plan): x.");
    let (_, out) = check(&r);
    assert_eq!(codes_of(&out, "SD940"), 0, "{out}");
}

#[test]
fn the_deviation_codes_are_off_in_lite_and_on_in_standard_and_strict_with_no_key() {
    for (profile, want) in [("lite", 0), ("standard", 2), ("strict", 2)] {
        let r = inv_repo(profile, "");
        inv_spec(&r, "approved", INV_DELTA, "- none");
        dev_plan(&r, "- d1: text\n- d2 (spec): text");
        let (_, out) = check(&r);
        assert_eq!(
            codes_of(&out, "SD939") + codes_of(&out, "SD940"),
            want,
            "{profile} {out}"
        );
    }
}

#[test]
fn a_plan_that_a_commit_added_at_pin_0_7_1_keeps_a_section_deviations_of_plain_text() {
    let plain = inv_config("standard", "");
    let r = Repo::new();
    r.write("spec-dna.toml", &format!("spec-dna = \"0.7.1\"\n{plain}"));
    r.write("docs/requirements/valves.md", VALVES);
    r.write("README.md", "# Project\n");
    inv_spec(&r, "approved", INV_DELTA, "- none");
    dev_plan(&r, "- It went well.\n- d9 (spec): text");
    r.commit("plan at 0.7.1");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{plain}"),
    );
    r.write("README.md", "# Project, new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/dose.py", "x = 1\n");
    r.commit("code");
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(
        codes_of(&out, "SD939") + codes_of(&out, "SD940"),
        0,
        "{out}"
    );
    // A plan that the branch adds is always read.
    let r = inv_repo("standard", "");
    on_branch(&r);
    inv_spec(&r, "draft", INV_DELTA, "- none");
    dev_plan(&r, "- It went well.");
    r.commit("plan");
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(codes_of(&out, "SD939"), 1, "{out}");
}

#[test]
fn the_deviation_codes_give_the_same_result_on_the_test_merge() {
    let r = inv_repo("standard", "");
    on_branch(&r);
    inv_spec(&r, "draft", INV_DELTA, "- none");
    dev_plan(&r, "- d1 (plan): An honest note.");
    r.commit("plan");
    in_test_merge(&r);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(
        codes_of(&out, "SD939") + codes_of(&out, "SD940"),
        0,
        "{out}"
    );
    r.git(&["checkout", "-q", "feature"]);
    dev_plan(&r, "- d1 (spec): No fix.");
    r.commit("a spec deviation with no fix");
    in_test_merge(&r);
    let (_, out, _) = r.run(&["check", "--base", "main"]);
    assert_eq!(codes_of(&out, "SD940"), 1, "{out}");
}

/// The output of `check --base main` in the 3 states of a pull request: the tip
/// of the branch, the test merge of a code host, and the branch after `main`
/// merged into it. `main` moves on with a commit to another path before the last
/// 2 states. The repository is on the branch `feature`.
fn three_states(r: &Repo) -> [String; 3] {
    let tip = r.run(&["check", "--base", "main"]).1;
    in_test_merge(r);
    let merge = r.run(&["check", "--base", "main"]).1;
    r.git(&["checkout", "-q", "feature"]);
    r.git(&["merge", "-q", "-m", "merge main", "main"]);
    let merged_in = r.run(&["check", "--base", "main"]).1;
    [tip, merge, merged_in]
}

#[test]
fn each_defect_of_the_new_codes_gives_the_same_code_in_the_3_states_of_a_pull_request() {
    // (setup, code, number of findings)
    type Setup<'a> = &'a dyn Fn(&Repo);
    let cases: [(&str, Setup); 3] = [
        ("SD936", &|r| {
            inv_spec(
                r,
                "draft",
                INV_DELTA,
                "- requirements/valves#close@0123456789abcdef",
            );
        }),
        ("SD939", &|r| {
            inv_spec(r, "draft", INV_DELTA, "- none");
            dev_plan(r, "- d1: no kind");
        }),
        ("SD940", &|r| {
            inv_spec(r, "draft", INV_DELTA, "- none");
            dev_plan(r, "- d1 (spec): no fix");
        }),
    ];
    for (code, setup) in cases {
        let r = inv_repo("standard", "invariants = true\n");
        on_branch(&r);
        setup(&r);
        r.commit("a defect");
        for (state, out) in three_states(&r).iter().enumerate() {
            assert_eq!(codes_of(out, code), 1, "{code} in state {state}: {out}");
        }
    }
}

#[test]
fn the_rule_of_the_pin_gives_the_same_silence_in_the_3_states_of_a_pull_request() {
    let free = "- The controller keeps the valve closed.";
    let keyed = inv_config("standard", "invariants = true\n");
    let r = Repo::new();
    r.write("spec-dna.toml", &format!("spec-dna = \"0.7.1\"\n{keyed}"));
    r.write("docs/requirements/valves.md", VALVES);
    r.write("docs/rules/safety.md", RULE_DOC);
    r.write("README.md", "# Project\n");
    inv_spec(&r, "approved", INV_DELTA, free);
    dev_plan(&r, "- It went well.");
    r.commit("spec and plan at 0.7.1");
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n{keyed}"),
    );
    r.write("README.md", "# Project, new pin\n");
    r.commit("move the pin");
    origin_head(&r.dir, "main");
    r.git(&["checkout", "-q", "-b", "feature"]);
    r.write("src/a.py", "x = 1\n");
    r.commit("code");
    for (state, out) in three_states(&r).iter().enumerate() {
        for code in ["SD936", "SD937", "SD939", "SD940"] {
            assert_eq!(codes_of(out, code), 0, "{code} in state {state}: {out}");
        }
    }
}

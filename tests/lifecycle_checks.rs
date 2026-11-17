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

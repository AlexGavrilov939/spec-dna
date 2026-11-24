//! Integration tests of the roll-up checks: SD641 to SD645 of the group
//! `rollup` (design 0.8.0, D50 to D52), and the rule that a project with no
//! `[layers]` table keeps its types (P5). The fixture is a small valve
//! project with the layers `requirements` and `summaries`. The helpers come
//! from tests/common.

mod common;

use common::Repo;

const CLOSING: &str = "---\nstatus: active\n---\n# Closing\n\n## Requirements\n\n- **close-on-timeout**: When the sensor stops, the valve SHALL close after 30 seconds.\n- **max-angle**: The valve SHALL NOT open more than 80 degrees.\n";

const ZONES: &str = "---\nstatus: active\n---\n# Zones\n\n## Requirements\n\n- **zone-count**: The controller SHALL support 8 zones.\n";

const MOTOR: &str = "---\nstatus: active\n---\n# Motor\n\n## Requirements\n\n- **spin-up**: The motor SHALL reach full speed in 2 seconds.\n";

const LAYERS: &str = "[layers.foundation]\ntypes = [\"product\", \"system\", \"terms\"]\nfiles = [\"product.md\", \"system.md\", \"terms.md\"]\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

/// A project of the profile `profile`, with `extra` before the layers.
fn project_with(profile: &str, extra: &str) -> Repo {
    let r = Repo::with_config(&format!("profile = \"{profile}\"\n{extra}\n{LAYERS}"));
    r.write("docs/requirements/valves/closing.md", CLOSING);
    r
}

fn project(profile: &str) -> Repo {
    project_with(profile, "")
}

/// The hash that `show` prints for an id.
fn shown_hash(r: &Repo, id: &str) -> String {
    let out = r.run(&["show", id]).1;
    let first = out.lines().next().unwrap_or("");
    let at = first
        .find(" h:")
        .unwrap_or_else(|| panic!("no hash: {out}"));
    first[at + 3..]
        .split_whitespace()
        .next()
        .unwrap()
        .to_string()
}

/// The lines of the output of `check` that carry `code`.
fn lines_of<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    // The finding lines, not the lines `<code> fix: ...`.
    out.lines()
        .filter(|l| {
            l.contains(&format!("error {code} ")) || l.contains(&format!("warning {code} "))
        })
        .collect()
}

fn check_all(r: &Repo) -> (i32, String) {
    let (code, out, err) = r.run(&["check", "--all"]);
    (code, format!("{out}{err}"))
}

/// The project with the summary `summaries/valves` that `new summary` wrote.
fn with_summary(profile: &str) -> Repo {
    with_summary_of(project(profile))
}

/// The same for a project that the test made.
fn with_summary_of(r: Repo) -> Repo {
    let (code, out, err) = r.run(&["new", "summary", "valves"]);
    assert_eq!(code, 0, "{out}{err}");
    // The body has no placeholder, so SD306 stays out of the way.
    let text = r.read("docs/summaries/valves.md");
    let at = text.find("# Valves").unwrap();
    r.write(
        "docs/summaries/valves.md",
        &format!(
            "{}# Valves\n\nA valve closes 30 seconds after its sensor stops. It never opens more than 80 degrees.\n",
            &text[..at]
        ),
    );
    r
}

fn edit(r: &Repo, path: &str, from: &str, to: &str) {
    let text = r.read(path);
    assert!(text.contains(from), "{path} has no `{from}`");
    r.write(path, &text.replacen(from, to, 1));
}

#[test]
fn a_summary_with_current_pins_gives_no_code_of_the_group() {
    for profile in ["lite", "standard", "strict"] {
        let r = with_summary(profile);
        let (_, out) = check_all(&r);
        assert!(!out.contains("SD64"), "{profile}: {out}");
    }
}

#[test]
fn a_changed_requirement_makes_its_pin_stale_an_error_in_standard_and_strict_and_off_in_lite() {
    for profile in ["standard", "strict"] {
        let r = with_summary(profile);
        edit(
            &r,
            "docs/requirements/valves/closing.md",
            "after 30 seconds",
            "after 20 seconds",
        );
        let (code, out) = check_all(&r);
        let found = lines_of(&out, "SD641");
        assert_eq!(found.len(), 1, "{profile}: {out}");
        assert!(
            found[0].contains("error SD641 summary-stale"),
            "{}",
            found[0]
        );
        assert!(
            found[0].contains("`requirements/valves/closing#close-on-timeout`")
                && found[0].contains("its text changed")
                && found[0].contains("spec-dna refresh summaries/valves"),
            "{}",
            found[0]
        );
        assert!(
            found[0].starts_with("docs/summaries/valves.md:"),
            "{}",
            found[0]
        );
        // The error fails the gate: the pin is new, because there is no base.
        assert_eq!(code, 1, "{profile}: {out}");
    }
    let r = with_summary("lite");
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 30 seconds",
        "after 20 seconds",
    );
    let (code, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_rule_of_the_group_turns_the_5_codes_off_or_down() {
    let r = with_summary_of(project_with("standard", "[rules]\nrollup = \"off\"\n"));
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 30 seconds",
        "after 20 seconds",
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD641"), "{out}");
    let r = with_summary_of(project_with("standard", "[rules]\nrollup = \"warning\"\n"));
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 30 seconds",
        "after 20 seconds",
    );
    let (code, out) = check_all(&r);
    assert!(out.contains("warning SD641 summary-stale"), "{out}");
    assert!(!out.contains("error SD641"), "{out}");
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_removed_requirement_gives_sd641_removed() {
    let r = with_summary("standard");
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "- **max-angle**: The valve SHALL NOT open more than 80 degrees.\n",
        "",
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD641");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`requirements/valves/closing#max-angle`")
            && found[0].contains("removed"),
        "{}",
        found[0]
    );
    assert!(lines_of(&out, "SD642").is_empty(), "{out}");
}

#[test]
fn a_requirement_that_is_no_longer_active_gives_sd641_not_active() {
    let r = with_summary("standard");
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "status: active",
        "status: retired",
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD641");
    // One for each pin of the document.
    assert_eq!(found.len(), 2, "{out}");
    assert!(found.iter().all(|l| l.contains("not active")), "{out}");
    assert!(lines_of(&out, "SD642").is_empty(), "{out}");
}

#[test]
fn a_new_requirement_gives_sd642_and_each_new_requirement_is_one_finding() {
    let r = with_summary("standard");
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "- **max-angle**",
        "- **min-angle**: The valve SHALL open at least 5 degrees.\n- **max-angle**",
    );
    let (code, out) = check_all(&r);
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("error SD642 summary-incomplete")
            && found[0].contains("`requirements/valves/closing#min-angle`")
            && found[0].contains("spec-dna refresh summaries/valves"),
        "{}",
        found[0]
    );
    assert!(lines_of(&out, "SD641").is_empty(), "{out}");
    assert_eq!(code, 1, "{out}");
    // A requirement that becomes active, and a new document: 1 finding each.
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "- **min-angle**: The valve SHALL open at least 5 degrees.\n",
        "",
    );
    r.write(
        "docs/requirements/valves/draft.md",
        &MOTOR.replace("status: active", "status: draft"),
    );
    let (_, out) = check_all(&r);
    assert!(lines_of(&out, "SD642").is_empty(), "a draft: {out}");
    edit(&r, "docs/requirements/valves/draft.md", "draft", "active");
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`requirements/valves/draft#spin-up`"),
        "{}",
        found[0]
    );
}

#[test]
fn a_requirement_of_another_area_is_not_in_the_frontier() {
    let r = with_summary("standard");
    r.write("docs/requirements/zones.md", ZONES);
    r.write("docs/requirements/sensors/probe.md", MOTOR);
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
}

#[test]
fn a_summary_of_a_sub_area_stands_for_its_requirements() {
    let r = with_summary("standard");
    r.write("docs/requirements/valves/drives/motor.md", MOTOR);
    // The new requirement of the sub-area is in the frontier of `valves`,
    // because the sub-area has no summary yet.
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`requirements/valves/drives/motor#spin-up`"),
        "{}",
        found[0]
    );
    // The sub-area gets a summary. Now `valves` must pin the summary, and the
    // requirement of the sub-area is no longer an item of `valves`.
    let (code, out, err) = r.run(&["new", "summary", "valves/drives"]);
    assert_eq!(code, 0, "{out}{err}");
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`summaries/valves/drives`"),
        "{}",
        found[0]
    );
    // A pin of the requirement of the sub-area in `valves` is stale: a
    // sub-summary now stands for it.
    let pin = format!(
        "  - requirements/valves/drives/motor#spin-up@{}\n",
        shown_hash(&r, "requirements/valves/drives/motor#spin-up")
    );
    edit(
        &r,
        "docs/summaries/valves.md",
        "---\n# Valves",
        &format!("{pin}---\n# Valves"),
    );
    let (_, out) = check_all(&r);
    let stale = lines_of(&out, "SD641");
    assert_eq!(stale.len(), 1, "{out}");
    assert!(
        stale[0].contains("motor#spin-up") && stale[0].contains("frontier"),
        "{}",
        stale[0]
    );
}

#[test]
fn a_pin_resolves_through_the_alias_of_a_moved_document() {
    let r = with_summary("standard");
    // The document moves to a new id. Its old id is an alias. The pin of the
    // summary names the old id and still matches.
    let moved = CLOSING.replace(
        "status: active\n",
        "status: active\naliases: [requirements/valves/closing]\n",
    );
    std::fs::remove_file(r.dir.join("docs/requirements/valves/closing.md")).unwrap();
    r.write("docs/requirements/valves/shutoff.md", &moved);
    let (_, out) = check_all(&r);
    assert!(lines_of(&out, "SD641").is_empty(), "{out}");
    assert!(lines_of(&out, "SD642").is_empty(), "{out}");
    assert!(lines_of(&out, "SD643").is_empty(), "{out}");
}

#[test]
fn a_pin_that_breaks_a_rule_of_form_gives_sd643() {
    let r = with_summary("standard");
    let hash = shown_hash(&r, "requirements/valves/closing#max-angle");
    let bad = [
        // Not in the pin form.
        "not-a-pin".to_string(),
        // A hash that is not 16 digits.
        format!("requirements/valves/closing#max-angle@{}", &hash[..12]),
        // No hash.
        "requirements/valves/closing#max-angle".to_string(),
        // A cross-repository reference.
        format!("other:requirements/valves/closing#max-angle@{hash}"),
        // A heading anchor, not a requirement.
        format!("requirements/valves/closing#requirements@{hash}"),
    ];
    for pin in bad {
        let text = r.read("docs/summaries/valves.md");
        let injected = text.replacen("---\n# Valves", &format!("  - {pin}\n---\n# Valves"), 1);
        r.write("docs/summaries/valves.md", &injected);
        let (code, out) = check_all(&r);
        let found = lines_of(&out, "SD643");
        assert_eq!(found.len(), 1, "{pin}: {out}");
        assert!(
            found[0].contains("error SD643 pin-invalid"),
            "{pin}: {}",
            found[0]
        );
        assert_eq!(code, 1, "{pin}: {out}");
        r.write("docs/summaries/valves.md", &text);
    }
}

#[test]
fn a_pin_to_the_parent_or_a_higher_level_gives_sd643() {
    let r = with_summary("standard");
    r.write("docs/requirements/valves/drives/motor.md", MOTOR);
    assert_eq!(r.run(&["new", "summary", "valves/drives"]).0, 0);
    // The summary of the sub-area pins its parent.
    let drives = r.read("docs/summaries/valves/drives.md");
    let injected = drives.replacen(
        "---\n# Drives",
        "  - summaries/valves@0000000000000000\n---\n# Drives",
        1,
    );
    r.write("docs/summaries/valves/drives.md", &injected);
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD643");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("summaries/valves/drives.md")
            && found[0].contains("`summaries/valves`")
            && found[0].contains("level"),
        "{}",
        found[0]
    );
    r.write("docs/summaries/valves/drives.md", &drives);
    // A `system` document that pins a `product` document.
    r.write(
        "docs/product.md",
        "# Product\n\n## Purpose\n\nIrrigation.\n\n## Scope\n\nValves.\n",
    );
    r.write(
        "docs/system.md",
        "---\nrests-on:\n  - product@0000000000000000\n---\n# System\n\n## Overview\n\nValves.\n\n## Constraints\n\n- None.\n",
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD643");
    assert!(
        found
            .iter()
            .any(|l| l.contains("docs/system.md") && l.contains("`product`")),
        "{out}"
    );
}

#[test]
fn a_pin_in_a_document_of_another_type_gives_sd643() {
    let r = with_summary("standard");
    let hash = shown_hash(&r, "requirements/valves/closing#max-angle");
    r.write(
        "docs/requirements/zones.md",
        &format!(
            "---\nstatus: active\nrests-on:\n  - requirements/valves/closing#max-angle@{hash}\n---\n# Zones\n\n## Requirements\n\n- **zone-count**: The controller SHALL support 8 zones.\n"
        ),
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD643");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("docs/requirements/zones.md") && found[0].contains("`requirements`"),
        "{}",
        found[0]
    );
}

#[test]
fn an_ambiguous_alias_gives_sd643() {
    let r = with_summary("standard");
    // 2 documents have the alias `old/closing`, and no document has that id.
    r.write(
        "docs/requirements/valves/a.md",
        &ZONES.replace(
            "status: active\n",
            "status: active\naliases: [old/closing]\n",
        ),
    );
    r.write(
        "docs/requirements/valves/b.md",
        &MOTOR.replace(
            "status: active\n",
            "status: active\naliases: [old/closing]\n",
        ),
    );
    let text = r.read("docs/summaries/valves.md");
    r.write(
        "docs/summaries/valves.md",
        &text.replacen(
            "---\n# Valves",
            "  - old/closing#zone-count@0000000000000000\n---\n# Valves",
            1,
        ),
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD643");
    assert_eq!(found.len(), 1, "{out}");
    assert!(found[0].contains("ambiguous"), "{}", found[0]);
}

#[test]
fn a_summary_of_no_area_gives_sd644() {
    let r = project("standard");
    r.write("docs/summaries/nope.md", "# Nope\n\nText.\n");
    r.write("docs/summaries/valves/nope.md", "# Deep\n\nText.\n");
    let (code, out) = check_all(&r);
    let found = lines_of(&out, "SD644");
    assert_eq!(found.len(), 2, "{out}");
    assert!(
        found
            .iter()
            .any(|l| l.contains("error SD644 summary-no-area") && l.contains("requirements/nope")),
        "{out}"
    );
    assert!(
        found.iter().any(|l| l.contains("requirements/valves/nope")),
        "{out}"
    );
    assert_eq!(code, 1, "{out}");
    // A summary of an area that has a folder gives none, also without pins.
    r.write("docs/summaries/valves.md", "# Valves\n\nText.\n");
    let (_, out) = check_all(&r);
    assert_eq!(lines_of(&out, "SD644").len(), 2, "{out}");
    // `lite`: off.
    let r = project("lite");
    r.write("docs/summaries/nope.md", "# Nope\n\nText.\n");
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
}

#[test]
fn the_top_documents_with_rests_on_pin_the_summaries_and_the_requirements_of_the_root() {
    let r = with_summary("standard");
    r.write("docs/requirements/zones.md", ZONES);
    // `system.md` is a roll-up document by the field. An empty field is one:
    // each item of its frontier is missing.
    r.write(
        "docs/system.md",
        "---\nrests-on: []\n---\n# System\n\n## Overview\n\nValves and zones.\n\n## Constraints\n\n- None.\n",
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 2, "{out}");
    assert!(
        found.iter().any(|l| l.contains("`summaries/valves`")),
        "{out}"
    );
    assert!(
        found
            .iter()
            .any(|l| l.contains("`requirements/zones#zone-count`")),
        "{out}"
    );
    // The requirements under `valves/` are not items of the root: the
    // summary of `valves` stands for them.
    assert!(!found.iter().any(|l| l.contains("closing")), "{out}");
    // A `system` document with no field is no roll-up document.
    r.write(
        "docs/system.md",
        "# System\n\n## Overview\n\nValves and zones.\n\n## Constraints\n\n- None.\n",
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
}

#[test]
fn a_repin_with_the_same_text_gives_sd645_against_the_base_and_nothing_without_one() {
    let r = with_summary("standard");
    // No commit, so no base: no SD645.
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD645"), "{out}");
    r.commit("base");
    // The requirement changes, and the pins follow, with the text of the
    // summary as it was: a claim that the text still holds.
    let old = shown_hash(&r, "requirements/valves/closing#close-on-timeout");
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 30 seconds",
        "after 30 sec",
    );
    let new = shown_hash(&r, "requirements/valves/closing#close-on-timeout");
    assert_ne!(old, new);
    edit(&r, "docs/summaries/valves.md", &old, &new);
    let (code, out) = check_all(&r);
    assert!(!out.contains("SD641"), "the pins match: {out}");
    let found = lines_of(&out, "SD645");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("warning SD645 summary-repinned")
            && found[0].starts_with("docs/summaries/valves.md:"),
        "{}",
        found[0]
    );
    assert_eq!(code, 0, "SD645 is a warning: {out}");
    // `check` without `--all` shows it too: it is new against the base.
    let (_, out, _) = r.run(&["check"]);
    assert!(out.contains("SD645"), "{out}");
    // An edit of the text with the new pins is no claim.
    edit(&r, "docs/summaries/valves.md", "30 seconds", "30 sec");
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD645"), "{out}");
}

#[test]
fn sd645_is_off_in_lite_and_a_warning_in_strict() {
    for (profile, want) in [("lite", false), ("strict", true)] {
        let r = with_summary(profile);
        r.commit("base");
        let old = shown_hash(&r, "requirements/valves/closing#close-on-timeout");
        edit(
            &r,
            "docs/requirements/valves/closing.md",
            "after 30 seconds",
            "after 30 sec",
        );
        let new = shown_hash(&r, "requirements/valves/closing#close-on-timeout");
        edit(&r, "docs/summaries/valves.md", &old, &new);
        let (_, out) = check_all(&r);
        assert_eq!(out.contains("warning SD645"), want, "{profile}: {out}");
        assert!(!out.contains("error SD645"), "{profile}: {out}");
    }
}

#[test]
fn a_stale_summary_of_the_base_is_not_new_and_does_not_fail_the_gate() {
    let r = with_summary("standard");
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 30 seconds",
        "after 20 seconds",
    );
    r.commit("base with a stale pin");
    // A later change that touches something else: the finding is of the base.
    r.write("docs/requirements/zones.md", ZONES);
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("error SD641"), "{out}");
    // `--all` shows it, and it is an error.
    let (_, out) = check_all(&r);
    assert!(out.contains("error SD641"), "{out}");
}

#[test]
fn the_budget_of_a_summary_is_the_budget_of_a_rule() {
    let r = project("standard");
    let body = format!("# Valves\n\n{}\n", "word ".repeat(700));
    r.write("docs/summaries/valves.md", &body);
    let (_, out) = check_all(&r);
    assert!(out.contains("SD701"), "3,500 characters: {out}");
    assert!(!out.contains("SD702"), "{out}");
    let body = format!("# Valves\n\n{}\n", "word ".repeat(1300));
    r.write("docs/summaries/valves.md", &body);
    let (_, out) = check_all(&r);
    assert!(out.contains("SD702"), "6,500 characters: {out}");
}

// P5: a project with no `[layers]` table keeps its types and its output.

fn plain_project() -> Repo {
    let r = Repo::with_config(
        "[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"research\"\n",
    );
    r.write("docs/summaries/notes.md", "# Notes\n\nText.\n");
    r.write("docs/requirements/valves.md", CLOSING);
    r.write("docs/rules/water.md", "---\nstatus: active\napplies-to: [\"**\"]\n---\n# Water\n\n## Rules\n\n- **flow**: The system SHALL limit the flow.\n");
    r
}

#[test]
fn a_project_with_no_layers_table_keeps_the_types_of_a_folder_named_summaries() {
    let r = plain_project();
    let (code, out, err) = r.run(&["layer", "list"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("summaries"), "no layer `summaries`: {out}");
    // The document of the folder keeps the type of the root: `research`.
    let research = out.lines().find(|l| l.starts_with("research ")).unwrap();
    assert!(research.contains("docs 1"), "{out}");
    let (code, out) = check_all(&r);
    assert!(!out.contains("SD303") && !out.contains("SD64"), "{out}");
    assert_eq!(code, 0, "{out}");
    // `new summary` has no layer to write to, and the message names the command.
    let (code, _, err) = r.run(&["new", "summary", "valves"]);
    assert_eq!(code, 2);
    assert!(err.contains("layer add summaries"), "{err}");
    // The type in a front matter has no place either: SD303 names the layer.
    r.write(
        "docs/other.md",
        "---\ntype: summary\n---\n# Other\n\nText.\n",
    );
    let (_, out) = check_all(&r);
    assert!(out.contains("SD303") && out.contains("layer add"), "{out}");
}

#[test]
fn a_project_with_no_layers_table_keeps_its_skill_text() {
    // The skill lists the types that a layer has. The implied graph has no
    // layer `summaries`, so the type is not in the list, and no generated
    // file of a 0.7.1 project is outdated.
    let r = plain_project();
    r.write("AGENTS.md", "# Agents\n");
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let skill = r.read(".claude/skills/spec-dna/SKILL.md");
    assert!(
        skill.contains("Types of this project:") && !skill.contains("summary"),
        "{skill}"
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD501") && !out.contains("SD502"), "{out}");
}

#[test]
fn a_project_with_layers_and_no_summary_gets_no_code_of_the_group_in_any_profile() {
    for profile in ["lite", "standard", "strict"] {
        let r = project(profile);
        r.write("docs/requirements/zones.md", ZONES);
        r.write(
            "docs/system.md",
            "# System\n\n## Overview\n\nValves.\n\n## Constraints\n\n- None.\n",
        );
        let (_, out) = check_all(&r);
        for code in ["SD641", "SD642", "SD643", "SD644", "SD645"] {
            assert!(!out.contains(code), "{profile}: {out}");
        }
    }
}

/// Puts one pin at the end of the pins of `summaries/valves`.
fn add_pin(r: &Repo, pin: &str) {
    let text = r.read("docs/summaries/valves.md");
    assert!(text.contains("---\n# Valves"), "{text}");
    r.write(
        "docs/summaries/valves.md",
        &text.replacen("---\n# Valves", &format!("  - {pin}\n---\n# Valves"), 1),
    );
}

#[test]
fn a_pin_to_an_item_that_no_frontier_can_hold_gives_sd643_and_not_sd641() {
    // Design 0.8.0, D52: SD641 is for a target that a frontier could hold and
    // now does not. SD643 is for a target that no frontier can hold.
    let r = with_summary_of(project_with(
        "standard",
        "[layers.research]\ntypes = [\"research\"]\n",
    ));
    r.write("docs/research/notes.md", "# Notes\n\nText.\n");
    r.write("docs/requirements/zones.md", ZONES);
    let zones = shown_hash(&r, "requirements/zones#zone-count");
    add_pin(&r, "research/notes@0000000000000000");
    add_pin(&r, "requirements/valves/closing@0000000000000000");
    // A requirement of another area: a frontier could hold it, but not this one.
    add_pin(&r, &format!("requirements/zones#zone-count@{zones}"));
    let (_, out) = check_all(&r);
    let invalid = lines_of(&out, "SD643");
    assert_eq!(invalid.len(), 2, "{out}");
    assert!(
        invalid.iter().any(|l| l.contains("`research/notes`")),
        "{out}"
    );
    assert!(
        invalid
            .iter()
            .any(|l| l.contains("`requirements/valves/closing`")),
        "{out}"
    );
    let stale = lines_of(&out, "SD641");
    assert_eq!(stale.len(), 1, "{out}");
    assert!(stale[0].contains("zones#zone-count") && stale[0].contains("frontier"));
}

#[test]
fn a_summary_of_a_project_with_no_summaries_layer_gives_sd303_and_no_sd644() {
    let r = Repo::with_config(
        "[layers.requirements]\ntypes = [\"requirements\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.write("docs/requirements/valves.md", CLOSING);
    r.write(
        "docs/other.md",
        "---\ntype: summary\n---\n# Other\n\nText.\n",
    );
    let (_, out) = check_all(&r);
    assert!(out.contains("SD303"), "{out}");
    assert!(!out.contains("SD644"), "{out}");
    let r = plain_project();
    r.write(
        "docs/other.md",
        "---\ntype: summary\n---\n# Other\n\nText.\n",
    );
    let (_, out) = check_all(&r);
    assert!(out.contains("SD303"), "{out}");
    assert!(!out.contains("SD644"), "{out}");
}

/// The fix line of SD303 in 0.7.1.
const SD303_FIX_0_7_1: &str = "Use a type of the root and of the `types` list of spec-dna.toml (without the list: product, system, terms, roadmap, questions, rule, requirements, interface, decision, procedure, incident, research, intent, spec, plan), or add the type to the list. If no layer has the type, run `spec-dna layer add <name> --types <type>`. The message names the new type of a type of 0.5.";

fn sd303_fix(r: &Repo) -> String {
    let (_, out, _) = r.run(&["check", "--all", "--json"]);
    let at = out.find("\"code\":\"SD303\"").expect(&out);
    let rest = &out[at..];
    let f = rest.find("\"fix\":\"").expect(rest) + 7;
    rest[f..].split('"').next().unwrap().to_string()
}

#[test]
fn the_fix_of_sd303_keeps_its_text_in_a_project_with_no_layers_table() {
    // P5: the line is byte for byte the line of 0.7.1. The new wording is for
    // a project that has a `[layers]` table.
    let r = plain_project();
    r.write(
        "docs/other.md",
        "---\ntype: summary\n---\n# Other\n\nText.\n",
    );
    assert_eq!(sd303_fix(&r), SD303_FIX_0_7_1);
    let r = Repo::with_config(
        "[layers.requirements]\ntypes = [\"requirements\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
    );
    r.write("docs/requirements/valves.md", CLOSING);
    r.write(
        "docs/other.md",
        "---\ntype: summary\n---\n# Other\n\nText.\n",
    );
    assert!(
        sd303_fix(&r).contains("layer add summaries"),
        "{}",
        sd303_fix(&r)
    );
}

#[test]
fn removing_rests_on_from_a_system_document_of_the_base_gives_sd645() {
    let r = with_summary("standard");
    let plain = "# System\n\n## Overview\n\nValves.\n\n## Constraints\n\n- None.\n";
    r.write(
        "docs/system.md",
        &format!("---\nrests-on: []\n---\n{plain}"),
    );
    assert_eq!(r.run(&["refresh", "system"]).0, 0);
    r.commit("system with pins");
    assert!(!check_all(&r).1.contains("SD64"));
    // The field goes. The document is no roll-up document now: no SD641, and
    // the removal is a repin with the same text.
    r.write("docs/system.md", plain);
    let (code, out) = check_all(&r);
    let found = lines_of(&out, "SD645");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("docs/system.md") && found[0].contains("rests-on"),
        "{}",
        found[0]
    );
    assert_eq!(code, 0, "{out}");
    assert!(!out.contains("SD641"), "{out}");
    // A change of the text with the removal is no claim.
    r.write(
        "docs/system.md",
        &plain.replace("Valves.", "Valves and zones."),
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD645"), "{out}");
}

// The source layers of the roll-up (design 0.8.1, D84).

const API: &str = "---\ntype: interface\nstatus: active\n---\n# Valve API\n\n- **open-close**: The valve API SHALL accept the command open and the command close.\n";

const LIMITS: &str = "---\ntype: rule\nstatus: active\n---\n# Limits\n\n- **max-pressure**: A valve SHALL NOT work above 10 bar.\n";

/// The layers of a project that has a layer for the interfaces and a layer for
/// the rules. `rolls_up` is the line of the key of the summaries, or empty.
fn source_layers(rolls_up: &str) -> String {
    format!(
        "[layers.foundation]\ntypes = [\"product\", \"system\", \"terms\"]\nfiles = [\"product.md\", \"system.md\", \"terms.md\"]\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.interfaces]\ntypes = [\"interface\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n{rolls_up}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
    )
}

/// A project of the profile `standard` with the valve requirements and the
/// layers of `source_layers`.
fn source_project(rolls_up: &str) -> Repo {
    let r = Repo::with_config(&format!(
        "profile = \"standard\"\n{}",
        source_layers(rolls_up)
    ));
    r.write("docs/requirements/valves/closing.md", CLOSING);
    r
}

#[test]
fn an_interface_source_gives_each_key_as_an_item() {
    let r = source_project("rolls-up = [\"requirements\", \"interfaces\"]\n");
    r.write("docs/interfaces/valves/api.md", API);
    let r = with_summary_of(r);
    // `new summary` writes the pin of the key of the interface.
    let text = r.read("docs/summaries/valves.md");
    assert!(
        text.contains("  - interfaces/valves/api#open-close@"),
        "{text}"
    );
    let (code, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    assert_eq!(code, 0, "{out}");
    // A new key has no pin: SD642 names it.
    r.write(
        "docs/interfaces/valves/api.md",
        &API.replace(
            "\n- **open-close**",
            "\n- **close-rate**: The valve API SHALL report the closing rate.\n- **open-close**",
        ),
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`interfaces/valves/api#close-rate`"),
        "{}",
        found[0]
    );
    // After a refresh, an edit of the key makes its pin stale: SD641.
    assert_eq!(r.run(&["refresh", "summaries/valves", "--holds"]).0, 0);
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    // An edit of a part with no key does not make the summary stale.
    edit(
        &r,
        "docs/interfaces/valves/api.md",
        "# Valve API\n",
        "# Valve API\n\nThe API of the valve.\n",
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    edit(
        &r,
        "docs/interfaces/valves/api.md",
        "command open and",
        "command open or",
    );
    let (code, out) = check_all(&r);
    let found = lines_of(&out, "SD641");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`interfaces/valves/api#open-close`")
            && found[0].contains("its text changed"),
        "{}",
        found[0]
    );
    assert_eq!(code, 1, "{out}");
}

#[test]
fn an_interface_document_that_is_not_active_gives_no_item() {
    let r = source_project("rolls-up = [\"requirements\", \"interfaces\"]\n");
    r.write(
        "docs/interfaces/valves/api.md",
        &API.replace("status: active", "status: draft"),
    );
    let r = with_summary_of(r);
    assert!(!r.read("docs/summaries/valves.md").contains("interfaces/"));
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    // The document becomes active: its key is a new item.
    edit(
        &r,
        "docs/interfaces/valves/api.md",
        "status: draft",
        "status: active",
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`interfaces/valves/api#open-close`"),
        "{out}"
    );
}

#[test]
fn an_area_can_exist_in_a_source_layer_only() {
    let r = source_project("rolls-up = [\"requirements\", \"interfaces\"]\n");
    r.write("docs/interfaces/pumps/api.md", API);
    r.write("docs/summaries/pumps.md", "# Pumps\n\nPumps move water.\n");
    let (_, out) = check_all(&r);
    // The folder `interfaces/pumps` is the area: no SD644, and the key of the
    // interface is an item with no pin.
    assert!(lines_of(&out, "SD644").is_empty(), "{out}");
    let found = lines_of(&out, "SD642");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`interfaces/pumps/api#open-close`"),
        "{out}"
    );
    // A summary of no folder of any source layer still gives SD644. The message
    // names the source folders.
    r.write("docs/summaries/zones.md", "# Zones\n\nText.\n");
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD644");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("requirements/zones") && found[0].contains("interfaces/zones"),
        "{}",
        found[0]
    );
}

#[test]
fn a_rule_source_gives_its_keys_and_a_layer_outside_the_key_gives_no_item() {
    // The key names the rules only, so the requirements are no source.
    let r = source_project("rolls-up = [\"rules\"]\n");
    r.write("docs/rules/valves/limits.md", LIMITS);
    let r = with_summary_of(r);
    let text = r.read("docs/summaries/valves.md");
    assert!(
        text.contains("  - rules/valves/limits#max-pressure@"),
        "{text}"
    );
    assert!(!text.contains("requirements/"), "{text}");
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    // A pin to a requirement that no source layer holds is SD643.
    let hash = shown_hash(&r, "requirements/valves/closing#max-angle");
    add_pin(&r, &format!("requirements/valves/closing#max-angle@{hash}"));
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD643");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`requirements/valves/closing#max-angle`")
            && found[0].contains("source layers"),
        "{}",
        found[0]
    );
    // A new requirement is no item and gives no SD642.
    assert!(lines_of(&out, "SD642").is_empty(), "{out}");
}

#[test]
fn a_project_with_no_key_has_no_item_of_an_interface_or_a_rule() {
    // Guard: the layers exist, and the summaries have no key.
    let r = source_project("");
    r.write("docs/interfaces/valves/api.md", API);
    r.write("docs/rules/valves/limits.md", LIMITS);
    let r = with_summary_of(r);
    let text = r.read("docs/summaries/valves.md");
    assert!(
        !text.contains("interfaces/") && !text.contains("rules/"),
        "{text}"
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    // A pin to the key of the interface is SD643, as in 0.8.0.
    let hash = shown_hash(&r, "interfaces/valves/api#open-close");
    add_pin(&r, &format!("interfaces/valves/api#open-close@{hash}"));
    let (_, out) = check_all(&r);
    assert_eq!(lines_of(&out, "SD643").len(), 1, "{out}");
}

// Decisions and research documents as whole items (design 0.8.1, D84).

const USE_SQLITE: &str = "---\nstatus: accepted\n---\n# Use SQLite\n\n## Context and Problem Statement\n\nThe service keeps its data in one place.\n\n## Decision Outcome\n\nUse one SQLite file with a write-ahead log.\n";

const USE_POSTGRES: &str = "---\nstatus: accepted\nsupersedes: [decisions/storage/use-sqlite]\n---\n# Use Postgres\n\n## Context and Problem Statement\n\nOne file is not enough for 2 writers.\n\n## Decision Outcome\n\nUse a Postgres server.\n";

const WRITE_LOAD: &str =
    "# Write load\n\nOne writer at a time is enough for 200 writes in a second.\n";

/// A project of the profile `profile` with the layers for the decisions and
/// the research, and the key `rolls-up` with the layers in `rolls_up`.
fn note_project(profile: &str, rolls_up: &str) -> Repo {
    let r = Repo::with_config(&format!(
        "profile = \"{profile}\"\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.decisions]\ntypes = [\"decision\"]\n\n[layers.research]\ntypes = [\"research\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\nrolls-up = {rolls_up}\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
    ));
    r.write("docs/requirements/valves/closing.md", CLOSING);
    r.write("docs/decisions/storage/use-sqlite.md", USE_SQLITE);
    r.write("docs/research/storage/write-load.md", WRITE_LOAD);
    r
}

/// The project with the summary `summaries/storage` that `new summary` wrote.
fn with_storage_summary(r: Repo) -> Repo {
    let (code, out, err) = r.run(&["new", "summary", "storage"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("docs/summaries/storage.md");
    let at = text.find("# Storage").unwrap();
    r.write(
        "docs/summaries/storage.md",
        &format!(
            "{}# Storage\n\nThe service keeps its data in one SQLite file with a write-ahead log.\n",
            &text[..at]
        ),
    );
    r
}

#[test]
fn an_accepted_decision_is_pinned_as_a_whole_document_and_an_amendment_makes_it_stale() {
    let r = with_storage_summary(note_project(
        "standard",
        "[\"requirements\", \"decisions\"]",
    ));
    let text = r.read("docs/summaries/storage.md");
    assert!(text.contains("  - decisions/storage/use-sqlite@"), "{text}");
    // The research note is no source here.
    assert!(!text.contains("research/"), "{text}");
    let (code, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    assert_eq!(code, 0, "{out}");
    // An edit of a live section changes no hash in force.
    edit(
        &r,
        "docs/decisions/storage/use-sqlite.md",
        "## Decision Outcome",
        "## Status\n\n- step 1: done\n\n## Decision Outcome",
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    // An amendment section does: SD641.
    let path = "docs/decisions/storage/use-sqlite.md";
    r.write(
        path,
        &format!(
            "{}\n## Amendments\n\n### 2026-11-24\n\nThe log is checked each hour.\n",
            r.read(path)
        ),
    );
    let (code, out) = check_all(&r);
    let found = lines_of(&out, "SD641");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`decisions/storage/use-sqlite`")
            && found[0].contains("text in force changed"),
        "{}",
        found[0]
    );
    assert_eq!(code, 1, "{out}");
    // After a refresh the pin holds again.
    assert_eq!(r.run(&["refresh", "summaries/storage", "--holds"]).0, 0);
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
}

#[test]
fn a_superseded_decision_gives_sd641_with_its_successor_and_the_new_one_gives_sd642() {
    let r = with_storage_summary(note_project(
        "standard",
        "[\"requirements\", \"decisions\"]",
    ));
    edit(
        &r,
        "docs/decisions/storage/use-sqlite.md",
        "status: accepted",
        "status: superseded",
    );
    r.write("docs/decisions/storage/use-postgres.md", USE_POSTGRES);
    let (_, out) = check_all(&r);
    let stale = lines_of(&out, "SD641");
    assert_eq!(stale.len(), 1, "{out}");
    assert!(
        stale[0].contains("`decisions/storage/use-sqlite`")
            && stale[0].contains("superseded")
            && stale[0].contains("successor: `decisions/storage/use-postgres`"),
        "{}",
        stale[0]
    );
    let missing = lines_of(&out, "SD642");
    assert_eq!(missing.len(), 1, "{out}");
    assert!(
        missing[0].contains("`decisions/storage/use-postgres`"),
        "{}",
        missing[0]
    );
    // A decision that is no longer accepted and has no successor: no name.
    edit(
        &r,
        "docs/decisions/storage/use-postgres.md",
        "supersedes: [decisions/storage/use-sqlite]\n",
        "",
    );
    let (_, out) = check_all(&r);
    let stale = lines_of(&out, "SD641");
    assert_eq!(stale.len(), 1, "{out}");
    assert!(
        stale[0].contains("superseded") && !stale[0].contains("successor"),
        "{}",
        stale[0]
    );
}

#[test]
fn a_decision_that_is_not_accepted_is_no_item() {
    let r = note_project("standard", "[\"requirements\", \"decisions\"]");
    edit(
        &r,
        "docs/decisions/storage/use-sqlite.md",
        "status: accepted",
        "status: proposed",
    );
    let r = with_storage_summary(r);
    assert!(!r.read("docs/summaries/storage.md").contains("decisions/"));
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
}

#[test]
fn a_changed_research_note_gives_sd647_a_warning_and_no_sd641() {
    let r = with_storage_summary(note_project(
        "standard",
        "[\"requirements\", \"decisions\", \"research\"]",
    ));
    let text = r.read("docs/summaries/storage.md");
    assert!(text.contains("  - research/storage/write-load@"), "{text}");
    let (code, out) = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    assert_eq!(code, 0, "{out}");
    edit(
        &r,
        "docs/research/storage/write-load.md",
        "200 writes",
        "400 writes",
    );
    let (code, out) = check_all(&r);
    let found = lines_of(&out, "SD647");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("warning SD647 summary-note-changed")
            && found[0].contains("`research/storage/write-load`")
            && found[0].contains("the note changed")
            && found[0].contains("spec-dna refresh summaries/storage"),
        "{}",
        found[0]
    );
    assert!(!out.contains("SD641"), "{out}");
    assert_eq!(code, 0, "{out}");
    // A new note of the area has no pin: SD647, not SD642.
    r.write(
        "docs/research/storage/read-load.md",
        "# Read load\n\nReads are cheap.\n",
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD647");
    assert_eq!(found.len(), 2, "{out}");
    assert!(
        found
            .iter()
            .any(|l| l.contains("`research/storage/read-load`") && l.contains("no pin")),
        "{out}"
    );
    assert!(lines_of(&out, "SD642").is_empty(), "{out}");
    // A removed note: SD647 too.
    r.write(
        "docs/research/storage/read-load.md",
        "# Read load\n\nReads are cheap.\n",
    );
    std::fs::remove_file(r.dir.join("docs/research/storage/write-load.md")).unwrap();
    let (_, out) = check_all(&r);
    assert!(
        lines_of(&out, "SD647")
            .iter()
            .any(|l| l.contains("`research/storage/write-load`") && l.contains("removed")),
        "{out}"
    );
    assert!(lines_of(&out, "SD641").is_empty(), "{out}");
}

#[test]
fn sd647_is_a_warning_in_strict_and_off_in_lite() {
    for (profile, want) in [("strict", true), ("lite", false)] {
        let r = with_storage_summary(note_project(
            profile,
            "[\"requirements\", \"decisions\", \"research\"]",
        ));
        edit(
            &r,
            "docs/research/storage/write-load.md",
            "200 writes",
            "400 writes",
        );
        let (_, out) = check_all(&r);
        assert_eq!(lines_of(&out, "SD647").len(), usize::from(want), "{out}");
        if want {
            assert!(out.contains("warning SD647"), "{out}");
        }
        assert!(
            !out.contains("SD641") && !out.contains("error SD647"),
            "{out}"
        );
    }
}

#[test]
fn a_decision_or_a_note_outside_the_key_gives_sd643() {
    let r = with_storage_summary(note_project(
        "standard",
        "[\"requirements\", \"decisions\"]",
    ));
    let hash = shown_hash(&r, "research/storage/write-load");
    let text = r.read("docs/summaries/storage.md");
    r.write(
        "docs/summaries/storage.md",
        &text.replacen(
            "---\n# Storage",
            &format!("  - research/storage/write-load@{hash}\n---\n# Storage"),
            1,
        ),
    );
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD643");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].contains("`research/storage/write-load`") && found[0].contains("source layers"),
        "{}",
        found[0]
    );
    assert!(lines_of(&out, "SD647").is_empty(), "{out}");
}

// A roll-up document that rests on nothing (design 0.8.1, D84, item 7).

const SYSTEM_TOP: &str =
    "---\nrests-on: []\n---\n# System\n\n## Overview\n\nValves.\n\n## Constraints\n\n- None.\n";

/// A project with an area `valves` that has only a draft requirement, the
/// summary of the area and a `system` document that pins the summary.
fn nothing_project(profile: &str) -> Repo {
    let r = project(profile);
    r.write(
        "docs/requirements/valves/closing.md",
        &CLOSING.replace("status: active", "status: draft"),
    );
    r.write(
        "docs/summaries/valves.md",
        "---\nrests-on: []\n---\n# Valves\n\nA valve closes after its sensor stops.\n",
    );
    r.write("docs/system.md", SYSTEM_TOP);
    let (code, out, err) = r.run(&["refresh", "system"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read("docs/system.md").contains("summaries/valves@"));
    r
}

#[test]
fn a_summary_with_no_pin_and_no_item_that_system_pins_gives_sd646() {
    // A warning in `standard`, an error in `strict`, and nothing in `lite`.
    for (profile, severity) in [("standard", "warning"), ("strict", "error")] {
        let r = nothing_project(profile);
        let (code, out) = check_all(&r);
        let found = lines_of(&out, "SD646");
        assert_eq!(found.len(), 1, "{profile}: {out}");
        assert!(
            found[0].starts_with("docs/summaries/valves.md:")
                && found[0].contains(&format!("{severity} SD646 summary-rests-on-nothing"))
                && found[0].contains("`system`")
                && found[0].contains("rolls-up"),
            "{}",
            found[0]
        );
        // The severity decides the exit code: the finding is new (no base).
        assert_eq!(code, i32::from(severity == "error"), "{profile}: {out}");
        assert!(!out.contains("SD641") && !out.contains("SD642"), "{out}");
    }
    let (_, out) = check_all(&nothing_project("lite"));
    assert!(!out.contains("SD646"), "{out}");
}

#[test]
fn sd646_is_not_new_against_a_base_that_has_it() {
    let r = nothing_project("strict");
    r.commit("rests on nothing");
    // A change of a draft requirement: no item of any frontier.
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 30 seconds",
        "after 31 seconds",
    );
    // The base has the same finding, so the gate does not stop on it.
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    // `--all` shows it, and it is an error in `strict`.
    let (code, out) = check_all(&r);
    assert!(out.contains("error SD646"), "{out}");
    assert_eq!(code, 1, "{out}");
}

#[test]
fn sd646_is_silent_where_sd641_or_sd642_speaks_and_when_the_document_has_an_item() {
    let r = nothing_project("standard");
    // The summary gets an active requirement: the frontier is not empty, and
    // the summary has no pin, so SD642 names the requirement and the pin of
    // `system` is stale (SD641).
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "status: draft",
        "status: active",
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD646"), "{out}");
    assert_eq!(lines_of(&out, "SD642").len(), 2, "{out}");
    // The summary pins both requirements, and the pin of `system` is stale
    // after the text changed.
    assert_eq!(r.run(&["refresh", "summaries/valves", "--holds"]).0, 0);
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD646"), "{out}");
    // A summary of the draft area whose pin in `system` has a wrong hash: SD641
    // speaks, and SD646 stays silent.
    let r = nothing_project("standard");
    let text = r.read("docs/system.md");
    let at = text.find("summaries/valves@").unwrap() + "summaries/valves@".len();
    r.write(
        "docs/system.md",
        &format!("{}{}{}", &text[..at], "0".repeat(16), &text[at + 16..]),
    );
    let (_, out) = check_all(&r);
    assert_eq!(lines_of(&out, "SD641").len(), 1, "{out}");
    assert!(!out.contains("SD646"), "{out}");
}

#[test]
fn a_system_with_no_pin_that_product_pins_gives_sd646_and_a_system_with_no_field_stays_sd641() {
    let r = project("standard");
    r.write(
        "docs/requirements/valves/closing.md",
        &CLOSING.replace("status: active", "status: draft"),
    );
    // `system` has the field and no item; `product` pins it.
    r.write("docs/system.md", SYSTEM_TOP);
    r.write(
        "docs/product.md",
        "---\nrests-on: []\n---\n# Product\n\n## Vision\n\nValves.\n",
    );
    assert_eq!(r.run(&["refresh", "product"]).0, 0);
    let (_, out) = check_all(&r);
    let found = lines_of(&out, "SD646");
    assert_eq!(found.len(), 1, "{out}");
    assert!(
        found[0].starts_with("docs/system.md:") && found[0].contains("`product`"),
        "{}",
        found[0]
    );
    // The field `rests-on` goes: `system` is no roll-up document, and the pin of
    // `product` is SD641, as in 0.8.0.
    r.write(
        "docs/system.md",
        &SYSTEM_TOP.replace("---\nrests-on: []\n---\n", ""),
    );
    let (_, out) = check_all(&r);
    assert!(!out.contains("SD646"), "{out}");
    assert_eq!(lines_of(&out, "SD641").len(), 1, "{out}");
}

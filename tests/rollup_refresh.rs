//! Integration tests of `spec-dna refresh` and its guard (design 0.8.0, D53).
//! The fixture is a small valve project with the layers `requirements` and
//! `summaries`, a summary `summaries/valves` with current pins, and 1 commit:
//! the base of the guard. The helpers come from tests/common, so each commit
//! and each run uses the isolated git environment that CI has.

mod common;

use common::Repo;

const CLOSING: &str = "---\nstatus: active\n---\n# Closing\n\n## Requirements\n\n- **close-on-timeout**: When the sensor stops, the valve SHALL close after 30 seconds.\n- **max-angle**: The valve SHALL NOT open more than 80 degrees.\n";

const ZONES: &str = "---\nstatus: active\n---\n# Zones\n\n## Requirements\n\n- **zone-count**: The controller SHALL support 8 zones.\n";

const LAYERS: &str = "[layers.foundation]\ntypes = [\"product\", \"system\", \"terms\"]\nfiles = [\"product.md\", \"system.md\", \"terms.md\"]\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

const BODY: &str = "# Valves\n\nA valve closes 30 seconds after its sensor stops. It never opens more than 80 degrees.\n";

/// The project with the summary of `valves`, committed.
fn project() -> Repo {
    let r = Repo::with_config(&format!("profile = \"standard\"\n\n{LAYERS}"));
    r.write("docs/requirements/valves/closing.md", CLOSING);
    let (code, out, err) = r.run(&["new", "summary", "valves"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read("docs/summaries/valves.md");
    let at = text.find("# Valves").unwrap();
    r.write(
        "docs/summaries/valves.md",
        &format!("{}{BODY}", &text[..at]),
    );
    r.commit("base");
    r
}

fn edit(r: &Repo, path: &str, from: &str, to: &str) {
    let text = r.read(path);
    assert!(text.contains(from), "{path} has no `{from}`");
    r.write(path, &text.replacen(from, to, 1));
}

/// A text change of the requirement that the summary pins.
fn change_requirement(r: &Repo) {
    edit(
        r,
        "docs/requirements/valves/closing.md",
        "after 30 seconds",
        "after 20 seconds",
    );
}

fn lines_of<'a>(out: &'a str, code: &str) -> Vec<&'a str> {
    // The finding lines, not the lines `<code> fix: ...`.
    out.lines()
        .filter(|l| {
            l.contains(&format!("error {code} ")) || l.contains(&format!("warning {code} "))
        })
        .collect()
}

fn check_all(r: &Repo) -> String {
    let (_, out, err) = r.run(&["check", "--all"]);
    format!("{out}{err}")
}

const SUMMARY: &str = "docs/summaries/valves.md";

#[test]
fn a_stale_summary_with_an_unchanged_body_is_refused_and_stays_as_it_was() {
    let r = project();
    change_requirement(&r);
    let before = r.read(SUMMARY);
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 1, "{out}{err}");
    let text = format!("{out}{err}");
    let refused: Vec<&str> = text
        .lines()
        .filter(|l| l.starts_with("refused summaries/valves"))
        .collect();
    assert_eq!(refused.len(), 1, "{text}");
    assert!(
        refused[0].contains("requirements/valves/closing#close-on-timeout")
            && refused[0].contains("--holds"),
        "{}",
        refused[0]
    );
    assert_eq!(r.read(SUMMARY), before, "a refusal writes nothing");
    let (code, out, _) = r.run(&["refresh", "summaries/valves", "--json"]);
    assert_eq!(code, 1);
    assert_eq!(
        out.trim(),
        "{\"refreshed\":[],\"held\":[],\"refused\":[{\"id\":\"summaries/valves\",\"pins\":[\"requirements/valves/closing#close-on-timeout\"]}]}"
    );
    // The finding stays, and so does the file.
    assert!(check_all(&r).contains("SD641"));
}

#[test]
fn holds_writes_the_pins_of_a_refused_summary_and_check_gives_sd645() {
    let r = project();
    change_requirement(&r);
    let (code, out, err) = r.run(&["refresh", "summaries/valves", "--holds"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("held summaries/valves: the text did not change (SD645 shows it)"),
        "{out}"
    );
    // The diff has a changed front matter and an unchanged body.
    let now = r.read(SUMMARY);
    assert!(now.ends_with(BODY), "{now}");
    let diff = r.git_out(&["diff", "--stat"]);
    assert!(diff.contains("docs/summaries/valves.md"), "{diff}");
    let out = check_all(&r);
    assert!(!out.contains("SD641") && !out.contains("SD642"), "{out}");
    let sd645 = lines_of(&out, "SD645");
    assert_eq!(sd645.len(), 1, "{out}");
    assert!(
        sd645[0].contains("warning SD645 summary-repinned"),
        "{}",
        sd645[0]
    );
    // `--json` has the id in `held`.
    let r = project();
    change_requirement(&r);
    let (code, out, _) = r.run(&["refresh", "summaries/valves", "--holds", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(
        out.trim(),
        "{\"refreshed\":[],\"held\":[\"summaries/valves\"],\"refused\":[]}"
    );
}

#[test]
fn an_edit_of_the_body_makes_the_refresh_pass_with_no_flag() {
    let r = project();
    change_requirement(&r);
    edit(&r, SUMMARY, "30 seconds", "20 seconds");
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("refreshed summaries/valves: 2 pins (0 added, 1 changed, 0 removed)"),
        "{out}"
    );
    let text = check_all(&r);
    assert!(!text.contains("SD641") && !text.contains("SD645"), "{text}");
    // Whitespace does not count: the hash of the text ignores it. A refresh
    // after a layout change of the body is still a refusal.
    let r = project();
    change_requirement(&r);
    edit(&r, SUMMARY, "A valve closes 30", "A valve  closes\n30");
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 1, "{out}{err}");
}

#[test]
fn after_one_edit_each_later_refresh_of_the_summary_in_the_branch_passes() {
    let r = project();
    change_requirement(&r);
    edit(&r, SUMMARY, "30 seconds", "20 seconds");
    assert_eq!(r.run(&["refresh", "summaries/valves"]).0, 0);
    // The requirement changes again. The body still differs from the base.
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 20 seconds",
        "after 10 seconds",
    );
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn all_with_holds_is_a_usage_error_and_writes_nothing() {
    let r = project();
    change_requirement(&r);
    let before = r.read(SUMMARY);
    let (code, out, err) = r.run(&["refresh", "--all", "--holds"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("--holds"), "{err}");
    assert_eq!(r.read(SUMMARY), before);
}

#[test]
fn a_removed_requirement_drops_its_pin_and_a_new_one_is_added() {
    let r = project();
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "- **max-angle**: The valve SHALL NOT open more than 80 degrees.\n",
        "- **min-angle**: The valve SHALL open at least 5 degrees.\n",
    );
    edit(&r, SUMMARY, "80 degrees", "5 degrees at least");
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("refreshed summaries/valves: 2 pins (1 added, 0 changed, 1 removed)"),
        "{out}"
    );
    let text = r.read(SUMMARY);
    assert!(text.contains("closing#min-angle@"), "{text}");
    assert!(!text.contains("max-angle"), "{text}");
    assert!(!check_all(&r).contains("SD64"));
}

#[test]
fn a_pin_through_an_alias_gets_the_canonical_id() {
    let r = project();
    // The document moves. The old id is an alias, and the pins keep the old id.
    r.git(&[
        "mv",
        "docs/requirements/valves/closing.md",
        "docs/requirements/valves/shutoff.md",
    ]);
    edit(
        &r,
        "docs/requirements/valves/shutoff.md",
        "status: active\n",
        "status: active\naliases: [requirements/valves/closing]\n",
    );
    assert!(!check_all(&r).contains("SD64"), "the alias resolves");
    // The text of the summary changes, so the guard lets the refresh pass.
    edit(&r, SUMMARY, "30 seconds", "30 s");
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read(SUMMARY);
    assert!(
        text.contains("  - requirements/valves/shutoff#close-on-timeout@")
            && text.contains("  - requirements/valves/shutoff#max-angle@"),
        "{text}"
    );
    assert!(!text.contains("closing"), "{text}");
}

#[test]
fn a_move_that_only_rewrites_the_pins_is_no_claim() {
    let r = project();
    r.git(&[
        "mv",
        "docs/requirements/valves/closing.md",
        "docs/requirements/valves/shutoff.md",
    ]);
    edit(
        &r,
        "docs/requirements/valves/shutoff.md",
        "status: active\n",
        "status: active\naliases: [requirements/valves/closing]\n",
    );
    // The text of the summary did not change, and no pin is stale: `refresh`
    // rewrites the pins to the canonical id, and `check` shows no SD645.
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.read(SUMMARY).contains("shutoff#close-on-timeout@"));
    let text = check_all(&r);
    assert!(!text.contains("SD645") && !text.contains("SD64"), "{text}");
}

#[test]
fn all_refreshes_each_summary_with_a_finding_and_lists_the_refused_ones() {
    let r = project();
    r.write("docs/requirements/zones.md", ZONES);
    assert_eq!(
        r.run(&["new", "summary", "x"]).0,
        2,
        "no folder, no summary"
    );
    r.write(
        "docs/requirements/sensors/probe.md",
        "---\nstatus: active\n---\n# Probe\n\n## Requirements\n\n- **probe-range**: The probe SHALL read 0 to 100 percent.\n",
    );
    assert_eq!(r.run(&["new", "summary", "sensors"]).0, 0);
    let t = r.read("docs/summaries/sensors.md");
    let at = t.find("# Sensors").unwrap();
    r.write(
        "docs/summaries/sensors.md",
        &format!("{}# Sensors\n\nA probe reads 0 to 100 percent.\n", &t[..at]),
    );
    r.commit("sensors");
    // `valves` has a new requirement and an edited body. `sensors` has a
    // changed requirement and its body as it was.
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "- **max-angle**",
        "- **min-angle**: The valve SHALL open at least 5 degrees.\n- **max-angle**",
    );
    edit(&r, SUMMARY, "80 degrees", "80 degrees, at least 5 degrees");
    edit(
        &r,
        "docs/requirements/sensors/probe.md",
        "0 to 100",
        "0 to 90",
    );
    let sensors = r.read("docs/summaries/sensors.md");
    let (code, out, err) = r.run(&["refresh", "--all"]);
    assert_eq!(code, 1, "{out}{err}");
    let text = format!("{out}{err}");
    assert!(
        text.contains("refreshed summaries/valves: 3 pins (1 added, 0 changed, 0 removed)"),
        "{text}"
    );
    assert!(text.contains("refused summaries/sensors"), "{text}");
    assert_eq!(r.read("docs/summaries/sensors.md"), sensors);
    // A summary with no finding is not listed and not touched.
    assert!(!text.contains("zones"), "{text}");
    let (code, out, _) = r.run(&["refresh", "--all", "--json"]);
    assert_eq!(code, 1);
    assert!(
        out.contains("\"refused\":[{\"id\":\"summaries/sensors\""),
        "{out}"
    );
}

#[test]
fn dry_run_writes_nothing_and_says_what_it_would_do() {
    let r = project();
    change_requirement(&r);
    edit(&r, SUMMARY, "30 seconds", "20 seconds");
    let before = r.read(SUMMARY);
    let (code, out, err) = r.run(&["refresh", "summaries/valves", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("would refresh summaries/valves"), "{out}");
    assert_eq!(r.read(SUMMARY), before);
    // A refusal is the same with `--dry-run`.
    let r = project();
    change_requirement(&r);
    let (code, _, _) = r.run(&["refresh", "summaries/valves", "--dry-run"]);
    assert_eq!(code, 1);
}

#[test]
fn an_id_check_comes_before_any_write() {
    let r = project();
    change_requirement(&r);
    edit(&r, SUMMARY, "30 seconds", "20 seconds");
    let before = r.read(SUMMARY);
    // No document: exit 5, and the good id is not written either.
    let (code, out, err) = r.run(&["refresh", "summaries/valves", "summaries/nope"]);
    assert_eq!(code, 5, "{out}{err}");
    assert_eq!(r.read(SUMMARY), before);
    // A document that is no roll-up document: exit 2.
    let (code, out, err) = r.run(&["refresh", "summaries/valves", "requirements/valves/closing"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(err.contains("roll-up"), "{err}");
    assert_eq!(r.read(SUMMARY), before);
    // No id and no `--all`: a usage error.
    assert_eq!(r.run(&["refresh"]).0, 2);
    assert_eq!(r.run(&["refresh", "summaries/valves", "--all"]).0, 2);
}

#[test]
fn a_summary_that_the_base_does_not_have_has_no_guard() {
    let r = project();
    r.write(
        "docs/requirements/valves/drives/motor.md",
        "---\nstatus: active\n---\n# Motor\n\n## Requirements\n\n- **spin-up**: The motor SHALL reach full speed in 2 seconds.\n",
    );
    assert_eq!(r.run(&["new", "summary", "valves/drives"]).0, 0);
    edit(
        &r,
        "docs/requirements/valves/drives/motor.md",
        "2 seconds",
        "3 seconds",
    );
    // The new summary is not in the base, and its text is a placeholder.
    let (code, out, err) = r.run(&["refresh", "summaries/valves/drives"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("refreshed summaries/valves/drives"), "{out}");
}

#[test]
fn a_repository_with_no_commit_has_no_guard() {
    let r = Repo::with_config(&format!("profile = \"standard\"\n\n{LAYERS}"));
    r.write("docs/requirements/valves/closing.md", CLOSING);
    assert_eq!(r.run(&["new", "summary", "valves"]).0, 0);
    change_requirement(&r);
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn refresh_keeps_the_other_fields_the_body_and_the_line_ends() {
    let r = project();
    let text = r.read(SUMMARY);
    let at = text.find("---\n# Valves").unwrap();
    let with_owner = format!("{}owner: team@example.org\n{}", &text[..at], &text[at..]);
    r.write(SUMMARY, &with_owner.replace('\n', "\r\n"));
    r.commit("crlf and an owner");
    change_requirement(&r);
    let (code, out, err) = r.run(&["refresh", "summaries/valves", "--holds"]);
    assert_eq!(code, 0, "{out}{err}");
    let now = r.read(SUMMARY);
    assert!(now.contains("owner: team@example.org\r\n"), "{now:?}");
    assert!(now.ends_with(&BODY.replace('\n', "\r\n")), "{now:?}");
    assert!(!now.replace("\r\n", "").contains('\n'), "{now:?}");
    assert!(now.contains("rests-on:\r\n  - requirements/valves/closing#close-on-timeout@"));
}

#[test]
fn the_level_above_pins_the_text_of_the_summary_and_not_its_pins() {
    let r = project();
    r.write(
        "docs/system.md",
        "---\nrests-on: []\n---\n# System\n\n## Overview\n\nValves.\n\n## Constraints\n\n- None.\n",
    );
    // A `system` document with the field and no pin: each item is missing.
    // It is not in the base yet, so the guard does not apply to it.
    assert!(check_all(&r).contains("SD642"));
    let (code, out, err) = r.run(&["refresh", "system"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        r.read("docs/system.md").contains("  - summaries/valves@"),
        "{}",
        r.read("docs/system.md")
    );
    assert!(!check_all(&r).contains("SD64"));
    r.commit("pinned");
    // A change of a requirement and a refresh of the summary change its pins,
    // and its text stays: the pin of `system` still holds.
    change_requirement(&r);
    assert_eq!(r.run(&["refresh", "summaries/valves", "--holds"]).0, 0);
    let out = check_all(&r);
    assert!(!out.contains("SD641") && !out.contains("SD642"), "{out}");
    // A change of the text of the summary makes the level above stale.
    edit(&r, SUMMARY, "30 seconds", "20 seconds");
    let out = check_all(&r);
    let stale = lines_of(&out, "SD641");
    assert_eq!(stale.len(), 1, "{out}");
    assert!(
        stale[0].contains("docs/system.md") && stale[0].contains("`summaries/valves`"),
        "{}",
        stale[0]
    );
    // `refresh --holds` of the system stops the cascade.
    let (code, out, err) = r.run(&["refresh", "system"]);
    assert_eq!(code, 1, "the text of `system` did not change: {out}{err}");
    let (code, out, err) = r.run(&["refresh", "system", "--holds"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!check_all(&r).contains("SD641"));
}

#[test]
fn a_project_with_no_summary_is_not_changed_by_the_command() {
    let r = Repo::with_config(&format!("profile = \"standard\"\n\n{LAYERS}"));
    r.write("docs/requirements/valves/closing.md", CLOSING);
    r.commit("base");
    // `--all` finds no roll-up document: nothing to do, exit 0, no write.
    let (code, out, err) = r.run(&["refresh", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        !out.contains("refreshed") && !out.contains("refused"),
        "{out}"
    );
    assert!(r.git_out(&["status", "--porcelain"]).is_empty());
    let (code, out, _) = r.run(&["refresh", "--all", "--json"]);
    assert_eq!(code, 0);
    assert_eq!(out.trim(), "{\"refreshed\":[],\"held\":[],\"refused\":[]}");
    // A project with no `[layers]` table has no layer `summaries` and no
    // roll-up document: an id is no roll-up document.
    let plain = Repo::with_config("[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n");
    plain.write("docs/requirements/valves.md", CLOSING);
    let (code, _, err) = plain.run(&["refresh", "requirements/valves"]);
    assert_eq!(code, 2, "{err}");
}

#[test]
fn a_pin_of_another_version_in_spec_dna_toml_is_exit_3() {
    let r = project();
    let cfg = r.read("spec-dna.toml");
    r.write("spec-dna.toml", &cfg.replacen(common::VERSION, "0.0.1", 1));
    let (code, _, err) = r.run(&["refresh", "--all"]);
    assert_eq!(code, 3, "{err}");
}

#[test]
fn a_summary_that_names_no_area_is_a_usage_error_and_never_a_panic() {
    let r = project();
    // A summary of no area, and a summary outside the folder of its layer.
    r.write("docs/summaries/nope.md", "# Nope\n\nText.\n");
    r.write(
        "docs/other/loose.md",
        "---\ntype: summary\n---\n# Loose\n\nText.\n",
    );
    r.commit("more");
    for id in ["summaries/nope", "other/loose"] {
        let (code, out, err) = r.run(&["refresh", id]);
        assert_eq!(code, 2, "{id}: {out}{err}");
        assert!(err.contains("area"), "{id}: {err}");
        assert!(!err.contains("panicked"), "{id}: {err}");
    }
    assert!(r.git_out(&["status", "--porcelain"]).is_empty());
    // `--all` leaves them out: SD644 names them, and no pin can fix them.
    let (code, out, err) = r.run(&["refresh", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("nope") && !out.contains("loose"), "{out}");
}

#[test]
fn the_first_rests_on_of_a_system_document_that_the_base_has_needs_no_claim() {
    let r = project();
    let plain = "# System\n\n## Overview\n\nValves.\n\n## Constraints\n\n- None.\n";
    r.write("docs/system.md", plain);
    r.commit("system");
    // The author opts in with an empty field. The base copy is no roll-up
    // document, so the guard does not apply, and the first pins are no claim.
    r.write(
        "docs/system.md",
        &format!("---\nrests-on: []\n---\n{plain}"),
    );
    assert!(check_all(&r).contains("SD642"));
    let (code, out, err) = r.run(&["refresh", "system"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("refreshed system: 1 pins (1 added, 0 changed, 0 removed)"),
        "{out}"
    );
    let text = check_all(&r);
    assert!(!text.contains("SD64"), "{text}");
    // Once the base has the field, the guard applies to the document.
    r.commit("opted in");
    change_requirement(&r);
    edit(&r, SUMMARY, "30 seconds", "20 seconds");
    assert_eq!(r.run(&["refresh", "summaries/valves"]).0, 0);
    let (code, out, err) = r.run(&["refresh", "system"]);
    assert_eq!(code, 1, "{out}{err}");
}

#[test]
fn a_pin_that_no_frontier_can_hold_is_no_stale_pin_so_the_guard_does_not_refuse() {
    // SD643 pins are not stale (SD641): `refresh` drops them with no claim.
    let r = project();
    r.write("docs/requirements/other.md", ZONES);
    let text = r.read(SUMMARY);
    r.write(
        SUMMARY,
        &text.replacen(
            "---\n# Valves",
            "  - requirements/valves/closing@0000000000000000\n---\n# Valves",
            1,
        ),
    );
    r.commit("a pin of a whole document");
    assert!(check_all(&r).contains("SD643"));
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("(0 added, 0 changed, 1 removed)"), "{out}");
    // The text did not change, so the written pins are a claim: SD645 only.
    let out = check_all(&r);
    for code in ["SD641", "SD642", "SD643"] {
        assert!(!out.contains(code), "{out}");
    }
}

#[test]
fn a_file_with_a_byte_order_mark_is_refused_and_a_file_with_crlf_keeps_one_block() {
    let r = project();
    let text = r.read(SUMMARY);
    // The tool does not read the front matter of a file with a BOM, so
    // `refresh` writes nothing there and says why.
    r.write(SUMMARY, &format!("\u{feff}{text}"));
    r.commit("bom");
    change_requirement(&r);
    edit(&r, SUMMARY, "30 seconds", "20 seconds");
    let before = r.read(SUMMARY);
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_ne!(code, 0, "{out}{err}");
    assert!(err.contains("byte order mark"), "{err}");
    assert_eq!(r.read(SUMMARY), before);
    // CRLF: one block, with the line ends of the file.
    r.write(SUMMARY, &text.replace('\n', "\r\n"));
    r.commit("crlf");
    edit(
        &r,
        "docs/requirements/valves/closing.md",
        "after 20 seconds",
        "after 10 seconds",
    );
    edit(&r, SUMMARY, "30 seconds", "10 seconds");
    let (code, out, err) = r.run(&["refresh", "summaries/valves"]);
    assert_eq!(code, 0, "{out}{err}");
    let now = r.read(SUMMARY);
    assert_eq!(now.matches("rests-on:").count(), 1, "{now:?}");
    assert_eq!(now.matches("---").count(), 2, "{now:?}");
    assert!(!now.replace("\r\n", "").contains('\n'), "{now:?}");
}

// The source layers of the roll-up (design 0.8.1, D84).

const STORAGE_LAYERS: &str = "[layers.decisions]\ntypes = [\"decision\"]\n\n[layers.research]\ntypes = [\"research\"]\n\n[layers.summaries]\ntypes = [\"summary\"]\nrolls-up = [\"decisions\", \"research\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

const USE_SQLITE: &str = "---\nstatus: accepted\n---\n# Use SQLite\n\n## Context and Problem Statement\n\nThe service keeps its data in one place.\n\n## Decision Outcome\n\nUse one SQLite file with a write-ahead log.\n";

const WRITE_LOAD: &str =
    "# Write load\n\nOne writer at a time is enough for 200 writes in a second.\n";

const STORAGE: &str = "docs/summaries/storage.md";

/// A project with only the layers `decisions` and `research`, and the summary
/// `summaries/storage` that `new summary` wrote, committed.
fn storage_project() -> Repo {
    let r = Repo::with_config(&format!("profile = \"standard\"\n\n{STORAGE_LAYERS}"));
    r.write("docs/decisions/storage/use-sqlite.md", USE_SQLITE);
    r.write("docs/research/storage/write-load.md", WRITE_LOAD);
    let (code, out, err) = r.run(&["new", "summary", "storage"]);
    assert_eq!(code, 0, "{out}{err}");
    let text = r.read(STORAGE);
    let at = text.find("# Storage").unwrap();
    r.write(
        STORAGE,
        &format!(
            "{}# Storage\n\nThe service keeps its data in one SQLite file with a write-ahead log.\n",
            &text[..at]
        ),
    );
    r.commit("base");
    r
}

#[test]
fn new_summary_in_a_project_with_only_decisions_and_research_writes_the_pins() {
    let r = storage_project();
    let text = r.read(STORAGE);
    assert!(text.contains("  - decisions/storage/use-sqlite@"), "{text}");
    assert!(text.contains("  - research/storage/write-load@"), "{text}");
    let out = check_all(&r);
    assert!(!out.contains("SD64"), "{out}");
    // A name that no source layer has as a folder is a usage error that names
    // the folders of the source layers.
    let (code, out, err) = r.run(&["new", "summary", "pumps"]);
    assert_eq!(code, 2, "{out}{err}");
    assert!(
        err.contains("`decisions/pumps`") && err.contains("`research/pumps`"),
        "{err}"
    );
}

#[test]
fn a_changed_note_with_the_text_of_the_base_is_refused_and_holds_writes_the_pins() {
    let r = storage_project();
    edit(
        &r,
        "docs/research/storage/write-load.md",
        "200 writes",
        "400 writes",
    );
    let before = r.read(STORAGE);
    let out = check_all(&r);
    assert_eq!(lines_of(&out, "SD647").len(), 1, "{out}");
    let (code, out, err) = r.run(&["refresh", "summaries/storage"]);
    assert_eq!(code, 1, "{out}{err}");
    let text = format!("{out}{err}");
    assert!(
        text.contains("refused summaries/storage")
            && text.contains("research/storage/write-load")
            && text.contains("--holds"),
        "{text}"
    );
    assert_eq!(r.read(STORAGE), before, "a refusal writes nothing");
    // An edit of the text lets the refresh pass with no flag.
    edit(
        &r,
        STORAGE,
        "write-ahead log.",
        "write-ahead log. One writer is enough.",
    );
    let (code, out, err) = r.run(&["refresh", "summaries/storage"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!check_all(&r).contains("SD64"));
    // With the text of the base, `--holds` is the claim, and SD645 shows it.
    r.commit("a new base");
    edit(
        &r,
        "docs/research/storage/write-load.md",
        "400 writes",
        "300 writes",
    );
    let (code, out, err) = r.run(&["refresh", "summaries/storage", "--holds"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("held summaries/storage: the text did not change (SD645 shows it)"),
        "{out}"
    );
    let out = check_all(&r);
    assert_eq!(lines_of(&out, "SD645").len(), 1, "{out}");
    assert!(!out.contains("SD647") && !out.contains("SD641"), "{out}");
}

#[test]
fn refresh_writes_the_frontier_of_the_source_layers() {
    let r = storage_project();
    // A new accepted decision, a new note and a note that is gone.
    r.write(
        "docs/decisions/storage/write-ahead-log.md",
        &USE_SQLITE.replace("Use SQLite", "Write-ahead log"),
    );
    r.write(
        "docs/research/storage/read-load.md",
        "# Read load\n\nReads are cheap.\n",
    );
    std::fs::remove_file(r.dir.join("docs/research/storage/write-load.md")).unwrap();
    edit(
        &r,
        STORAGE,
        "write-ahead log.",
        "write-ahead log. Reads are cheap.",
    );
    let (code, out, err) = r.run(&["refresh", "summaries/storage"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("refreshed summaries/storage: 3 pins (2 added, 0 changed, 1 removed)"),
        "{out}"
    );
    let text = r.read(STORAGE);
    assert!(
        text.contains("decisions/storage/write-ahead-log@")
            && text.contains("research/storage/read-load@")
            && !text.contains("write-load"),
        "{text}"
    );
    assert!(!check_all(&r).contains("SD64"));
}

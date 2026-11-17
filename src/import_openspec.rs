//! The reader of a project of OpenSpec 2.0.0 (design 0.7.0, D24; decisions
//! I13 to I15 of the plan). It reads the domain specs, the open changes with
//! their delta specs, and the archive, and it gives a `Read`.
//!
//! - A domain spec `openspec/specs/<domain>/spec.md` moves to a `requirements`
//!   document. Each `### Requirement: <Name>` becomes a list item with the
//!   slug of the name as key, and each `#### Scenario:` stays below it as a
//!   nested item.
//! - An open change becomes a draft `intent.md`, `spec.md` and `plan.md`. A
//!   change with `RENAMED`, or with a block that has no base requirement,
//!   stays in place.
//! - An archived change stays in place, or moves to the research layer.

#![allow(dead_code)]

use crate::import_docs::{
    Ctx, Lines, Pair, PairKind, Read, SourceFile, State, Target, count_scenarios, id_of, indent,
    requirements_path, slug, split_sentence, with_front_matter,
};
use crate::model::is_slug;
use crate::repo::Fatal;
use std::collections::BTreeMap;

/// The supported version of the tool, for a message.
pub const VERSION: &str = "OpenSpec 2.0.0";

fn layout_error(why: &str) -> Fatal {
    Fatal::Config(format!(
        "{why}; the import supports {VERSION} (`openspec/specs/<domain>/spec.md` and `openspec/changes/<name>/`)"
    ))
}

/// The name of a requirement of a heading `### Requirement: <Name>`.
fn requirement_name(heading: &str) -> Option<&str> {
    heading.trim().strip_prefix("Requirement:").map(str::trim)
}

/// Reads a project of OpenSpec.
pub fn read(ctx: &Ctx) -> Result<Read, Fatal> {
    let all = ctx.under("openspec/");
    if all.is_empty() {
        return Err(layout_error("`openspec/` is not in the tree"));
    }
    let specs = ctx.under("openspec/specs/");
    let changes = ctx.under("openspec/changes/");
    let domains: Vec<&str> = specs
        .iter()
        .filter_map(|f| f["openspec/specs/".len()..].split_once('/').map(|(d, _)| d))
        .collect();
    if domains.is_empty() && !specs.is_empty() {
        return Err(layout_error(
            "`openspec/specs/` has no domain folder, so the layout belongs to another major version",
        ));
    }
    let known = ["openspec/config.yaml", "openspec/project.md"];
    if specs.is_empty() && changes.is_empty() && !all.iter().any(|f| known.contains(f)) {
        return Err(layout_error(
            "`openspec/` has no `specs/`, no `changes/` and no `config.yaml`",
        ));
    }
    let mut read = Read::default();
    read.universe = all.iter().map(|f| f.to_string()).collect();
    for f in &all {
        if f.starts_with("openspec/specs/") || f.starts_with("openspec/changes/") {
            continue;
        }
        let reason = match *f {
            "openspec/config.yaml" => "a file of the tool",
            "openspec/project.md" => "a legacy file",
            _ => "not read by the import",
        };
        read.sources.push(SourceFile::kept(f, reason));
    }
    let mut by_domain: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for f in &specs {
        match f["openspec/specs/".len()..].split_once('/') {
            Some((d, rest)) => by_domain.entry(d).or_default().push(rest),
            None => read
                .sources
                .push(SourceFile::skipped(f, "not in a domain folder")),
        }
    }
    for (domain, files) in by_domain {
        domain_spec(ctx, &mut read, domain, &files)?;
    }
    for f in &changes {
        read.sources.push(SourceFile::kept(
            f,
            "a change; the import does not read it yet",
        ));
    }
    read.sources.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(read)
}

// --- the domain specs ---

/// A requirement block of a spec: the key, the line of its heading (from 0),
/// the sentence, the lines after the sentence, and the scenario blocks.
struct Block {
    key: String,
    line: usize,
    tree_end: usize,
    /// The paragraphs before the first list.
    sentence: String,
    /// The list that follows the sentence, before the first scenario.
    rest: String,
    /// The sub-sections: the heading text and the lines under it.
    scenarios: Vec<(String, String)>,
}

/// The requirement blocks of the headings at `level`, from heading `from` to
/// the end of its tree.
fn blocks(l: &Lines, path: &str) -> Result<Vec<Block>, Fatal> {
    let mut out = Vec::new();
    for (i, h) in l.headings.iter().enumerate() {
        if h.level != 3 {
            continue;
        }
        let Some(name) = requirement_name(&h.text) else {
            continue;
        };
        let key = slug(name);
        if key.is_empty() {
            return Err(Fatal::Refused(format!(
                "{path}:{}: cannot make a key from the requirement name `{name}` (it has no letter or digit); rename it, commit, then run import again",
                h.line + 1
            )));
        }
        let (sentence, rest) = split_sentence(&l.block(h.line + 1, l.own_end(i)));
        let end = l.tree_end(i);
        let mut scenarios = Vec::new();
        for (j, c) in l.headings.iter().enumerate().skip(i + 1) {
            if c.line >= end {
                break;
            }
            scenarios.push((c.text.clone(), l.block(c.line + 1, l.own_end(j))));
        }
        out.push(Block {
            key,
            line: h.line,
            tree_end: end,
            sentence,
            rest,
            scenarios,
        });
    }
    Ok(out)
}

/// The list item of a requirement: the sentence, the list after it, and each
/// scenario as a nested item.
fn requirement_item(b: &Block) -> String {
    let mut out = crate::import_speckit::item(&b.key, &b.sentence);
    let mut nested: Vec<String> = Vec::new();
    if !b.rest.is_empty() {
        nested.push(indent(&b.rest, 2));
    }
    let scen: Vec<String> = b
        .scenarios
        .iter()
        .map(|(title, body)| {
            let mut s = format!("  - {title}");
            if !body.is_empty() {
                s.push('\n');
                s.push_str(&indent(body, 4));
            }
            s
        })
        .collect();
    if !scen.is_empty() {
        nested.push(scen.join("\n"));
    }
    for n in nested {
        out.push_str("\n\n");
        out.push_str(&n);
    }
    out
}

fn domain_spec(ctx: &Ctx, read: &mut Read, domain: &str, files: &[&str]) -> Result<(), Fatal> {
    let path = |f: &str| format!("openspec/specs/{domain}/{f}");
    if !is_slug(domain) {
        read.sources.push(SourceFile::skipped(
            &format!("openspec/specs/{domain}/"),
            "the domain name is not a slug (lowercase letters and digits joined by hyphens)",
        ));
        return Ok(());
    }
    for f in files.iter().filter(|f| **f != "spec.md") {
        read.sources
            .push(SourceFile::kept(&path(f), "an extra file of the domain"));
    }
    if !files.contains(&"spec.md") {
        return Ok(());
    }
    let from = path("spec.md");
    let orig = ctx.text(&from)?;
    let ol = Lines::new(&orig);
    let found = blocks(&ol, &from)?;
    if found.is_empty() {
        return Err(Fatal::Config(format!(
            "{from}: the spec file has no requirement (no `### Requirement:` heading); the import supports {VERSION}"
        )));
    }
    let to = requirements_path(ctx.places.requirements_dir()?, ctx.area_of(domain), domain);
    let id = id_of(ctx.cfg, &to).unwrap_or_default();
    let linked = (ctx.link)(&orig, &from, &to);
    let l = Lines::new(&linked);
    let lb = blocks(&l, &from)?;
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < l.lines.len() {
        if let Some(b) = lb.iter().find(|b| b.line == i) {
            out.push(requirement_item(b));
            out.push(String::new());
            i = b.tree_end;
            continue;
        }
        out.push(l.lines[i].to_string());
        i += 1;
    }
    let mut text = out.join("\n");
    while text.ends_with("\n\n") {
        text.pop();
    }
    if !text.ends_with('\n') {
        text.push('\n');
    }
    let mut aliases = vec![domain.to_string()];
    aliases.extend(ctx.old_alias(&from, &id));
    let text = with_front_matter(&text, "active", &[], &aliases);
    for b in &found {
        read.pairs.push(Pair {
            from: format!("{from}:{}", b.line + 1),
            id: format!("{id}#{}", b.key),
            delta: None,
            target: to.clone(),
            key: b.key.clone(),
            kind: PairKind::Requirement,
            sentence: b.sentence.clone(),
            question: None,
            verb: None,
        });
    }
    read.sources.push(SourceFile {
        path: from.clone(),
        state: State::Moved,
        to: vec![to.clone()],
        reason: "a domain spec".to_string(),
    });
    read.targets.push(Target {
        path: to,
        text,
        moved_from: Some(from.clone()),
        sources: vec![from],
        link_of: None,
        scenarios: Some(count_scenarios(&orig)),
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::import_docs::{Places, State, corpus_of};
    use crate::upgrade_docs::TreeData;

    const CFG: &str = "spec-dna = \"0.6.1\"\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.intent]\ntypes = [\"intent\", \"spec\", \"plan\"]\n\n[layers.research]\ntypes = [\"research\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n";

    const AUTH: &str = "# Auth Specification\n\n## Purpose\n\nPeople sign in.\n\n## Requirements\n\n### Requirement: Password Sign In\n\nThe system SHALL accept a valid password.\n\n#### Scenario: Valid password\n\n- **WHEN** a person sends a valid password\n- **THEN** the system opens a session\n\n#### Scenario: Wrong password\n\n- **WHEN** a person sends a wrong password\n- **THEN** the system refuses\n\n### Requirement: Session Expiry\n\nThe system SHALL end an idle session.\n\nLong sessions are a risk.\n\n- A list in the sentence.\n\n#### Scenario: Idle\n\n- **WHEN** a session is idle\n- **THEN** it ends\n";

    struct Run {
        read: Result<Read, Fatal>,
    }

    fn run_with(
        files: &[(&str, &str)],
        skip_open: bool,
        keep: bool,
        areas: &[(&str, &str)],
    ) -> Run {
        let mut all: Vec<(&str, &str)> = vec![("spec-dna.toml", CFG)];
        all.extend_from_slice(files);
        let tree = TreeData::from_map(
            all.iter()
                .map(|(p, t)| (p.to_string(), t.as_bytes().to_vec()))
                .collect(),
        );
        let cfg = Config::parse(CFG).unwrap();
        let places = Places::of(&cfg);
        let areas: Vec<(String, String)> = areas
            .iter()
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        let read = tree.with(|t| {
            let (before, _) = corpus_of(t, &cfg)?;
            read(&Ctx {
                tree: t,
                cfg: &cfg,
                places: &places,
                areas: &areas,
                skip_open,
                keep_research: keep,
                link: &|text, _, _| text.to_string(),
                before: &before,
            })
        });
        Run { read }
    }

    fn run(files: &[(&str, &str)]) -> Run {
        run_with(files, false, false, &[])
    }

    impl Run {
        fn ok(&self) -> &Read {
            self.read
                .as_ref()
                .unwrap_or_else(|e| panic!("{}", e.message()))
        }

        fn target(&self, path: &str) -> &str {
            let r = self.ok();
            r.targets
                .iter()
                .find(|t| t.path == path)
                .map(|t| t.text.as_str())
                .unwrap_or_else(|| {
                    panic!(
                        "no target {path}: {:?}",
                        r.targets.iter().map(|t| &t.path).collect::<Vec<_>>()
                    )
                })
        }

        fn source(&self, path: &str) -> &crate::import_docs::SourceFile {
            self.ok()
                .sources
                .iter()
                .find(|s| s.path == path)
                .unwrap_or_else(|| panic!("no source {path}: {:?}", self.ok().sources))
        }

        fn err(&self) -> &Fatal {
            self.read.as_ref().err().expect("an error")
        }
    }

    fn base(extra: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
        let mut v = vec![("openspec/config.yaml", "schema: x\n")];
        v.extend_from_slice(extra);
        v
    }

    // --- the layout, D25 ---

    #[test]
    fn a_layout_that_the_import_does_not_know_is_exit_code_3_and_names_the_rule() {
        let r = run(&[("docs/a.md", "# A\n")]);
        let e = r.err();
        assert!(matches!(e, Fatal::Config(_)));
        assert!(
            e.message().contains("openspec/") && e.message().contains(VERSION),
            "{}",
            e.message()
        );
        // A specs folder with a file and no domain folder is another major version.
        let r = run(&base(&[("openspec/specs/auth.md", "# Auth\n")]));
        assert!(
            r.err().message().contains("another major version"),
            "{}",
            r.err().message()
        );
        // A folder with only the tool file is no stop: a second run moved the specs.
        assert!(run(&base(&[])).read.is_ok());
        // A spec file with no requirement stops, and names the file.
        let r = run(&base(&[(
            "openspec/specs/auth/spec.md",
            "# Auth\n\n## Requirements\n\nNone yet.\n",
        )]));
        let m = r.err().message().to_string();
        assert!(
            m.contains("openspec/specs/auth/spec.md") && m.contains("no requirement"),
            "{m}"
        );
    }

    // --- the domain specs ---

    #[test]
    fn a_domain_spec_moves_to_a_requirements_document() {
        let r = run(&base(&[("openspec/specs/auth/spec.md", AUTH)]));
        let t = r.target("docs/requirements/auth.md");
        let want = "---\nstatus: active\naliases:\n  - auth\n---\n# Auth Specification\n\n## Purpose\n\nPeople sign in.\n\n## Requirements\n\n- **password-sign-in**: The system SHALL accept a valid password.\n\n  - Scenario: Valid password\n    - **WHEN** a person sends a valid password\n    - **THEN** the system opens a session\n  - Scenario: Wrong password\n    - **WHEN** a person sends a wrong password\n    - **THEN** the system refuses\n\n- **session-expiry**: The system SHALL end an idle session.\n\n  Long sessions are a risk.\n\n  - A list in the sentence.\n\n  - Scenario: Idle\n    - **WHEN** a session is idle\n    - **THEN** it ends\n";
        assert_eq!(t, want);
        let s = r.source("openspec/specs/auth/spec.md");
        assert_eq!(
            (s.state, s.to.as_slice()),
            (State::Moved, &["docs/requirements/auth.md".to_string()][..])
        );
        let tg = r
            .ok()
            .targets
            .iter()
            .find(|t| t.path == "docs/requirements/auth.md")
            .unwrap();
        assert_eq!(
            tg.moved_from.as_deref(),
            Some("openspec/specs/auth/spec.md")
        );
        assert_eq!(tg.scenarios, Some(3));
    }

    #[test]
    fn each_requirement_is_a_pair_with_its_sentence_up_to_the_first_list() {
        let r = run(&base(&[("openspec/specs/auth/spec.md", AUTH)]));
        let p: Vec<(&str, &str, &str)> = r
            .ok()
            .pairs
            .iter()
            .map(|p| (p.from.as_str(), p.id.as_str(), p.sentence.as_str()))
            .collect();
        assert_eq!(
            p,
            [
                (
                    "openspec/specs/auth/spec.md:9",
                    "requirements/auth#password-sign-in",
                    "The system SHALL accept a valid password."
                ),
                (
                    "openspec/specs/auth/spec.md:23",
                    "requirements/auth#session-expiry",
                    "The system SHALL end an idle session.\n\nLong sessions are a risk."
                ),
            ]
        );
        assert!(
            r.ok()
                .pairs
                .iter()
                .all(|p| p.kind == crate::import_docs::PairKind::Requirement && p.delta.is_none())
        );
    }

    #[test]
    fn an_area_places_the_domain_spec_and_a_name_with_no_letter_stops() {
        let r = run_with(
            &base(&[("openspec/specs/auth/spec.md", AUTH)]),
            false,
            false,
            &[("auth", "media")],
        );
        r.target("docs/requirements/media/auth.md");
        assert_eq!(
            r.ok().pairs[0].id,
            "requirements/media/auth#password-sign-in"
        );
        let r = run(&base(&[(
            "openspec/specs/auth/spec.md",
            "# A\n\n### Requirement: !!!\n\nThe system SHALL do it.\n",
        )]));
        assert!(matches!(r.err(), Fatal::Refused(_)));
        assert!(
            r.err().message().contains("openspec/specs/auth/spec.md:3"),
            "{}",
            r.err().message()
        );
    }

    #[test]
    fn the_tool_files_and_the_extra_files_of_a_domain_stay() {
        let r = run(&base(&[
            ("openspec/specs/auth/spec.md", AUTH),
            ("openspec/specs/auth/notes.md", "# N\n"),
            ("openspec/project.md", "# P\n"),
            ("openspec/schemas/x/schema.yaml", "a: 1\n"),
            ("openspec/specs/Bad_Name/spec.md", AUTH),
        ]));
        assert_eq!(
            r.source("openspec/config.yaml").reason,
            "a file of the tool"
        );
        assert_eq!(r.source("openspec/project.md").reason, "a legacy file");
        assert_eq!(
            r.source("openspec/schemas/x/schema.yaml").state,
            State::Kept
        );
        assert_eq!(r.source("openspec/specs/auth/notes.md").state, State::Kept);
        let s = r.source("openspec/specs/Bad_Name/");
        assert_eq!(s.state, State::Skipped);
        assert!(s.reason.contains("not a slug"), "{}", s.reason);
    }

    // --- the open changes ---

    const PROPOSAL: &str = "## Why\n\nPasswords alone are weak.\n\n## What Changes\n\n- Add a second factor.\n\n## Capabilities\n\n### Modified Capabilities\n\n- auth\n\n## Impact\n\nClients need an update. See the [design](./design.md).\n";
    const DESIGN: &str = "## Context\n\nA 6 digit code.\n\n### Detail\n\nMore.\n";
    const TASKS: &str = "## 1. Backend\n\n- [ ] 1.1 Add the table\n- [x] 1.2 Add the check\n\n## 2. Clients\n\n- [ ] 2.3.1 Ask for the code\n- [ ] Update the help\n";
    const DELTA: &str = "## ADDED Requirements\n\n### Requirement: Second Factor\n\nThe system SHALL ask for a code.\n\nThe code MUST come from a device.\n\n#### Scenario: Valid code\n\n- **WHEN** the code is right\n- **THEN** a session opens\n\n## MODIFIED Requirements\n\n### Requirement: Password Sign In\n\nThe system SHALL accept a valid password and then ask for a code.\n\n#### Scenario: Valid password\n\n- **WHEN** a password is valid\n- **THEN** the system asks for a code\n\n## REMOVED Requirements\n\n### Requirement: Session Expiry\n\n**Reason**: The code ends a session.\n\n**Migration**: Use the code expiry.\n";

    fn change(extra: &[(&'static str, &'static str)]) -> Vec<(&'static str, &'static str)> {
        let mut v = base(&[
            ("openspec/specs/auth/spec.md", AUTH),
            ("openspec/changes/add-two-factor/proposal.md", PROPOSAL),
            ("openspec/changes/add-two-factor/design.md", DESIGN),
            ("openspec/changes/add-two-factor/tasks.md", TASKS),
            ("openspec/changes/add-two-factor/specs/auth/spec.md", DELTA),
        ]);
        v.extend_from_slice(extra);
        v
    }

    #[test]
    fn a_change_gives_a_draft_intent_spec_and_plan_and_its_sources_stay() {
        let r = run(&change(&[]));
        for f in ["intent", "spec", "plan"] {
            r.target(&format!("docs/intent/add-two-factor/{f}.md"));
        }
        let to = |p: &str| r.source(p).to.clone();
        let c = "openspec/changes/add-two-factor";
        assert_eq!(
            to(&format!("{c}/proposal.md")),
            ["docs/intent/add-two-factor/intent.md"]
        );
        assert_eq!(
            to(&format!("{c}/design.md")),
            ["docs/intent/add-two-factor/plan.md"]
        );
        assert_eq!(
            to(&format!("{c}/tasks.md")),
            ["docs/intent/add-two-factor/plan.md"]
        );
        assert_eq!(
            to(&format!("{c}/specs/auth/spec.md")),
            ["docs/intent/add-two-factor/spec.md"]
        );
        assert_eq!(r.source(&format!("{c}/tasks.md")).state, State::Written);
        assert_eq!(r.ok().removes, [c]);
        let link: Vec<(&str, Option<&str>)> = r
            .ok()
            .targets
            .iter()
            .filter(|t| t.moved_from.is_none())
            .map(|t| (t.path.as_str(), t.link_of.as_deref()))
            .collect();
        assert!(
            link.contains(&(
                "docs/intent/add-two-factor/intent.md",
                Some("openspec/changes/add-two-factor/proposal.md")
            )),
            "{link:?}"
        );
        assert!(
            link.contains(&(
                "docs/intent/add-two-factor/spec.md",
                Some("openspec/changes/add-two-factor/specs/auth/spec.md")
            )),
            "{link:?}"
        );
    }

    #[test]
    fn the_proposal_gives_why_now_and_the_intent() {
        let r = run(&change(&[]));
        let t = r.target("docs/intent/add-two-factor/intent.md");
        let want = "---\nstatus: draft\nclass: higher-risk\n---\n# Add two factor\n\n## Intent\n\n- Add a second factor.\n\nClients need an update. See the [design](./design.md).\n\n### Capabilities\n\n#### Modified Capabilities\n\n- auth\n\n## Why now\n\nPasswords alone are weak.\n";
        assert_eq!(t, want);
    }

    #[test]
    fn a_proposal_with_none_of_the_headings_is_the_intent_as_a_whole() {
        let r = run(&base(&[
            (
                "openspec/changes/x/proposal.md",
                "Some text.\n\n## Other\n\nMore.\n",
            ),
            ("openspec/changes/x/tasks.md", "- [ ] 1.1 A\n"),
        ]));
        let t = r.target("docs/intent/x/intent.md");
        assert!(
            t.contains("\n## Intent\n\nSome text.\n\n## Other\n\nMore.\n"),
            "{t}"
        );
        assert!(
            t.starts_with("---\nstatus: draft\nclass: routine\n---\n"),
            "{t}"
        );
    }

    #[test]
    fn the_design_and_the_tasks_give_the_sections_of_the_plan() {
        let r = run(&change(&[]));
        let t = r.target("docs/intent/add-two-factor/plan.md");
        let want = "---\nstatus: draft\n---\n# Add two factor\n\n## Files\n\n- none\n\n## Design\n\n### Context\n\nA 6 digit code.\n\n#### Detail\n\nMore.\n\n## Order\n\n**1. Backend**\n\n- [ ] t1-1: Add the table.\n- [x] t1-2: Add the check.\n\n**2. Clients**\n\n- [ ] t2-3-1: Ask for the code.\n- [ ] t4: Update the help.\n";
        assert_eq!(t, want);
    }

    #[test]
    fn the_delta_has_a_line_for_each_block_and_the_hash_of_the_imported_text() {
        let r = run(&change(&[]));
        let t = r.target("docs/intent/add-two-factor/spec.md");
        let doc = crate::parse::parse(r.target("docs/requirements/auth.md"));
        let h = |k: &str| crate::hash::hex(&doc.reqs.iter().find(|r| r.key == k).unwrap().hash);
        let want = format!(
            "---\nstatus: draft\n---\n# Add two factor\n\n## Delta\n\n- ADD requirements/auth#second-factor: The system SHALL ask for a code. The code MUST come from a device.\n- CHANGE requirements/auth#password-sign-in@{}: The system SHALL accept a valid password and then ask for a code.\n- REMOVE requirements/auth#session-expiry@{}\n\n## Scenarios\n\n### auth#second-factor\n\n#### Scenario: Valid code\n\n- **WHEN** the code is right\n- **THEN** a session opens\n\n### auth#password-sign-in\n\n#### Scenario: Valid password\n\n- **WHEN** a password is valid\n- **THEN** the system asks for a code\n\n### auth#session-expiry\n\n**Reason**: The code ends a session.\n\n**Migration**: Use the code expiry.\n\n## Open questions\n\n- none.\n\n## Concerns\n\n- none.\n",
            h("password-sign-in"),
            h("session-expiry")
        );
        assert_eq!(t, want);
        let tg = r
            .ok()
            .targets
            .iter()
            .find(|t| t.path.ends_with("add-two-factor/spec.md"))
            .unwrap();
        assert_eq!(tg.scenarios, Some(2));
        let p: Vec<(&str, Option<&str>)> = r
            .ok()
            .pairs
            .iter()
            .filter(|p| p.delta.is_some())
            .map(|p| (p.id.as_str(), p.verb))
            .collect();
        assert_eq!(
            p,
            [
                ("requirements/auth#second-factor", Some("ADD")),
                ("requirements/auth#password-sign-in", Some("CHANGE")),
                ("requirements/auth#session-expiry", Some("REMOVE")),
            ]
        );
        let first = r.ok().pairs.iter().find(|p| p.delta.is_some()).unwrap();
        assert_eq!(
            first.sentence,
            "The system SHALL ask for a code.\n\nThe code MUST come from a device."
        );
    }

    #[test]
    fn a_purpose_of_a_delta_spec_goes_to_the_scenarios_under_its_domain() {
        let delta = "## Purpose\n\nWhy this domain.\n\n## ADDED Requirements\n\n### Requirement: Report\n\nThe system SHALL report.\n";
        let r = run(&base(&[
            ("openspec/changes/add-reports/proposal.md", "## Why\n\nx\n"),
            ("openspec/changes/add-reports/specs/reports/spec.md", delta),
        ]));
        let t = r.target("docs/intent/add-reports/spec.md");
        assert!(
            t.contains("- ADD requirements/reports#report: The system SHALL report.\n"),
            "{t}"
        );
        assert!(
            t.contains("\n## Scenarios\n\n### reports\n\nWhy this domain.\n"),
            "{t}"
        );
    }

    #[test]
    fn a_change_with_renamed_or_with_no_base_requirement_stays_in_place() {
        let renamed =
            "## RENAMED Requirements\n\n- FROM: `### Requirement: A`\n- TO: `### Requirement: B`\n";
        let r = run(&change(&[
            ("openspec/changes/rn/proposal.md", PROPOSAL),
            ("openspec/changes/rn/specs/auth/spec.md", renamed),
        ]));
        let s = r.source("openspec/changes/rn/specs/auth/spec.md");
        assert_eq!(
            (s.state, s.reason.as_str()),
            (State::Kept, "the change has `RENAMED`")
        );
        assert_eq!(
            r.source("openspec/changes/rn/proposal.md").state,
            State::Kept
        );
        assert!(r.ok().targets.iter().all(|t| !t.path.contains("/rn/")));
        let orphan = "## MODIFIED Requirements\n\n### Requirement: Rate Limit\n\nThe system SHALL limit it.\n";
        let r = run(&base(&[
            ("openspec/changes/lim/proposal.md", PROPOSAL),
            ("openspec/changes/lim/specs/security/spec.md", orphan),
        ]));
        let s = r.source("openspec/changes/lim/specs/security/spec.md");
        assert_eq!(s.reason, "no base requirement for `rate-limit`");
        assert_eq!(s.state, State::Kept);
        assert_eq!(
            r.source("openspec/changes/lim/proposal.md").state,
            State::Kept
        );
        // A base in a document of the tree is a base too.
        let doc = "---\nstatus: active\naliases:\n  - security\n---\n# S\n\n- **rate-limit**: The system SHALL limit.\n";
        let r = run(&base(&[
            ("docs/requirements/security.md", doc),
            ("openspec/changes/lim/proposal.md", PROPOSAL),
            ("openspec/changes/lim/specs/security/spec.md", orphan),
        ]));
        let t = r.target("docs/intent/lim/spec.md");
        assert!(
            t.contains("- CHANGE requirements/security#rate-limit@"),
            "{t}"
        );
    }

    #[test]
    fn a_delta_spec_with_no_requirement_stops_the_import() {
        let r = run(&base(&[
            ("openspec/changes/x/proposal.md", PROPOSAL),
            (
                "openspec/changes/x/specs/auth/spec.md",
                "## ADDED Requirements\n\nNone.\n",
            ),
        ]));
        let m = r.err().message().to_string();
        assert!(matches!(r.err(), Fatal::Config(_)));
        assert!(
            m.contains("openspec/changes/x/specs/auth/spec.md") && m.contains("no requirement"),
            "{m}"
        );
    }

    #[test]
    fn skip_open_does_not_read_a_change() {
        let files = base(&[
            ("openspec/changes/x/proposal.md", PROPOSAL),
            (
                "openspec/changes/x/specs/auth/spec.md",
                "## ADDED Requirements\n\nNone.\n",
            ),
        ]);
        assert!(run(&files).read.is_err());
        let r = run_with(&files, true, false, &[]);
        let s = r.source("openspec/changes/x/proposal.md");
        assert_eq!(s.state, State::Skipped);
        assert!(s.reason.contains("--skip-open"), "{}", s.reason);
    }

    #[test]
    fn the_archive_stays_with_a_count_or_moves_to_research() {
        let files = base(&[
            (
                "openspec/changes/archive/2026-01-10-first/proposal.md",
                PROPOSAL,
            ),
            ("openspec/changes/archive/2026-01-10-first/tasks.md", TASKS),
            (
                "openspec/changes/archive/2026-01-10-first/data.json",
                "{}\n",
            ),
        ]);
        let r = run(&files);
        let s = r.source("openspec/changes/archive/2026-01-10-first/");
        assert_eq!(
            (s.state, s.reason.as_str()),
            (State::Kept, "an archived change (3 files)")
        );
        let r = run_with(&files, false, true, &[]);
        r.target("docs/research/imported/2026-01-10-first/proposal.md");
        let s = r.source("openspec/changes/archive/2026-01-10-first/tasks.md");
        assert_eq!(s.state, State::Moved);
        assert_eq!(
            r.source("openspec/changes/archive/2026-01-10-first/data.json")
                .reason,
            "not a Markdown file"
        );
        // A skip of the open changes keeps the archive in place.
        let r = run_with(&files, true, false, &[]);
        assert_eq!(
            r.source("openspec/changes/archive/2026-01-10-first/").state,
            State::Kept
        );
    }

    #[test]
    fn the_extra_files_of_a_change_stay_and_an_area_places_the_delta_id() {
        let r = run_with(
            &change(&[
                ("openspec/changes/add-two-factor/notes.md", "# N\n"),
                ("openspec/changes/add-two-factor/.openspec.yaml", "a: 1\n"),
            ]),
            false,
            false,
            &[("auth", "media")],
        );
        assert_eq!(
            r.source("openspec/changes/add-two-factor/notes.md").state,
            State::Kept
        );
        assert_eq!(
            r.source("openspec/changes/add-two-factor/.openspec.yaml")
                .state,
            State::Kept
        );
        let t = r.target("docs/intent/add-two-factor/spec.md");
        assert!(
            t.contains("- ADD requirements/media/auth#second-factor:"),
            "{t}"
        );
    }
}

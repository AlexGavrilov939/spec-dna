//! The template of each document type (docs/types.md). `spec-dna new` writes a
//! document from it. A template has the front matter, the title and the
//! required sections of its type. `check` reports each placeholder
//! (`{{text}}`) that stays in a document (SD306).

use crate::model::DocType;
use std::path::Path;

/// The embedded templates: the name, the type and the body, after the front
/// matter and the title (design 0.6.0, "Templates"). The default template of a
/// type has a name from `default_name`.
const EMBEDDED: [(&str, DocType, &str); 18] = [
    (
        "product",
        DocType::Product,
        "## Purpose

{{What the product does, and for whom.}}

## Scope

{{What the product covers, and what it leaves out.}}
",
    ),
    (
        "system",
        DocType::System,
        "## Overview

{{The parts of the system and how they work together.}}

## Constraints

- {{A constraint that every part keeps.}}
",
    ),
    (
        "terms",
        DocType::Terms,
        "## Terms

| Term | Meaning | Forbidden synonyms |
| --- | --- | --- |
| {{term}} | {{meaning}} | {{synonyms}} |
",
    ),
    (
        "roadmap",
        DocType::Roadmap,
        "## Next

- {{Planned work.}}

## Later

- {{Work with no date.}}
",
    ),
    (
        "questions",
        DocType::Questions,
        "## Open

- {{A question that has no answer yet, and who can answer it.}}
",
    ),
    (
        "requirements",
        DocType::Requirements,
        "## Requirements

- **{{key}}**: When {{trigger}}, the {{system}} SHALL {{response}}.

## Notes

{{Notes. Cite the code of the requirements in inline code spans.}}
",
    ),
    (
        "interface",
        DocType::Interface,
        "## Interface

{{The operations, or a link to the OpenAPI, AsyncAPI or schema file.}}

## Requirements

- **{{key}}**: The {{service}} SHALL {{behavior of the interface}}.
",
    ),
    (
        "procedure",
        DocType::Procedure,
        "## Purpose

{{When to use this procedure, and what it gives.}}

## Steps

1. {{Command or action.}}
2. {{Check.}}
",
    ),
    (
        "runbook",
        DocType::Procedure,
        "## When to use

{{The alert or the event that starts this runbook.}}

## Steps

1. {{Command or action.}}

## Verify

{{How to see that the steps worked.}}

## Rollback

1. {{How to undo the steps.}}
",
    ),
    (
        "madr",
        DocType::Decision,
        "## Context and Problem Statement

{{The problem in 2 or 3 sentences.}}

## Considered Options

- {{Option.}}
- {{Option.}}

## Decision Outcome

Chosen option: {{option}}, because {{reason}}.

## Consequences

- Good: {{effect}}.
- Bad: {{effect}}.

## Status

{{The state of the work that the decision starts. This section can change in place.}}

## Amendments
",
    ),
    (
        "intent",
        DocType::Intent,
        "## Intent

{{What this change gives, and for whom.}}

## Why now

{{The reason to do it now.}}

## Out of scope

- {{What this change does not do.}}
",
    ),
    (
        "prd",
        DocType::Intent,
        "## Problem

{{The problem, and who has it.}}

## Users

{{Who needs this change.}}

## Goals

- {{A goal that the change reaches.}}

## Non-goals

- {{What this change does not do.}}

## Success measures

- {{A measure, and its target.}}

## Open questions

- {{A question that has no answer yet.}}
",
    ),
    (
        "rfc",
        DocType::Intent,
        "## Summary

{{The proposal in 2 or 3 sentences.}}

## Motivation

{{Why the change is needed now.}}

## Proposal

{{The design of the change.}}

## Alternatives

- {{An option that the change does not take, and why.}}

## Unresolved

- {{A question that has no answer yet.}}
",
    ),
    (
        "spec",
        DocType::Spec,
        "## Delta

{{The requirements that this change adds, changes or removes.}}

## Interfaces

{{The interfaces that this change touches.}}

## Decisions

{{The decisions that this change needs.}}

## Concerns

- [ ] {{An open concern.}}

## Open questions

- [ ] {{A question that has no answer yet.}}
",
    ),
    (
        "plan",
        DocType::Plan,
        "## Files

- `{{path}}` {{Why this change touches the path.}}

## Order

- [ ] t1: {{The first step.}} Files: `{{path}}`. Verify: {{The check of the step.}}
- [ ] t2: {{The next step.}} Files: `{{path}}`. Verify: {{The check of the step.}}

## Risks

- {{Risk.}}

## Proof

{{How to see that the change works.}}
",
    ),
    (
        "rule",
        DocType::Rule,
        "## Rules

- **{{key}}**: The {{system}} SHALL {{rule that every change in the scope keeps}}.

## Rationale

{{Why the rule exists. Link the decision.}}
",
    ),
    (
        "post-mortem",
        DocType::Incident,
        "## Summary

{{What happened, when, and the impact, in 2 or 3 sentences.}}

## Timeline

- {{Time}}: {{event}}.

## Root cause

{{The cause.}}

## Links

- {{Links to the requirements, rules, decisions or procedures, such as [[doc-id#key]]}}

## Follow-ups

- [ ] {{Action.}}

## Eval

{{The eval file in the evals folder that covers this incident.}}
",
    ),
    (
        "research",
        DocType::Research,
        "{{Research, data or other cold material. spec-dna context does not serve this document unless a target names it.}}\n",
    ),
];

/// Returns the name of the default embedded template of a type.
pub fn default_name(ty: DocType) -> &'static str {
    match ty {
        DocType::Decision => "madr",
        DocType::Incident => "post-mortem",
        t => t.name(),
    }
}

/// Returns the name of each embedded template, with its type.
pub fn embedded_names() -> impl Iterator<Item = (&'static str, DocType)> {
    EMBEDDED.iter().map(|(n, t, _)| (*n, *t))
}

/// Returns the first status of a new document of a type.
pub fn first_status(ty: DocType) -> Option<&'static str> {
    match ty {
        DocType::Decision => Some("proposed"),
        t if t.has_status() => Some("draft"),
        _ => None,
    }
}

/// Returns the type and the text of an embedded template: the front matter
/// with the type and the first status, the title and the body.
pub fn embedded(name: &str) -> Option<(DocType, String)> {
    let (_, ty, body) = EMBEDDED.iter().find(|(n, _, _)| *n == name)?;
    let ty = *ty;
    let mut fm = format!("type: {}\n", ty.name());
    if let Some(s) = first_status(ty) {
        fm.push_str(&format!("status: {s}\n"));
    }
    if ty == DocType::Intent {
        // The author of a change chooses its class. `routine` is the usual
        // one, and `higher-risk` adds a spec (docs/types.md).
        fm.push_str("class: routine\n");
    }
    if ty == DocType::Rule {
        // A rule applies to the paths of its scope. "**" is the whole
        // project: narrow it.
        fm.push_str("applies-to: [\"**\"]\n");
    }
    Some((ty, format!("---\n{fm}---\n# Title\n\n{body}")))
}

/// Returns the text of a template of a type, from the project folder or from
/// the embedded templates, and the name that selected it. `name` is the value
/// of `--template`. Without it, the template is the default of the type. The
/// file `<dir>/<name>.md` of the project has priority over the embedded
/// template of that name. The error is a message for exit code 2.
pub fn resolve(
    repo_root: &Path,
    dir: &str,
    ty: DocType,
    name: Option<&str>,
) -> Result<(String, String), String> {
    let chosen = match name {
        Some(n) => {
            let n = n.trim();
            let n = n.strip_suffix(".md").unwrap_or(n);
            if n.is_empty() || n.starts_with('.') || n.contains(['/', '\\', ':']) {
                return Err(format!(
                    "`{n}` is not a template name: write one name, with no path"
                ));
            }
            n
        }
        None => default_name(ty),
    };
    let file = repo_root.join(dir).join(format!("{chosen}.md"));
    if file.is_file() {
        let text = std::fs::read_to_string(&file)
            .map_err(|e| format!("cannot read the template `{dir}/{chosen}.md`: {e}"))?;
        return match front_matter_type(&text) {
            None => Err(format!(
                "the template `{dir}/{chosen}.md` has no `type` in its front matter; write `type: {}`",
                ty.name()
            )),
            Some(t) if t == ty.name() => Ok((chosen.to_string(), text)),
            Some(t) => Err(format!(
                "the template `{dir}/{chosen}.md` is a template of type `{t}`, not `{}`",
                ty.name()
            )),
        };
    }
    match embedded(chosen) {
        Some((t, text)) if t == ty => Ok((chosen.to_string(), text)),
        Some((t, _)) => Err(format!(
            "the template `{chosen}` is a template of type `{}`, not `{}`",
            t.name(),
            ty.name()
        )),
        None => {
            let names: Vec<&str> = embedded_names()
                .filter(|(_, t)| *t == ty)
                .map(|(n, _)| n)
                .collect();
            Err(format!(
                "unknown template `{chosen}` for `{}`; embedded: {}; a project template is `{dir}/{chosen}.md`",
                ty.name(),
                names.join(", ")
            ))
        }
    }
}

/// Returns the lines of the front matter of a text, if it has one.
fn front_matter_lines(text: &str) -> Option<Vec<&str>> {
    let mut lines = text.lines();
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    let mut out = Vec::new();
    for l in lines {
        if l.trim_end() == "---" {
            return Some(out);
        }
        out.push(l);
    }
    None
}

/// Returns the value of the front matter field `type` of a template, with no
/// quotes.
fn front_matter_type(text: &str) -> Option<String> {
    front_matter_lines(text)?.into_iter().find_map(|l| {
        let v = l.strip_prefix("type:")?.trim();
        let v = v.trim_matches(['"', '\'']).trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// Makes a new document from the text of a template: it writes the H1 from
/// `title`, sets the first status of the type, removes `was-type` (`upgrade`
/// writes it, and a copy of it would give a new document the exemption of
/// SD202), and removes `type` where the place of the document gives its type
/// (`with_type` is false). The text of a template that has no front matter gets
/// the front matter that its type needs.
pub fn instantiate(text: &str, ty: DocType, title: &str, with_type: bool) -> String {
    let (fm, body) = match front_matter_lines(text) {
        Some(lines) => {
            let n = lines.len() + 2;
            let rest: String = text.split_inclusive('\n').skip(n).collect();
            (lines, rest)
        }
        None => (Vec::new(), text.to_string()),
    };
    let key = |l: &str| l.split(':').next().unwrap_or("").trim().to_string();
    let mut lines: Vec<String> = Vec::new();
    let mut status_done = false;
    for l in fm {
        match key(l).as_str() {
            "was-type" => {}
            "type" if !with_type => {}
            "status" if first_status(ty).is_some() => {
                lines.push(format!("status: {}", first_status(ty).unwrap_or("")));
                status_done = true;
            }
            _ => lines.push(l.trim_end().to_string()),
        }
    }
    if let Some(s) = first_status(ty).filter(|_| !status_done) {
        lines.push(format!("status: {s}"));
    }
    if with_type && !lines.iter().any(|l| key(l) == "type") {
        lines.insert(0, format!("type: {}", ty.name()));
    }
    let mut out = String::new();
    if !lines.is_empty() {
        out.push_str("---\n");
        for l in &lines {
            out.push_str(l);
            out.push('\n');
        }
        out.push_str("---\n");
    }
    // The first H1 outside a code fence gets the title.
    let mut fence = false;
    let mut titled = false;
    let mut rest = String::new();
    for l in body.split_inclusive('\n') {
        let t = l.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            fence = !fence;
        }
        if !fence && !titled && l.starts_with("# ") {
            rest.push_str(&format!("# {title}\n"));
            titled = true;
        } else {
            rest.push_str(l);
        }
    }
    if !titled {
        out.push_str(&format!("# {title}\n\n"));
    }
    out.push_str(&rest);
    out
}

/// The section that `new spec` adds with `[change] invariants` (design 0.8.0,
/// D6): the item `none` says that no rule applies, until the author lists one.
const INVARIANTS_SECTION: &str = "## Invariants\n\n- none\n\n";

/// The text of a spec template with the section `Invariants`, for a project
/// with `[change] invariants = true`. The section comes before `Concerns`, or at
/// the end if the template has no such section. A template that has the section
/// stays as it is.
pub fn with_invariants(text: &str) -> String {
    let heading = |l: &str, name: &str| {
        l.trim_start_matches('#').trim().eq_ignore_ascii_case(name) && l.starts_with('#')
    };
    if text.lines().any(|l| heading(l, "Invariants")) {
        return text.to_string();
    }
    let mut out = String::new();
    let mut done = false;
    for l in text.split_inclusive('\n') {
        if !done && heading(l.trim_end(), "Concerns") {
            out.push_str(INVARIANTS_SECTION);
            done = true;
        }
        out.push_str(l);
    }
    if !done {
        if !out.ends_with("\n\n") {
            out.push_str(if out.ends_with('\n') { "\n" } else { "\n\n" });
        }
        out.push_str(INVARIANTS_SECTION.trim_end_matches('\n'));
        out.push('\n');
    }
    out
}

/// The H1 of a new `intent.md`, `spec.md` or `plan.md`. A slug is a short
/// folder name, and no rule makes a title from it, so the author writes the
/// title. SD306 reports the placeholder until then (design 0.7.0, section 15).
pub const CHANGE_TITLE: &str = "{{The title of the change.}}";

/// Makes a title from the last segment of an id: `cache-policy` gives
/// "Cache policy".
pub fn title_of(id: &str) -> String {
    let slug = id.rsplit('/').next().unwrap_or(id);
    let words = slug.replace(['-', '_'], " ");
    let mut c = words.chars();
    match c.next() {
        Some(f) => f.to_uppercase().chain(c).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::model::Corpus;
    use std::sync::Arc;

    fn corpus_of(cfg: &Config, path: &str, text: &str) -> Corpus {
        Corpus::build(
            cfg,
            vec![path.to_string()],
            vec![(path.to_string(), Arc::new(crate::parse::parse(text)))],
        )
    }

    /// The place of a file of a type in the default graph.
    fn place_of(ty: DocType) -> String {
        match ty {
            t if t.is_intent_type() => format!("docs/intent/x/{}.md", t.name()),
            DocType::Rule => "docs/rules/x.md".into(),
            DocType::Requirements => "docs/requirements/x.md".into(),
            DocType::Interface => "docs/interfaces/x.md".into(),
            DocType::Decision => "docs/decisions/x.md".into(),
            DocType::Procedure => "docs/procedures/x.md".into(),
            DocType::Incident => "docs/incidents/x.md".into(),
            DocType::Research => "docs/research/x.md".into(),
            t => format!("docs/{}.md", t.name()),
        }
    }

    #[test]
    fn every_embedded_template_parses_as_its_type_with_its_required_sections() {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        for (name, ty) in embedded_names() {
            for with_type in [true, false] {
                let (_, raw) = embedded(name).unwrap();
                let text = instantiate(&raw, ty, "Example", with_type);
                assert_eq!(text.contains("type:"), with_type, "{name}: {text}");
                let c = corpus_of(&cfg, &place_of(ty), &text);
                let d = &c.docs[0];
                assert_eq!(d.ty, ty, "{name}: {text}");
                let codes: Vec<&str> = d.issues.iter().map(|i| i.code.code()).collect();
                assert!(codes.is_empty(), "{name}: {codes:?}");
                assert!(!d.parsed.placeholders.is_empty(), "{name}");
                assert!(text.len() < 700, "{name}: {} bytes", text.len());
            }
        }
        assert_eq!(title_of("ops/cache-policy"), "Cache policy");
    }

    #[test]
    fn the_spec_and_plan_templates_hold_the_items_that_the_parser_reads() {
        use crate::change::{plan_tasks, section_items};
        use crate::parse::ItemSection;
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        let build = |name: &str, ty| {
            let (_, raw) = embedded(name).unwrap();
            let text = instantiate(&raw, ty, "Example", true);
            corpus_of(&cfg, &place_of(ty), &text)
        };
        let c = build("plan", DocType::Plan);
        // The code spans of the template hold a placeholder. It is no path, so
        // an untouched plan lists no path, and `next` says why.
        let change = &crate::change::collect(&c)[0];
        assert!(change.files(&c).is_empty());
        assert_eq!(
            change.files_not_paths(&c),
            ["`{{path}}` {{Why this change touches the path.}}"]
        );
        assert!(change.tasks(&c).iter().all(|t| t.files.is_empty()));
        let text = {
            let (_, raw) = embedded("plan").unwrap();
            instantiate(&raw, DocType::Plan, "Example", true)
        };
        assert!(text.contains("Files: `{{path}}`. Verify:"), "{text}");
        let tasks = plan_tasks(&c.docs[0]);
        let got: Vec<_> = tasks
            .iter()
            .map(|t| (t.id.as_str(), t.done, t.duplicate, t.verify.is_some()))
            .collect();
        assert_eq!(
            got,
            [("t1", false, false, true), ("t2", false, false, true)]
        );
        assert!(tasks.iter().all(|t| !t.text.is_empty()));
        let c = build("spec", DocType::Spec);
        let q: Vec<_> = section_items(&c.docs[0], ItemSection::OpenQuestions)
            .map(|i| i.task)
            .collect();
        assert_eq!(q, [Some(false)]);
    }

    #[test]
    fn with_invariants_adds_the_section_once_before_concerns_or_at_the_end() {
        let t = "# T\n\n## Delta\n\nx\n\n## Concerns\n\n- [ ] c\n";
        let w = with_invariants(t);
        assert_eq!(
            w,
            "# T\n\n## Delta\n\nx\n\n## Invariants\n\n- none\n\n## Concerns\n\n- [ ] c\n"
        );
        assert_eq!(with_invariants(&w), w);
        // No `Concerns`: at the end, with one blank line before it.
        assert_eq!(
            with_invariants("# T\n\n## Delta\n\nx\n"),
            "# T\n\n## Delta\n\nx\n\n## Invariants\n\n- none\n"
        );
        assert_eq!(with_invariants("# T"), "# T\n\n## Invariants\n\n- none\n");
        // A heading of another level, in another case, counts.
        let t = "# T\n\n### invariants\n\n- a\n";
        assert_eq!(with_invariants(t), t);
    }

    #[test]
    fn an_intent_template_has_the_class_routine_and_no_other_template_has_one() {
        for (name, ty) in embedded_names() {
            let (_, raw) = embedded(name).unwrap();
            let fm = front_matter_lines(&raw).unwrap();
            let has = fm.contains(&"class: routine");
            assert_eq!(has, ty == DocType::Intent, "{name}");
            if has {
                // The order is type, status, class. `new` keeps the line.
                assert_eq!(
                    &fm[..3],
                    ["type: intent", "status: draft", "class: routine"]
                );
                let text = instantiate(&raw, ty, "T", false);
                assert!(
                    text.starts_with("---\nstatus: draft\nclass: routine\n---\n# T\n"),
                    "{name}: {text}"
                );
                let cfg =
                    Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
                let c = corpus_of(&cfg, &place_of(ty), &text);
                assert_eq!(c.docs[0].class, crate::model::Class::Routine, "{name}");
                assert!(c.docs[0].issues.is_empty(), "{name}");
            }
        }
    }

    #[test]
    fn each_type_has_a_default_template_of_its_type() {
        for ty in DocType::ALL {
            let (t, _) = embedded(default_name(ty)).unwrap_or_else(|| panic!("{}", ty.name()));
            assert_eq!(t, ty);
        }
    }

    #[test]
    fn instantiate_sets_the_title_the_status_and_drops_was_type() {
        let text =
            "---\ntype: research\nwas-type: note\nstatus: active\n---\n# Old\n\nwas-type: note\n";
        assert_eq!(
            instantiate(text, DocType::Procedure, "New", true),
            "---\ntype: research\nstatus: draft\n---\n# New\n\nwas-type: note\n"
        );
        // No status for a type that has none, no type where the place gives it.
        assert_eq!(
            instantiate(text, DocType::Research, "New", false),
            "---\nstatus: active\n---\n# New\n\nwas-type: note\n"
        );
        // A `#` line in a code fence is not the title.
        let fenced = "---\ntype: plan\n---\n```\n# Code\n```\n";
        assert_eq!(
            instantiate(fenced, DocType::Plan, "New", true),
            "---\ntype: plan\nstatus: draft\n---\n# New\n\n```\n# Code\n```\n"
        );
    }

    #[test]
    fn the_status_section_comes_before_amendments() {
        let (_, text) = embedded("madr").unwrap();
        let status = text.find("## Status").unwrap();
        let amendments = text.find("## Amendments").unwrap();
        assert!(status < amendments);
    }
}

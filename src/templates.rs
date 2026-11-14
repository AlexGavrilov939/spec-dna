//! The template of each document type (docs/types.md). `spec-dna new` writes a
//! document from it. A template has the front matter, the title and the
//! required sections of its type. `check` reports each placeholder
//! (`{{text}}`) that stays in a document (SD306).

use crate::model::DocType;

/// Returns the body of the default template of a type, after the front
/// matter.
fn body(ty: DocType) -> &'static str {
    match ty {
        DocType::Product => {
            "## Purpose

{{What the product does, and for whom.}}

## Scope

{{What the product covers, and what it leaves out.}}
"
        }
        DocType::System => {
            "## Overview

{{The parts of the system and how they work together.}}

## Constraints

- {{A constraint that every part keeps.}}
"
        }
        DocType::Terms => {
            "## Terms

| Term | Meaning | Forbidden synonyms |
| --- | --- | --- |
| {{term}} | {{meaning}} | {{synonyms}} |
"
        }
        DocType::Roadmap => {
            "## Next

- {{Planned work.}}

## Later

- {{Work with no date.}}
"
        }
        DocType::Questions => {
            "## Open

- {{A question that has no answer yet, and who can answer it.}}
"
        }
        DocType::Requirements => {
            "## Requirements

- **{{key}}**: When {{trigger}}, the {{system}} SHALL {{response}}.

## Notes

{{Notes. Cite the code of the requirements in inline code spans.}}
"
        }
        DocType::Interface => {
            "## Interface

{{The operations, or a link to the OpenAPI, AsyncAPI or schema file.}}

## Requirements

- **{{key}}**: The {{service}} SHALL {{behavior of the interface}}.
"
        }
        DocType::Procedure => {
            "## Purpose

{{When to use this procedure, and what it gives.}}

## Steps

1. {{Command or action.}}
2. {{Check.}}
"
        }
        DocType::Decision => {
            "## Context and Problem Statement

{{The problem in 2 or 3 sentences.}}

## Decision Outcome

Chosen option: {{option}}, because {{reason}}.

## Consequences

- Good: {{effect}}.
- Bad: {{effect}}.
"
        }
        DocType::Intent => {
            "## Intent

{{What this change gives, and for whom.}}

## Why now

{{The reason to do it now.}}

## Out of scope

- {{What this change does not do.}}
"
        }
        DocType::Spec => {
            "## Delta

{{The requirements that this change adds, changes or removes.}}

## Interfaces

{{The interfaces that this change touches.}}

## Decisions

{{The decisions that this change needs.}}

## Concerns

- {{An open concern.}}
"
        }
        DocType::Plan => {
            "## Files

- {{A path that this change touches.}}

## Order

1. {{Step.}}

## Risks

- {{Risk.}}

## Proof

{{How to see that the change works.}}
"
        }
        DocType::Rule => {
            "## Rules

- **{{key}}**: The {{system}} SHALL {{rule that every change in the scope keeps}}.

## Rationale

{{Why the rule exists. Link the decision.}}
"
        }
        DocType::Incident => {
            "## Summary

{{What happened, when, and the impact, in 2 or 3 sentences.}}

## Timeline

- {{Time}}: {{event}}.

## Root cause

{{The cause.}}

## Links

- Requirements: {{links to the requirements, such as [[doc-id#key]]}}
- Decisions: {{links to the decisions}}
- Procedures: {{links to the procedures}}

## Follow-ups

- {{Action.}}
"
        }
        DocType::Research => {
            "{{Research, data or other cold material. spec-dna context does not serve this document unless a target names it.}}\n"
        }
    }
}

/// Returns the first status of a new document of a type.
pub fn first_status(ty: DocType) -> Option<&'static str> {
    match ty {
        DocType::Decision => Some("proposed"),
        t if t.has_status() => Some("draft"),
        _ => None,
    }
}

/// Returns a new document of a type with a title. `with_type` is false where
/// the place of the document gives its type: the file then has no `type:`
/// line (design, "Place gives the type").
pub fn render(ty: DocType, title: &str, with_type: bool) -> String {
    let mut fm = String::new();
    if with_type {
        fm.push_str(&format!("type: {}\n", ty.name()));
    }
    if let Some(s) = first_status(ty) {
        fm.push_str(&format!("status: {s}\n"));
    }
    if ty == DocType::Rule {
        // A rule applies to the paths of its scope. "**" is the whole
        // project: narrow it.
        fm.push_str("applies-to: [\"**\"]\n");
    }
    let fm = if fm.is_empty() {
        String::new()
    } else {
        format!("---\n{fm}---\n")
    };
    format!("{fm}# {title}\n\n{}", body(ty))
}

/// Removes the front matter field `was-type` from the text of a template.
/// `upgrade` writes the field. A copy of a document that has it would give
/// a new document the exemption of SD202.
pub fn strip_was_type(text: &str) -> String {
    let mut out = String::new();
    let mut in_fm = false;
    for (i, line) in text.split_inclusive('\n').enumerate() {
        let t = line.trim_end();
        if i == 0 && t == "---" {
            in_fm = true;
        } else if in_fm && t == "---" {
            in_fm = false;
        } else if in_fm && t.trim_start().starts_with("was-type:") {
            continue;
        }
        out.push_str(line);
    }
    out
}

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

    /// The place of a file of a type in a mixed root.
    fn place_of(ty: DocType) -> String {
        if ty.is_intent_type() {
            format!("docs/intent/x/{}.md", ty.name())
        } else {
            format!("docs/{}.md", ty.name())
        }
    }

    #[test]
    fn every_template_parses_as_its_type_with_its_required_sections() {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        for ty in DocType::ALL {
            let text = render(ty, "Example", true);
            let c = corpus_of(&cfg, &place_of(ty), &text);
            let d = &c.docs[0];
            assert_eq!(d.ty, ty, "{text}");
            let codes: Vec<&str> = d.issues.iter().map(|i| i.code.code()).collect();
            assert!(!codes.contains(&"SD303"), "{}: {codes:?}", ty.name());
            assert!(!codes.contains(&"SD305"), "{}: {codes:?}", ty.name());
            assert!(!codes.contains(&"SD304"), "{}: {codes:?}", ty.name());
            assert!(!d.parsed.placeholders.is_empty(), "{}", ty.name());
            assert!(text.len() < 700, "{}: {} bytes", ty.name(), text.len());
        }
        assert_eq!(title_of("ops/cache-policy"), "Cache policy");
    }

    #[test]
    fn a_template_without_type_gets_its_type_from_the_place() {
        // The place of a type in the implied graph: a top file, or the folder
        // of the layer.
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        for ty in DocType::ALL {
            let text = render(ty, "Example", false);
            assert!(!text.contains("type:"), "{text}");
            let path = match ty {
                t if t.is_intent_type() => place_of(t),
                DocType::Rule => "docs/rules/x.md".into(),
                DocType::Requirements => "docs/requirements/x.md".into(),
                DocType::Interface => "docs/interfaces/x.md".into(),
                DocType::Decision => "docs/decisions/x.md".into(),
                DocType::Procedure => "docs/procedures/x.md".into(),
                DocType::Incident => "docs/incidents/x.md".into(),
                DocType::Research => "docs/research/x.md".into(),
                t => place_of(t),
            };
            let c = corpus_of(&cfg, &path, &text);
            let d = &c.docs[0];
            assert_eq!(d.ty, ty, "{path}: {text}");
            let codes: Vec<&str> = d.issues.iter().map(|i| i.code.code()).collect();
            assert!(codes.is_empty(), "{path}: {codes:?}");
        }
    }

    #[test]
    fn was_type_leaves_the_copy_of_a_template() {
        let text = "---\ntype: research\nwas-type: note\n---\n# T\n\nwas-type: note\n";
        assert_eq!(
            strip_was_type(text),
            "---\ntype: research\n---\n# T\n\nwas-type: note\n"
        );
        let plain = render(DocType::Rule, "Water", true);
        assert_eq!(strip_was_type(&plain), plain);
    }
}

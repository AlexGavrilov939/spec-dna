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

/// The namespace of a new document of a type, when its id has none: each type
/// gets its own directory in a mixed root. The foundation and pending types
/// are files of the root, and a requirements document has no namespace.
pub fn namespace(ty: DocType) -> Option<&'static str> {
    match ty {
        DocType::Product
        | DocType::System
        | DocType::Terms
        | DocType::Roadmap
        | DocType::Questions
        | DocType::Requirements => None,
        DocType::Rule => Some("rules"),
        DocType::Interface => Some("interfaces"),
        DocType::Decision => Some("decisions"),
        DocType::Procedure => Some("procedures"),
        DocType::Incident => Some("incidents"),
        DocType::Research => Some("research"),
        DocType::Intent | DocType::Spec | DocType::Plan => Some("intent"),
    }
}

/// Returns true for a type that has a file of its own in the folder of a
/// change: `intent/<slug>/<type>.md`.
pub fn in_intent_folder(ty: DocType) -> bool {
    matches!(ty, DocType::Intent | DocType::Spec | DocType::Plan)
}

/// Returns the first status of a new document of a type.
pub fn first_status(ty: DocType) -> Option<&'static str> {
    match ty {
        DocType::Decision => Some("proposed"),
        t if t.has_status() => Some("draft"),
        _ => None,
    }
}

/// Returns a new document of a type with a title.
pub fn render(ty: DocType, title: &str) -> String {
    let mut fm = format!("---\ntype: {}\n", ty.name());
    if let Some(s) = first_status(ty) {
        fm.push_str(&format!("status: {s}\n"));
    }
    if ty == DocType::Rule {
        // A rule applies to the paths of its scope. "**" is the whole
        // project: narrow it.
        fm.push_str("applies-to: [\"**\"]\n");
    }
    format!("{fm}---\n# {title}\n\n{}", body(ty))
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

    #[test]
    fn every_template_parses_as_its_type_with_its_required_sections() {
        let cfg = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        for ty in DocType::ALL {
            let text = render(ty, "Example");
            let path = format!("docs/{}.md", ty.name());
            let c = Corpus::build(
                &cfg,
                vec![path.clone()],
                vec![(path, Arc::new(crate::parse::parse(&text)))],
            );
            let d = &c.docs[0];
            assert_eq!(d.ty, ty, "{text}");
            let codes: Vec<&str> = d.issues.iter().map(|i| i.code.code()).collect();
            assert!(!codes.contains(&"SD305"), "{}: {codes:?}", ty.name());
            assert!(!codes.contains(&"SD304"), "{}: {codes:?}", ty.name());
            assert!(!d.parsed.placeholders.is_empty(), "{}", ty.name());
            assert!(text.len() < 700, "{}: {} bytes", ty.name(), text.len());
        }
        assert_eq!(title_of("ops/cache-policy"), "Cache policy");
    }
}

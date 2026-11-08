//! The template of each document type (docs/types.md). `spec-dna new` writes a
//! document from it. A template has the front matter, the title and the
//! required sections of its type. The placeholders in angle brackets are not
//! valid ids or keys, so `check` reports a placeholder that stays.

use crate::model::DocType;

/// Returns the body of the template of a type, after the front matter.
fn body(ty: DocType) -> &'static str {
    match ty {
        DocType::Spec => {
            "## Requirements

- **<key>**: When <trigger>, the <system> SHALL <response>.

## Notes

Cite the code in inline code spans, such as `src/<path>`.
"
        }
        DocType::Contract => {
            "## Interface

<The operations, or a link to the OpenAPI, AsyncAPI or schema file.>

## Requirements

- **<key>**: The <service> SHALL <behavior of the interface>.
"
        }
        DocType::Guide => {
            "## Rules

- <One convention for each item.>

## Procedure

1. <Step.>
"
        }
        DocType::Decision => {
            "## Context and Problem Statement

<The problem in 2 or 3 sentences.>

## Decision Outcome

Chosen option: <option>, because <reason>.

## Consequences

- Good: <effect>.
- Bad: <effect>.
"
        }
        DocType::Change => {
            "## Proposal

<What changes, and why.>

## Plan

1. <Step.>
"
        }
        DocType::Note => "<Observations, research or reference material.>\n",
        DocType::Invariant => {
            "## Rules

- **<key>**: The <system> SHALL <rule that every change in the scope keeps>.

## Rationale

<Why the rule exists. Link the decision.>
"
        }
        DocType::Runbook => {
            "## When to use

<The alert, symptom or request that starts this runbook.>

## Steps

1. <Command or action.>
2. <Check.>

## Verify

<How to see that the system is healthy again.>

## Rollback

<How to undo the steps.>
"
        }
        DocType::Incident => {
            "## Summary

<What happened, when, and the impact, in 2 or 3 sentences.>

## Timeline

- <Time>: <event>.

## Root cause

<The cause.>

## Links

- Requirements: [[<doc-id>#<key>]]
- Decisions: [[<decision-id>]]
- Runbooks: [[<runbook-id>]]

## Follow-ups

- <Action.>
"
        }
        DocType::Reference => {
            "<Research, data or other cold material. `spec-dna context` does not serve this document unless a target names it.>\n"
        }
    }
}

/// The namespace of a new document of a type, when its id has none: each type
/// gets its own directory in a mixed root. A spec has no namespace.
pub fn namespace(ty: DocType) -> Option<&'static str> {
    match ty {
        DocType::Spec => None,
        DocType::Contract => Some("contracts"),
        DocType::Guide => Some("guides"),
        DocType::Decision => Some("decisions"),
        DocType::Change => Some("changes"),
        DocType::Note => Some("notes"),
        DocType::Invariant => Some("invariants"),
        DocType::Runbook => Some("runbooks"),
        DocType::Incident => Some("incidents"),
        DocType::Reference => Some("reference"),
    }
}

/// Returns the first status of a new document of a type.
pub fn first_status(ty: DocType) -> Option<&'static str> {
    match ty {
        DocType::Decision | DocType::Change => Some("proposed"),
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
    if ty == DocType::Invariant {
        // An invariant applies to the paths of its scope. "**" is the whole
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
            assert!(text.len() < 700, "{}: {} bytes", ty.name(), text.len());
        }
        assert_eq!(title_of("ops/cache-policy"), "Cache policy");
    }
}

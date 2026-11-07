//! Stable diagnostic codes (docs/diagnostics.md). A code never changes its
//! meaning; a retired code is not reused.

use crate::json::Obj;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
        }
    }
}

macro_rules! codes {
    ($($variant:ident = $code:literal, $name:literal, $sev:ident, $fix:literal;)*) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum Code { $($variant),* }

        impl Code {
            #[cfg(test)]
            pub const ALL: &[Code] = &[$(Code::$variant),*];
            pub fn code(self) -> &'static str { match self { $(Code::$variant => $code),* } }
            pub fn name(self) -> &'static str { match self { $(Code::$variant => $name),* } }
            pub fn severity(self) -> Severity { match self { $(Code::$variant => Severity::$sev),* } }
            pub fn fix(self) -> &'static str { match self { $(Code::$variant => $fix),* } }
        }
    };
}

codes! {
    UnresolvedReference = "SD101", "unresolved-reference", Error, "Point the reference at an existing id, alias or `id#key`, or add the old id to the target's `aliases`.";
    UnresolvedLink = "SD102", "unresolved-link", Error, "Fix the link target; it must be a file or directory tracked by git (exact case).";
    EvidenceMissing = "SD103", "evidence-missing", Error, "Fix the `verified-by` path; it must exist in the git tree.";
    AmbiguousReference = "SD104", "ambiguous-reference", Error, "Use the full document id instead of an alias that names several documents.";
    DuplicateId = "SD201", "duplicate-id", Error, "Give each document a unique id; remove the alias from all but one document.";
    IdRemoved = "SD202", "id-removed", Error, "Keep the document (set a final status such as `retired`), or move it with `spec-dna mv` so the old id becomes an alias.";
    KeyFormat = "SD203", "key-format", Error, "Use a slug key (lowercase letters and digits joined by hyphens), or declare the legacy pattern in `legacy-keys`.";
    KeyDuplicate = "SD204", "key-duplicate", Error, "Rename one of the requirements; keys are unique within a document.";
    FrontMatterSyntax = "SD301", "front-matter-syntax", Error, "Use the front matter subset: `key: value` lines, values are strings or lists of strings.";
    FrontMatterField = "SD302", "front-matter-field", Error, "Give the field a string or a list of strings.";
    TypeInvalid = "SD303", "type-invalid", Error, "Use one of spec, contract, guide, decision, change, note, matching the root's type.";
    StatusInvalid = "SD304", "status-invalid", Error, "Set a `status` that the document type allows.";
    SectionMissing = "SD305", "section-missing", Error, "Add the required section heading.";
    StatusTransition = "SD401", "status-transition", Error, "Statuses only move forward; write a new document instead of moving one back.";
    SupersededWithoutSuccessor = "SD402", "superseded-without-successor", Error, "Name this id in the `supersedes` field of an active (or accepted) successor.";
    SupersedeCycle = "SD403", "supersede-cycle", Error, "Remove one `supersedes` entry so the chain ends.";
    DependsOnInactive = "SD404", "depends-on-inactive", Error, "Depend on the successor instead of a superseded or retired document.";
    DecisionFrozen = "SD405", "decision-frozen", Error, "Restore the accepted text and append the change under `## Amendments`, or write a superseding decision.";
    AdapterModified = "SD501", "adapter-modified", Error, "Run `spec-dna init` to regenerate the adapter; put own text outside the generated block.";
    AdapterOutdated = "SD502", "adapter-outdated", Error, "Run `spec-dna init` to regenerate the adapter for this version.";
    TokenUnresolved = "SD601", "token-unresolved", Warning, "Check the id; a bare token that names no document may be a typo, or plain text such as a ticket number.";
    AnchorMissing = "SD602", "anchor-missing", Warning, "Point the link at an existing heading anchor or requirement key.";
    CrossRepoUnpinned = "SD603", "cross-repo-unpinned", Warning, "Cross-repository references resolve only against a pinned index of that repository, which is not supported yet.";
    DependsOnDeprecated = "SD604", "depends-on-deprecated", Warning, "Plan a move to the successor of the deprecated document.";
    AmendmentOutcome = "SD605", "amendment-outcome", Warning, "An amendment that changes the decision outcome needs a new superseding decision.";
    BudgetSoft = "SD701", "budget-soft", Warning, "Split the document or move detail into linked documents.";
    BudgetHard = "SD702", "budget-hard", Warning, "Split the document; it is over the hard budget.";
    BudgetGrew = "SD703", "budget-grew", Warning, "The document is over its budget and grew; split it instead of adding to it.";
    RequirementKeyword = "SD801", "requirement-keyword", Warning, "State the requirement with an uppercase BCP 14 keyword (MUST, SHALL, SHOULD, MAY).";
    EarsPattern = "SD802", "ears-pattern", Warning, "Use an EARS pattern; an `If` condition is followed by `then`.";
    SentenceLength = "SD803", "sentence-length", Warning, "Split the requirement; one sentence states one requirement.";
    VagueTerm = "SD804", "vague-term", Warning, "Replace the vague term with a testable value or condition.";
    PassiveVoice = "SD805", "passive-voice", Warning, "Name the actor as the subject (`the gateway SHALL reject`, not `SHALL be rejected`).";
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diag {
    pub path: String,
    pub line: u32,
    pub code: Code,
    pub doc: String,
    pub target: String,
    pub message: String,
}

impl Diag {
    /// Base-relative gating compares diagnostics by this fingerprint
    /// (ADR 0001, "Growth controls").
    pub fn fingerprint(&self) -> (Code, &str, &str) {
        (self.code, &self.doc, &self.target)
    }

    pub fn text(&self) -> String {
        format!(
            "{}:{}: {} {} {}: {}",
            self.path,
            self.line,
            self.code.severity().name(),
            self.code.code(),
            self.code.name(),
            self.message
        )
    }

    pub fn json(&self, new: bool) -> String {
        Obj::new()
            .str("code", self.code.code())
            .str("name", self.code.name())
            .str("severity", self.code.severity().name())
            .bool("new", new)
            .str("doc", &self.doc)
            .str("path", &self.path)
            .num("line", self.line)
            .str("target", &self.target)
            .str("message", &self.message)
            .str("fix", self.code.fix())
            .end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for c in Code::ALL {
            assert!(seen.insert(c.code()), "duplicate {}", c.code());
            assert!(seen.insert(c.name()), "duplicate {}", c.name());
            assert!(c.code().starts_with("SD") && c.code().len() == 5);
            let is_warning = c.code().as_bytes()[2] >= b'6';
            assert_eq!(
                is_warning,
                c.severity() == Severity::Warning,
                "{}",
                c.code()
            );
        }
    }
}

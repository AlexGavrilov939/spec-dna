//! Stable diagnostic codes (docs/diagnostics.md). A code never changes its
//! meaning. spec-dna never uses a retired code again.

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
            pub const ALL: &[Code] = &[$(Code::$variant),*];
            /// Finds a code by its `SDnnn` form.
            pub fn from_code(s: &str) -> Option<Code> { Code::ALL.iter().copied().find(|c| c.code() == s) }
            pub fn code(self) -> &'static str { match self { $(Code::$variant => $code),* } }
            pub fn name(self) -> &'static str { match self { $(Code::$variant => $name),* } }
            pub fn severity(self) -> Severity { match self { $(Code::$variant => Severity::$sev),* } }
            pub fn fix(self) -> &'static str { match self { $(Code::$variant => $fix),* } }
            /// The prose lint of requirement sentences (SD801-SD805). The
            /// human output of `check` gives one count for each code.
            pub fn is_prose_lint(self) -> bool {
                matches!(self.code(), "SD801" | "SD802" | "SD803" | "SD804" | "SD805")
            }
        }
    };
}

codes! {
    UnresolvedReference = "SD101", "unresolved-reference", Error, "Refer to an existing id, alias or `id#key`. As an alternative, add the old id to the `aliases` of the target.";
    UnresolvedLink = "SD102", "unresolved-link", Error, "Fix the link target. The target must be a file or a directory that git tracks, with the exact case.";
    EvidenceMissing = "SD103", "evidence-missing", Error, "Fix the `verified-by` path. The path must exist in the git tree.";
    AmbiguousReference = "SD104", "ambiguous-reference", Error, "Use the full document id instead of an alias that names several documents.";
    CoversUnresolved = "SD105", "covers-unresolved", Error, "Name an existing requirement as `<doc-id>#<key>` after `covers:`, or remove the tag.";
    ConventionUnresolved = "SD106", "convention-unresolved", Error, "Restore the document, the file or the heading that `conventions` in spec-dna.toml points to. As an alternative, change the pointer in a change of its own.";
    DuplicateId = "SD201", "duplicate-id", Error, "Give each document a unique id. Remove the alias from all documents except one.";
    IdRemoved = "SD202", "id-removed", Error, "Keep the document and set a final status such as `retired`. As an alternative, use `spec-dna mv` to move it, so that the old id becomes an alias.";
    KeyFormat = "SD203", "key-format", Error, "Use a slug key (groups of lowercase letters and digits, with a hyphen between groups). As an alternative, add the legacy pattern to `legacy-keys`.";
    KeyDuplicate = "SD204", "key-duplicate", Error, "Rename one of the requirements. The keys in a document must be unique.";
    FrontMatterSyntax = "SD301", "front-matter-syntax", Error, "Use the front matter subset: `key: value` lines, where each value is a string or a list of strings.";
    FrontMatterField = "SD302", "front-matter-field", Error, "Give the field a string or a list of strings.";
    TypeInvalid = "SD303", "type-invalid", Error, "Use a type of the root and of the `types` list of spec-dna.toml (without the list: spec, contract, guide, decision, change, note, invariant, runbook, incident, reference), or add the type to the list.";
    StatusInvalid = "SD304", "status-invalid", Error, "Set a `status` that the document type allows.";
    SectionMissing = "SD305", "section-missing", Error, "Add the required section heading.";
    PlaceholderLeft = "SD306", "placeholder-left", Error, "Replace the template placeholder (`{{...}}`) with the text of the document.";
    StatusTransition = "SD401", "status-transition", Error, "A status can only move forward. To return to an earlier status, write a new document.";
    SupersededWithoutSuccessor = "SD402", "superseded-without-successor", Error, "Name this id in the `supersedes` field of an active (or accepted) successor.";
    SupersedeCycle = "SD403", "supersede-cycle", Error, "Remove one `supersedes` entry, so that the chain ends.";
    DependsOnInactive = "SD404", "depends-on-inactive", Error, "Depend on the successor instead of a superseded or retired document.";
    DecisionFrozen = "SD405", "decision-frozen", Error, "Restore the accepted text and write the change in a new amendment section (`amendment-sections`, default `## Amendments`). Only a live section (`live-sections`, such as a progress section) can change in place. As an alternative, write a decision that supersedes this one.";
    AdapterModified = "SD501", "adapter-modified", Error, "Run `spec-dna init` to regenerate the adapter. Put your own text outside of the generated block.";
    AdapterOutdated = "SD502", "adapter-outdated", Error, "Run `spec-dna init` to regenerate the adapter for this version.";
    ConfigNotAlone = "SD503", "config-not-alone", Error, "Do not edit spec-dna.toml in a change of documents or code. Revert the edit, or make the configuration change a change of its own.";
    HarnessMissing = "SD504", "harness-missing", Error, "Run `spec-dna init` to write the harness files. To keep a file of your own, rename its agent or set the role to \"none\" in [harness.roles].";
    HarnessSettingsInvalid = "SD505", "harness-settings-invalid", Error, "Make .claude/settings.json valid JSON. spec-dna cannot check the harness settings in a file that it cannot read.";
    TokenUnresolved = "SD601", "token-unresolved", Warning, "Check the id. A bare token that names no document can be a typo, or plain text such as a ticket number.";
    AnchorMissing = "SD602", "anchor-missing", Warning, "Link to an existing heading anchor or requirement key.";
    CrossRepoUnpinned = "SD603", "cross-repo-unpinned", Warning, "A cross-repository reference resolves only against a pinned index of that repository. spec-dna does not support this yet.";
    DependsOnDeprecated = "SD604", "depends-on-deprecated", Warning, "Plan a move to the successor of the deprecated document.";
    AmendmentOutcome = "SD605", "amendment-outcome", Warning, "If the amendment changes the decision outcome, write a new decision that supersedes this one.";
    CitedPathRemoved = "SD606", "cited-path-removed", Warning, "Update the document: cite the new path, or change the text about the removed file.";
    CitedSymbolRemoved = "SD607", "cited-symbol-removed", Warning, "Update the document: cite the new identifier, or change the text about the removed code.";
    RequirementUnverified = "SD608", "requirement-unverified", Warning, "Link a test: add `covers: <doc-id>#<key>` in the test file, or `<!-- verified-by: <path> -->` at the end of the requirement. `verified-by` in the front matter links every requirement of the document.";
    IncidentUnlinked = "SD609", "incident-unlinked", Warning, "Link the requirements, decisions and runbooks that the incident involves, with `[[<id>]]` or a Markdown link.";
    IndexStale = "SD610", "index-stale", Warning, "Run `spec-dna index <path>` to generate the index again. Do not edit a generated index by hand.";
    BudgetSoft = "SD701", "budget-soft", Warning, "As a warning, it needs no change in this task. Do not split the document for a warning.";
    BudgetHard = "SD702", "budget-hard", Error, "The document is new and too large, or it went over the hard budget, or it grew by more than `[budgets] growth` characters. Put the new text into a new document and link it, or move sections out with `spec-dna mv`.";
    BudgetGrew = "SD703", "budget-grew", Warning, "The document is over a budget and it grew a little. Add only the text that the task needs. Do not split the document for a warning.";
    BudgetHardKept = "SD704", "budget-hard-kept", Warning, "The document was over its hard budget in the base and did not grow, or a split moved it out of such a document, or its root does not enforce budgets. It needs no change in this task.";
    RequirementKeyword = "SD801", "requirement-keyword", Warning, "Write the requirement with an uppercase BCP 14 keyword (MUST, SHALL, SHOULD, MAY).";
    EarsPattern = "SD802", "ears-pattern", Warning, "Use an EARS pattern. Put `then` after an `If` condition.";
    SentenceLength = "SD803", "sentence-length", Warning, "Split the requirement. Write one requirement in one sentence.";
    VagueTerm = "SD804", "vague-term", Warning, "Replace the vague term with a testable value or condition.";
    PassiveVoice = "SD805", "passive-voice", Warning, "Name the actor as the subject (`the controller SHALL close`, not `SHALL be closed`).";
    RequirementsUnrecognized = "SD806", "requirements-unrecognized", Warning, "Write each requirement as a list item that starts with its bold key: `- **R-12**: sentence` or `- **R-12** sentence`. A key in a table, a heading or a paragraph is not a requirement.";
    RequirementColonMissing = "SD807", "requirement-colon-missing", Warning, "Write the requirement as `- **key**: sentence`, with the colon right after the bold key. Put a note, such as a rule number, after the colon.";
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Diag {
    pub path: String,
    pub line: u32,
    pub code: Code,
    pub doc: String,
    pub target: String,
    pub message: String,
    /// The severity of the code, or the `lint-severity` of the root for the
    /// requirement lint.
    pub severity: Severity,
}

impl Diag {
    /// The gate compares diagnostics with the base by this fingerprint (ADR
    /// 0001, "Growth controls").
    pub fn fingerprint(&self) -> (Code, &str, &str) {
        (self.code, &self.doc, &self.target)
    }

    pub fn text(&self) -> String {
        format!(
            "{}:{}: {} {} {}: {}",
            self.path,
            self.line,
            self.severity.name(),
            self.code.code(),
            self.code.name(),
            self.message
        )
    }

    pub fn json(&self, new: bool) -> String {
        Obj::new()
            .str("code", self.code.code())
            .str("name", self.code.name())
            .str("severity", self.severity.name())
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
            // Codes from 600 up are warnings. SD702 became an error by a decision at
            // checkpoint 2.
            let is_warning = c.code().as_bytes()[2] >= b'6' && c.code() != "SD702";
            assert_eq!(
                is_warning,
                c.severity() == Severity::Warning,
                "{}",
                c.code()
            );
        }
    }
}

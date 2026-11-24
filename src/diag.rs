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
            /// The codes of the test reports (SD613 to SD617). They run only
            /// with `[evidence] reports`.
            pub const REPORT_CODES: &[Code] = &[
                Code::ReportMissing,
                Code::ReportStale,
                Code::ReportUnreadable,
                Code::RequirementUnproven,
                Code::ChangeUnproven,
            ];
            /// The codes of the proof levels and the proof debt (SD618 to
            /// SD620). SD618 and SD619 run on a marker `proof-owed`, and SD620
            /// on `[evidence] reliable-for`.
            pub const PROOF_CODES: &[Code] = &[
                Code::ProofOwed,
                Code::ProofOwedInvalid,
                Code::ProofNotReliable,
            ];
            /// Finds a code by its `SDnnn` form.
            pub fn from_code(s: &str) -> Option<Code> { Code::ALL.iter().copied().find(|c| c.code() == s) }
            pub fn code(self) -> &'static str { match self { $(Code::$variant => $code),* } }
            pub fn name(self) -> &'static str { match self { $(Code::$variant => $name),* } }
            pub fn severity(self) -> Severity { match self { $(Code::$variant => Severity::$sev),* } }
            pub fn fix(self) -> &'static str { match self { $(Code::$variant => $fix),* } }
            /// A state code reports the state of the tree, not a change of
            /// the diff. It is never "new" against the base, and `check --all`
            /// and `status` show it (design 0.6.0, K4, D35).
            pub fn is_state(self) -> bool {
                matches!(self.code(), "SD911" | "SD616" | "SD618")
            }
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
    TypeInvalid = "SD303", "type-invalid", Error, "Use a type of the root and of the `types` list of spec-dna.toml (without the list: product, system, terms, roadmap, questions, rule, requirements, interface, decision, procedure, incident, research, intent, spec, plan), or add the type to the list. If no layer has the type, run `spec-dna layer add <name> --types <type>`. The message names the new type of a type of 0.5.";
    StatusInvalid = "SD304", "status-invalid", Error, "Set a `status` that the document type allows.";
    SectionMissing = "SD305", "section-missing", Error, "Add the required section heading.";
    PlaceholderLeft = "SD306", "placeholder-left", Error, "Replace the template placeholder (`{{...}}`) with the text of the document.";
    StatusTransition = "SD401", "status-transition", Error, "A status can only move forward. To return to an earlier status, write a new document.";
    SupersededWithoutSuccessor = "SD402", "superseded-without-successor", Error, "Name this id in the `supersedes` field of an active (or accepted) successor.";
    SupersedeCycle = "SD403", "supersede-cycle", Error, "Remove one `supersedes` entry, so that the chain ends.";
    DependsOnInactive = "SD404", "depends-on-inactive", Error, "Depend on the successor instead of a superseded or retired document.";
    DecisionFrozen = "SD405", "decision-locked", Error, "Restore the accepted text and write the change in a new amendment section (`amendment-sections`, default `## Amendments`). Only a live section (`live-sections`, default `Status`) can change in place. As an alternative, write a decision that supersedes this one.";
    AdapterModified = "SD501", "adapter-modified", Error, "Run `spec-dna init` to regenerate the adapter. For a harness file or the model-team line, delete it first. Put your own text outside of the generated text. If the configuration does not generate the file, delete the file, or delete its marker line to keep it as a file of the project.";
    AdapterOutdated = "SD502", "adapter-outdated", Error, "Run `spec-dna init` to regenerate the adapter for this version. If the configuration does not generate the file, run `spec-dna init --reconfigure` to remove it.";
    ConfigNotAlone = "SD503", "config-not-alone", Error, "Do not edit spec-dna.toml in a change of documents or code. Revert the edit, or make the configuration change a change of its own.";
    HarnessMissing = "SD504", "harness-missing", Error, "Run `spec-dna init` to write the harness files. To keep a file of your own at an agent path, rename its agent or set the role to \"none\" in [harness.roles]. For a skill, remove it from `skills` in [harness]. For `REVIEW.md`, set `review-md = false` in [harness].";
    HarnessSettingsInvalid = "SD505", "harness-settings-invalid", Error, "Make .claude/settings.json valid JSON, with the value types that the message names. spec-dna cannot check the harness settings in a file that it cannot read.";
    EvalsInvalid = "SD506", "evals-invalid", Error, "Fix the eval file as the message says. The message names the key of the broken rule. If `check.sh` is absent, add it to [evals] dir. `spec-dna eval run` reads the same files, and it stops with exit code 3 on the same error.";
    SkillUnproven = "SD507", "skill-unproven", Warning, "Measure the skill with `spec-dna eval pack` and `spec-dna eval run --ab <skill>`, and commit the result file of `<[evals] dir>/results`. `spec-dna eval verdict` prints the verdict of each skill. As an alternative, remove the skill from `skills` in [harness], and run `spec-dna init --reconfigure`.";
    TokenUnresolved = "SD601", "token-unresolved", Warning, "Check the id. A bare token that names no document can be a typo, or plain text such as a ticket number.";
    AnchorMissing = "SD602", "anchor-missing", Warning, "Link to an existing heading anchor or requirement key.";
    CrossRepoUnpinned = "SD603", "cross-repo-unpinned", Warning, "A cross-repository reference resolves only against a pinned index of that repository. spec-dna does not support this yet.";
    DependsOnDeprecated = "SD604", "depends-on-deprecated", Warning, "Plan a move to the successor of the deprecated document.";
    AmendmentOutcome = "SD605", "amendment-outcome", Warning, "If the amendment changes the decision outcome, write a new decision that supersedes this one.";
    CitedPathRemoved = "SD606", "cited-path-removed", Warning, "Update the document: cite the new path, or change the text about the removed file.";
    CitedSymbolRemoved = "SD607", "cited-symbol-removed", Warning, "Update the document: cite the new identifier, or change the text about the removed code.";
    RequirementUnverified = "SD608", "requirement-unverified", Warning, "Link a test: add `covers: <doc-id>#<key>` in the test file, or `<!-- verified-by: <path> -->` at the end of the requirement. `verified-by` in the front matter links every requirement of the document. A `covers: partial` tag alone is not evidence.";
    IncidentUnlinked = "SD609", "incident-unlinked", Warning, "Link the requirements, rules, decisions and procedures that the incident involves, with `[[<id>]]` or a Markdown link.";
    IndexStale = "SD610", "index-stale", Warning, "Run `spec-dna index <path>` to generate the index again. Do not edit a generated index by hand.";
    DecisionLinksChange = "SD611", "decision-links-intent", Warning, "Link the decision from the intent instead. A finished intent can be deleted (SD202 allows it), but the locked text of an accepted decision cannot drop a link (SD405). Remove the link before you accept the decision. In an accepted decision, name the intent as plain text, not as a link, in an amendment or a live section. SD901 and `finish` would stop on a link.";
    LinkIntoChange = "SD612", "link-into-change", Warning, "Name the change as plain text. A reference into the folder of a change (a link, `[[id]]`, `depends-on`, `supersedes` or `from`) breaks when `spec-dna finish` deletes the folder, and `finish` stops on it. As an alternative, link the document that the change moves its text into.";
    ReportMissing = "SD613", "report-missing", Warning, "Run the tests so that they write the report, then run `spec-dna evidence stamp` in the same job. `[evidence] reports` lists the globs of the report files. A report needs its stamp (`<report>.stamp`).";
    ReportStale = "SD614", "report-stale", Warning, "Run the tests again, then run `spec-dna evidence stamp` on the same tree. The stamp names another tree, or the bytes of the report changed after the stamp. A change of code makes the report stale. A change of Markdown does not.";
    ReportUnreadable = "SD615", "report-unreadable", Warning, "Write the report in a format that spec-dna reads: a JUnit XML subset or the `spec-dna-report 1` line format, as UTF-8, with no DTD, whole, and not above 64 MiB. Do not commit a report or a stamp: add them to `.gitignore`.";
    RequirementUnproven = "SD616", "requirement-unproven", Warning, "A `covers:` tag names the requirement, and no test case that binds to the tag passed in the report. Make the test run and pass. Put the tag above the test, or add `case:<name>` to the tag. As an alternative, add `<!-- verified-by: <path> -->` for a check by a person.";
    ChangeUnproven = "SD617", "change-unproven", Warning, "A requirement that the change adds or changes has a `covers:` tag, and no test case that binds to the tag passed in the report. Make the test run and pass. Put the tag above the test, or add `case:<name>` to the tag.";
    ProofOwed = "SD618", "proof-owed", Warning, "The requirement has a proof debt and no evidence. Pay the debt: add a test with a `covers:` tag, or a `<!-- verified-by: <path> -->` marker, and remove the marker `proof-owed`. A debt is no proof, so `finish` still needs evidence for a requirement that the change adds or changes.";
    ProofOwedInvalid = "SD619", "proof-owed-invalid", Warning, "Write `<!-- proof-owed: <reason> -->` with one reason: manual, environment, external or pending. Remove the marker from a requirement that has evidence.";
    ProofNotReliable = "SD620", "proof-not-reliable", Warning, "A requirement of the change needs the level `reliable`: each bound case must pass `[evidence] reliable-runs` times on one code tree. Run the tests that many times, write one report for each run, then run `spec-dna evidence stamp`.";
    BudgetSoft = "SD701", "budget-soft", Warning, "As a warning, it needs no change in this task. Do not split the document for a warning.";
    BudgetHard = "SD702", "budget-hard", Error, "The document is new and too large, or it went over the hard budget, or it grew by more than `[budgets] growth` characters. Put the new text into a new document and link it, or move sections out with `spec-dna mv`.";
    BudgetGrew = "SD703", "budget-grew", Warning, "The document is over a budget and it grew a little. Add only the text that the task needs. Do not split the document for a warning.";
    BudgetHardKept = "SD704", "budget-hard-kept", Warning, "The document was over its hard budget in the base and did not grow, or a split moved it out of such a document, or its root does not enforce budgets. It needs no change in this task.";
    AreaBudgetSoft = "SD705", "area-budget-soft", Warning, "As a warning, it needs no change in this task. To keep the area small, split it into sub-areas with `spec-dna mv` and give each one a summary (`spec-dna new summary <area>`), or condense its documents.";
    AreaBudgetHard = "SD706", "area-budget-hard", Warning, "The area is new and too large, or it went over the hard budget `[budgets.area] hard`, or it grew by more than `[budgets] growth` characters. Split the area into sub-areas with `spec-dna mv` and give each one a summary (`spec-dna new summary <area>`): the area then counts the summaries of its sub-areas, not their documents. As an alternative, condense its documents.";
    AreaBudgetHardKept = "SD707", "area-budget-hard-kept", Warning, "The area was over its hard budget in the base and grew by at most `[budgets] growth` characters, or did not grow. It needs no change in this task. To fix it, split the area into sub-areas and give each one a summary.";
    RequirementKeyword = "SD801", "requirement-keyword", Warning, "Write the requirement with an uppercase BCP 14 keyword (MUST, SHALL, SHOULD, MAY).";
    EarsPattern = "SD802", "ears-pattern", Warning, "Use an EARS pattern. Put `then` after an `If` condition.";
    SentenceLength = "SD803", "sentence-length", Warning, "Split the requirement. Write one requirement in one sentence.";
    VagueTerm = "SD804", "vague-term", Warning, "Replace the vague term with a testable value or condition.";
    PassiveVoice = "SD805", "passive-voice", Warning, "Name the actor as the subject (`the controller SHALL close`, not `SHALL be closed`).";
    RequirementsUnrecognized = "SD806", "requirements-unrecognized", Warning, "Write each requirement as a list item that starts with its bold key: `- **R-12**: sentence` or `- **R-12** sentence`. A key in a table, a heading or a paragraph is not a requirement.";
    RequirementColonMissing = "SD807", "requirement-colon-missing", Warning, "Write the requirement as `- **key**: sentence`, with the colon right after the bold key. Put a note, such as a rule number, after the colon.";
    LinkDirection = "SD901", "link-direction", Warning, "Remove the reference (a link, `[[id]]`, `depends-on` or `supersedes`), or move the text to a layer that is not more stable than the target. As an alternative, add the layer of the document to `linked-from` of the target layer in spec-dna.toml.";
    LayerLimit = "SD902", "layer-limit", Warning, "Move or merge documents of the layer. As an alternative, change `max-docs` or `max-chars` of the layer in a change of its own.";
    HistoryInRequirements = "SD903", "history-in-requirements", Warning, "Remove the date, the round or the history phrase from the requirement. Write the current rule only. Keep the history in the git log.";
    TermForbidden = "SD904", "term-forbidden", Warning, "Use the term that the message names instead of the forbidden synonym. The terms are in the table of the `terms` document.";
    FanOut = "SD905", "fan-out", Warning, "Split the change. One change touches more requirement documents than `[change] max-fanout` allows. Write each part as its own intent.";
    GateApprovalMissing = "SD910", "gate-approval-missing", Warning, "Get the approval of the earlier artifact in place before the work. The order of artifacts is `intent.md`, `spec.md`, `plan.md`, then code. A `draft` spec or plan needs no approval of the artifact before it. The message names one of 9 reasons. 1: the base tree has no approval. Approve the earlier artifact in its own pull request and merge it, or use `[change] approval = \"branch\"`. 2: no approval commit comes before the work. Commit the approval alone, with the `status` line only, by an approver, before the work. 3: the author of the approval commit is not in `[change] approvers`. Ask an approver to commit it. 4: the commit changes more than the `status` line. Split it. 5: the clone is shallow. Fetch the full history (`fetch-depth: 0`). 6: a merge commit stops the rule. Rebase the branch, or merge only the base into it. 7: the change was added and finished in this branch. Merge the artifacts and their approvals in a pull request first, then finish in a later one. 8: the path is not in the `Files` of the plan at the approval commit. Put it in `Files` before the approval commit, or move the edit to a change of its own. 9: a later commit set the artifact back after its approval, so the approval no longer holds. Approve it again with an approval commit before the work, or take the work out of the branch.";
    ChangeStale = "SD911", "change-stale", Warning, "Finish the change, or close it. To drop it, set the status of its intent to `closed` in a pull request. A later pull request runs `spec-dna finish`.";
    StartedLimit = "SD912", "started-limit", Warning, "Finish or close a started change before you start a new one. The number of `accepted` intents is above `max-started`.";
    OutsidePlan = "SD913", "outside-plan", Warning, "Add the path to `Files` of the plan, or move the edit to a pull request of its own. Each edited file must match an item of `Files`. An item is a path or a glob in a code span, or a plain path with no white space.";
    DeltaInvalid = "SD914", "delta-invalid", Warning, "Fix the delta line. Use `ADD <id>#<key>: <sentence>`, `CHANGE <id>#<key>@<hash>: <sentence>` or `REMOVE <id>#<key>@<hash>`. Name an existing target in a layer that `moves-to` allows. Copy the hash from `spec-dna show`.";
    FinishIncomplete = "SD915", "finish-incomplete", Warning, "Apply the delta, or set the intent to `closed` before you delete the folder. Run `spec-dna finish <slug>` to apply the delta and delete the folder in one step.";
    DeltaEvidenceMissing = "SD916", "delta-evidence-missing", Warning, "Link a test for each requirement that the change adds or changes. Add `covers: <doc-id>#<key>` in a test file, or `<!-- verified-by: <path> -->` at the end of the requirement. A `covers: partial` tag alone is not evidence.";
    ClassTooLow = "SD917", "class-too-low", Warning, "Raise the `class` of the intent. The diff or the delta needs a higher class (`higher-risk`), because it touches a layer of `high-risk-layers` or a path of `high-risk-paths`.";
    ConcernOpen = "SD919", "concern-open", Warning, "Resolve each open concern of the spec, or remove it, before the spec is `approved`.";
    DecisionNeededUnaccepted = "SD920", "decision-needed-unaccepted", Warning, "Write the decision that the spec needs, and accept it. A change cannot finish while a needed decision is absent or is not `accepted`.";
    IncidentEvalMissing = "SD918", "incident-eval-missing", Warning, "Add an eval for the incident to [evals] dir in the same change. Name its file in the `Eval` section of the incident.";
    TruthChangedWithoutChange = "SD921", "truth-changed-without-change", Warning, "Write the change as an intent folder of the `intent` layer in the same pull request (`spec-dna new intent <slug>`). A move that keeps the text of each requirement gives no SD921.";
    QuestionOpen = "SD922", "question-open", Warning, "Answer each open question of the spec, or remove it, before the spec is `approved`. Check the box (`- [x]`) and write the answer after the question.";
    TaskIncomplete = "SD923", "task-incomplete", Warning, "Complete the task of the approved plan. Write `Files:` with each exact path of the task (no glob, no folder), or `Files: none`. Write `Verify:` with a command in a code span, or the check in words.";
    TaskTooLarge = "SD924", "task-too-large", Warning, "Split the task. Write each part as its own task with at most `[change] max-task-files` paths in `Files:` and one `Verify:`.";
    CheckboxMissing = "SD925", "checkbox-missing", Warning, "Write the task box at the start of the item: `- [ ]` for an open item, `- [x]` for a resolved item. SD919 and SD922 read the box. To state that the section has no item, write one item that starts with the word `none`, followed by `:`, `.` or the end.";
ApproverAgent = "SD926", "approver-agent", Warning, "Remove the address from `[change] approvers` in spec-dna.toml, in a pull request of its own (SD503). List the e-mails of the persons that approve.";
    AcceptanceMissing = "SD927", "acceptance-missing", Warning, "The change needs an acceptance record, and the head has none. Start an agent `acceptor` with the prompt that `spec-dna next` prints: `cd '<root>' && spec-dna accept <slug> --brief`. For a second acceptor in a new session, run `claude --agent acceptor '<prompt>'`. The acceptor runs the prompt, then `spec-dna accept <slug>`, and writes `acceptance.toml` in the folder of the change. Then run `spec-dna finish <slug>`. In a shallow clone, the commit that added the plan can be absent from the history: fetch the full history (`fetch-depth: 0`).";
    AcceptanceInvalid = "SD928", "acceptance-invalid", Warning, "The record breaks a rule of a valid record. The message names the rule. Fix the record: write each answer, and replace each `pending` value. Do not edit the code. If the `brief` of an entry is not the brief of its commit, remove the entry, and accept again from a new brief.";
    AcceptanceStale = "SD929", "acceptance-stale", Warning, "The code that the record lists changed after the acceptance, or the diff has a code path of the scope that the record does not list. Before `finish`: delete the record, run `spec-dna accept <slug>` again from a new brief, and run `spec-dna finish <slug>` again. After `finish`: run `git revert --no-commit` on the commit of `finish`, delete `<folder>/acceptance.toml`, commit, merge the base, and do the same.";
    AcceptanceRejected = "SD930", "acceptance-rejected", Warning, "The record says `rejected`. Fix each finding in the code, delete the record, and accept again.";
    SelfAccepted = "SD931", "self-accepted", Warning, "2 acceptor entries have the same `session` and the same `agent`. An acceptor must be another agent than the builder. Remove the entry, and accept again with an acceptor that has its own session or agent id. The value `unknown` matches no value.";
    RecordCommitInvalid = "SD932", "record-commit-invalid", Warning, "With `[change] approval = \"branch\"`, a commit that adds or edits the acceptance record changes the record alone, and a commit adds an entry to `acknowledged-by` only as an approval commit of its author (`spec-dna approve <slug> acceptance`) that changes nothing else. A commit that changes the record and keeps an entry also gives SD932. For a commit that changes the record and another path: delete the record in a commit of its own, then run `spec-dna accept <slug>` again. SD932 reads only the commits of the current life of the record, so no rewrite of the history is needed. For an entry that does not count: ask an approver to run `spec-dna approve <slug> acceptance` again. The commit of `finish` is the only other exception.";
    AcceptorMissing = "SD933", "acceptor-missing", Warning, "The record has fewer acceptors than the change needs: 1 for a routine change, `[change] higher-risk-acceptors` for a higher-risk change, and an entry that names each role of `[owners]` that matches a role path of the change in `roles`. One entry can name more than one role. Run `spec-dna accept <slug> [--role <name>]` for each missing acceptor.";
    AckMissing = "SD934", "ack-missing", Warning, "With `[change] approval = \"branch\"`, a record that has a deviation needs the acknowledgement of an approver. Ask an approver to run `spec-dna approve <slug> acceptance` in a terminal of their own. The entry counts only when its approval commit changes `acknowledged-by` alone and no later commit changes the record.";
    RecordEdited = "SD935", "record-edited", Warning, "A file of `.spec-dna/accepted/` is an archive of a finished change. Do not edit or delete it. Revert the edit.";
    InvariantInvalid = "SD936", "invariant-invalid", Warning, "Fix the item of `Invariants` in the spec. Write `- <doc-id>#<key>@<hash>` for a requirement of a `rule` document, or write one item `- none`. Copy the hash from `spec-dna show <doc-id>#<key>`. Remove an item that names a rule which the delta of the same spec adds or removes: such a rule cannot stay true across the change.";
    InvariantStale = "SD937", "invariant-stale", Warning, "The hash of the item is not the hash of the rule. Run `spec-dna show <doc-id>#<key>`, read the rule again, and copy the new hash into the item. If the delta of the same spec changes the rule with a `CHANGE` line, the item must have the hash of that line.";
    RecordIncomplete = "SD938", "record-incomplete", Warning, "The record has no answer for a declared invariant, or no entry with the same text for a deviation. The message names the entry to add. An acceptor writes the entry, and commits the record alone. Run `spec-dna accept <slug> --brief` to see the items.";
    DeviationInvalid = "SD939", "deviation-invalid", Warning, "Fix the item of `Deviations` in the plan. Write `- <id> (<kind>): <text>`, where the kind is `plan` or `spec`, and each id is used once.";
    AcceptanceRerun = "SD941", "acceptance-rerun", Warning, "With `[change] approval = \"branch\"`, the record comes after 2 rejected records of the change, and no approval commit of `plan.md` comes after the second. A third run needs the word of the owner. Commit `status: draft` of `plan.md` on its own, skip the commit hook for that commit only, and name the skip in the body of the commit. The draft commit gives SD401 and SD910 reason 9. Then the approver runs `spec-dna approve <slug> plan`, and both codes are gone. In a shallow clone, fetch the full history (`fetch-depth: 0`).";
    SummaryStale = "SD641", "summary-stale", Warning, "A pin of a summary, or of a `system` or `product` document with `rests-on`, no longer holds: the text under it changed, the target was removed or is not active, or the target is no longer in the frontier of the document. Read the item that the message names. Edit the text of the summary so that it states the new meaning, then run `spec-dna refresh <id>`. If the text of the summary still holds, run `spec-dna refresh <id> --holds`: the diff and SD645 show that claim. `refresh` refuses a document whose text did not change, unless it has `--holds`.";
    SummaryIncomplete = "SD642", "summary-incomplete", Warning, "An item of the frontier of the document has no pin: a new requirement, a requirement that became active, or a new summary of a sub-area. Read the item that the message names. Edit the text of the summary to cover it, then run `spec-dna refresh <id>`. If the text of the summary still holds, run `spec-dna refresh <id> --holds`.";
    PinInvalid = "SD643", "pin-invalid", Warning, "Fix the pin. Write `<doc-id>#<key>@<16 hex>` for a requirement, or `<doc-id>@<16 hex>` for a document, with the hash that `spec-dna show` prints. A pin names an item below its document: a summary pins the requirements and the summaries of its area, a `system` document pins the summaries, and a `product` document pins the `system` documents. An alias that names more than one document is ambiguous: use the full id. Put `rests-on` only in a summary, a `system` document or a `product` document. Run `spec-dna refresh <id>` to write the pins again.";
    SummaryNoArea = "SD644", "summary-no-area", Warning, "A summary is the file `<summaries folder>/<area>.md` of an area, and `<requirements folder>/<area>/` must be a folder of the layer `requirements`. Move the summary to the path of an existing area, or delete it. `spec-dna new summary <area>` writes a summary at the right path.";
    SummaryRepinned = "SD645", "summary-repinned", Warning, "Against the base, the pins of the document changed and its text did not. This is the claim of `spec-dna refresh <id> --holds`: the text still holds. Check that the text still says the truth about the changed items. If it does not, edit it. The finding is a warning that shows the claim, and it needs no other action.";
    SummaryRestsOnNothing = "SD646", "summary-rests-on-nothing", Warning, "A roll-up document has no pin and an empty frontier, and another roll-up document pins it, so the chain rests on nothing. The message names the document that pins it. Add the layer of the sources of its area to `rolls-up`, or add the sources. If the document is not needed, delete it and the pin that names it.";
    SummaryNoteChanged = "SD647", "summary-note-changed", Warning, "A pin to a research document no longer holds: the note changed, was removed or left the frontier. Or a research document of the frontier has no pin. A changed note does not make a normative text wrong, so this finding is a warning. Read the note. If the meaning of the summary changes, edit its text. Then run `spec-dna refresh <id>`. If the text still holds, run `spec-dna refresh <id> --holds`: the diff and SD645 show that claim.";
    DeviationUnresolved = "SD940", "deviation-unresolved", Warning, "A `spec` deviation needs a way to fix the difference. Write `Delta: <verb> <doc-id>#<key>` that names a line of the delta of the spec, or `Follow-up: <slug>` that names a change folder of the tree.";
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

/// The fix of SD935 when the history is cut: the finding says that the bytes
/// cannot be compared, not that they differ.
pub const CUT_HISTORY_FIX: &str = "Fetch the full history (`fetch-depth: 0`) and run `check` again. Do not change the archive file because of this finding: it is not an edit.";

/// The fix of SD929 for a record in the folder of the change, before `finish`.
pub const STALE_BEFORE_FINISH_FIX: &str = "Delete the record, run `spec-dna accept <slug>` again from a new brief, and run `spec-dna finish <slug>` again.";

/// The fix of SD303 for the type `summary` in a project that has a `[layers]`
/// table and no layer for it (design 0.8.0, D50). A project with no table keeps
/// the fix of 0.7.1.
pub const NO_SUMMARY_LAYER_FIX: &str = "Use a type of the root and of the `types` list of spec-dna.toml (without the list: product, system, terms, roadmap, questions, rule, requirements, interface, decision, procedure, incident, research, intent, spec, plan, summary), or add the type to the list. If no layer has the type, run `spec-dna layer add <name> --types <type>`. For the type `summary`, run `spec-dna layer add summaries`. The message names the new type of a type of 0.5.";

/// The part of the message of SD303 that selects `NO_SUMMARY_LAYER_FIX`.
pub const NO_SUMMARY_LAYER_MARK: &str = "no layer of spec-dna.toml has type `summary`";

/// The part of the message of SD401 for a spec or a plan that moves from
/// `approved` to `draft` (design 0.8.1, D91). It selects `REWORK_FIX`.
pub const REWORK_MARK: &str = "this is the step back of a rework: a hook that runs `spec-dna check` rejects its commit, so skip only that hook for this commit, name the skip in its body, and make the approval commit next";

/// The fix of SD401 for the step back of a rework.
pub const REWORK_FIX: &str = "To rework an approved spec or plan, set `status: draft` in the commit of the edit. Skip only the hook that runs `spec-dna check` for that commit, and name the skip in the body of the commit. Then the approver makes the approval commit, and SD401 is gone. For another document, write a new document.";

/// The fix of SD929 for an archive file, after `finish`.
/// The words that end the message of SD642, SD643 and SD644 when the layer of
/// the summaries has the key `rolls-up` (design 0.8.1, D84). The fix of these
/// codes follows the mark, so a project with no key keeps the text of 0.8.0.
pub const ROLLUP_KEY_MARK: &str = "(source layers of the roll-up: ";

const ROLLUP_INCOMPLETE_FIX: &str = "An item of the frontier of the document has no pin: a new requirement, a requirement that became active, a new accepted decision, or a new summary of a sub-area. A research document of the frontier with no pin gives SD647. Read the item that the message names. Edit the text of the summary to cover it, then run `spec-dna refresh <id>`. If the text of the summary still holds, run `spec-dna refresh <id> --holds`.";

const ROLLUP_INVALID_FIX: &str = "Fix the pin. Write `<doc-id>#<key>@<16 hex>` for a requirement, or `<doc-id>@<16 hex>` for a document. Do not write a hash by hand: `spec-dna refresh <id>` writes each pin with its hash. A pin names an item below its document: a summary pins the requirements of its area (also the requirements of an interface or a rule document, and each accepted decision or research document as a whole, where the layer of that document is in `rolls-up`) and the summaries of its area, a `system` document pins the summaries, and a `product` document pins the `system` documents. An alias that names more than one document is ambiguous: use the full id. Put `rests-on` only in a summary, a `system` document or a `product` document. Run `spec-dna refresh <id>` to write the pins again.";

const ROLLUP_NO_AREA_FIX: &str = "A summary is the file `<summaries folder>/<area>.md` of an area, and `<area>` must be a folder of a source layer: the layer `requirements`, or a layer that `rolls-up` names. Move the summary to the path of an existing area, or delete it. `spec-dna new summary <area>` writes a summary at the right path.";

pub const STALE_AFTER_FINISH_FIX: &str = "Run `git revert --no-commit` on the commit of `finish`, delete `<folder>/acceptance.toml`, and commit. The revert commit then holds no record, so it gives no SD932. Merge the base, run `spec-dna accept <slug>` again from a new brief, and run `spec-dna finish <slug>` again.";

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

    /// The fix text of the finding. It is the fix of the code, with one case:
    /// SD935 for a cut history (a shallow clone). The archive file is not wrong
    /// there, so the fix must not tell the user to revert it.
    pub fn fix(&self) -> &'static str {
        let said = |mark: &str| self.message.contains(mark);
        match self.code {
            Code::RecordEdited if said(crate::acceptance::CUT_MARK) => CUT_HISTORY_FIX,
            Code::TypeInvalid if said(NO_SUMMARY_LAYER_MARK) => NO_SUMMARY_LAYER_FIX,
            Code::StatusTransition if said(REWORK_MARK) => REWORK_FIX,
            Code::SummaryIncomplete if said(ROLLUP_KEY_MARK) => ROLLUP_INCOMPLETE_FIX,
            Code::PinInvalid if said(ROLLUP_KEY_MARK) => ROLLUP_INVALID_FIX,
            Code::SummaryNoArea if said(ROLLUP_KEY_MARK) => ROLLUP_NO_AREA_FIX,
            // The message names the place: the fix follows it.
            Code::AcceptanceStale if said(crate::acceptance::STALE_AFTER_FINISH) => {
                STALE_AFTER_FINISH_FIX
            }
            Code::AcceptanceStale if said(crate::acceptance::STALE_BEFORE_FINISH) => {
                STALE_BEFORE_FINISH_FIX
            }
            _ => self.code.fix(),
        }
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
            .str("fix", self.fix())
            .end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fix_of_sd927_names_the_prompt_line_and_the_agent_session() {
        let f = Code::AcceptanceMissing.fix();
        assert!(
            f.contains("`cd '<root>' && spec-dna accept <slug> --brief`"),
            "{f}"
        );
        assert!(f.contains("`claude --agent acceptor '<prompt>'`"), "{f}");
        assert!(!f.contains("unchanged"), "{f}");
    }

    #[test]
    fn codes_are_unique_and_well_formed() {
        let mut seen = std::collections::HashSet::new();
        for c in Code::ALL {
            assert!(seen.insert(c.code()), "duplicate {}", c.code());
            assert!(seen.insert(c.name()), "duplicate {}", c.name());
            assert!(c.code().starts_with("SD") && c.code().len() == 5);
            assert_eq!(
                c.is_state(),
                matches!(c.code(), "SD911" | "SD616" | "SD618")
            );
            // Codes from 600 up are warnings. SD702 became an error by a decision at
            // checkpoint 2. SD507 is a warning, because ADR 0016 says that it is
            // never an error.
            let is_warning =
                (c.code().as_bytes()[2] >= b'6' && c.code() != "SD702") || c.code() == "SD507";
            assert_eq!(
                is_warning,
                c.severity() == Severity::Warning,
                "{}",
                c.code()
            );
        }
    }

    #[test]
    fn the_fix_of_sd935_for_a_cut_history_does_not_say_revert() {
        let mut d = Diag {
            path: ".spec-dna/accepted/a.toml".into(),
            line: 1,
            code: Code::RecordEdited,
            doc: String::new(),
            target: String::new(),
            message: "the archive file differs".into(),
            severity: Severity::Error,
        };
        assert_eq!(d.fix(), Code::RecordEdited.fix());
        assert!(d.fix().contains("Revert the edit"));
        d.message = format!("the history {} and more", crate::acceptance::CUT_MARK);
        assert!(!d.fix().contains("Revert"));
        assert!(d.fix().contains("fetch-depth: 0"));
        assert!(d.json(false).contains("fetch-depth: 0"));
        // SD929 follows the place that its message names.
        d.code = Code::AcceptanceStale;
        d.message = format!(
            "the record is stale; {}, merge",
            crate::acceptance::STALE_AFTER_FINISH
        );
        assert_eq!(d.fix(), STALE_AFTER_FINISH_FIX);
        d.message = format!(
            "the record is stale; {}, run",
            crate::acceptance::STALE_BEFORE_FINISH
        );
        assert_eq!(d.fix(), STALE_BEFORE_FINISH_FIX);
        d.message = "the record is stale".into();
        assert_eq!(d.fix(), Code::AcceptanceStale.fix());
        // Another code keeps the fix of the code.
        d.code = Code::AcceptanceInvalid;
        assert_eq!(d.fix(), Code::AcceptanceInvalid.fix());
    }

    #[test]
    fn no_name_or_fix_text_says_frozen() {
        for c in Code::ALL {
            let text = format!("{} {}", c.name(), c.fix()).to_lowercase();
            assert!(!text.contains("frozen"), "{}", c.code());
        }
    }
}

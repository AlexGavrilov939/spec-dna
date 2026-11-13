# Diagnostics

Every diagnostic has a stable code. A code keeps its meaning. spec-dna does not use a retired code again. An error fails `check` when the error is new against the base. A warning never causes a failure. The table gives the severity of each code in the `standard` profile. The profile and the rules of the root can change it or turn the code off ([profiles.md](profiles.md)).

The text output of `check` lists the errors, then the drift warnings, then the other warnings, then one count line for each prose lint code. After them, it gives the fix of each listed code once (`SD102 fix: ...`), so an agent needs no lookup. `check --json` prints `code`, `name`, `severity`, `new`, `doc`, `path`, `line`, `target`, `message` and `fix` for each diagnostic. It also prints `documents`: the id, path, type, status, body characters and number of recognized requirements of each document.

Base-relative gating compares diagnostics by their fingerprint (code, document id, target). For a moved document, the comparison uses its old id, which the document keeps as an alias. Codes in the 400 range and SD202, SD605, SD606, SD607, SD702 and SD703 compare the base tree with the head tree. Thus, these diagnostics are always new. SD704 is new only when the root does not enforce budgets and the document is new or grew.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD101 | unresolved-reference | error | A `[[id]]`, `[[id#key]]`, `supersedes` or `depends-on` value names no document, section or requirement. |
| SD102 | unresolved-link | error | A Markdown link or image points to a path that is not in the git tree. |
| SD103 | evidence-missing | error | A `verified-by` path (in the front matter, or in the inline `<!-- verified-by: ... -->` marker of a requirement) is not in the git tree. |
| SD104 | ambiguous-reference | error | A reference names an alias that more than one document has. |
| SD105 | covers-unresolved | error | A `covers:` tag in a test file names no requirement: the document, the key or the requirement is missing, or the id names a section. |
| SD106 | convention-unresolved | error | A pointer of `conventions` in spec-dna.toml names no document, or no section of the document. |
| SD201 | duplicate-id | error | Two documents have the same id or alias. |
| SD202 | id-removed | error | An id of the base tree is gone, and no document keeps it as an alias. You can delete a `done` or `rejected` change, or a note. |
| SD203 | key-format | error | A requirement key in a sentence with a BCP 14 keyword is not a slug and not a legacy key. |
| SD204 | key-duplicate | error | A requirement key occurs two times in one document. |
| SD301 | front-matter-syntax | error | The front matter is not in the supported YAML subset. |
| SD302 | front-matter-field | error | A known field has a value of the wrong kind. |
| SD303 | type-invalid | error | The `type` is unknown, it does not agree with a single-type root, or it is not in the `types` list of the project. |
| SD304 | status-invalid | error | After the root's `status-map` applies, the status is missing or the type does not allow it. |
| SD305 | section-missing | error | A document has no section with a heading from one group of the root's `required-sections` for its type. The defaults: a decision needs `Decision Outcome` or `Decision`, an invariant `Rules`, a runbook `Steps`, an incident `Summary` and `Root cause`. |
| SD306 | placeholder-left | error | A template placeholder (`{{text}}`, from `spec-dna new`) is still in the document. |
| SD401 | status-transition | error | A status moved backwards or left a final state. |
| SD402 | superseded-without-successor | error | A superseded document has no active (or accepted) successor that names it in `supersedes`. |
| SD403 | supersede-cycle | error | The `supersedes` chain returns to its start. |
| SD404 | depends-on-inactive | error | `depends-on` names a superseded or retired document. |
| SD405 | decision-frozen | error | Text outside the amendment sections (`amendment-sections`, default `## Amendments`), the history sections and the live sections (`live-sections`) changed after the decision became accepted. |
| SD501 | adapter-modified | error | A generated adapter (the line in `AGENTS.md` or `CLAUDE.md`, or the skill block) has a manual edit. |
| SD502 | adapter-outdated | error | A generated adapter is different from the adapter that this version generates. The multi-line block of an earlier version also gives SD502: `spec-dna init` replaces it with one line. |
| SD503 | config-not-alone | error | The change edits `spec-dna.toml` and other files (documents or code). A configuration change must be a change of its own. The files that `init` writes with it (the adapters, and files whose name starts with `spec-dna`) do not count, and a new `spec-dna.toml` is not checked. `check` compares the working tree with the base (with `--base`, tracked files; without it, also untracked files). |
| SD601 | token-unresolved | warning | A bare `PREFIX-NUMBER` token with a configured legacy prefix names no document. |
| SD602 | anchor-missing | warning | A Markdown link fragment names no heading or requirement of its target. |
| SD603 | cross-repo-unpinned | warning | spec-dna cannot check a `<repo>:<id>` reference without a pinned index. |
| SD604 | depends-on-deprecated | warning | `depends-on` names a deprecated document. |
| SD605 | amendment-outcome | warning | New amendment text states a requirement, a BCP 14 keyword or a status word. |
| SD606 | cited-path-removed | warning | A document cites a file or a directory that the change removed or renamed. The message names the new path of a file with the same content. A link gives SD102 instead. |
| SD607 | cited-symbol-removed | warning | A document cites an identifier in an inline code span. The change removed the identifier from a file, and no other file outside of Markdown has it. If the file has exactly one new identifier, the message names it as a possible new name. |
| SD608 | requirement-unverified | warning | A requirement of an active spec, contract or invariant has no evidence: no `verified-by` and no `covers:` tag. |
| SD609 | incident-unlinked | warning | An incident links no requirement, decision or runbook. |
| SD610 | index-stale | warning | A generated index (`spec-dna index`) does not match its entries: an entry document was added, removed or renamed, or its title or status changed, or the rows were edited by hand. |
| SD701 | budget-soft | warning | A document is over its soft budget. |
| SD702 | budget-hard | error | A document is over its hard budget, and against the base it is new, it was under the hard budget, or it grew by more than `[budgets] growth` characters (default 4,000). |
| SD703 | budget-grew | warning | A document over its soft budget grew against the base: under its hard budget, or over it by at most `[budgets] growth` characters. |
| SD704 | budget-hard-kept | warning | A document is over its hard budget, but it was over it in the base and did not grow, or it is a new document that a split moved out of a document over its hard budget (its section aliases name that document). In a root with `enforce-budgets = false`, every hard overage is SD704. |
| SD801 | requirement-keyword | warning | A requirement has no uppercase BCP 14 keyword. |
| SD802 | ears-pattern | warning | An `If` requirement has no `then`. |
| SD803 | sentence-length | warning | A requirement sentence is longer than `lint.max-words`. |
| SD804 | vague-term | warning | A requirement uses a term from the vague-term list. |
| SD805 | passive-voice | warning | A requirement uses a passive construction (heuristic). |
| SD806 | requirements-unrecognized | warning | A spec or contract has bare tokens that match a `legacy-keys` pattern, but no recognized requirement. Or some requirements are recognized, but a list item or a table row starts with a `legacy-keys` token that is not the key of a recognized requirement. The legacy keys are probably in a syntax that spec-dna does not read, such as a table. |
| SD807 | requirement-colon-missing | warning | In a spec or contract, a list item starts with a bold slug key and has a BCP 14 keyword, but no colon follows the key (`- **key** [1.1]: sentence`). Thus the item is not a requirement. |

The requirement lint (SD801 to SD805) is off in the `lite` and `standard` profiles, and a warning in `strict`. The rule `lint` changes it.

The [research matrix](research/matrix.md), section "Controlled-language lint", describes the lint warnings SD803 to SD805 and their sources.

## Code citations

SD606 and SD607 use the code citations of a document ([ADR 0001](adr/0001-information-model.md), amendment "code citations"). `context <code path>` uses the same citations. A document cites code in these ways:

- A link to a file that is not a document. The text of the link is not a second citation.
- A path in an inline code span or in prose, with an optional location: `src/loans.py`, `src/loans.py:42`, `src/loans.py#L3-L9`. A path is relative to the repository root or to the document. A file name or a path tail that only one file has also resolves: `loans.py`.
- A distinctive identifier in an inline code span: a name with an inner `_` (`max_loans`), a name with a lowercase letter before an uppercase letter (`loanBook`), or a called name with 4 characters or more (`renew()`). Words such as `status` or `HTTP` are not identifier citations. A short list of common call names (`read()`, `main()`) and product names (`GitHub`, `macOS`) is also excluded.

An identifier matches a file if it occurs in the file as a whole word, also in a comment or a string. A citation that still resolves after the change, such as a file name after a move, gives no SD606.

An accepted decision cannot change its frozen text (SD405). Thus, when the frozen text of an accepted decision cites a removed path or identifier, an amendment section of the decision clears SD606 and SD607 for that decision if it cites new code: a path that the head tree has (the rename target, if git found one) or an identifier that the code has, and that the base text of the decision does not cite. A citation of removed code in the amendment section itself still gives a warning, because that text can change.

The rule `drift` sets the severity of SD606 and SD607 (`warning` in `standard`), or turns them off. Only current documents give SD606 and SD607. A superseded, retired or rejected document, a `done` change, a note, an incident and a reference can cite code that is gone.

# Diagnostics

Every diagnostic has a stable code. A code keeps its meaning. spec-dna does not use a retired code again. An error fails `check` when the error is new against the base. A warning never fails `check`.

The table gives the severity of each code in the `standard` profile. The profile and the rules of the root can change the severity or disable the code ([profiles.md](profiles.md)).

The text output of `check` has this order: the errors, the drift warnings, the other warnings, and one count line for each prose lint code. Then it gives the fix of each listed code once (`SD102 fix: ...`). Thus an agent needs no lookup. `check --json` prints `code`, `name`, `severity`, `new`, `doc`, `path`, `line`, `target`, `message` and `fix` for each diagnostic. It also prints `documents`: the id, path, type, status, body characters and number of recognized requirements of each document.

The gate compares each diagnostic with the base by its fingerprint (code, document id, target). For a moved document, the comparison uses its old id, which the document keeps as an alias. The codes SD401 to SD405, SD202, SD605, SD606, SD607, SD702, SD703 and SD921 compare the base tree with the head tree. Thus these diagnostics are always new. SD704 is new only if the root does not enforce budgets and the document is new or grew.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD101 | unresolved-reference | error | A `[[id]]`, `[[id#key]]`, `supersedes` or `depends-on` value names no document, section or requirement. |
| SD102 | unresolved-link | error | A Markdown link or image points to a path that is not in the git tree. |
| SD103 | evidence-missing | error | A `verified-by` path (in the front matter, or in the inline `<!-- verified-by: ... -->` marker of a requirement) is not in the git tree. |
| SD104 | ambiguous-reference | error | A reference names an alias that more than one document has. |
| SD105 | covers-unresolved | error | A `covers:` tag in a test file names no requirement. The document, the key or the requirement does not exist, or the id names a section. |
| SD106 | convention-unresolved | error | A pointer of `conventions` in spec-dna.toml names a document or a Markdown file that is not in the git tree. Or it names a section that the document or file does not have. |
| SD201 | duplicate-id | error | Two documents have the same id or alias. |
| SD202 | id-removed | error | The head tree does not have an id of the base tree, and no document keeps it as an alias. The layer of the base document decides: a layer with `keep = "until-finished"` (the `intent` layer by default) allows the deletion. In a `forever` layer, only the field `was-type: note` or `was-type: change` allows it. The status gives no exemption. |
| SD203 | key-format | error | A requirement key in a sentence with a BCP 14 keyword is not a slug and not a legacy key. A key that is a template placeholder (`{{key}}`) gives SD306 only. |
| SD204 | key-duplicate | error | A requirement key occurs two times in one document. |
| SD301 | front-matter-syntax | error | The front matter is not in the supported YAML subset. |
| SD302 | front-matter-field | error | A known field has a value of the wrong kind. |
| SD303 | type-invalid | error | The `type` is unknown (the message names the new type of a type of 0.5), it does not match a single-type root, or it is not in the `types` list of the project. The document has no type: its front matter has none, its place gives none, and the root has no `default-type`. The type is valid only in the `intent` layer (`intent`, `spec` or `plan`), and the document is at another place. A type that no layer has also gives SD303, with the hint to run `spec-dna layer add`. In the first 3 cases (an unknown type, no type, and an intent-only type at a wrong place), the document has no valid type. It gets no SD304 for its status and no SD305 for the sections of a type, and no budget applies to it. |
| SD304 | status-invalid | error | After the `status-map` of the root applies, the status is absent, or the type does not allow it. The types `requirements`, `interface`, `rule`, `procedure` and `intent` use `draft` for a proposal. `proposed` is for a decision. On an `intent`, the message names `closed` for `rejected` and `research` for `done`. |
| SD305 | section-missing | error | A document has no section with a heading from one group of `required-sections` of the root for its type. The defaults are: decision `Decision Outcome` or `Decision`; rule `Rules`; incident `Summary` and `Root cause`; intent `Intent`, `Problem` or `Summary`; spec `Delta`; plan `Files`. |
| SD306 | placeholder-left | error | A template placeholder (`{{text}}`, from `spec-dna new`) is still in the document. |
| SD401 | status-transition | error | A status moved back, or it left a final state. |
| SD402 | superseded-without-successor | error | A superseded document has no active (or accepted) successor that names it in `supersedes`. |
| SD403 | supersede-cycle | error | The `supersedes` chain returns to its start. |
| SD404 | depends-on-inactive | error | `depends-on` names a superseded or retired document. |
| SD405 | decision-locked | error | Text outside the amendment sections (`amendment-sections`, default `## Amendments`), the history sections and the live sections (`live-sections`, default `Status`) changed after the decision got the status `accepted`. |
| SD501 | adapter-modified | error | A generated adapter (the line in `AGENTS.md` or `CLAUDE.md`, or the skill block) has a manual edit. A generated harness file (an agent file, `model-team.md` or the model-team line of `CLAUDE.md`) has a manual edit, or text after its marker. |
| SD502 | adapter-outdated | error | A generated adapter differs from the adapter that this version generates. The multi-line block of an earlier version also gives SD502: `spec-dna init` replaces it with one line. A generated harness file or line differs from the text for the configuration. A `.claude/settings.json` entry of the harness is absent or different (the message names its JSON path). A file with a harness marker is no longer in the configuration (`spec-dna init --reconfigure` removes it). `CLAUDE.md` has a second model-team line (delete it). |
| SD503 | config-not-alone | error | The change edits `spec-dna.toml` and other files (documents or code). A configuration change must be a change of its own. The files that `init` writes with it (the adapters, the harness files, `.pre-commit-config.yaml`, `.husky/pre-commit`, and files whose name starts with `spec-dna`) do not count. `check` does not examine a new `spec-dna.toml`. `check` compares the working tree with the base (with `--base`, tracked files; without it, also untracked files). |
| SD504 | harness-missing | error | `[harness]` has a level that is not `none`, and a file that the harness needs is absent: an agent file, `.claude/agents/model-team.md`, or the model-team line of `CLAUDE.md`. A file at the path of an agent file, with no harness marker, also gives SD504: spec-dna does not replace a file that it did not generate. |
| SD505 | harness-settings-invalid | error | `.claude/settings.json` is not valid JSON, or its root is not an object, or a key that spec-dna writes into has a value of the wrong type (the message names it). `check` cannot compare the harness entries. |
| SD601 | token-unresolved | warning | A bare `PREFIX-NUMBER` token with a configured legacy prefix names no document. |
| SD602 | anchor-missing | warning | A Markdown link fragment names no heading or requirement of its target. |
| SD603 | cross-repo-unpinned | warning | spec-dna cannot check a `<repo>:<id>` reference without a pinned index. |
| SD604 | depends-on-deprecated | warning | `depends-on` names a deprecated document. |
| SD605 | amendment-outcome | warning | New amendment text states a requirement, a BCP 14 keyword or a status word. |
| SD606 | cited-path-removed | warning | A document cites a file or a directory that the change removed or renamed. The message names the new path of a file with the same content. A link gives SD102 instead. |
| SD607 | cited-symbol-removed | warning | A document cites an identifier in an inline code span. The change removed the identifier from a file, and no other file that is not Markdown has it. If the file has exactly one new identifier, the message names it as a possible new name. |
| SD608 | requirement-unverified | warning | A requirement of an active `requirements`, `interface` or `rule` document has no evidence: no `verified-by` and no `covers:` tag. |
| SD609 | incident-unlinked | warning | An incident links no requirement, rule, decision or procedure. |
| SD610 | index-stale | warning | A generated index (`spec-dna index`) does not match its entries. A change added, removed or renamed an entry document, or changed its title or status, or edited the rows manually. |
| SD611 | decision-links-intent | warning | An accepted decision links an `intent` document in its locked text, or in `depends-on`. You can delete a finished intent (SD202 allows it), but you cannot remove the link from the locked text (SD405). SD611 does not report a link in an amendment, history or live section. SD901 reports it, so name the intent as plain text there. [types.md](types.md#links-between-types) gives the link directions. |
| SD701 | budget-soft | warning | A document is over its soft budget. |
| SD702 | budget-hard | error | A document is over its hard budget. Against the base, it is new, it was under the hard budget, or it grew by more than `[budgets] growth` characters (default 4,000). |
| SD703 | budget-grew | warning | A document over its soft budget grew against the base. It is under its hard budget, or over it by at most `[budgets] growth` characters. |
| SD704 | budget-hard-kept | warning | A document is over its hard budget, but it was over it in the base and did not grow. Or a split moved it out of a document over its hard budget (its section aliases name that document). In a root with `enforce-budgets = false`, every document over its hard budget gives SD704. |
| SD801 | requirement-keyword | warning | A requirement has no uppercase BCP 14 keyword. |
| SD802 | ears-pattern | warning | An `If` requirement has no `then`. |
| SD803 | sentence-length | warning | A requirement sentence is longer than `lint.max-words`. |
| SD804 | vague-term | warning | A requirement uses a term from the vague-term list. The word "many" after "how" ("how many") is a question about a number, so it is not vague. |
| SD805 | passive-voice | warning | A requirement uses a passive construction (heuristic). |
| SD806 | requirements-unrecognized | warning | A spec or contract has bare tokens that match a `legacy-keys` pattern, but no recognized requirement. Or spec-dna recognizes some requirements, but a list item or a table row starts with a `legacy-keys` token. This token is not the key of one of them. The legacy keys are probably in a syntax that spec-dna does not read, such as a table. |
| SD807 | requirement-colon-missing | warning | In a `requirements`, `interface` or `rule` document, a list item starts with a bold slug key and has a BCP 14 keyword. But no colon follows the key (`- **key** [1.1]: sentence`). Thus the item is not a requirement. |
| SD901 | link-direction | warning | A document of a layer X references a document of a layer Y. X is more stable than Y (X is reachable from Y by `moves-to` edges), and Y does not list X in `linked-from`. A reference is a Markdown link to a document, `[[id]]`, `[[id#key]]`, `depends-on` or `supersedes`. A document in no layer, a layer with no path to the other, an unresolved target, an image, a stub and a generated index give no diagnostic. SD901 gives one diagnostic for each document and target. A link of a locked decision that gives SD611 does not give SD901. |
| SD902 | layer-limit | warning | A layer has more documents than `max-docs`, or more body characters than `max-chars`. The documents of all roots count. A stub and a generated index do not count. The diagnostic is on `spec-dna.toml`, so the rules and the profile of a root do not apply. |
| SD903 | history-in-requirements | warning | A recognized requirement of a `rule`, `requirements` or `interface` document has a calendar date, `round <n>`, `iteration <n>`, or a phrase of `[lint] history-terms`. |
| SD904 | term-forbidden | warning | A recognized requirement of a `rule`, `requirements` or `interface` document uses a forbidden synonym of a `terms` document. The check is off when `language` is not English. |
| SD921 | truth-changed-without-change | warning | A requirement of a `rule`, `requirements` or `interface` document of a `forever` layer changed, was added or was removed, and no file under the folder of the `intent` layer changed. It runs only with `--base`, and only with a `[change] policy` other than `none`. In `strict` it is an error. |

The requirement lint (SD801 to SD805) is off in the `lite` and `standard` profiles. It is a warning in `strict`. The rule `lint` changes this.

The requirement lint (SD801 to SD805) and SD904 read English word lists. A `language` with a primary subtag other than `en` turns them off, also with `check --lint`. SD903 does not depend on `language`.

The [research matrix](research/matrix.md), section "Controlled-language lint", describes the lint warnings SD803 to SD805 and their sources.

## Checks of the layers and of the requirement text

The codes SD901 to SD904 and SD921 are new in 0.6.0. They are warnings in `standard` ([profiles.md](profiles.md)). The rule groups are `layers` (SD901 to SD904) and `changes` (SD921).

- SD901. The graph of the layers gives the direction. A document can link a layer that its own layer moves into. It can also link a layer that lists its own layer in `linked-from`. A rule that links an intent gives SD901, because an intent moves into a rule, and a finished intent is deleted. An accepted decision that links an intent in its locked text gives SD611, not SD901. In an amendment section, a link to an intent gives SD901. Name the intent as plain text there.
- SD902. `max-docs` and `max-chars` of a layer set the limits. The default graph gives limits to `foundation`, `rules`, `intent` and `pending`. A project that has no `[layers]` table gets these limits.
- SD903. The check reads the text of each recognized requirement. It ignores link destinations, autolinks, bare URLs, code spans and `[[id]]` references. It matches whole words, in any case. A date is `YYYY-MM-DD`, `YYYY/MM/DD`, `D.M.YYYY`, `D/M/YYYY`, `D Month YYYY`, `Month D, YYYY` or `Month YYYY`, with an English month name and a year from 1900 to 2099. A duration such as "30 days" is not a date. `used to` after a form of "be" ("SHALL be used to") is not history. The default phrases are "previously", "formerly", "no longer", "used to", "was changed", "before this change" and "after this change". `[lint] history-terms` replaces them. The key does not change the dates. The match does not use the meaning of the sentence. "no longer than 2 seconds" matches "no longer", and "round 2 decimals" matches `round <n>`. To avoid such a match, rewrite the sentence, or list your own phrases in `history-terms`.
- SD904. The table of terms has the header cells `Term` and `Forbidden synonyms` (in any order). spec-dna splits a cell of synonyms at `,` and `;`. A synonym that matches a whole phrase of the requirement text gives SD904. The target of the diagnostic is `<key>:<synonym>`.
- SD921. The check compares each requirement of the base with the requirement of the head that has the same key. It applies the aliases of `mv`. A requirement has changed if the hash of its text, without link destinations, is different. A key that has no match on the other side was added or removed. A move or a split with `spec-dna mv` keeps the keys through aliases, so it gives nothing. A split by hand with no alias gives 2 diagnostics (an added key and a removed key). SD921 gives one diagnostic for each document. It does not run without `--base`. The diagnostic is also off in the `lite` profile and with `[change] policy = "none"` (no rule turns it on).

## Code citations

SD606 and SD607 use the code citations of a document ([ADR 0001](adr/0001-information-model.md), amendment "code citations"). `context <code path>` uses the same citations. A document cites code in these ways:

- A link to a file that is not a document. The text of the link is not a second citation.
- A path in an inline code span or in prose, with an optional location: `src/loans.py`, `src/loans.py:42`, `src/loans.py#L3-L9`. A path is relative to the repository root or to the document. A file name or a path tail that only one file has also resolves, for example `loans.py`.
- A distinctive identifier in an inline code span:
  - a name with an inner `_` (`max_loans`);
  - a name with a lowercase letter before an uppercase letter (`loanBook`);
  - a called name with 4 characters or more (`renew()`).

  Words such as `status` or `HTTP` are not identifier citations. spec-dna also excludes a short list of common call names (`read()`, `main()`) and product names (`GitHub`, `macOS`).

An identifier matches a file if it occurs in the file as a whole word, even in a comment or a string. A citation that still resolves after the change, such as a file name after a move, gives no SD606.

The frozen text of an accepted decision cannot change (SD405). If this text cites a removed path or identifier, an amendment section of the decision clears SD606 and SD607 for that decision. The amendment section must cite new code. New code is a path that the head tree has (the rename target, if git found one), or an identifier that the code has. The base text of the decision does not cite it. A citation of removed code in the amendment section itself still gives a warning, because that text can change.

The rule `drift` sets the severity of SD606 and SD607 (`warning` in `standard`), or disables them. Only current documents give SD606 and SD607. These documents can cite code that does not exist: a superseded, retired or closed document, an incident, a research document, a roadmap, a list of questions, a spec and a plan.

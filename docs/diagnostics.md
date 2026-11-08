# Diagnostics

Every diagnostic has a stable code. A code keeps its meaning. spec-dna does not use a retired code again. An error fails `check` when the error is new against the base. A warning never causes a failure. `check --json` prints `code`, `name`, `severity`, `new`, `doc`, `path`, `line`, `target`, `message` and `fix` for each diagnostic. It also prints `documents`: the id, path, type, status, body characters and number of recognized requirements of each document.

Base-relative gating compares diagnostics by their fingerprint (code, document id, target). For a moved document, the comparison uses its old id, which the document keeps as an alias. Codes in the 400 range and SD202, SD605, SD702 and SD703 compare the base tree with the head tree. Thus, these diagnostics are always new. SD704 is new only when the root does not enforce budgets and the document is new or grew.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD101 | unresolved-reference | error | A `[[id]]`, `[[id#key]]`, `supersedes` or `depends-on` value names no document, section or requirement. |
| SD102 | unresolved-link | error | A Markdown link or image points to a path that is not in the git tree. |
| SD103 | evidence-missing | error | A `verified-by` path is not in the git tree. |
| SD104 | ambiguous-reference | error | A reference names an alias that more than one document has. |
| SD201 | duplicate-id | error | Two documents have the same id or alias. |
| SD202 | id-removed | error | An id of the base tree is gone, and no document keeps it as an alias. You can delete a `done` or `rejected` change, or a note. |
| SD203 | key-format | error | A requirement key in a sentence with a BCP 14 keyword is not a slug and not a legacy key. |
| SD204 | key-duplicate | error | A requirement key occurs two times in one document. |
| SD301 | front-matter-syntax | error | The front matter is not in the supported YAML subset. |
| SD302 | front-matter-field | error | A known field has a value of the wrong kind. |
| SD303 | type-invalid | error | The `type` is unknown, or it does not agree with a single-type root. |
| SD304 | status-invalid | error | After the root's `status-map` applies, the status is missing or the type does not allow it. |
| SD305 | section-missing | error | A document has no section with a heading from the root's `required-sections` for its type. The default applies only to a decision: `Decision Outcome` or `Decision`. |
| SD401 | status-transition | error | A status moved backwards or left a final state. |
| SD402 | superseded-without-successor | error | A superseded document has no active (or accepted) successor that names it in `supersedes`. |
| SD403 | supersede-cycle | error | The `supersedes` chain returns to its start. |
| SD404 | depends-on-inactive | error | `depends-on` names a superseded or retired document. |
| SD405 | decision-frozen | error | Text above `## Amendments` changed after the decision became accepted. |
| SD501 | adapter-modified | error | A generated adapter block has a manual edit. |
| SD502 | adapter-outdated | error | A generated adapter is different from the adapter that this version generates. |
| SD601 | token-unresolved | warning | A bare `PREFIX-NUMBER` token with a configured legacy prefix names no document. |
| SD602 | anchor-missing | warning | A Markdown link fragment names no heading or requirement of its target. |
| SD603 | cross-repo-unpinned | warning | spec-dna cannot check a `<repo>:<id>` reference without a pinned index. |
| SD604 | depends-on-deprecated | warning | `depends-on` names a deprecated document. |
| SD605 | amendment-outcome | warning | New amendment text states a requirement, a BCP 14 keyword or a status word. |
| SD701 | budget-soft | warning | A document is over its soft budget. |
| SD702 | budget-hard | error | A document is over its hard budget, and against the base it is new, it was under the hard budget, or it grew. |
| SD703 | budget-grew | warning | A document over its soft budget (but not over its hard budget) grew against the base. |
| SD704 | budget-hard-kept | warning | A document is over its hard budget, but it was over it in the base and did not grow. In a root with `enforce-budgets = false`, every hard overage is SD704. |
| SD801 | requirement-keyword | warning | A requirement has no uppercase BCP 14 keyword. |
| SD802 | ears-pattern | warning | An `If` requirement has no `then`. |
| SD803 | sentence-length | warning | A requirement sentence is longer than `lint.max-words`. |
| SD804 | vague-term | warning | A requirement uses a term from the vague-term list. |
| SD805 | passive-voice | warning | A requirement uses a passive construction (heuristic). |
| SD806 | requirements-unrecognized | warning | A spec or contract has bare tokens that match a `legacy-keys` pattern, but no recognized requirement. The legacy keys are probably in a syntax that spec-dna does not read, such as a table. |

The [research matrix](research/matrix.md), section "Controlled-language lint", describes the lint warnings SD803 to SD805 and their sources.

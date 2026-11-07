# Diagnostics

Every diagnostic has a stable code. A code keeps its meaning; a retired code is not reused. Errors fail `check` when they are new against the base; warnings never fail it. `check --json` prints `code`, `name`, `severity`, `new`, `doc`, `path`, `line`, `target`, `message` and `fix` for each one.

Base-relative gating compares diagnostics by the fingerprint (code, document id, target). A moved document is compared under its old id, which it keeps as an alias. Codes in the 400 range and SD202, SD605 and SD703 compare the base tree with the head tree, so they are always new.

| Code | Name | Severity | Meaning |
| --- | --- | --- | --- |
| SD101 | unresolved-reference | error | A `[[id]]`, `[[id#key]]`, `supersedes` or `depends-on` value names no document, section or requirement |
| SD102 | unresolved-link | error | A Markdown link or image points to a path that is not in the git tree |
| SD103 | evidence-missing | error | A `verified-by` path is not in the git tree |
| SD104 | ambiguous-reference | error | A reference names an alias shared by several documents |
| SD201 | duplicate-id | error | Two documents share an id or alias |
| SD202 | id-removed | error | An id of the base tree is gone and was not kept as an alias. Deleting a `done` or `rejected` change, or a note, is allowed |
| SD203 | key-format | error | A requirement key is neither a slug nor a legacy key, in a sentence with a BCP 14 keyword |
| SD204 | key-duplicate | error | A requirement key appears twice in one document |
| SD301 | front-matter-syntax | error | The front matter is not in the supported YAML subset |
| SD302 | front-matter-field | error | A known field holds the wrong kind of value |
| SD303 | type-invalid | error | Unknown `type`, or one that disagrees with a single-type root |
| SD304 | status-invalid | error | A missing status, or one the type does not allow |
| SD305 | section-missing | error | A decision has no `Decision Outcome` (or `Decision`) section |
| SD401 | status-transition | error | A status moved backwards or left a final state |
| SD402 | superseded-without-successor | error | A superseded document has no active (or accepted) successor that names it in `supersedes` |
| SD403 | supersede-cycle | error | The `supersedes` chain returns to its start |
| SD404 | depends-on-inactive | error | `depends-on` names a superseded or retired document |
| SD405 | decision-frozen | error | Text above `## Amendments` changed after the decision was accepted |
| SD501 | adapter-modified | error | A generated adapter block was edited by hand |
| SD502 | adapter-outdated | error | A generated adapter differs from what this version generates |
| SD601 | token-unresolved | warning | A bare `PREFIX-NUMBER` token with a configured legacy prefix names no document |
| SD602 | anchor-missing | warning | A Markdown link fragment names no heading or requirement of its target |
| SD603 | cross-repo-unpinned | warning | A `<repo>:<id>` reference cannot be checked without a pinned index |
| SD604 | depends-on-deprecated | warning | `depends-on` names a deprecated document |
| SD605 | amendment-outcome | warning | New amendment text states a requirement, a BCP 14 keyword or a status word |
| SD701 | budget-soft | warning | A document is over its soft budget |
| SD702 | budget-hard | warning | A document is over its hard budget |
| SD703 | budget-grew | warning | An over-budget document grew against the base |
| SD801 | requirement-keyword | warning | A requirement has no uppercase BCP 14 keyword |
| SD802 | ears-pattern | warning | An `If` requirement has no `then` |
| SD803 | sentence-length | warning | A requirement sentence is longer than `lint.max-words` |
| SD804 | vague-term | warning | A requirement uses a term from the vague-term list |
| SD805 | passive-voice | warning | A requirement uses a passive construction (heuristic) |

The lint warnings SD803 to SD805 and their sources are described in the [research matrix](research/matrix.md), section "Controlled-language lint".

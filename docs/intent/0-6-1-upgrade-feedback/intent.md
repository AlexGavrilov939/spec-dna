---
status: accepted
class: routine
---
# 0.6.1 upgrade feedback

## Intent

Fix the problems that the first upgrade of a consumer repository from 0.5 to 0.6.0 found, and release the fixes as 0.6.1. The owner wrote the list. It has 13 fixes and 3 additions:

- `mv`: allow a move to the path of an alias of the moved document, remove the `type` and `status` lines that the new place makes wrong, and remove the folders that the move leaves empty.
- `log`: a move of an `intent.md` out of the intent layer is not a closed change.
- `layer add --types`: add the types to the `types` list, and print the next steps.
- `upgrade` and `init --reconfigure`: add `intent`, `spec` and `plan` to `types` when a change policy needs them, keep the right `high-risk-layers`, and remove the empty folders.
- Generated agent and skill files pass markdownlint, and a test checks each one.
- Docs: the files that SD503 counts, and a routine change with no files.
- `check --all` names a link in locked text whose target was removed.
- The default graph lets requirements and decisions link to the `pending` layer.
- A key `[change] base` for a repository with no remote, and a local-only path in the upgrade guide.
- `upgrade --layout` maps common file names to top files, and `--areas <file>` moves documents into areas.
- A benchmark workflow for GitHub Actions. Its first numbers go into the CHANGELOG.

Each fix has a test that fails before the fix.

The class is `routine`. The documents of this repository hold no requirement, so the change edits no requirement. The brief of the owner is the acceptance of this intent. `docs/design/0.6.1.md` (it comes with the plan) gives the decisions.

## Why now

The owner found these problems in the first real upgrade. Each one stops or slows the upgrade of the next repository.

## Out of scope

- The pin of `spec-dna.toml` of this repository. It moves in its own pull request after the release (SD503).
- A change to the 0.5 reader of `upgrade`.
- A new diagnostic code, a new dependency, or a new cache format.
- Any text, path or name of a consumer repository.

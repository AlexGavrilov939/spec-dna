---
status: proposed
---

# 0022. The plan of the import

## Context and Problem Statement

[ADR 0014](0014-import.md) gives `spec-dna import speckit|openspec`. Before the build of slice 6 of 0.7.0, it left 4 points open:

- Which sources move with `git mv`, and which become new files.
- How `--keep research` keeps a folder of an archived change: as one file, or as one document for each file.
- Where a target goes when the layer graph has no layer for its type.
- What the import does with a source line that the target rejects. A probe with the binary of 0.6.1 gave SD203 for an `SC-nnn` line or an entity line with a BCP 14 word.

ADR 0014 is at its hard budget. Thus this ADR holds the rules.

## Considered Options

1. Write each target as a new file, and merge the Markdown files of an archived change into one `research` document.
2. Move each source that gives one target with `git mv`, write the other targets, and move each Markdown file of an archived change as one `research` document.

## Decision Outcome

Option 2. Section 15 of [the design](../design/0.7.0.md#15-deviations-during-the-build) gives the detail of D21 to D27.

- A source that gives one target moves with `git mv`, and the plan then edits it in place, as `upgrade` does: the constitution, a done Spec Kit feature and an OpenSpec domain spec. A moved file that was a document keeps its old id as an alias.
- The files of an open Spec Kit feature or of an open OpenSpec change are the sources of written targets. The sources stay, and the report prints one `git rm` command for each folder.
- A target goes to the folder of the layer of its type, in the first root that can hold the type. With no such layer, `import` stops with exit code 2 and names `spec-dna layer add`, as for `--keep research`.
- `--keep research` moves each Markdown file with `git mv` to `<folder of the research layer>/imported/<source folder>/`, as one `research` document. A file that is not Markdown stays in place, and the report names it.
- The import changes no source sentence. A line that the target rejects stops the proof, and the message names the line and 2 fixes.
- The import edits each relative link of a moved or written file as `mv` does. A link that is broken in the source stays broken, and the proof stops on the new SD102.

### Rejected

- Option 1: a write loses the history that `git mv` keeps, and a merge of files is a write.
- An edit of a source sentence to avoid a diagnostic: the import keeps the text of the source. The Spec Kit template words an `SC-nnn` line and an entity line with no BCP 14 word.

### Consequences

- Good: git keeps the history of a moved source.
- Good: the proof compares the text of the source, with no edit.
- Bad: an `SC-nnn` line or an entity line with a BCP 14 word stops the proof. The person adds the pattern to `legacy-keys`, or removes the bold key in the source.
- Bad: `git mv` with a large edit, such as the constitution, can fall below 50% similarity, so git loses the rename. The report says when an edit is large.
- Bad: with `strict`, SD608 is an error for each imported active requirement. With `--skip-open`, SD917 and SD921 are errors too. The report names them.

## Amends

ADR 0014 is accepted and locked, so this ADR does not edit it. It closes the open points of ADR 0014, "Command", "Proof" and "Archive and other files". The outcome of ADR 0014 does not change.

## Amendments

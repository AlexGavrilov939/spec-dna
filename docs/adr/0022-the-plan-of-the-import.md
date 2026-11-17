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
- Bad: `git mv` with a large edit can fall below 50% similarity, so git loses the rename. The report says when an edit is large. The constitution is a small case. The real loss is each OpenSpec domain spec, because each requirement heading becomes a list item.
- Bad: with `strict`, SD608 is an error for each imported active requirement. With `--skip-open`, SD917 and SD921 are errors too. The report names them.

## Amends

ADR 0014 is accepted and locked, so this ADR does not edit it. It closes the open points of ADR 0014, "Command", "Proof" and "Archive and other files". The outcome of ADR 0014 does not change.

## Amendments

### 2026-11-18: the build of slice 6

The check of the build gave these rules.

- A root of the type of a target takes the target first, and the first mixed root takes it if no root has the type. If a layer holds a type and no root can hold it (the root that `init` writes for Spec Kit has the type `requirements`), the first run adds a root of the type `mixed` (`docs`) to `spec-dna.toml`.
- A source file that is not valid UTF-8 stops the plan, because a copy would change a character with no sign in the proof.
- A target that differs from an existing file only in case stops the plan before a write.
- No line of a source with a BCP 14 word may vanish. The proof checks that a target holds it, else it stops. Another line that no target holds is listed as `dropped` in the report. The headings and the task lines that the reader rewrites are no loss. The reader keeps a line that is no task as plain text.
- The report names each file that the plan wrote from a source. It names a folder for `git rm` only if the folder holds no kept file.
- A source file that is a document of the project (for example a file of the folder `specs/` when `init` made it a root) is written, not moved. The plan then writes the line `moved-to` in its front matter and gives its old id as an alias to its first target. The source keeps no id, so there is no duplicate id, and its removal with `git rm` is no lost id (SD202).
- Known limits (a build of 2026-11-18): the check of the lines compares text as a part of a target line, so a repeated line can hide a lost one; a text of a task that the reader moves to `Files` is not seen; a second run on a project whose tool folder is gone stops with exit code 3. `docs/commands.md`, section `import`, lists them.

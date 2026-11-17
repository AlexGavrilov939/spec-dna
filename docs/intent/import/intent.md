---
status: draft
class: higher-risk
---
# Import of a Spec Kit or OpenSpec project

## Intent

Ship `spec-dna import speckit|openspec`, as slice 6 of [the design](../../design/0.7.0.md) says (D21 to D27, section 15) and [ADR 0014](../../adr/0014-import.md) gives the reasons. A project that uses Spec Kit 1.4.1 or OpenSpec 2.0.0 then moves into the layout of spec-dna in one reviewable diff. A proof shows that each requirement sentence and each id stayed.

The change adds these items:

- D21: the command, `--dry-run`, `--skip-open`, `--areas`, `--keep none|research`, `--json` and the exit codes 0 to 4. The first run writes only `spec-dna.toml` when the plan needs a legacy key, an area or a type.
- D22: the proof of 5 steps and the count of the scenarios. A failed step stops the write with exit code 1.
- D23: the constitution, the done features and the open features of Spec Kit.
- D24: the domain specs, the open changes and the delta specs of OpenSpec.
- D25: an unknown layout stops with exit code 3 and names the file, the rule and the supported versions.
- D26, D27: the archive and the extra files stay in place by default, and a change with `RENAMED` stays in place.
- 2 fixture projects in the public layouts of the 2 versions, with text in the words of spec-dna and no consumer data.

## Why now

A team that uses one of these tools moves each file by hand today, and nothing proves that a sentence or an id survived. The design and ADR 0014 are accepted, and slice 1 (`tree-binary`) is merged.

## Out of scope

- An import of another tool, or of another major version of these 2 tools.
- `RENAMED`, `.openspec.yaml`, `openspec/schemas/` and the extensions of Spec Kit (D27).
- A guide page for the import. The section `import` of `docs/commands.md` is the reference, and slice 9 can add a guide.
- The deletion of a source file and a commit. The command prints the commands, and the person runs them.
- The text of the guard message of `mv`, `split` and `finish`.

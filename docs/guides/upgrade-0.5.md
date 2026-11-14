# Upgrade from 0.5 to 0.6

Version 0.6.0 renames document types, statuses, configuration keys and harness roles, and it keeps no old name as input. `spec-dna upgrade` moves a project to 0.6 in one step and proves that no id and no requirement text changed. This guide gives the procedure. [commands.md](../commands.md) gives the options and the exit codes.

## What changes

- **Types.** `spec` is now `requirements`, `contract` is `interface`, `invariant` is `rule`, `guide` and `runbook` are `procedure`, and `note` and `reference` are `research`. A `change` is now an `intent`. [types.md](../types.md#types-of-05) gives the full table.
- **Changes.** Each `proposed`, `accepted` or `rejected` change moves to `intent/<slug>/intent.md`, because the type `intent` is valid only in the `intent` layer. Its status becomes `draft`, `accepted` or `closed`. A `done` change becomes a `research` document.
- **Decisions.** The word "frozen" is now "locked". SD405 checks the same text as before.
- **Context.** The order of `context` follows the levels of the layers, so `foundation` comes before `rules`. `context` counts incidents and research documents in one line, and does not list them.
- **Harness.** The role `reviewer` is now `verifier`, and its file `checker.md` is now `verifier.md`. The role `spec-writer` no longer exists, and the stage skills replace it.
- **Diagnostics.** The code names of SD405 and SD611 changed. The codes stay. A `--json` consumer must read `code`, not `name`.

## Before you start

- The project pins spec-dna 0.4.0 or later. For an older pin, first upgrade to 0.4.2.
- The working tree is clean. `upgrade` refuses to run on uncommitted edits.
- Install the 0.6 binary ([quickstart.md](../quickstart.md#install)). Download `spec-dna-pin.toml` of the release. Verify it with `gh attestation verify spec-dna-pin.toml --repo AlexGavrilov939/spec-dna`.

## Run the upgrade

```sh
spec-dna upgrade --dry-run --pin-file spec-dna-pin.toml
spec-dna upgrade --pin-file spec-dna-pin.toml
```

`--dry-run` prints the diff of each file and writes nothing. Without it, `upgrade` does these steps:

1. It reads the old configuration. It makes the whole plan in memory.
2. It renames the keys and the types in `spec-dna.toml`. It adds the layers that the types in use need, and `[change] policy = "none"`. It moves the pin to the version of the binary, with the sums of `--pin-file`.
3. It rewrites each `type:` and `status:` that changed. It marks a former `note` or `done` change with `was-type`.
4. It renames the harness roles. It deletes a generated file under its old name if nobody edited it. It keeps an edited file, and `check` reports it (SD501).
5. It moves each `proposed`, `accepted` or `rejected` change to `intent/<slug>/intent.md` with `git mv`, and writes the old id into `aliases`.
6. It proves the result (see [What the proof checks](#what-the-proof-checks)). If the proof fails, it writes nothing.
7. It prints a report.

`upgrade` never commits and never deletes a document. A second run changes nothing.

The folders of the other types stay where they are. A root maps each folder to its new type. To move them to the 0.6 folders too (`contracts/` to `interfaces/`, for example), add `--layout`. It moves them with `git mv` and writes each old id into `aliases`.

## What the proof checks

- Each old id resolves in the new tree, as an id or as an alias.
- The hash of each requirement and each section is the same as before.

If one of these checks fails, `upgrade` exits with code 1 and writes nothing.

## Read the report

The report lists these items, among others:

- the changes that no document cites;
- the decisions with no `Status` section;
- the harness files that `upgrade` kept;
- the changes of behavior from [What changes](#what-changes).

[commands.md](../commands.md) gives the full report.

`upgrade` deletes no document. The report proposes each `done` or `rejected` change that no document cites for deletion. The owner approves each batch, and a later pull request deletes it.

## Open the first pull request

Commit the result. Open one pull request. The base of this pull request has a pin before 0.6.0, so `check` first runs the plan of `upgrade` on the base tree in memory. Then it compares the 2 trees. The gate stays active.

One pull request can hold the configuration and the documents. A later pin change, such as 0.6.0 to 0.6.1, needs its own pull request (SD503).

## After the upgrade

An upgraded project has the policy `none`: it has the layers and their checks, but no change process. The harness keeps its roles, and the new parts (the stage skills, `REVIEW.md` and the hook `protect-tests`) are off. The project has no evals.

To enable any of them, run `spec-dna init --reconfigure`. Choose the change policy, the harness parts and the evals. [The change guide](change.md) gives the change process.

## Undo the upgrade

Revert the upgrade before the merge, if possible. After the merge, the revert needs 2 pull requests. The reason is SD503: the old binary rejects a pull request that changes the configuration and the documents together. In the second pull request, `check` also reports SD202 for each moved change, and for each moved id if you used `--layout`.

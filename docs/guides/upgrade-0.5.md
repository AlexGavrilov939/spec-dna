# Upgrade from 0.5 to 0.6

Version 0.6.0 renames document types, statuses, configuration keys and harness roles, and it keeps no old name as input. `spec-dna upgrade` moves a project to 0.6 in one step and proves that no id and no requirement text changed. This guide gives the procedure. [commands.md](../commands.md#upgrade) gives the options and the exit codes.

## What changes

- **Types.** `spec` is now `requirements`, `contract` is `interface`, `invariant` is `rule`, `guide` and `runbook` are `procedure`, and `note` and `reference` are `research`. A `change` is now an `intent`. In the key `types` of `spec-dna.toml`, `change` becomes `intent`, `spec` and `plan`, because the intent layer holds the 3 types together. [types.md](../types.md#types-of-05) gives the full table.
- **Changes.** Each `proposed`, `accepted` or `rejected` change moves to `<root>/intent/<slug>/intent.md`, because the type `intent` is valid only in the `intent` layer. Its status becomes `draft`, `accepted` or `closed`. A `done` change stays in place and becomes a `research` document.
- **Decisions.** The word "frozen" is now "locked". SD405 checks the same text as before.
- **Context.** The order of `context` follows the levels of the layers, so `foundation` comes before `rules`. `context` counts incidents and research documents in one line, and does not list them.
- **Harness.** In the harness of the Claude Code adapter, the role `reviewer` is now `verifier`, and its file `checker.md` is now `verifier.md`. The role `spec-writer` no longer exists, and the stage skills replace it.
- **Diagnostics.** The code names of SD405 and SD611 changed. The codes stay. A `--json` consumer must read `code`, not `name`.

## Before you start

- The project pins spec-dna 0.4.0 or later. For an older pin, first move the project to 0.4.2 with the 0.4.2 binary and `spec-dna init`.
- The repository has a commit, and the working tree is clean. `upgrade` reads `HEAD`, and it refuses a staged change, an unstaged change and an untracked file that git does not ignore.
- Install the 0.6 binary ([quickstart.md](../quickstart.md#install)). Download `spec-dna-pin.toml` of the release.
- Save `spec-dna-pin.toml` outside the repository, for example in the folder above it. A file inside the repository makes the tree not clean. Verify it with `gh attestation verify ../spec-dna-pin.toml --repo AlexGavrilov939/spec-dna`.

A project that has the table `[spec-dna-sha256]` needs this file. A project with no table can run `upgrade` without it. With the file, it gets the table.

## Run the upgrade

```sh
spec-dna upgrade --dry-run --pin-file ../spec-dna-pin.toml
spec-dna upgrade --pin-file ../spec-dna-pin.toml
```

`--dry-run` prints the diff of each file and writes nothing. Without it, `upgrade` does these steps:

1. It reads the old configuration. It makes the whole plan in memory.
2. It renames the keys and the types in `spec-dna.toml`, and it keeps each comment. It adds the layers that the types in use need, and `[change] policy = "none"`. It moves the pin to the version of the binary, with the sums of `--pin-file`.
3. It writes the keys that keep a 0.5 document as it was. One of them is `live-sections = { decision = [] }`, so that no 0.5 decision gets a live section by default.
4. It rewrites each `type:` and `status:` that changed. It marks a former `note` or `done` change with `was-type`.
5. It renames the harness roles. It deletes a generated file under its old name if nobody edited it. It keeps an edited file, and `check` reports it (SD501).
6. It moves each `proposed`, `accepted` or `rejected` change to `<root>/intent/<slug>/intent.md` with `git mv`, and writes the old id into `aliases`. It removes each folder that a move left empty. A folder with a file stays, and so does the root folder. `--dry-run` removes no folder.
7. It proves the result (see [What the proof checks](#what-the-proof-checks)). If the proof fails, it writes nothing. Then it prints a report.

`upgrade` never commits and never deletes a document. A second run after the commit changes nothing. A second run before the commit gives exit code 1, because the tree is not clean.

The folders of the other types stay where they are. A root maps each folder to its new type. To move the folders of a mixed root to the 0.6 folders too (`contracts/` to `interfaces/`, for example), add `--layout`. It moves them with `git mv` and writes each old id into `aliases`. A root of one type keeps its path.

### If `upgrade` stops

`upgrade` stops before it writes. Each stop names the cause:

- A hand edit (exit code 3). `spec-dna.toml` has a form that `upgrade` cannot edit without a loss of text, such as a budget with dotted keys. The message gives the form to write. Edit the file with the 0.5 names, commit the change, and run `upgrade` again. If the default branch has the same form, make this edit in a pull request of its own first and merge it. The first pull request after the upgrade plans the upgrade of its base, so a form that `upgrade` cannot edit in the base stops `check --base` with exit code 3. The same rule holds for a move stop of a change in the base.
- A move (exit code 1). Two changes have one slug, or a target path exists. The message names each path. Rename a file, commit the change, and run `upgrade` again.
- A file in the place of a folder, or a symbolic link in a path (exit code 1). Move or delete it, commit the change, and run `upgrade` again.
- A failed proof (exit code 1). The message names each failure. A pointer of `[conventions]` that names a document by its path breaks with `--layout`. Write the pointer as an id, and commit the change.

## What the proof checks

- Each old id, alias and requirement key resolves in the new tree to the same document or requirement.
- The hash of each requirement and each section is the same as before. The locked text of each decision is the same.
- Each link of a document goes to the same file as before.
- Each document has the 0.6 type that the mapping gives.
- The new configuration parses, and the layer graph has no cycle.
- The upgrade adds no error of the rule groups `references` (SD101, SD102, SD104, SD106) and `ids` (SD201 to SD204).

If one of these checks fails, `upgrade` exits with code 1 and writes nothing.

## Read the report

The report lists these items, among others:

- each `done` or `rejected` change that no document references;
- the decisions with no `Status` section;
- each locked decision that cites a migrated change;
- the harness files that `upgrade` kept, and each ended role with its model;
- the links that `upgrade` did not rewrite;
- each edit of `spec-dna.toml` beyond the pin, for a 0.6 project (see [Upgrade a 0.6 project](#upgrade-a-06-project));
- the count of each error code of `check --all` on the new tree;
- the changes of behavior from [What changes](#what-changes). A 0.6 project gets no such list.

[commands.md](../commands.md#upgrade) gives the full report.

`upgrade` deletes no document. The report proposes each `done` or `rejected` change that no document references for deletion. The owner approves each batch, and a later pull request deletes it.

`upgrade` rewrites the links of the documents of the roots. It does not rewrite a link in a file that is not a document. Examples are `README.md`, an instruction file and a file outside the roots. The report counts these links and lists the first 5. Search for the old paths of the moved files, and fix the links. Do the same for a link to a folder that the report lists.

The proof refuses a new error of the groups `references` and `ids`. Another group can report an error for a 0.5 form that 0.6 checks. An example is SD304 for a status word that the type does not know. The base plan of `check` gives the same error for the base, so it is not new.

## Commit the result

`upgrade` stages the moves with `git mv` and leaves the other changes unstaged. It prints the `git add` command for the result after the report. Run that command, and commit.

`git log --follow` can lose the history of a small moved change, because the commit moves the file and edits its front matter and links. A squash merge keeps one commit, so it keeps no history anyway. Commit the whole result at once.

## Open the first pull request

Open one pull request. The base of this pull request pins a version before 0.6.0. Thus `check` first plans the upgrade of the base tree in memory. The plan is the plan of `upgrade` without `--layout`.

Then `check` compares the head with that plan. The gate stays active, and the 0.5 documents of the base give no new diagnostic. A moved document keeps its old id as an alias, so the comparison also works after `--layout`. Run `spec-dna check --base origin/main` to see the result.

One pull request can hold the configuration and the documents, because SD503 does not apply when the base pins a version before 0.6.0. A later change of the configuration keeps SD503. A later pin change, such as 0.6.0 to 0.6.1, needs its own pull request.

The change checks (SD910 to SD921) need a lifecycle base from 0.6.0. If that base pins an earlier version, `check` prints a note on stderr and runs the checks of one tree. An upgraded project has the policy `none`, so it gets no note.

A branch that started before the upgrade has 0.5 documents. Merge the default branch into it, and run `spec-dna upgrade` again. Commit the result. Then the base of the branch pins 0.6.0.

If the base pins a version before 0.4.0, `check` exits with code 3. It asks you to move the base to 0.4.2 first.

`status`, `next` and `finish` read a lifecycle base that pins a version before 0.6.0 with the rules of 0.6. `check` does not.

## A repository with no remote

A repository with no remote has no `origin/HEAD`. The change commands then have no base, unless you pass `--base` or set `[change] base`. Use these steps for the upgrade:

1. Run `spec-dna upgrade` on a branch, and commit the result.
2. Run `spec-dna check --base main`. The base pins a version before 0.6.0, so `check` plans the upgrade of that tree in memory, as for a pull request ([Open the first pull request](#open-the-first-pull-request)).
3. Merge the branch into `main`.
4. On a new branch, add the key `base` to the table `[change]`, and commit it in a commit of its own. `upgrade` has written the table with `policy = "none"`.

```toml
[change]
policy = "none"
base = "main"
```

The key `base` names a local branch. Without `--base` and without `origin/HEAD`, the change commands use the merge base of this branch and `HEAD`. With the policy `none`, only `finish` reads the key. `status`, `next` and the change checks of `check` read it after you set a policy ([After the upgrade](#after-the-upgrade)). [config.md](../config.md#the-table-change) gives the rules.

Step 4 needs a commit of its own, because the key edits `spec-dna.toml`. If the same commit changes another file, `check --base main` gives SD503. If the commit has `spec-dna.toml` only, it gives no SD503.

A binary of 0.6.0 does not know the key, and it stops with exit code 3. Pin 0.6.1 or later first, in a pull request of its own, and then set the key.

With a binary of 0.6.0, or before you set the key, you can make `origin/HEAD` by hand:

```sh
git symbolic-ref refs/remotes/origin/HEAD refs/heads/main
```

This ref is local to one clone. Each clone needs the command. It is not in a commit. If `origin/HEAD` exists, it has priority over the key.

## After the upgrade

An upgraded project has the policy `none`: it has the layers and their checks, but no change process. The harness keeps its roles, and the new parts (the stage skills, `REVIEW.md` and the hook `protect-tests`) are off. The project has no evals.

To enable any of them, run `spec-dna init --reconfigure`. Choose the change policy, the harness parts and the evals. The command writes the table `[change]` again and does not keep the key `base`. In a repository with no remote, add the key again in a commit of its own. [The change guide](change.md) gives the change process.

## Upgrade a 0.6 project

A project that pins 0.6.0 or later runs the same command. `upgrade` changes the pin, and it makes 2 more edits of `spec-dna.toml`. Both add values only, and the report names each edit:

- If the file has the key `types`, the change policy is not `none`, and the list lacks `intent`, `spec` or `plan`, `upgrade` adds the absent types. A policy needs the 3 types. With the policy `none`, the list stays.
- If `linked-from` of `[layers.pending]` is exactly `["intent"]`, `upgrade` adds `requirements` and `decisions`, so that a requirement or a decision can link the roadmap and the question list with no SD901. Another value stays.

These edits change `spec-dna.toml`. A new `types` list also changes the generated skill, which lists the types of the project. SD503 does not count the skill, so the pull request of the pin can hold these edits. If the `types` list has many lines, `upgrade` does not edit it. It prints a warning that names the types to add by hand.

If you set a change policy later with `spec-dna init --reconfigure`, and the `types` list lacks one of the 3 types, `init` prints a warning. It does not edit the key.

## Undo the upgrade

Revert the upgrade before the merge, if possible. Before the commit, run `git reset --hard` and `git clean -fd`. After the merge, the revert needs 2 pull requests. The reason is SD503: the old binary rejects a pull request that changes the configuration and the documents together. In the second pull request, `check` also reports SD202 for each moved change, and for each moved id if you used `--layout`.

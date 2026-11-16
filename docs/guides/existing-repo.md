# Adopt spec-dna in an existing repository

This guide adds spec-dna to a repository that already has documents. spec-dna does not move or edit them. The gate reports only the problems that a change adds, so the old problems do not block your work. [Quickstart](../quickstart.md) gives the install.

## Before you start

- Install the spec-dna binary and git ([quickstart.md](../quickstart.md#install)).
- Start from a clean working tree on a new branch.
- Find the folders that hold your documents: specs, decisions, guides, runbooks, post-mortems and research.

## Run init on your folders

```sh
spec-dna init
```

`init` detects the document folders, their types, the tests, the agent files, CI and the hooks. Then it asks its questions with the detected answers as defaults. These answers have the largest effect on an existing repository:

- **Profile.** In `lite`, only the structural errors fail `check`, such as an invalid front matter or an edit of an accepted decision. Most other codes are warnings, and some are off. [profiles.md](../profiles.md) gives each code. Use `lite` for the first weeks. Move to `standard` later.
- **Layer graph.** `init` proposes `small` for fewer than 30 documents and `default` for 30 documents or more. The small graph has no layer for `interface`, `procedure` and `research`, so `init` writes no root for a folder of these types. Choose `default` to keep these folders.
- **Change policy.** Answer `none` to start with no change process. The layers and their checks still work.
- **Roots.** `init` shows each detected folder and its type. Accept it, drop it or edit it, for example `type=requirements status=active`. A root of one type gives that type to each document in it, so no document needs a `type:` field. Examine the proposal: `init` can propose one root for all of `docs/`, with one default type. Give each folder its own root, as in [example 2](../../examples/existing-repo/README.md).
- **Budgets.** The question shows the number of documents over the hard budget at each level. `relaxed` doubles the budgets.

`init` shows a preview of each file. It writes nothing before you confirm. [init.md](../init.md) gives each question.

Your folders keep their names. A root maps a folder to a type, for example `adr/` to `decision` and `runbooks/` to `procedure`. Old numbers such as `ADR-12` continue to resolve, if you set `legacy-ids` for the root. Old status words map to the statuses of spec-dna with `status-map`. [config.md](../config.md) gives these keys.

## Take the baseline

```sh
spec-dna check --all
```

`check --all` shows each problem in the documents once. You do not have to fix them. Without `--all`, `check` reports only the problems that are new against the base. Fix the old problems when you edit a document for another reason.

Commit `spec-dna.toml` and the files that `init` wrote. `init` prints the `git add` command.

## Add the gate to CI

Add `spec-dna check --base origin/<main branch>` to CI, or let `init` write the job. [quickstart.md](../quickstart.md#ci) gives the job and the hook.

## Write new requirements

Write each new requirement as a list item with a bold key, and cite the code that implements it:

```md
- **sensor-timeout**: When a moisture sensor times out, the controller SHALL close the valve of its bed. The check is in `src/valves.rs`.
```

Link a test to the requirement with a `covers:` tag in a comment. `spec-dna show <document id>` lists the tests of each requirement.

You do not have to convert the old documents. spec-dna reads old text as sections, and it checks their links, ids and citations. A requirement needs the list item form, or a `legacy-keys` pattern of its root, such as `R-{n}`.

## Move documents one by one

To move a document into a layer folder, use `spec-dna mv`. It moves the file with git and writes the old id into `aliases`, so each old reference continues to resolve. The first argument is a document id. The second is a document id or a path:

```sh
spec-dna mv valves docs/requirements/valves/closing.md
```

`spec-dna mv <document id>#<key> <document id>` moves one requirement. `spec-dna history` moves the amendments of a long decision into a research document. The small graph has no `research` layer. Run `spec-dna layer add research --types research` first, then `spec-dna init`. [commands.md](../commands.md#mv) gives the options.

## Turn on the change process

When the team is ready, run `spec-dna init --reconfigure`. Choose the policy `three-class` or `two-class`. From then on, a change of a requirement needs an intent folder. In the profiles `standard` and `strict`, `check` reports SD921 for a change with no intent folder. [The change guide](change.md) gives the steps.

The intent folder must be in a root of the type `mixed`. If each root of your repository has one type, `spec-dna new intent` stops with exit code 2. Add a mixed root for the changes to `spec-dna.toml`:

```toml
[[roots]]
path = "docs/work"
type = "mixed"
```

Then `spec-dna new intent <slug>` writes `docs/work/intent/<slug>/intent.md`.

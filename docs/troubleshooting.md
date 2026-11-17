# Troubleshooting

This page gives the cause and the fix of the frequent stops of a new user. Each section starts with the message. [Diagnostics](diagnostics.md) gives each diagnostic code, and [Commands](commands.md#exit-codes) gives the exit codes.

## Install

### install: the folder does not exist

```text
install: /Users/you/.local/bin/INS@aoSeXv: No such file or directory
```

The folder `~/.local/bin` does not exist. The message above is the message of macOS. On Linux, the message of `install` is different. Make the folder, then run `install` again:

```sh
mkdir -p ~/.local/bin
```

Make sure that `~/.local/bin` is on your `PATH`.

### macOS does not start the binary

Apple does not notarize the macOS binaries. `curl` does not set the quarantine attribute, but a browser does. For a binary that you downloaded with a browser, remove the attribute:

```sh
xattr -d com.apple.quarantine spec-dna-0.6.1-aarch64-apple-darwin
```

## Start

### No spec-dna.toml

```text
spec-dna: no spec-dna.toml at /path/to/repo; run `spec-dna init`
```

The command runs in a repository with no configuration. Run `spec-dna init` in the root of the repository. The exit code is 3.

### Not inside a git working tree

```text
spec-dna: /path/to/folder is not inside a git working tree; spec-dna reads documents through git
```

spec-dna reads the documents through git. Run it in a git repository, or run `git init` first. The exit code is 3.

### The pin and the binary have different versions

```text
spec-dna: spec-dna.toml pins version 0.6.0, but this binary is 0.6.1.
  To use 0.6.0, install it:
    cargo install --locked --force --git https://github.com/AlexGavrilov939/spec-dna --tag v0.6.0 spec-dna
  To move the project to 0.6.1, run `spec-dna upgrade`.
```

Each project pins one version in `spec-dna.toml`, so each person and CI get the same result. The message gives 2 fixes:

- Install the pinned version with the command of the message.
- Move the project to the version of the binary with `spec-dna upgrade`. Commit the result as a change of its own ([Upgrade](quickstart.md#upgrade)).

The exit code is 3.

## check

### Each document gives SD303

```text
docs/adr/0001-use-postgres.md:1: error SD303 type-invalid: the document has no type: ...
```

No root or layer folder gives a type to the folder of the document. A mature repository keeps its folder names, so map each folder to a type with a root in `spec-dna.toml`:

```toml
[[roots]]
path = "docs/adr"
type = "decision"
```

[The adoption guide](guides/existing-repo.md#run-init-on-your-folders) gives the steps, and [example 2](../examples/existing-repo/README.md) shows the result.

### SD304: a decision has its status in the text

```text
docs/decisions/0001-use-sqlite.md:1: error SD304 status-invalid: type `decision` needs a `status` (proposed, accepted, superseded, rejected)
```

spec-dna reads the status from the front matter only. Many ADRs keep the status in a `## Status` section, and spec-dna does not read that section. Add the status in front matter at the top of the file:

```md
---
status: accepted
---
```

If the old documents use other status words, map them with `status-map` of the root. If each document of a root has the same status, set `status` of the root ([Configuration](config.md#keys)).

### SD306 after spec-dna new

```text
docs/decisions/storage.md:8: error SD306 placeholder-left: the template placeholder `{{The problem in 2 or 3 sentences.}}` is still in the document
```

`spec-dna new` writes a document from a template. Replace each `{{...}}` placeholder with your text.

### No base for the change checks

```text
note: no base for the change checks; pass --base <rev>, or run git remote set-head origin <branch>, or set [change] base
```

The change checks compare a branch with its base. The default base is `origin/HEAD`, and a repository with no remote, or a new clone, can have no `origin/HEAD`. Do one of these steps:

- Run `git remote set-head origin <main branch>`.
- Give `--base <rev>`.
- For a repository with no remote, set `base` in the `[change]` table of `spec-dna.toml` ([Configuration](config.md)).

Without a change policy (`[change] policy = "none"`), these checks do not run.

### The base is not a commit

```text
spec-dna: base `origin/main` is not a commit here; fetch it first (a depth-1 fetch of the base commit is enough)
```

The clone does not have the base branch. This is frequent in CI, because a checkout fetches one commit by default. Fetch the base branch, or check out with `fetch-depth: 0` ([CI](quickstart.md#ci)). The exit code is 3.

### SD503: the configuration is not alone

```text
spec-dna.toml:1: error SD503 config-not-alone: the change edits spec-dna.toml and 4 other files (first: `docs/decisions/storage.md`); a configuration change must be a change of its own
```

A change of `spec-dna.toml` changes the rules of `check`. Thus a change of `spec-dna.toml` must not include documents or code. Commit `spec-dna.toml` in a change of its own.

### SD921: requirements changed with no intent

```text
docs/requirements/valves.md:8: warning SD921 truth-changed-without-change: requirements changed `overflow`, and no file of the intent folder changed
```

The project has a change policy, and the change edits a requirement with no intent folder. Start the change with `spec-dna new intent <slug>` in the same pull request ([the change guide](guides/change.md)). A move that keeps the text of each requirement gives no SD921.

## Changes

### No root takes an intent

```text
spec-dna: no root takes a `intent`; add a root to spec-dna.toml or use --root
```

Each root of the project has one type, so no root can hold an intent folder. Add a mixed root for the changes ([the adoption guide](guides/existing-repo.md#turn-on-the-change-process)). The exit code is 2.

### finish stops at a step

```text
spec-dna: finish `sensor-timeout` stopped at step 5: a requirement that the change adds or changes has no evidence (...); nothing changed
```

`finish` stops at the first step that fails, and it changes nothing. The lines before this message give the cause. At step 5, add a `covers: <requirement id>` comment in a test file for each requirement of the delta. `spec-dna next <slug>` lists the open steps ([example 3](../examples/one-change/README.md)).

## Known limits

- The compare of area names changes only the letters A to Z. `Été` and `été` are 2 areas for `areas`, `new` and `mv`.
- On a file system that ignores case, a folder whose name on the disk differs in case from the name in `areas` is the same folder for the system. `new` and `mv` compare the path that you give, not the name on the disk, so they can stop for a path that the disk would accept.
- A rename that changes only the case of an area (`valves` to `Valves`) needs 2 steps. First, write the new name in `areas`, because `mv` stops for a path whose area differs from a name of `areas` only in case. Then run `spec-dna mv` through a temporary name: each document goes to a temporary area, then to the new area. On a file system that ignores case, a one-step move stops with `already exists`.

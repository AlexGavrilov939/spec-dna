# Quickstart

This page gives the steps from install to uninstall. You need git, and Rust 1.88 or later to build spec-dna.

## Install

Build the version that you want from its tag (phase 1 of [ADR 0003](adr/0003-distribution.md)):

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.1.0 spec-dna
spec-dna --version
```

To build from a local clone, give its URL to `--git`, for example `--git file:///home/me/spec-dna`. A plain path does not work. Keep the package name `spec-dna` at the end: the repository has other packages.

## Start

Run these commands in the root of your git repository:

```sh
spec-dna init                # or: spec-dna init --root <documents directory>
spec-dna check
git add spec-dna.toml CLAUDE.md .claude/skills/spec-dna   # and AGENTS.md, if init wrote it
git commit -m "build: adopt spec-dna"
```

`init` writes these files:

- `spec-dna.toml`, with the pinned version and one document root (`docs` by default). In this root, a document without a `type` is a `spec` with the status `draft`. [docs/config.md](config.md) gives all keys.
- One line in `AGENTS.md` and in `CLAUDE.md`. The line tells agents to run `spec-dna context` before a change and `spec-dna check` before a commit. On an existing project, `init` adds only this line. It never imports other files.
- The skill `.claude/skills/spec-dna/SKILL.md`, with the full procedure for agents.

`init` prints the bytes that it adds to the prompt prefix of each agent request, and the `git add` command for the files that it wrote.

## Write documents

The id of a document is its path under the root, without `.md`: `docs/loans.md` has the id `loans`. Write a requirement as a list item with a bold key. Cite the code in inline code spans:

```md
# Loans

## Requirements

- **loan-period**: When a member borrows a book, the library SHALL set the due date 14 days later.

The code is in `src/library/loans.py` (`checkout_book()`).
```

Set another type in the front matter: `type: contract`, `guide`, `decision`, `change` or `note`. For a decision, also set `status: accepted` and write a `## Decision Outcome` section. [ADR 0001](adr/0001-information-model.md) gives the types and their states.

## Daily use

```sh
spec-dna context src/library/loans.py   # before you change a file: the sections that cite it
spec-dna context loans                  # a document, its dependencies and its decisions
spec-dna show loans#loan-period         # one section or requirement
spec-dna check                          # before a commit: exit 1 on a new error
spec-dna mv loans circulation/loans     # move a document; the old id stays as an alias
spec-dna mv loans#loan-period policy    # move a requirement into the existing spec `policy`
```

`check` warns when a change removes a path or an identifier that a document cites (SD606, SD607). Update that document in the same change.

## CI

Run the gate against the merge base. This GitHub Actions job installs the pinned version:

```yaml
name: Specs
on: pull_request
permissions: {}
jobs:
  spec-dna:
    runs-on: ubuntu-24.04
    permissions:
      contents: read
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          fetch-depth: 0
          persist-credentials: false
      - name: Install the pinned spec-dna
        run: |
          pin=$(sed -n 's/^spec-dna = "\(.*\)"/\1/p' spec-dna.toml)
          cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag "v$pin" spec-dna
      - run: spec-dna check --base "origin/$GITHUB_BASE_REF"
```

## Upgrade

1. Install the new version: `cargo install --locked --force --git <repository> --tag v<new> spec-dna`.
2. Set `spec-dna = "<new>"` in `spec-dna.toml`.
3. Run `spec-dna init`. It writes the adapters of the new version.
4. Run `spec-dna check`, and commit the result.

If the pin and the binary have different versions, `check` and `init` stop with exit code 3. The message gives the command for each direction. To roll back, install the old version and revert the upgrade commit.

## Uninstall

```sh
rm spec-dna.toml
rm -r .claude/skills/spec-dna "$(git rev-parse --git-dir)/spec-dna"
for f in AGENTS.md CLAUDE.md; do
  [ -f "$f" ] || continue
  awk '/<!-- spec-dna v/ { held = 0; next } $0 == "" { held++; next }
    { for (; held > 0; held--) print ""; print }
    END { for (; held > 0; held--) print "" }' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
  [ -s "$f" ] || rm "$f"
done
cargo uninstall spec-dna
```

The loop removes the generated line and the blank line before it. It deletes a file that only had this line. Your documents are plain Markdown files, and they do not change.

# Quickstart

This page gives the steps from install to uninstall. You need git.

## Install

Each [release](https://github.com/AlexGavrilov939/spec-dna/releases) has these assets ([ADR 0003](adr/0003-distribution.md)):

- a binary for each of 7 targets: `spec-dna-<version>-<target>`, with `.exe` for Windows;
- `SHA256SUMS`: the SHA-256 sum of each binary;
- `spec-dna-pin.toml`: the version and the sums, in the form of `spec-dna.toml`;
- a build-provenance attestation for each of these files.

| Platform | Target |
| --- | --- |
| macOS, Apple silicon | `aarch64-apple-darwin` |
| macOS, Intel | `x86_64-apple-darwin` |
| Linux x86-64, glibc 2.17 or later | `x86_64-unknown-linux-gnu` |
| Linux arm64, glibc 2.17 or later | `aarch64-unknown-linux-gnu` |
| Linux x86-64, static (musl) | `x86_64-unknown-linux-musl` |
| Windows x86-64 | `x86_64-pc-windows-gnu` |
| Windows arm64 | `aarch64-pc-windows-gnullvm` |

On macOS and Linux, download the binary of your target, compare its sum with `SHA256SUMS`, and put it on your `PATH`:

```sh
v=0.4.0 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS     # Linux: sha256sum -c --ignore-missing SHA256SUMS
install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
spec-dna --version
```

The macOS binaries have no notarization. `curl` does not set the quarantine attribute, so macOS starts the binary. If you download it with a browser, remove the attribute: `xattr -d com.apple.quarantine <file>`.

On Windows, in PowerShell:

```powershell
$v = "0.4.0"; $t = "x86_64-pc-windows-gnu"
$base = "https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v"
Invoke-WebRequest "$base/spec-dna-$v-$t.exe" -OutFile spec-dna.exe
Invoke-WebRequest "$base/SHA256SUMS" -OutFile SHA256SUMS
(Get-FileHash spec-dna.exe -Algorithm SHA256).Hash.ToLower()
Select-String "spec-dna-$v-$t.exe" SHA256SUMS
```

The 2 sums must be equal. Then put `spec-dna.exe` in a directory on your `PATH`.

To verify the build provenance of a file, use the [GitHub CLI](https://cli.github.com). This step needs the network:

```sh
gh attestation verify "spec-dna-$v-$t" --repo AlexGavrilov939/spec-dna
```

### Build from source

On another platform, or if your policy does not allow downloaded binaries, build the version from its tag. You need Rust 1.88 or later. Keep the package name `spec-dna` at the end, because the repository has other packages.

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.4.0 spec-dna
```

A team can also build from a local or a private clone. Give its git URL to `--git`. A plain path does not work.

```sh
cargo install --locked --git file:///srv/git/spec-dna --tag v0.4.0 spec-dna            # a local clone
cargo install --locked --git ssh://git@git.example.org/tools/spec-dna.git --tag v0.4.0 spec-dna  # a private host
```

Put the same URL into `spec-dna.toml` as `install-source = "<url>"`. Then the message of a version mismatch shows your source, and the CI job below builds from it.

## Start

Run these commands in the root of your git repository:

```sh
spec-dna init                # questions with the detected defaults, a preview, then a confirmation
spec-dna check
git add spec-dna.toml CLAUDE.md .claude/skills/spec-dna   # init prints the full `git add` command
git commit -m "build: adopt spec-dna"
```

`init` detects the repository: the document directories and their types, ids, requirement keys, statuses, heading languages, vendored and generated directories, tests, agent files, CI and hooks. Then it asks at most 8 questions, for example the profile and the knowledge types. Press Enter to keep a detected default. It shows the full `spec-dna.toml` and the files to create or change, and it writes them only after you confirm. It never edits a document. Without a terminal, or with `--non-interactive`, it uses the detected defaults. `--dry-run` shows the preview only. [docs/init.md](init.md) gives the details.

`init` writes these files:

- `spec-dna.toml`, with the pinned version, the profile and the document roots. A new project gets one root (`docs`, or `--root <dir>`); there, a document without a `type` is a `spec` with the status `draft`. [docs/config.md](config.md) gives all keys.
- One line in `AGENTS.md` and in `CLAUDE.md`. The line tells agents to run `spec-dna context` once for a task that edits code or docs, and `spec-dna check` before a commit. On an existing project, `init` adds only this line. It never imports other files.
- The skill `.claude/skills/spec-dna/SKILL.md`, with the full procedure for agents.
- If you choose them: the CI job `.github/workflows/spec-dna.yml`, and a hook in `.pre-commit-config.yaml` or `.husky/pre-commit`.

`init` prints the bytes that it adds to the prompt prefix of each agent request, and the `git add` command for the files that it wrote.

The profile sets what fails `check`: `lite` (only warnings, for adoption), `standard` (the default) or `strict`. [docs/profiles.md](profiles.md) gives the table.

### Pin the sums

`init` pins the version: `spec-dna = "<version>"`. To pin the sums of the binaries too, copy the table `[spec-dna-sha256]` from `spec-dna-pin.toml` of the release to the end of `spec-dna.toml`. A table must come after the keys at the top of the file. You can remove the lines of the targets that your team does not use. Do this before the commit of `init`, or later in a commit that changes only `spec-dna.toml` (a configuration change is a change of its own, SD503).

With the sums, the CI job below downloads the release binary and verifies it, and the message of a version mismatch gives the URL and the sum of the pinned binary. `check` validates the table: each key must be an asset of the pinned version, and each sum must have 64 lowercase hexadecimal characters.

## Write documents

The id of a document is its path under the root, without `.md`: `docs/loans.md` has the id `loans`. Write a requirement as a list item with a bold key. Cite the code in inline code spans:

```md
# Loans

## Requirements

- **loan-period**: When a member borrows a book, the library SHALL set the due date 14 days later.

The code is in `src/library/loans.py` (`checkout_book()`).
```

Set another type in the front matter, or let `spec-dna new` write the document from the template of its type:

```sh
spec-dna new decision psp-choice        # docs/decisions/psp-choice.md
spec-dna new invariant money             # docs/invariants/money.md: rules that every change in its scope keeps
spec-dna new runbook restart-worker      # docs/runbooks/restart-worker.md
spec-dna new incident 2026-11-double-charge
```

There are 10 types: `spec`, `contract`, `guide`, `decision`, `change`, `note`, `invariant`, `runbook`, `incident` and `reference`. [docs/types.md](types.md) gives their statuses, required sections, budgets and templates. `context` serves a matching invariant first. It never serves a `reference` document unless a target names it.

Link a test to a requirement with a `covers:` tag in a comment of the test, in any language:

```python
# covers: loans#loan-period
def test_due_date_is_14_days_later():
    ...
```

`check` resolves each tag. In the `standard` profile, an active requirement without a test gives a warning (SD608). `show loans` lists the evidence of each requirement.

## Daily use

```sh
spec-dna context src/library/loans.py   # before you change a file: the sections that cite it
spec-dna context loans                  # a document, its dependencies and its decisions
spec-dna show loans#loan-period         # one section or requirement
spec-dna check                          # before a commit: exit 1 on a new error
spec-dna mv loans circulation/loans     # move a document; the old id stays as an alias
spec-dna mv loans#loan-period policy    # move a requirement into the spec `policy`
spec-dna mv loans#holds loans#fines circulation  # split: move 2 sections into a new spec
spec-dna show --conventions decision    # the project rules for a type: `[conventions]` in spec-dna.toml
```

`check` warns when a change removes a path or an identifier that a document cites (SD606, SD607). Update that document in the same change. For an accepted decision, do not change its text: add an amendment section (`## Amendments`) that names the new path or identifier. To make these warnings errors, set `[rules] drift = "error"`, or use the `strict` profile.

Amend an accepted decision when its outcome stays: add a detail, a limit or a new code path under `## Amendments`. Supersede it when its outcome changes: write a new decision with `supersedes: [<old id>]` and set the old one to `superseded`.

`check` lists errors and drift warnings first, and then the fix of each listed code. The prose lint warnings (SD801 to SD805) are one count for each code; `spec-dna check --lint` lists them. The lint is on in the `strict` profile only. Another warning code with more than 10 diagnostics, such as SD608 on a mature corpus, is one count too; `spec-dna check --json` lists each diagnostic.

When `check` reports SD702 (a document over its hard budget), split the document with one `mv`: name the keys or the sections, and a new or an existing spec. The old ids of `HEAD` stay as aliases (a key that is new in the change needs none), and a preservation check runs before any file changes. Split only for a size error: SD702, and in the `strict` profile also SD701, SD703 and SD704. A size warning needs no split.

## CI

Run the gate against the merge base. This GitHub Actions job installs the pinned version. `spec-dna init` writes it, with your base branch in place of `main`. If `spec-dna.toml` has the sum of the Linux asset in `[spec-dna-sha256]` and no `install-source`, the job downloads that asset and verifies its SHA-256 sum. A different sum stops the job. Otherwise, the job builds the pinned version from source. The job needs no action other than `actions/checkout`, no token and no cache.

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
          src=$(sed -n 's/^install-source = "\(.*\)"/\1/p' spec-dna.toml)
          asset="spec-dna-$pin-x86_64-unknown-linux-gnu"
          sum=$(sed -n "s/^\"$asset\" = \"\(.*\)\"/\1/p" spec-dna.toml)
          if [ -z "$src" ] && [ -n "$sum" ]; then
            curl --fail --silent --show-error --location --output "$RUNNER_TEMP/$asset" "https://github.com/AlexGavrilov939/spec-dna/releases/download/v$pin/$asset"
            echo "$sum  $RUNNER_TEMP/$asset" | sha256sum --check -
            install -D -m 755 "$RUNNER_TEMP/$asset" "$HOME/.local/bin/spec-dna"
            echo "$HOME/.local/bin" >> "$GITHUB_PATH"
          else
            cargo install --locked --git "${src:-https://github.com/AlexGavrilov939/spec-dna}" --tag "v$pin" spec-dna
          fi
      - run: spec-dna check --base "origin/${GITHUB_BASE_REF:-main}"
```

For a private `install-source`, give the job read access to it, for example with a deploy key.

## Upgrade

1. Download `spec-dna-pin.toml` of the new release. Verify it with `gh attestation verify spec-dna-pin.toml --repo AlexGavrilov939/spec-dna`.
2. In `spec-dna.toml`, replace the key `spec-dna` and the table `[spec-dna-sha256]` with the content of that file.
3. Install the new binary, and verify it against the new sum in `spec-dna.toml`.
4. Run `spec-dna init`. It writes the adapters of the new version. Then run `spec-dna check`, and commit the result.

If the pin and the binary have different versions, `check` and `init` stop with exit code 3. The message gives the command for each direction. To roll back, install the old version and revert the upgrade commit.

From 0.1 to 0.2: a configuration without `profile` uses `standard`. Then the requirement lint is off, and an active requirement without evidence gives SD608. [docs/profiles.md](profiles.md#changes-from-01) gives the 2 rules that keep the behavior of 0.1. `spec-dna init --reconfigure` asks the questions again, with the current values as defaults.

## Uninstall

```sh
rm spec-dna.toml
rm -r .claude/skills/spec-dna "$(git rev-parse --git-dir)/spec-dna"
rmdir -p .claude/skills 2>/dev/null || true
for f in AGENTS.md CLAUDE.md; do
  [ -f "$f" ] || continue
  awk '/<!-- spec-dna v/ { held = 0; next } $0 == "" { held++; next }
    { for (; held > 0; held--) print ""; print }
    END { for (; held > 0; held--) print "" }' "$f" > "$f.tmp" && mv "$f.tmp" "$f"
  [ -s "$f" ] || rm "$f"
done
cargo uninstall spec-dna
```

The loop removes the generated line and the blank line before it. It deletes a file that only had this line. If `init` wrote the CI job or a hook, also remove `.github/workflows/spec-dna.yml` and the `spec-dna` hook in `.pre-commit-config.yaml` or `.husky/pre-commit`. Your documents are plain Markdown files, and they do not change.

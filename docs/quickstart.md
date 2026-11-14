# Quickstart

This page gives the steps from install to uninstall. You need git.

## Install

Each [release](https://github.com/AlexGavrilov939/spec-dna/releases) has these assets ([ADR 0003](adr/0003-distribution.md)):

- a binary for each of 7 targets: `spec-dna-<version>-<target>`, with the suffix `.exe` for Windows
- `SHA256SUMS`: the SHA-256 sum of each binary
- `spec-dna-pin.toml`: the version and the sums, in the form of `spec-dna.toml`
- a build-provenance attestation for each of these files

| Platform | Target |
| --- | --- |
| macOS, Apple silicon | `aarch64-apple-darwin` |
| macOS, Intel | `x86_64-apple-darwin` |
| Linux x86-64, glibc 2.17 or later | `x86_64-unknown-linux-gnu` |
| Linux arm64, glibc 2.17 or later | `aarch64-unknown-linux-gnu` |
| Linux x86-64, static (musl) | `x86_64-unknown-linux-musl` |
| Windows x86-64 | `x86_64-pc-windows-gnu` |
| Windows arm64 | `aarch64-pc-windows-gnullvm` |

On macOS and Linux, use these commands. They download the binary of your target, compare its sum with `SHA256SUMS`, and install it to `~/.local/bin`. Make sure that `~/.local/bin` is on your `PATH`:

```sh
v=0.4.2 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS     # Linux: sha256sum -c --ignore-missing SHA256SUMS
install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
spec-dna --version
```

Apple does not notarize the macOS binaries. `curl` does not set the quarantine attribute, so macOS starts the binary. If you download the binary with a browser, remove the attribute with `xattr -d com.apple.quarantine <file>`.

On Windows, use these commands in PowerShell:

```powershell
$v = "0.4.2"; $t = "x86_64-pc-windows-gnu"
$base = "https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v"
Invoke-WebRequest "$base/spec-dna-$v-$t.exe" -OutFile spec-dna.exe
Invoke-WebRequest "$base/SHA256SUMS" -OutFile SHA256SUMS
(Get-FileHash spec-dna.exe -Algorithm SHA256).Hash.ToLower()
Select-String "spec-dna-$v-$t.exe" SHA256SUMS
```

Make sure that the 2 sums are equal. Then put `spec-dna.exe` in a directory on your `PATH`.

To verify the build provenance of a file, use the [GitHub CLI](https://cli.github.com). This step uses the network:

```sh
gh attestation verify "spec-dna-$v-$t" --repo AlexGavrilov939/spec-dna
```

### Build from source

On another platform, or if your policy does not allow downloaded binaries, build the version from its tag. You need Rust 1.88 or later. From v0.4.2 on, the command needs no package name. For v0.4.1 and earlier, add `spec-dna` at the end, because those tags have other packages with binaries.

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.4.2
```

A team can also build from a local or a private clone. Give its git URL to `--git`. A plain path does not work.

```sh
cargo install --locked --git file:///srv/git/spec-dna --tag v0.4.2            # a local clone
cargo install --locked --git ssh://git@git.example.org/tools/spec-dna.git --tag v0.4.2  # a private host
```

Put the same URL into `spec-dna.toml` as `install-source = "<url>"`. Then the message of a version mismatch shows your source. The CI job below also builds from it.

## Start

Run these commands in the root of your git repository:

```sh
spec-dna init                # asks the questions with the detected defaults, shows a preview, then asks for a confirmation
spec-dna check
git add spec-dna.toml CLAUDE.md .claude/skills/spec-dna   # `init` prints the full `git add` command
git commit -m "build: adopt spec-dna"
```

`init` detects the facts of the repository, for example the document directories, their types, the tests, the agent files, CI and hooks. [docs/init.md](init.md) lists all of them. `init` asks at most 8 questions, for example the profile and the knowledge types. Press Enter to keep a detected default. It shows the full `spec-dna.toml` and the files to create or change. It writes them only after you confirm.

`init` never edits a document. Without a terminal, or with `--non-interactive`, it uses the detected defaults. `--dry-run` shows the preview only.

`init` writes these files:

- `spec-dna.toml`, with the pinned version, the profile and the document roots. A new project gets one root (`docs`, or the directory of `--root <dir>`). In this root, a document without a `type` is a `spec` with the status `draft`. [docs/config.md](config.md) gives all keys.
- One line in `AGENTS.md` and in `CLAUDE.md`. The line tells agents to run `spec-dna context` once for a task that edits code or docs. It also tells them to run `spec-dna check` before a commit. On an existing project, `init` adds only this line. The line never imports another file.
- The skill `.claude/skills/spec-dna/SKILL.md`, with the full procedure for agents.
- If you choose them: the CI job `.github/workflows/spec-dna.yml`, and a hook in `.pre-commit-config.yaml` or `.husky/pre-commit`.

`init` prints the bytes that it adds to the prompt prefix of each agent request. It also prints the `git add` command for the files that it wrote.

The profile sets which diagnostics fail `check`. The profiles are `lite` (only warnings, for adoption), `standard` (the default) and `strict`. [docs/profiles.md](profiles.md) gives the table.

### Pin the sums

`init` pins the version with `spec-dna = "<version>"`. To pin the sums of the binaries too, copy the table `[spec-dna-sha256]` from `spec-dna-pin.toml` of the release to the end of `spec-dna.toml`. The table must come after the keys at the top of the file. You can remove the lines of the targets that your team does not use. Copy the table before the commit of `init`, or later in a commit that changes only `spec-dna.toml`. A configuration change is a change of its own (SD503).

With the sums, the CI job below downloads the release binary and verifies it. The message of a version mismatch gives the URL and the sum of the pinned binary. `check` validates the table. Each key must be an asset of the pinned version. Each sum must have 64 lowercase hexadecimal characters.

## Write documents

The id of a document is its path under the root, without `.md`. For example, `docs/loans.md` has the id `loans`. Write a requirement as a list item with a bold key. Cite the code in an inline code span:

```md
# Loans

## Requirements

- **loan-period**: When a member borrows a book, the library SHALL set the due date 14 days later.

The code is in `src/library/loans.py` (`checkout_book()`).
```

To set another type, write it in the front matter. Or use `spec-dna new` to write the document from the template of its type:

```sh
spec-dna new decision pump-choice       # docs/decisions/pump-choice.md
spec-dna new invariant water             # docs/invariants/water.md: rules that every change in its scope keeps
spec-dna new runbook restart-worker      # docs/runbooks/restart-worker.md
spec-dna new incident 2026-11-flooded-bed
```

There are 10 types: `spec`, `contract`, `guide`, `decision`, `change`, `note`, `invariant`, `runbook`, `incident` and `reference`. [docs/types.md](types.md) gives their statuses, required sections, budgets and templates. `context` serves a matching invariant first. It never serves a `reference` document unless a target names it.

To link a test to a requirement, write a `covers:` tag in a comment of the test. This works in any language:

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

`check` warns when a change removes a path or an identifier that a document cites (SD606, SD607). Update that document in the same change. For an accepted decision, do not change its text. Add an amendment section (`## Amendments`) that names the new path or identifier. To make these warnings errors, set `[rules] drift = "error"`, or use the `strict` profile.

To change an accepted decision, do one of these actions:

- Amend the decision if its outcome stays. Add the detail, the limit or the new code path under `## Amendments`.
- Supersede the decision if its outcome changes. Write a new decision with `supersedes: [<old id>]`. Set the status of the old decision to `superseded`.

`check` lists the errors and the drift warnings first. Then it gives the fix of each listed code. For the prose lint warnings (SD801 to SD805), it gives one count for each code. `spec-dna check --lint` lists them. The lint is on in the `strict` profile only. `--lint` runs it in any profile, as a report that never fails the gate.

For another warning code with more than 10 diagnostics, such as SD608 on a mature corpus, `check` also gives one count. `spec-dna check --json` lists each diagnostic.

If `check` reports SD702 (a document over its hard budget), split the document with one `mv` command. Name the keys or the sections, and a new or an existing spec. The old ids of `HEAD` stay as aliases. A key that is new in the change needs no alias. A preservation check runs before `mv` changes a file.

Split a document only for a size error: SD702, and in the `strict` profile also SD701, SD703 and SD704. A size warning needs no split.

## CI

Run the gate against the merge base. This GitHub Actions job installs the pinned version. `spec-dna init` writes the job, with your base branch in place of `main`. The job installs the pinned version in one of 2 ways:

- If `spec-dna.toml` has the sum of the Linux asset in `[spec-dna-sha256]` and no `install-source`, the job downloads that asset and verifies its SHA-256 sum. A different sum stops the job.
- Otherwise, the job builds the pinned version from source.

The job needs no action other than `actions/checkout`. It needs no token and no cache.

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
3. Install the new binary. Verify it against the new sum in `spec-dna.toml`.
4. Run `spec-dna init`. It writes the adapters of the new version.
5. Run `spec-dna check`. Commit the result.

If the pin and the binary have different versions, `check` and `init` stop with exit code 3. The message gives the steps to install the pinned version. It also gives the steps to move the project to the version of the binary. To roll back, install the old version and revert the upgrade commit.

To upgrade from 0.1 to 0.2, note this change. A configuration without `profile` uses `standard`. In this profile, the requirement lint is off, and an active requirement without evidence gives SD608. [docs/profiles.md](profiles.md#changes-from-01) gives the 2 rules that keep the behavior of 0.1. `spec-dna init --reconfigure` asks the questions again, with the current values as defaults.

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

The loop removes the generated line and the blank line before it. It deletes a file that has no other text. If `init` wrote the CI job or a hook, also remove `.github/workflows/spec-dna.yml` and the `spec-dna` hook in `.pre-commit-config.yaml` or `.husky/pre-commit`. Your documents are plain Markdown files. They do not change.

# Quickstart

This page gives the steps to install spec-dna, start it, use it with a coding agent, run a change, upgrade and uninstall. You need git, and you can use any coding agent. [Agents](agents.md) describes what each agent gets.

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
v=0.5.0 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS     # Linux: sha256sum -c --ignore-missing SHA256SUMS
install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
spec-dna --version
```

Apple does not notarize the macOS binaries. `curl` does not set the quarantine attribute, so macOS starts the binary. If you download the binary with a browser, remove the attribute with `xattr -d com.apple.quarantine <file>`.

On Windows, use these commands in PowerShell:

```powershell
$v = "0.5.0"; $t = "x86_64-pc-windows-gnu"
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

On another platform, or if your policy does not allow downloaded binaries, build the version from its tag. You need Rust 1.88 or later. For v0.4.2 and later, the command needs no package name. For v0.4.1 and earlier, add `spec-dna` at the end, because those tags have other packages with binaries.

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.5.0
```

A team can also build from a local or a private clone. Give its git URL to `--git`. A path without `file://` does not work.

```sh
cargo install --locked --git file:///srv/git/spec-dna --tag v0.5.0            # a local clone
cargo install --locked --git ssh://git@git.example.org/tools/spec-dna.git --tag v0.5.0  # a private host
```

Put the same URL into `spec-dna.toml` as `install-source = "<url>"`. Then the message of a version mismatch shows your source. The CI job below also builds from it.

## Start

Run these commands in the root of your git repository:

```sh
spec-dna init                # asks the questions with the detected defaults, shows a preview, then asks for a confirmation
spec-dna check
git add spec-dna.toml CLAUDE.md REVIEW.md .claude evals   # `init` prints the full `git add` command
git commit -m "build: adopt spec-dna"
```

`init` detects the facts of the repository, for example the document directories, their types, the tests, the agent files, CI and hooks. [docs/init.md](init.md) lists all of them. `init` asks at most 14 questions, for example the profile, the layer graph, the change policy, the agent harness and the evals. Press Enter to keep a detected default. It shows the full `spec-dna.toml` and the files to create or change. It writes them only after you confirm.

`init` never edits a document. Without a terminal, or with `--non-interactive`, it uses the detected defaults. `--dry-run` shows the preview only.

`init` writes these files:

- `spec-dna.toml`, with the pinned version, the profile, the layer graph, the change policy and the document roots. A new project gets one mixed root (`docs`, or the directory of `--root <dir>`) and no `default-type`. The folder of a layer gives the type of a document, so `docs/requirements/loans.md` is a `requirements` document. [docs/config.md](config.md) gives all keys.
- One line in `CLAUDE.md`, and in `AGENTS.md` when `AGENTS.md` exists or `CLAUDE.md` does not exist. Codex, Cursor and many other agents read `AGENTS.md`. Claude Code reads `CLAUDE.md`. The line tells an agent to run `spec-dna context` once for a task that edits code or documents. It also tells the agent to run `spec-dna check` before the work is complete. On an existing project, `init` adds only this line, and the line never imports another file.
- The skill `.claude/skills/spec-dna/SKILL.md` for Claude Code, with the full procedure for agents.
- If you choose them: the CI job `.github/workflows/spec-dna.yml`, and a hook in `.pre-commit-config.yaml` or `.husky/pre-commit`.
- The Claude Code harness (level `lean` by default). It has agent files in `.claude/agents/`, the stage skills in `.claude/skills/spec-dna-*`, `REVIEW.md`, entries in `.claude/settings.json` and a model-team line in `CLAUDE.md`. The file `.claude/agents/Explore.md` replaces the built-in `Explore` subagent. [docs/harness.md](harness.md) describes the roles, the models, the hooks and how to remove that file. Each teammate needs spec-dna on `PATH` for the hooks.
- The starter files of the evals: the table `[evals]` in `spec-dna.toml`, and `evals/check.sh`, an example eval and its fixture in `evals/`. Question 14 writes them with the answer `skeleton`, the default of a new project, and writes nothing with the answer `none`. For the CI answer `github`, `init` also writes `.github/workflows/agent-evals.yml`. `init` starts no run, and the CI job runs only after you set the repository variable `SPEC_DNA_EVALS`. `init` prints the 2 commands for it. [docs/evals.md](evals.md) describes the evals.

`init` prints the bytes that it adds to the prompt prefix of each agent request. It also prints the `git add` command for the files that it wrote.

A new project gets the change process. Questions 4 and 5 propose the policy `three-class` and the gate `block`, and `init` writes the table `[change]`. Answer `none` to question 4 if you do not want the change process. `init` prints the line `docs/intent/*/spec.md @OWNER` for your `CODEOWNERS` file. It does not write that file. [docs/init.md](init.md) gives the details.

The profile sets which diagnostics fail `check`. The profiles are `lite` (only warnings, for adoption), `standard` (the default) and `strict`. [docs/profiles.md](profiles.md) gives the table.

### Pin the sums

`init` pins the version with `spec-dna = "<version>"`. To pin the sums of the binaries too, copy the table `[spec-dna-sha256]` from `spec-dna-pin.toml` of the release to the end of `spec-dna.toml`. The table must come after the keys at the top of the file. You can remove the lines of the targets that your team does not use. Copy the table before the commit of `init`, or later in a commit that changes only `spec-dna.toml`. A configuration change is a change of its own (SD503).

With the sums, the CI job below downloads the release binary and verifies it. The message of a version mismatch gives the URL and the sum of the pinned binary. `check` validates the table. Each key must be an asset of the pinned version. Each sum must have 64 lowercase hexadecimal characters.

## Layers and areas

A layer is a named set of documents. It has a folder (`requirements/`, `decisions/`) and the types of the documents in it. Question 2 of `init` chooses the layers:

- `small`: for fewer than 30 documents, also in a new project. It has the layers `foundation`, `pending`, `rules`, `requirements`, `decisions`, `intent` and `incidents`.
- `default`: for 30 documents or more. It adds the layers `interfaces`, `procedures` and `research`.
- `custom`: with `--reconfigure`, it keeps the `[layers]` tables of the current `spec-dna.toml`.

A document of a type that no layer has gives SD303. To add a layer later, run `spec-dna layer add <name> --types <type>`. Then run `spec-dna init`, so that the skill lists the types of the new layers. Commit `spec-dna.toml` and the skill as a change of their own (SD503). `spec-dna layer list` shows the layers.

Question 3 chooses the areas: the folders of `requirements/`. `init` proposes at most 8 areas, and only if it finds 2 or more. It takes them from the top folders of the roots that hold requirements and from the app folders. The answer `none` gives one folder. `init` never proposes a group.

## Write documents

The id of a document is its path under the root, without `.md`. For example, `docs/requirements/loans.md` has the id `requirements/loans`. Write a requirement as a list item with a bold key. Cite the code in an inline code span:

```md
# Loans

## Requirements

- **loan-period**: When a member borrows a book, the library SHALL set the due date 14 days later.

The code is in `src/library/loans.py` (`checkout_book()`).
```

A document needs no `type:` line in the folder of its layer. To set another type, write it in the front matter. Or use `spec-dna new` to write the document from the template of its type, in the folder of its layer:

```sh
spec-dna new requirements loans          # docs/requirements/loans.md
spec-dna new decision pump-choice        # docs/decisions/pump-choice.md
spec-dna new rule water                  # docs/rules/water.md: rules that every change in its scope keeps
spec-dna new incident 2026-11-flooded-bed
spec-dna new terms                       # docs/terms.md: a top file has no id
spec-dna new intent add-drip-lines       # docs/intent/add-drip-lines/intent.md: one change
```

A new project has the small graph, with no layer for `procedure`, `interface` and `research`. `spec-dna layer add` adds a layer ([commands.md](commands.md#layer-add)).

There are 15 types: `product`, `system`, `terms`, `roadmap`, `questions`, `rule`, `requirements`, `interface`, `decision`, `procedure`, `incident`, `research`, `intent`, `spec` and `plan`. [docs/types.md](types.md) gives their statuses, required sections, budgets and templates. `context` serves the layers in the order of the graph, from the most stable layer. It never serves a `research` document unless a target names it.

To link a test to a requirement, write a `covers:` tag in a comment of the test. This works in any language:

```python
# covers: requirements/loans#loan-period
def test_due_date_is_14_days_later():
    ...
```

`check` resolves each tag. In the `standard` profile, an active requirement without a test gives a warning (SD608). `show requirements/loans` lists the evidence of each requirement.

## Work with an agent

The instruction line that `init` wrote tells an agent what to do. The agent can be any coding agent. The steps are the same:

1. Before the agent edits a file, it runs `spec-dna context <path>`. The command gives the sections that cite the file, with their dependencies and decisions.
2. The agent edits the code and the documents of the change.
3. Before the work is complete, the agent runs `spec-dna check`. It fixes each new error that `check` reports.

A person can run the same commands. [Daily use](#daily-use) lists them. The skill, the hooks and the evals are for Claude Code. [Agents](agents.md) gives the details.

## Daily use

```sh
spec-dna context src/library/loans.py   # before you change a file: the sections that cite it
spec-dna context requirements/loans     # a document, its dependencies and its decisions
spec-dna show requirements/loans#loan-period   # one section or requirement
spec-dna check                          # before a commit: exit 1 on a new error
spec-dna mv requirements/loans requirements/circulation/loans  # move a document; the old id stays as an alias
spec-dna mv requirements/loans#loan-period requirements/policy # move a requirement into the document `requirements/policy`
spec-dna mv requirements/loans#holds requirements/loans#fines requirements/circulation  # split: move 2 sections into a new document
spec-dna show --conventions decision    # the project rules for a type: `[conventions]` in spec-dna.toml
```

`check` warns when a change removes a path or an identifier that a document cites (SD606, SD607). Update that document in the same change. For an accepted decision, do not change its text. Add an amendment section (`## Amendments`) that names the new path or identifier. To make these warnings errors, set `[rules] drift = "error"`, or use the `strict` profile.

To change an accepted decision, do one of these actions:

- Amend the decision if its outcome stays. Add the detail, the limit or the new code path under `## Amendments`.
- Supersede the decision if its outcome changes. Write a new decision with `supersedes: [<old id>]`. Set the status of the old decision to `superseded`.

`check` lists the errors and the drift warnings first. Then it gives the fix of each listed code. For the prose lint warnings (SD801 to SD805), it gives one count for each code. `spec-dna check --lint` lists them. The lint runs in the `strict` profile only. `--lint` runs it in any profile, as a report that never fails the gate.

For another warning code with more than 10 diagnostics, such as SD608 on a mature corpus, `check` also gives one count. `spec-dna check --json` lists each diagnostic.

If `check` reports SD702 (a document over its hard budget), split the document with one `mv` command. Name the keys or the sections. Name a new or an existing requirements document. The old ids of `HEAD` stay as aliases. A key that is new in the change needs no alias. A preservation check runs before `mv` changes a file.

Split a document only for a size error: SD702, and in the `strict` profile also SD701, SD703 and SD704. A size warning needs no split.

## Run a change

A change is a folder `docs/intent/<slug>/` with an intent, a spec and a plan. Question 4 of `init` chooses the change policy, and question 5 chooses the gate. A new project gets `three-class` and `block`. A project with no `[change]` table has the policy `none`, so it has no change gate.

```sh
spec-dna new intent sensor-timeout   # docs/intent/sensor-timeout/intent.md: start a change
spec-dna next sensor-timeout         # the checklist to the next gate
spec-dna status                      # each change, its class, its stage and its next gate
spec-dna finish sensor-timeout       # apply the delta, check the evidence, delete the folder
```

`next` marks each line done or open. Its last line gives the edit that approves the artifact. To approve a document, set its status in a pull request. `finish` works in the working tree and commits nothing. `spec-dna finish <slug> --dry-run` prints the diff only. [The change guide](guides/change.md) takes one change from the intent to `finish`.

## CI

Run the gate against the merge base. `spec-dna init` writes this GitHub Actions job, with your base branch in place of `main`. The job installs the pinned version in one of 2 ways:

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

For the evals skeleton, `init` writes a second workflow, `.github/workflows/agent-evals.yml`. [docs/evals.md](evals.md) describes it.

## Upgrade

1. Download `spec-dna-pin.toml` of the new release. Verify it with `gh attestation verify spec-dna-pin.toml --repo AlexGavrilov939/spec-dna`.
2. Install the new binary. Verify it against its sum in `spec-dna-pin.toml`.
3. Run `spec-dna upgrade --pin-file spec-dna-pin.toml`. It moves the pin and the sums in `spec-dna.toml` to the new version.
4. Run `spec-dna init`. It writes the adapters and the harness files of the new version. It keeps a harness file that you edited.
5. Run `spec-dna check`. Commit the result.

If the pin and the binary have different versions, `check` and `init` stop with exit code 3. The message gives the steps to install the pinned version. It also gives the steps to move the project to the version of the binary. To return to the old version, install it and revert the upgrade commit.

To upgrade a project of 0.5 to 0.6, use `spec-dna upgrade`. It moves the pin, the types, the statuses and the harness names. The [upgrade guide](guides/upgrade-0.5.md) gives the steps.

To add the agent harness to a project of 0.4 or older, run `spec-dna init --reconfigure`. Choose a level. An upgrade alone does not add it.

A change applies when you upgrade from 0.1 to 0.2. A configuration without `profile` uses `standard`. In this profile, the requirement lint is off, and an active requirement without evidence gives SD608. [docs/profiles.md](profiles.md#changes-from-01) gives the 2 rules that keep the behavior of 0.1. `spec-dna init --reconfigure` asks the questions again, with the current values as defaults.

## Uninstall

If the project has the agent harness, first run `spec-dna init --reconfigure` and choose `none`. `init` removes the generated agent files, the stage skills, `REVIEW.md`, the model-team line of `CLAUDE.md` and the hook handlers. It keeps the table `[evals]` and the files of the evals.

```sh
rm spec-dna.toml
rm -r .claude/skills/spec-dna "$(git rev-parse --git-dir)/spec-dna"
rmdir .claude/skills/spec-dna-* 2>/dev/null || true
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

The loop removes the generated line and the blank line before it. It deletes a file that has no other text. If `init` wrote the CI job or a hook, also remove `.github/workflows/spec-dna.yml` and the `spec-dna` hook in `.pre-commit-config.yaml` or `.husky/pre-commit`. If `init` wrote the evals, also remove the folder `evals` (the value of `[evals] dir`) and `.github/workflows/agent-evals.yml`. `init --reconfigure` keeps these files, because it never deletes a file that you can edit. Your documents are Markdown files, and they do not change.

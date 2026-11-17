# Contributing

## Setup

You need Rust 1.88 or later, and [pre-commit](https://pre-commit.com) 4.x. You can also run pre-commit with [uv](https://docs.astral.sh/uv/): `uvx pre-commit ...`.

```sh
pre-commit install
```

This command installs 2 git hooks. The `pre-commit` hook runs the checks from `.pre-commit-config.yaml` on staged files. The `commit-msg` hook checks the format of the commit message. CI runs the same checks on each push and pull request.

To run all checks manually:

```sh
pre-commit run --all-files
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

The golden outputs of the integration tests are in `tests/golden/`. After an intended change of the output, run `UPDATE_GOLDEN=1 cargo test` to regenerate them. Then examine the diff.

## Repository layout

| Path | Content |
| --- | --- |
| `src/` | The `spec-dna` crate |
| `tests/` | Integration tests, fixtures and golden outputs |
| `schema/spec-dna.schema.json` | The JSON Schema of `spec-dna.toml`. [docs/config.md](docs/config.md) describes the keys. |
| `docs/` | The user documentation. [docs/README.md](docs/README.md) is its map |
| `docs/guides/` | How-to guides |
| `docs/assets/` | Images, each in a light and a dark file |
| `docs/design/` | The design of a version, with the deviations of each step |
| `docs/adr/` | The decisions of the project (ADRs) |
| `docs/research/` | Standards and prior art |
| `examples/` | Example scripts and their pages. `tests/examples.rs` runs each script and compares its output with the pages |
| `scripts/` | The cross-build, the run test, the release assets and the demo replay |
| `bench/` | Benchmarks, retrieval measurements and the prototypes of the language decision |
| `AGENTS.md` | Rules for AI agents that work in this repository. `CLAUDE.md` imports it. |
| `CHANGELOG.md` | The changes of each version |
| `SECURITY.md` | How to report a vulnerability |
| `.claude/` | Claude Code settings, the agent team and its RACI matrix |
| `.github/` | CI workflows, Dependabot, issue and pull request templates, code owners |

## Formatting

`.editorconfig` contains the whitespace rules: a 2-space indent by default, 4 spaces for Python, TOML and Rust, and tabs for Go. Use `gofmt` to format Go and `cargo fmt` to format Rust.

Markdown files contain no HTML, except the elements `picture`, `source` and `img` for an image with a dark variant. `.markdownlint-cli2.yaml` enforces this rule.

## Writing

Write all specifications, documentation and code comments in ASD-STE100 Simplified Technical English (STE). Make the text clear and concise. This rule applies to Markdown files, ADRs and JSON Schema descriptions. It also applies to comments and doc comments in code and scripts, and to the text that `spec-dna init` writes for agents.

- Write one topic in one sentence.
- Use 20 words or fewer in a procedural sentence and 25 words or fewer in a descriptive sentence. Use 6 sentences or fewer in a paragraph.
- Use the active voice. Name the actor: "`check` reports SD702", not "SD702 is reported".
- Use only the simple tenses: present, past and future.
- Do not use -ing forms, except in technical names such as "heading".
- Use the imperative in an instruction. Give one instruction in one sentence.
- Use a single verb, not a phrasal verb: "start", not "set up".
- Use one word for one meaning, and one term for one thing.
- Use 3 nouns or fewer in a noun cluster.
- Do not remove articles or verbs to make a sentence shorter.
- Use a vertical list for a sequence of steps or for many conditions.
- Do not use idioms, metaphors or vague words such as "quickly" or "etc.".

Code names, commands, configuration keys and diagnostic codes are technical names. Write them as they are, in backticks.

Write a code comment only when it gives information that the code and its names cannot give. Examples are a reason, a constraint, an invariant, a format, an edge case or a reference to an ADR. Do not write a comment that repeats the code. Remove a comment that is wrong or that has no use. A doc comment on a public item states its contract: what it means, its inputs, its outputs and its errors.

### Docs layout

The user documentation is in `docs/`. [docs/README.md](docs/README.md) is its map.

- Give each page one kind: a tutorial, an explanation, a how-to guide or a reference.
- Put a how-to guide in `docs/guides/` and an image in `docs/assets/`.
- Write each detail in one reference page. Other pages link to it and do not copy it.
- Use the terms of [docs/glossary.md](docs/glossary.md). Add a new term to the glossary in the same pull request.
- Add each new page to the map in the same pull request.
- Keep the path of a reference page. The code, the schema and the tests name these paths.
- Give an image a light file and a dark file with the same geometry. Show it with `picture`.
- Keep an animation slow and quiet: no motion that blinks or loops faster than 10 seconds. Stop it for `prefers-reduced-motion`.

Each page follows this form:

1. The H1 names the subject, for example "Commands" or "Adopt spec-dna in an existing repository".
2. The first paragraph says what the page gives and who needs it, in 3 sentences or fewer.
3. The sections go from the frequent task to the rare detail.
4. A reference page gives keys, options, codes and statuses in tables.
5. Each code block has a language tag, for example `sh`, `console`, `toml`, `md`, `json` or `text`. A `console` block shows real output of the current version.
6. The page ends with its last section. It has no summary and no "see also" list, because the map links all pages.

spec-dna works with any coding agent. Write "the agent" or "an agent" for the core behavior. Name a specific agent, such as Claude Code or Codex, only in the text of its adapter: [docs/agents.md](docs/agents.md), [docs/harness.md](docs/harness.md), the evals and the transcript adapters. In other pages, link to that text. The one exception is the short summary of the agents in `README.md` and `docs/concepts.md`.

## Branches

- `master` is the default branch. Merge changes into `master` only through pull requests. The `no-commit-to-branch` hook stops local commits to `master`.
- Give each branch a name of the form `<type>/<short-description>`, for example `feat/spec-template` or `fix/readme-links`.

## Commits

- Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): subject`. The scope is optional.
- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style`, `revert`.
- Write the subject in the imperative, with a lowercase first letter and no period at the end.
- Sign your commits (`git config commit.gpgsign true`).

## Changes

This repository uses its own change process ([ADR 0006](docs/adr/0006-change-lifecycle.md)). `spec-dna.toml` sets the policy `three-class` and the gate `warn`.

- A small fix needs only a pull request. Examples are a typo, a dependency update and a CI fix.
- Start each other change with `spec-dna new intent <slug>`. The command writes `docs/intent/<slug>/intent.md`. Replace each `{{...}}` placeholder, because `check` reports it as an error (SD306).
- Set `class: higher-risk` in the intent of a change of a contract. The change sets in `.claude/agents/ownership.md` list the contracts. Such a change also has a `spec.md`, and the maintainer approves it.
- Merge the accepted intent in a pull request before the code. If the intent and the code merge together, `check` gives the warning SD910, and `spec-dna log` has no record of the change.
- A `spec.md` or a `plan.md` can stay a `draft` until its approval. The plan can be a draft before its spec is approved, and a `draft` artifact needs no approval of the artifact before it. The code and the status `approved` of the plan still need the approvals.
- Approve in the branch ([ADR 0012](docs/adr/0012-approval-in-the-branch.md)). `spec-dna.toml` sets `[change] approval = "branch"`. An approval commit of an approver then counts, if it comes before the work that it approves. The commit is not a merge commit, and it changes the `status` line of one artifact file only. Its author e-mail is in `[change] approvers`. The approvers are the 2 addresses of the owner: `alexgavrilov939@gmail.com` and `5443983+AlexGavrilov939@users.noreply.github.com`. The code host stays the control (`CODEOWNERS`, a protected `master`, signed commits). The jobs of CI fetch the full history (`fetch-depth: 0`).
- An agent writes an approval commit only when the owner says so explicitly in the chat. The owner can make the approval commit in the web editor of the host.
- The order of the pull requests. The first pull request holds the intent, the spec and the plan with their approval commits. The code and the result of `spec-dna finish` follow in a second pull request. The squash merge keeps the net diff only, so a change that starts and ends in one pull request leaves no record in `spec-dna log`, and `check` gives SD910 for it.
- `spec-dna next <slug>` gives the next step. `spec-dna finish <slug>` applies the change and deletes the folder. Commit the result with the code.
- Use spec-dna at the version of the pin in `spec-dna.toml`, because `check` does not run with another version. Before the release of that version, run the binary of this tree: `cargo run --quiet -- <command>`.

## Pull requests

- Put one logical change in each pull request. Complete the template.
- The PR title must be a Conventional Commit. The `PR title` workflow checks it.
- GitHub merges each pull request as one squashed commit, so the PR title becomes the commit message on `master`.
- Merge only when all CI jobs pass. If GitHub Actions does not start the jobs, do these steps:
    1. Run the same jobs locally.
    2. Put the results in the pull request.
    3. Merge only if all jobs pass.

## Issues

- Use the issue forms for a bug report or a feature request. Give the output of `spec-dna --version`.
- Do not report a security problem in a public issue. [SECURITY.md](SECURITY.md) tells you how to report it.
- Do not put text, paths or names from a private repository into an issue, a test fixture or a commit message. Make a small example that shows the same problem.

## Releases

1. In a pull request, set the new version in `Cargo.toml` and `Cargo.lock`. Set the pin in `spec-dna.toml` to the same version in the same commit. Add its section to `CHANGELOG.md`, in the form `## <version> - <date>`.
2. Merge the pull request when CI passes. CI builds the 7 targets and runs each binary on a runner of its platform.
3. Push a signed tag `v<version>` on that commit of `master`.

The tag starts `.github/workflows/release.yml`. The workflow compares the tag with the version. It builds and runs the binaries again. It publishes the binaries, `SHA256SUMS` and `spec-dna-pin.toml` with build-provenance attestations. The notes of the release are the section of the version in `CHANGELOG.md`.

The repository has immutable releases. After the publication, nobody can change the assets or the tag of a release. To correct a release, publish a new version.

The binary of this tree is a tree binary ([docs/diagnostics.md](docs/diagnostics.md#files-that-sd503-counts)). SD503 does not count a change of the pin line of `spec-dna.toml` in such a binary, so the pin moves with the version. The exact pin rule stays: between a commit of the version and a commit of the pin, `check` stops with exit code 3. Make one commit for both.

## Dependencies

spec-dna has 4 direct dependencies. Give each new dependency a one-line reason in the table below.

| Crate | Reason |
| --- | --- |
| `pulldown-cmark` | CommonMark parser with byte offsets for headings, list items and links. Served sections and requirement keys need these offsets. |
| `sha2` | SHA-256 for the normalized section and requirement hashes of ADR 0001 |
| `toml` | Parses `spec-dna.toml` (TOML 1.0 syntax) |
| `unicode-normalization` | Unicode NFC: the definitions of ids and hashes use it |

These crates come with the direct dependencies:

| Crate | Required by, and reason |
| --- | --- |
| `bitflags`, `memchr`, `unicase` | `pulldown-cmark`: option flags, fast byte search, link labels without case |
| `digest`, `block-buffer`, `crypto-common`, `generic-array`, `typenum` | `sha2`: the shared interfaces of hash functions, and buffers of fixed size |
| `cpufeatures`, `libc`, `cfg-if` | `sha2`: detection of SHA CPU instructions at run time |
| `serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, `winnow` (2 versions) | `toml`: value model, spans, date values and the parser |
| `tinyvec` | `unicode-normalization`: small buffers for combining characters |

The tests on Unix also use `libc`: `openpty` gives a pseudo-terminal to the test of the interactive `init`.

The repository has its own code, without dependencies, for argument parsing, front matter (a YAML subset), globs, JSON output and input, parallelism and git access. Git access uses the `git` command line ([ADR 0002](docs/adr/0002-language.md)).

### Updates

Dependabot opens grouped pull requests each week for GitHub Actions and pre-commit hooks. It waits 7 days after a release before it proposes that release.

Pin each `uses:` in a workflow to a full commit SHA, with the version tag in a trailing comment. Keep this format when you add or change a workflow, because `zizmor` and Dependabot need it.

## Claude Code agents

`.claude/settings.json` and `.claude/agents/` give Claude Code a team of models and agents. `CLAUDE.md` tells the main session when to use an agent.

- `.claude/agents/README.md` lists the agents, the model of each agent, the model team, and the steps to check it.
- `.claude/agents/ownership.md` gives the responsible agent of each task and each path, the change sets, and the escalation rules.

To change the team, follow `.claude/agents/README.md`, section "Change the team".

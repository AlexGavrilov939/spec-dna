# Contributing

## Setup

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

## Formatting

`.editorconfig` contains the whitespace rules: a 2-space indent by default, 4 spaces for Python, TOML and Rust, and tabs for Go. Use `gofmt` to format Go and `cargo fmt` to format Rust.

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

## Branches

- `master` is the default branch. Merge changes into `master` only through pull requests. The `no-commit-to-branch` hook stops local commits to `master`.
- Give each branch a name of the form `<type>/<short-description>`, for example `feat/spec-template` or `fix/readme-links`.

## Commits

- Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): subject`. The scope is optional.
- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style`, `revert`.
- Write the subject in the imperative, with a lowercase first letter and no period at the end.
- Sign your commits (`git config commit.gpgsign true`).

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

1. In a pull request, set the new version in `Cargo.toml` and `Cargo.lock`. Add its section to `CHANGELOG.md`, in the form `## <version> - <date>`.
2. Merge the pull request when CI passes. CI builds the 7 targets and runs each binary on a runner of its platform.
3. Push a signed tag `v<version>` on that commit of `master`.

The tag starts `.github/workflows/release.yml`. The workflow compares the tag with the version. It builds and runs the binaries again. It publishes the binaries, `SHA256SUMS` and `spec-dna-pin.toml` with build-provenance attestations. The notes of the release are the section of the version in `CHANGELOG.md`.

## Dependencies

Dependabot opens grouped pull requests each week for GitHub Actions and pre-commit hooks. It waits 7 days after a release before it proposes that release.

Pin each `uses:` in a workflow to a full commit SHA, with the version tag in a trailing comment. Keep this format when you add or change a workflow, because `zizmor` and Dependabot need it.

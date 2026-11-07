# Contributing

## Setup

```sh
pre-commit install
```

This installs 2 git hooks. `pre-commit` runs the checks from `.pre-commit-config.yaml` on staged files. `commit-msg` checks the commit message format. CI runs the same checks on every push and pull request.

To run everything by hand:

```sh
pre-commit run --all-files
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```

Golden outputs of the integration tests live in `tests/golden/`. After an intended output change, regenerate them with `UPDATE_GOLDEN=1 cargo test` and review the diff.

## Formatting

`.editorconfig` holds the whitespace rules: 2-space indent by default, 4 spaces for Python, TOML and Rust, tabs for Go. Format Go with `gofmt` and Rust with `cargo fmt`.

## Writing

Write all specifications, documentation and code comments in ASD-STE100 Simplified Technical English (STE). Make the text clear and concise. This rule applies to Markdown files, ADRs, JSON Schema descriptions, comments and doc comments in code and scripts, and the text that `spec-dna init` writes for agents.

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

## Branches

- `master` is the default branch. Changes land through pull requests only. The `no-commit-to-branch` hook blocks local commits to `master`.
- Name branches `<type>/<short-description>`, for example `feat/spec-template` or `fix/readme-links`.

## Commits

- Use [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/): `type(scope): subject`. The scope is optional.
- Types: `feat`, `fix`, `docs`, `refactor`, `perf`, `test`, `build`, `ci`, `chore`, `style`, `revert`.
- Subject: imperative mood, lowercase first letter, no trailing period.
- Sign your commits (`git config commit.gpgsign true`).

## Pull requests

- One logical change per pull request. Fill in the template.
- The PR title must be a Conventional Commit. The `PR title` workflow checks it.
- Pull requests are squash-merged, so the PR title becomes the commit message on `master`.
- Merge only with green CI. If GitHub Actions does not start the jobs (for example, because of account billing), run the same jobs locally, put the results in the pull request, and merge only if all jobs pass.

## Dependencies

Dependabot opens weekly grouped pull requests for GitHub Actions and pre-commit hooks, and waits 7 days after a release before proposing it.

Every `uses:` in a workflow is pinned to a full commit SHA, with the version tag in a trailing comment. Keep that format when you add or edit a workflow, `zizmor` and Dependabot rely on it.

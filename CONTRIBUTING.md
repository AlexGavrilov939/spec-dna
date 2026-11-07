# Contributing

## Setup

```sh
pre-commit install
```

This installs 2 git hooks. `pre-commit` runs the checks from `.pre-commit-config.yaml` on staged files. `commit-msg` checks the commit message format. CI runs the same checks on every push and pull request.

To run everything by hand:

```sh
pre-commit run --all-files
```

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
- Merge only with green CI.

## Dependencies

Dependabot opens weekly grouped pull requests for GitHub Actions and pre-commit hooks, and waits 7 days after a release before proposing it.

Every `uses:` in a workflow is pinned to a full commit SHA, with the version tag in a trailing comment. Keep that format when you add or edit a workflow, `zizmor` and Dependabot rely on it.

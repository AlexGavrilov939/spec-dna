# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

Spec-driven development for AI coding agents.

> [!NOTE]
> Early stage. The repository has the skeleton and the working conventions. The spec format and the tooling are not designed yet.

## Idea

Write the spec first, then let the agent build from it. spec-dna will hold the spec format, templates for typical tasks, and the instructions that make agents such as Claude Code and Codex follow a spec instead of a chat thread.

## Repository layout

| Path | Purpose |
| --- | --- |
| `AGENTS.md` | Rules for AI agents working in this repo. `CLAUDE.md` imports it. |
| `CONTRIBUTING.md` | Branches, commits, pull requests. |
| `.pre-commit-config.yaml` | Checks shared by local git hooks and CI. |
| `.github/` | CI workflows, Dependabot, issue and PR templates, code owners. |

## Development

You need [pre-commit](https://pre-commit.com) 4.x, or [uv](https://docs.astral.sh/uv/) to run it without installing (`uvx pre-commit ...`).

```sh
pre-commit install           # git hooks: checks on commit, commit message format
pre-commit run --all-files   # every check, same as CI
```

## License

[Apache-2.0](LICENSE)

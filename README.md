# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

spec-dna keeps engineering specifications small, linked and machine-checked. It serves the minimum sufficient context to AI coding agents. It works in any git repository, with any stack and any agent.

> [!NOTE]
> The project is at an early stage: version 0.1.0 is not released. The information model is in [ADR 0001](docs/adr/0001-information-model.md).

## Install

This is phase 1 of [ADR 0003](docs/adr/0003-distribution.md): build from a tag with the locked dependency versions.

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v<version> spec-dna
```

The project pins the version in `spec-dna.toml`. `check` does not run with a binary of a different version.

## Use

```sh
spec-dna init                       # spec-dna.toml (if absent) and the agent adapters
spec-dna check                      # new diagnostics against HEAD; exit 1 on a new error
spec-dna check --base origin/master # in CI: new diagnostics against the merge base
spec-dna check --all                # every diagnostic
spec-dna context auth/session --path src/gateway.rs
spec-dna context auth/session --query "renew a session after a password change"
spec-dna show auth/session#token-expiry
spec-dna mv auth/session auth/sessions          # the old id becomes an alias
spec-dna mv auth/session#token-expiry auth/tokens
```

Every command accepts `--json`. spec-dna does not use the network. It reads documents through local git plumbing.

| Command | What it does |
| --- | --- |
| `init [--root <dir>]` | Writes `spec-dna.toml` if it is missing. Generates the adapters: a block in `AGENTS.md`, an `@AGENTS.md` import in `CLAUDE.md` (only if the file has no such import), and the skill `.claude/skills/spec-dna/SKILL.md`. Each adapter has the generator version and a hash. `check` reports a manual edit or an old version. A second run updates the adapters. |
| `check [--base <rev>] [--all]` | Runs the controls of ADR 0001 on the working tree. Without `--all`, it reports only the diagnostics that are new against the base. The base is the merge base of `<rev>` and `HEAD`, or `HEAD` itself. Codes: [docs/diagnostics.md](docs/diagnostics.md). |
| `context <target>... [--path <file>]... [--budget <chars>] [--query <text>]` | Prints the sections that an agent needs: the entry documents (or sections), documents whose `applies-to` matches an entry or a path, `depends-on` targets at depth 1, linked accepted decisions as summaries, then the rest. If a section does not fit the budget, the command lists its id, hash and size. Every section has its id and hash. With `--query` (the task in plain words), the sections that match its words come first. BM25 ranks them, without a network or a model. This option is for long documents without requirement lists. |
| `show <id>[#<anchor>]` | Prints a document, a section with its subsections, or one requirement, with ids and hashes. |
| `mv <from> <to>` | Moves a document (`git mv`), adds the old id to its `aliases`, and rewrites relative links to the document. `mv <doc-id>#<key> <doc-id>` moves one requirement and adds an inline `<!-- aliases: ... -->` marker. |

A target is a document id, an alias such as `ADR-12`, `<id>#<anchor or key>`, or a repository path.

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. For `check`: no new error (with `--all`: no error) |
| 1 | `check` found a new error (with `--all`: any error) |
| 2 | Usage error |
| 3 | Configuration error: `spec-dna.toml` is missing or not valid, the version pin does not match, the base revision is unknown, or the directory is not a git repository |
| 4 | Git or file system error |
| 5 | A target names no document, section or requirement |

### Index

spec-dna keeps parsed documents in a cache at `<git-dir>/spec-dna/index-v1.bin`, with the git blob id as the key. Thus, the cache never goes into a commit, and it stays after a branch switch. spec-dna parses files that differ from the git index, and untracked files, on every run. It writes the cache to a temporary file and then renames that file into place. You can delete the cache at any time, or pass `--no-cache`.

### Uninstall

Delete these items:

- `spec-dna.toml`
- the generated `AGENTS.md` block
- the `CLAUDE.md` import, if `init` added it
- `.claude/skills/spec-dna/`
- `<git-dir>/spec-dna/`

The documents are plain Markdown files. They do not change.

## Dependencies

Direct dependencies, with the reason for each:

| Crate | Why |
| --- | --- |
| `pulldown-cmark` | CommonMark parser with byte offsets for headings, list items and links. Served sections and requirement keys need these offsets. |
| `sha2` | SHA-256 for the normalized section and requirement hashes of ADR 0001 |
| `toml` | Parses `spec-dna.toml` (TOML 1.0 syntax) |
| `unicode-normalization` | Unicode NFC: the definitions of ids and hashes use it |

Transitive dependencies of the crates above:

| Crate | Required by, and why |
| --- | --- |
| `bitflags`, `memchr`, `unicase` | `pulldown-cmark`: option flags, fast byte search, case-insensitive link labels |
| `digest`, `block-buffer`, `crypto-common`, `generic-array`, `typenum` | `sha2`: the shared hash-function interfaces and fixed-size buffers |
| `cpufeatures`, `libc`, `cfg-if` | `sha2`: runtime detection of SHA CPU instructions |
| `serde_core`, `serde_spanned`, `toml_datetime`, `toml_parser`, `winnow` (2 versions) | `toml`: value model, spans, date values and the parser |
| `tinyvec` | `unicode-normalization`: small buffers for combining characters |

This repository has its own code, without dependencies, for argument parsing, front matter (a YAML subset), globs, JSON output, parallelism and git access. Git access uses the `git` command line ([ADR 0002](docs/adr/0002-language.md)).

## Repository layout

| Path | Purpose |
| --- | --- |
| `src/` | The `spec-dna` crate |
| `tests/` | Integration tests, fixtures and golden outputs (`UPDATE_GOLDEN=1 cargo test` regenerates them) |
| `schema/spec-dna.schema.json` | JSON Schema of `spec-dna.toml`; [docs/config.md](docs/config.md) describes it |
| `docs/adr/` | Decisions: information model, language, distribution |
| `docs/research/` | Standards and prior art |
| `bench/` | Benchmarks and retrieval measurements; prototypes from the language decision |
| `AGENTS.md` | Rules for AI agents that work in this repository. `CLAUDE.md` imports it |
| `CONTRIBUTING.md` | Branches, commits, pull requests |
| `.github/` | CI workflows, Dependabot, issue and PR templates, code owners |

## Development

You need Rust 1.88 or later, and [pre-commit](https://pre-commit.com) 4.x (or [uv](https://docs.astral.sh/uv/) to run it with `uvx pre-commit ...`).

```sh
cargo test                   # unit and integration tests
pre-commit install           # git hooks: checks on commit, commit message format
pre-commit run --all-files   # every check, same as CI
```

## License

[Apache-2.0](LICENSE)

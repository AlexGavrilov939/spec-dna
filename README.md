# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

spec-dna keeps engineering specifications small, linked and machine-checked. It serves the minimum sufficient context to AI coding agents. It works in any git repository, with any stack and any agent.

> [!NOTE]
> The project is at an early stage. Version 0.3.0 adds hygiene functions: a split of every document type, history extraction, generated indexes and stubs. It is a local tag, not a public release yet. [CHANGELOG.md](CHANGELOG.md) lists the changes. The information model is in [ADR 0001](docs/adr/0001-information-model.md).

## Install

This is phase 1 of [ADR 0003](docs/adr/0003-distribution.md): build from a tag with the locked dependency versions.

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v<version> spec-dna
```

The project pins the version in `spec-dna.toml`. `check` and `init` do not run with a binary of a different version. [docs/quickstart.md](docs/quickstart.md) gives the steps from install to uninstall, with a CI job and the upgrade.

## Use

```sh
spec-dna init                       # detection, 8 questions, a preview, then spec-dna.toml and the adapters
spec-dna init --dry-run             # the preview only (also --non-interactive, --answers <file>, --reconfigure)
spec-dna new decision psp-choice    # a new document from the template of its type
spec-dna context src/gateway.rs     # the sections that cite a code file
spec-dna check                      # new diagnostics against HEAD; exit 1 on a new error
spec-dna check --base origin/master # in CI: new diagnostics against the merge base
spec-dna check --all                # every diagnostic
spec-dna check --lint               # also list each prose lint warning
spec-dna context auth/session --path src/gateway.rs
spec-dna context auth/session --query "renew a session after a password change"
spec-dna show auth/session#token-expiry
spec-dna mv auth/session auth/sessions          # the old id becomes an alias
spec-dna mv auth/session#token-expiry auth/tokens
spec-dna mv auth/session#renewal auth/session#token-expiry auth/tokens   # several keys
spec-dna mv auth/session#mfa auth/second-factor  # a whole section into a new spec
spec-dna mv guides/deploy#rollback runbooks/rollback --type runbook   # a section of a guide into a new runbook
spec-dna history decisions/cache docs/history/cache.md   # the amendment and history sections into a reference document
spec-dna index docs/incidents --replace         # a generated index in place of a hand-kept registry
spec-dna mv guides/old guides/new --stub        # a stub at the old path for tools that read by path
spec-dna transcript ~/.claude/projects/<project> --expect auth/session
spec-dna --help                     # commands and options (-h); --version (-V) prints the version
spec-dna mv --help                  # the help of one command
```

Every command accepts `--json`. spec-dna does not use the network. It reads documents through local git plumbing.

| Command | What it does |
| --- | --- |
| `init [--non-interactive \| --answers <file>] [--reconfigure] [--dry-run] [--root <dir>]` | Detects the repository: document roots and types, ids, requirement keys, statuses, heading languages, vendored, generated and large reference directories, budgets, stack, tests, agent files, CI, hooks and the default branch. Asks at most 8 questions with the detected defaults, shows the full `spec-dna.toml` and the files to create or change, and writes them after a confirmation. It never edits a document. Without a terminal it uses the defaults. If `spec-dna.toml` exists, it updates only the adapters, unless `--reconfigure` is set. [docs/init.md](docs/init.md) gives the details. Generates the adapters: one line in `AGENTS.md` and one line in `CLAUDE.md`, and the skill `.claude/skills/spec-dna/SKILL.md`. The line never imports another file. `AGENTS.md` gets the line if the file exists, or if neither file exists. `CLAUDE.md` gets the line, unless it imports such an `AGENTS.md`. Each adapter has the generator version and a hash. `check` reports a manual edit or an old version. A second run updates the adapters and replaces the multi-line blocks of earlier versions. `init` prints the bytes that the adapters add to the prompt prefix of each agent request (`--json`: `prefix_bytes`), and the `git add` command for the files that it wrote. It does not run if the pin in `spec-dna.toml` is a different version. |
| `check [--base <rev>] [--all] [--lint]` | Runs the controls of ADR 0001 on the working tree. Without `--all`, it reports only the diagnostics that are new against the base. The base is the merge base of `<rev>` and `HEAD`, or `HEAD` itself. `--all` uses the same base, but it reports every diagnostic and fails on any error. Without `--base`, `check` also reads untracked documents that git does not ignore. With `--base`, it reads tracked files only, as CI does. `--json` also lists each document with its number of recognized requirements. If the change removes or renames a path or an identifier that a document cites, `check` gives a warning that names the document (SD606, SD607). In an accepted decision, an amendment section that cites the new path or identifier clears these warnings. The text output lists the errors first, then SD606 and SD607, then the other warnings. It gives one line with a count for each prose lint code (SD801 to SD805); `--lint` lists each of these warnings. Then it gives the fix of each listed code once. The JSON output lists all diagnostics. The profile sets the severity of each code ([docs/profiles.md](docs/profiles.md)). Codes: [docs/diagnostics.md](docs/diagnostics.md). |
| `context <target>... [--path <file>]... [--budget <chars>] [--query <text>] [--include-inactive]` | Prints the sections that an agent needs. First come the entry documents (or sections). For a code path (a target or `--path`, also an untracked file), the sections that cite the file by its path or by an identifier that occurs in the file come next ([docs/diagnostics.md](docs/diagnostics.md#code-citations)). No `applies-to` is necessary. The requirement sections and the summary of the best citing documents follow. Then come the documents whose `applies-to` matches an entry or a path, and the sections that cite only a directory of the path. The command serves at most 8 citing documents and 5 summaries; it lists the other citing sections. Then come the `depends-on` targets at depth 1, the linked accepted decisions as summaries, and the rest. A citing change or note is only listed. In a document with requirements, the requirement sections come first, then the title section; the other sections are listed. A superseded, retired or rejected document is listed with its status and successor, but not served, unless a target names it or `--include-inactive` is set. If a section does not fit the budget, the command lists its id, hash and size. Every section has its id and hash. With `--query` (the task in plain words), the sections that match its words come first. BM25 ranks them, without a network or a model. This option is for long documents without requirement lists. |
| `show <id>[#<anchor>]` | Prints a document, a section with its subsections, or one requirement, with ids and hashes. For a spec or contract, it also prints the number of recognized requirements (`--json`: their keys), and the evidence of each requirement: its `verified-by` paths and the `covers:` tags of the tests. |
| `new <type> <id> [--root <dir>]` | Writes a new document from the template of its type ([docs/types.md](docs/types.md)). In a mixed root, an id without a namespace gets the namespace of its type, such as `decisions/<id>`. |
| `mv <from> <to> [--stub]` | Moves a document (`git mv`), adds the old id to its `aliases`, and rewrites relative links to the document. `--stub` leaves a stub at the old path (front matter `moved-to`, no id, no budget) for tools that read a file by its path. |
| `mv <doc-id>#<key or anchor>... <doc-id or path> [--type <type>]` | Splits a document of any type: moves requirements, or whole sections with their subsections, into an existing document of any type, or into a new document that it creates with front matter and a title (the type of its root, `--type`, `reference` for the history of a decision, or the type of the source). Requirements move only into a spec, contract or invariant. Each moved section of `HEAD` keeps its old id as an alias (`<old-id>#<anchor>` in the `aliases` of the destination). The rows of `companion-sections` (such as an acceptance map) move with their requirements; a row of a key that moves and a key that stays refuses the move. A move into a reference document leaves a one-line pointer in place of each section. A decision keeps its frozen text: only its amendment and history sections move, into a reference document, and `mv` lists the moved amendments that can change the outcome as candidates for a superseding decision. Each moved requirement whose id is in `HEAD` gets an inline `<!-- aliases: ... -->` marker with that id. A key that is new in the uncommitted change gets no alias. Links to a moved key or section follow it, also in accepted decisions, where a link destination is not frozen text. First, a preservation check compares the result with the current tree: the same requirement hashes (without link destinations), every requirement id resolves to the same requirement, no change of the frozen text of an accepted decision, and no new error. If a check fails, `mv` changes nothing, names each requirement whose text would change, and exits with code 1. |
| `history <doc-id> [<to>]` | Moves the history sections of a document (`history-sections`, and the amendment sections of a decision) into a reference document, with a pointer for each section and a backlink. The default destination is `<name>-history.md` next to the document, in a mixed root. |
| `index <dir or file> [--replace]` | Writes a generated index of the documents under a directory: one row for each entry with its name, title and status. `--replace` replaces a hand-kept file; links to a section of the old file that names an entry go to the entry. `check` reports a stale or hand-edited index (SD610). |
| `transcript <file or dir>... [--expect <id>]... [--format <name>]` | Reads agent transcripts (Claude Code JSONL first). For each session and episode, it reports input, cache-creation, cache-read and output tokens, tool calls, and the documents and sections that the agent read. It prints counts and ids only, never prompt, command or file text. See [docs/transcript.md](docs/transcript.md). |

A target is a document id, an alias such as `ADR-12`, `<id>#<anchor or key>`, or a repository path.

### Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. For `check`: no new error (with `--all`: no error) |
| 1 | `check` found a new error (with `--all`: any error), or the preservation check of `mv`, `history` or `index` refused the change |
| 2 | Usage error |
| 3 | Configuration error: `spec-dna.toml` is missing or not valid, the version pin does not match, the base revision is unknown, or the directory is not a git repository |
| 4 | Git or file system error |
| 5 | A target names no document, section or requirement |

### Index

spec-dna keeps parsed documents in a cache at `<git-dir>/spec-dna/index-v1.bin`, with the git blob id as the key. Thus, the cache never goes into a commit, and it stays after a branch switch. spec-dna parses files that differ from the git index, and untracked files, on every run. It writes the cache to a temporary file and then renames that file into place. You can delete the cache at any time, or pass `--no-cache`.

### Uninstall

Delete these items:

- `spec-dna.toml`
- the generated line in `AGENTS.md` and in `CLAUDE.md` (it ends with `<!-- spec-dna v<version> h:<hash> -->`)
- `.claude/skills/spec-dna/`
- `<git-dir>/spec-dna/`
- the binary: `cargo uninstall spec-dna`

The documents are plain Markdown files. They do not change. [docs/quickstart.md](docs/quickstart.md#uninstall) gives the commands.

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

Development dependency (tests only, on Unix):

| Crate | Why |
| --- | --- |
| `libc` | `openpty` for the test that runs the interactive `init` on a pseudo-terminal |

This repository has its own code, without dependencies, for argument parsing, front matter (a YAML subset), globs, JSON output and input, parallelism and git access. Git access uses the `git` command line ([ADR 0002](docs/adr/0002-language.md)).

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
| `CHANGELOG.md` | The changes of each version |
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

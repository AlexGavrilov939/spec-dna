# spec-dna

[![CI](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/AlexGavrilov939/spec-dna/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/AlexGavrilov939/spec-dna)](https://github.com/AlexGavrilov939/spec-dna/releases)
[![License: Apache-2.0](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)

spec-dna checks the Markdown specifications of a git repository, and gives coding agents the sections that a task needs. Your documents stay plain Markdown files. spec-dna reads their types, ids, requirements, links and code citations. It does not change a document unless you tell it to move or split one.

In this example, an agent changed the outcome of an accepted decision. `check` stops the change:

```console
$ spec-dna check
docs/decisions/token-store.md:1: error SD405 decision-frozen: the text outside the amendment and live sections changed after the decision was accepted
SD405 fix: Restore the accepted text and write the change in a new amendment section (`amendment-sections`, default `## Amendments`). Only a live section (`live-sections`, such as an implementation status) can change in place. As an alternative, write a decision that supersedes this one.
spec-dna check: 2 documents; 1 new error and 0 new warnings against d8f644fbb546 (1 error and 1 warning in total)
```

Before an agent edits `src/gateway.rs`, `context` gives it the requirement that cites the file:

```console
$ spec-dna context src/gateway.rs
<!-- spec-dna context 0.4.0: src/gateway.rs; 2 sections in full, 0 listed; 264 of 8000 characters; about 67 tokens (estimated) -->

<!-- session#requirements h:a524a6b73357c9f9 spec/active cites -->
## Requirements

- **expiry**: The gateway SHALL reject a session token after 15 minutes. The check is in `src/gateway.rs`.

<!-- session#session h:b73c41331dcdd962 spec/active cites -->
# Session
```

## Install

Each [release](https://github.com/AlexGavrilov939/spec-dna/releases) has binaries for macOS, Linux and Windows. On macOS (Apple silicon):

```sh
v=0.4.0 t=aarch64-apple-darwin
base=https://github.com/AlexGavrilov939/spec-dna/releases/download/v$v
curl -fsSLO "$base/spec-dna-$v-$t" -fsSLO "$base/SHA256SUMS"
shasum -a 256 -c --ignore-missing SHA256SUMS
install -m 755 "spec-dna-$v-$t" ~/.local/bin/spec-dna
```

To build from source, you need Rust 1.88 or later:

```sh
cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v0.4.0 spec-dna
```

[docs/quickstart.md](docs/quickstart.md#install) gives the targets of the other platforms, the commands for Windows, and the verification of the attestations. You also need git.

## Start

In the root of your repository:

```sh
spec-dna init         # find the documents, ask at most 8 questions, then write spec-dna.toml and the agent adapters
spec-dna check --all  # show every diagnostic once
spec-dna check        # before a commit: exit code 1 on a new error
```

`init` writes one line into each of `AGENTS.md` and `CLAUDE.md`, and one skill for the agent. `spec-dna.toml` pins the version: a binary of another version does not run the gate. [docs/quickstart.md](docs/quickstart.md) gives each step, the CI job, the upgrade and the uninstall. [docs/commands.md](docs/commands.md) gives each command.

## Status

spec-dna is at version 0.x. A minor version can change the configuration keys and the diagnostics. [CHANGELOG.md](CHANGELOG.md) lists each change.

In agent experiments, 2 blind graders checked each run. Agents with the gate kept accepted decisions intact in 15 of 15 runs, against 6 of 15 runs without it. The agents also read more tokens with spec-dna than without it. [docs/evidence.md](docs/evidence.md) gives the method, the other results and the limits.

## Documentation

- [Quickstart](docs/quickstart.md): install, setup, CI job, upgrade, uninstall
- [Commands](docs/commands.md): each command, its options and the exit codes
- [init](docs/init.md): what `init` finds and asks
- [Configuration](docs/config.md): each key of `spec-dna.toml`
- [Document types](docs/types.md): the types, their statuses and templates
- [Diagnostics](docs/diagnostics.md): each diagnostic code and its fix
- [Profiles](docs/profiles.md): the severity of each code in each profile
- [Transcripts](docs/transcript.md): token and read counts from agent sessions
- [Evidence](docs/evidence.md): the experiments and their limits
- [Decisions](docs/adr/): the information model, the language and the distribution

## Contributing

spec-dna is written in Rust. It has 4 direct dependencies, has no network code, and runs `git` to read the repository. [CONTRIBUTING.md](CONTRIBUTING.md) gives the setup, the rules for commits and pull requests, the dependencies and the release steps. To report a vulnerability, follow [SECURITY.md](SECURITY.md).

## License

[Apache-2.0](LICENSE)

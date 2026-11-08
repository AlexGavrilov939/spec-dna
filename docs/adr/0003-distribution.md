---
status: accepted
---

# 0003. Distribution

## Context and Problem Statement

The same spec-dna must serve many independent repositories. A project must continue to work when the central spec-dna repository is not available. Updates of the standard must have versions, and they must be reviewable, reproducible and reversible. Project knowledge stays in the consumer repository, and the central repository never stores the specifications of another project. How does spec-dna reach a project, and how does a project pin and upgrade it?

## Considered Options

1. Filesystem symlinks to a central checkout
2. Git submodule of the spec-dna repository
3. Git subtree, or vendored copies of the standard text
4. Versioned release artifacts: a native binary with the standard embedded, pinned per project
5. Package managers only (Homebrew, crates.io, npm or pip wrappers)
6. A container image

Generated agent adapters with a pinned generator version are not a channel. They are part of every option, and the decision below includes them.

Assessment (H good, L poor; the judgment of this project, not measurements):

| Option | Setup | Reproducible pin | Portability | Git and CI | Offline | Update and rollback | Ergonomics | Consistency across projects | Trust boundary |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1 Symlinks | H on one machine | L: follows the checkout | L: breaks on Windows, CI and new clones | L | M | L: global, not per project | M | L: every project changes at the same time | L: shared mutable state |
| 2 Submodule | M | H | M | L: recursive clones, CI credentials, detached HEAD | H after clone | M | L | M | M |
| 3 Subtree or vendored text | M | H | H | M: large noisy diffs | H | L: merges against local edits | M | L: copies drift | M |
| 4 Release binary, pinned | H: one command | H: version and checksums committed | H: static binaries per OS | H: CI installs the pin | H: no network in normal use | H: change the pin, revert the commit | H | H | H: checksums in the repository, build provenance |
| 5 Package managers only | H | M: floating versions by default | M: one manager per OS | M | H after install | M | H | M | M |
| 6 Container image | M | H | M: needs a container runtime | H | M | H | L: start-up cost in agent hooks | H | H |

## Decision Outcome

Primary model: option 4, delivered in 2 phases.

| Phase | Starts | Delivery | Pin in `spec-dna.toml` |
| --- | --- | --- | --- |
| 1 | Before the first consumer | `cargo install --locked --git <spec-dna repository> --tag v<version> spec-dna`, from a signed git tag | The version only |
| 2 | Before a second consumer or a public release | Release artifacts for the 7 targets, per-target SHA-256 sums in the pin, build-provenance attestations, the dispatch shim | The version and the per-target sums |

spec-dna enforces the pin in both phases: a binary whose version differs from the pin refuses to run a gate. Checksum verification starts in phase 2. The bullets below describe the target state after phase 2.

- spec-dna ships as versioned release artifacts with SHA-256 checksums and build-provenance attestations. The artifacts are static binaries for macOS (arm64, x86-64), Linux (x86-64, arm64, musl) and Windows (x86-64, arm64). The binary embeds the standard (rules, schemas, type definitions, diagnostic codes), so one version number names both.
- Each project pins the exact version in `spec-dna.toml`. From phase 2, it also pins the per-target SHA-256 sums there. `spec-dna upgrade` writes these values. The project commits the sums, so a user can verify a binary offline against the repository, and not against the host that served it.
- spec-dna enforces the pin. A binary whose version differs from the pin refuses to run a gate and prints the install command. Side-by-side versions are in a per-version directory behind a small dispatch shim that reads the pin. They can also be in a tool manager such as mise or aqua. CI installs through an action pinned by commit SHA and verifies the committed checksum. After CI restores the binary from its cache, it verifies the checksum again.
- `spec-dna init` writes the configuration. It proposes a layout for a new project, or it maps existing paths of a mature project ([ADR 0001](0001-information-model.md)). It also generates the agent adapters: an `AGENTS.md` block, a `CLAUDE.md` import line, one skill that calls the CLI, and a namespace-level index. Each adapter contains the generator version and a content hash. `check` reports a hand-edited or outdated adapter, and `spec-dna upgrade` regenerates it as a reviewable diff.
- Normal operation needs no network. If the central repository or the release host is not available, an installed binary and the project files are sufficient.
- Upgrade: `spec-dna upgrade --to <version>` changes the pin, the sums and the adapters in one reviewable commit. A new rule starts as a warning in a minor release and becomes an error only in a major release. A fix to an existing error rule can still change results in a minor release. The release notes list such fixes. Rollback: revert the commit.
- Uninstall: delete the binary, `spec-dna.toml` and the generated adapters. Project knowledge is plain Markdown and stays.

Build from source (the phase 1 channel, and later an alternative): `cargo install --locked` from the tag. This channel is for the first consumer, for a platform without a prebuilt binary, or for a policy that forbids downloaded binaries. The build uses the same source and the same locked dependency versions. But its result is not byte-identical to a release binary, so after phase 2 the committed sums do not apply. Such users disable checksum verification explicitly.

Package managers (Homebrew tap, crates.io) are convenient ways to install option 4. The pin in `spec-dna.toml` stays authoritative. Work on thin npm or pip wrappers waits until a consumer needs one.

Rejected:

- Symlinks: they are machine-local and unpinned, and they do not work on Windows and in CI. They also make the central checkout into shared mutable state for every project.
- Submodules and subtrees: they copy or mount the standard text into each project. This creates a second source of truth and noisy merges. They also still need the binary.
- Container image as the primary model: it adds a start-up cost on every agent hook call, and it needs a container runtime on developer machines.

### Consequences

- Good: one command to install, an exact and enforced pin per project, offline verification and operation, and reversible upgrades. On removal, there is nothing to delete from project knowledge.
- Good: the central repository holds only the tool and the standard.
- Bad: a release pipeline for 7 targets with checksums and attestations must exist before a second consumer adopts the tool or the first public release. The dispatch shim must also exist by then. Until then, the first consumer builds from source and trusts the tag, not a checksum.
- Bad: the standard cannot change without a release. This limit is intentional, because it keeps the standard reviewable and reproducible.

## Amendments

### 2026-11-07: Windows ABI

The owner selected the GNU ABI for the Windows binaries: `x86_64-pc-windows-gnu` and `aarch64-pc-windows-gnullvm`. One host builds them with zig, as it builds the other 5 targets. This build does not need the Microsoft SDK or a Windows runner. No test ran the Windows binaries on Windows yet.

### 2026-11-08: one build target in phase 1

The owner ruled that phase 1 needs a build for one platform only: `aarch64-apple-darwin`, the platform of the first consumer. CI builds only this target. `scripts/cross-build.sh --all` keeps the 7 targets of phase 2, so phase 2 needs no new build recipe. ADR 0002 recorded the proof that one Linux host builds all 7 targets.

### 2026-11-08: adapters and upgrade in phase 1

The owner gave these rulings after checkpoint 4.

- Adapters. This rule replaces "an `AGENTS.md` block, a `CLAUDE.md` import line" in "Decision Outcome". `init` writes one line into each root instruction file, and one skill ([ADR 0001](0001-information-model.md), amendment "code citations"). The line never imports another file, so it adds only its own bytes to each request.
- Upgrade in phase 1. `spec-dna upgrade` does not exist yet. An upgrade has 4 steps: install the new version from its tag, set the pin in `spec-dna.toml`, run `spec-dna init`, and commit the result. A rollback installs the old version and reverts that commit. `init` does not run if the pin and the binary have different versions. Thus it cannot write adapters of the wrong version. The message of a pin mismatch gives the command for each direction.

### 2026-11-08: install source

The owner approved this rule after checkpoint 5. In phase 1, a project can install spec-dna from a local or a private git clone. The optional key `install-source` in `spec-dna.toml` gives that git URL. The message of a pin mismatch and the CI job of the quickstart use it. Without the key, they use the public repository.

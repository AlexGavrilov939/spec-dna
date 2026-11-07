---
status: accepted
---

# 0002. Implementation language

## Context and Problem Statement

The brief prefers Rust or Go and asks for a choice from evidence, not preference. The tool must start fast, scan fast cold and warm, process changes incrementally, stay small in memory, and ship as one native binary on macOS, Linux and Windows.

To get evidence, the same minimal scanner was written in both languages ([bench/](../../bench/README.md)). It walks a tree, parses front matter, links and id tokens, and resolves them. Both use the standard library only, the same algorithm, the same worker-pool parallelism and the same index format. A test oracle keeps them honest: on every corpus both print identical counts; on synthetic corpora the counts equal the generator's ground truth; and the index build, warm and incremental runs agree with the full scan.

An independent review of the first run found prototype artifacts that favored Rust (Go copied every file buffer, Rust deep-cloned cached entries, the Go binary was not stripped, Go always ran first). They were fixed before the run reported here.

## Considered Options

1. Go
2. Rust

## Measurements

Hardware: Apple M4 Max, 16 cores, 128 GB, macOS 26.6.1, APFS SSD. Toolchains: go1.25.4 darwin/arm64 (`-trimpath -ldflags=-s -w`); rustc 1.99.1 (release, LTO, 1 codegen unit). Measured 2026-11-07.

Method: `bench/run.py` exports a snapshot of the corpus (`git archive` for a repository), then runs each mode 15 times after 1 warm-up, and startup 20 times. Go and Rust alternate run by run, and the one that goes first alternates too. Wall time and peak RSS come from `wait4` per process. Cells show the median, with the minimum in brackets, in milliseconds.

- Startup: `scan --version`.
- Full scan: no index. The OS file cache is warm; dropping it needs root on macOS, so a true cold cache was not measured.
- Index build: full scan that also writes the index.
- Warm: index present, nothing changed. Walk, stat, load the index, resolve.
- Incremental: 10 documents changed since the index was written.

The host was shared with other work. The 1-minute load average on 16 cores is in the first table. Treat differences under about 20% as ties.

| Corpus | Docs | Files | MB | Load (1 min avg) | Equal / modes agree |
| --- | ---: | ---: | ---: | --- | --- |
| Synthetic small | 60 | 81 | 0.5 | 7.36 to 6.85 | yes / yes |
| Synthetic medium | 1,500 | 1,901 | 11.4 | 6.85 to 6.85 | yes / yes |
| Synthetic large | 20,000 | 24,001 | 159.0 | 18.92 to 22.95 | yes / yes |
| Corpus B | 714 | 2,300 | 22.9 | 22.15 to 20.91 | yes / yes |
| Corpus A | 2,309 | 6,537 | 39.0 | 20.91 to 21.64 | yes / yes |

Corpus A is a mature product repository with mixed languages (anonymized, tracked files at a pinned revision). Corpus B is the public `kubernetes/enhancements` repository at `72907deaa85eae6e208bc95c5f74dd27c1911345`, about 700 proposals with YAML metadata.

| Corpus | Scanner | Startup | Full scan | Index build | Warm | Incremental (10 changed) | Peak RSS full / warm (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Synthetic small | Go | 2.7 (2.4) | 4.5 (4.3) | 5.2 (4.7) | 3.4 (3.2) | 4.1 (3.9) | 10.9 / 4.8 |
| Synthetic small | Rust | 2.3 (2.1) | 4.0 (3.7) | 4.4 (4.1) | 3.2 (2.9) | 3.8 (3.5) | 3.2 / 2.4 |
| Synthetic medium | Go | 2.9 (2.5) | 35.7 (33.8) | 38.4 (36.8) | 13.0 (12.4) | 16.7 (15.6) | 27.8 / 10.4 |
| Synthetic medium | Rust | 2.5 (2.2) | 32.3 (29.1) | 34.6 (29.8) | 13.4 (13.0) | 16.8 (15.9) | 7.0 / 5.9 |
| Synthetic large | Go | 3.3 (2.7) | 703.6 (582.7) | 1145.2 (679.8) | 159.1 (146.6) | 227.9 (171.6) | 237.4 / 52.5 |
| Synthetic large | Rust | 2.8 (2.3) | 711.8 (530.5) | 904.3 (664.8) | 173.0 (154.7) | 235.8 (176.0) | 43.7 / 54.2 |
| Corpus B | Go | 3.7 (3.4) | 73.9 (66.5) | 89.0 (78.8) | 34.1 (32.2) | 53.7 (43.1) | 28.2 / 11.8 |
| Corpus B | Rust | 3.2 (2.8) | 54.3 (41.6) | 65.9 (59.8) | 31.8 (30.6) | 47.7 (42.4) | 11.5 / 7.6 |
| Corpus A | Go | 3.8 (3.3) | 123.1 (96.8) | 103.8 (96.8) | 35.1 (33.0) | 45.0 (39.5) | 78.5 / 12.1 |
| Corpus A | Rust | 3.3 (2.9) | 95.5 (73.8) | 79.8 (74.3) | 31.2 (29.4) | 39.4 (34.4) | 14.2 / 8.3 |

Index size is the same for both: 0.03, 0.68, 9.56, 1.25 and 0.68 MB in table order.

Other measured facts (same host):

| Item | Go | Rust |
| --- | --- | --- |
| Release binary, standard library only, stripped | 1.74 MB | 0.50 MB |
| Clean build of the prototype (empty cache) | 1.6 s | 2.1 s |
| Cross-build to Linux x86-64 from macOS | 1.6 s, no extra setup (`GOOS=linux GOARCH=amd64 CGO_ENABLED=0`) | Works with cargo-zigbuild and zig; see Confirmation |
| Prototype size | 534 lines | 601 lines |

Reproduce:

```sh
cd bench
python3 gen/gen.py --size large --out work/gen/large
python3 run.py --corpus work/gen/large --label large --runs 15 --out results/large.json
git init -q work/enhancements
git -C work/enhancements fetch -q --depth 1 https://github.com/kubernetes/enhancements 72907deaa85eae6e208bc95c5f74dd27c1911345
python3 run.py --corpus work/enhancements --rev FETCH_HEAD --label corpus-b --runs 15 --out results/corpus-b.json
```

Raw results: `bench/results/*.json`. Corpus A results stay with its owner.

## Ecosystem (not measured)

| Need | Go | Rust |
| --- | --- | --- |
| CommonMark parser with source positions | goldmark: block nodes expose `Lines()`, segments whose `Start` and `Stop` slice the source bytes (checked in `ast/ast.go` and `text/segment.go`, 2026-11-07; version not checked) | pulldown-cmark 0.13 (0.13.6 in use), tracks CommonMark 0.31.2, offset iterator; comrak for full GFM |
| YAML front matter | `go-yaml` archived 2025-04-01, continued as `yaml/go-yaml` v4 | `serde_yaml` deprecated; successors yaml-rust2, saphyr |
| TOML configuration | BurntSushi/toml | toml |
| Git trees for base-relative checks | `git` CLI plumbing or go-git | `git` CLI plumbing or gix |
| Release tooling | GoReleaser; trivial cross-compilation | cargo-dist; cross-compilation needs per-target linkers or native runners |

[ADR 0001](0001-information-model.md) restricts front matter to a small YAML subset, so a small own parser is an option in either language.

## Decision Outcome

Chosen option: Rust, by a small margin.

Latency does not decide. Startup is 2 to 4 ms in both. Warm and incremental runs, the path agent hooks use, differ by less than 15% everywhere, with Go slightly ahead on the large synthetic corpus and Rust slightly ahead on the real ones. Full scans were 20 to 27% faster in Rust on corpora A and B, under load average 21, and tied on the large synthetic corpus. Every median on corpora A and B is under 125 ms.

The deciding factors are secondary. The first version also counted the parser; goldmark exposes source byte ranges too, so the parser does not separate the languages and is no longer a criterion.

- Memory. With the default garbage collector, Go's peak RSS on a full scan was 2.5 to 5.5 times Rust's (Corpus A 78.5 against 14.2 MB; large synthetic 237 against 44 MB). On the warm path they are close. Full scans happen in CI, after a branch switch, and in a possible long-lived mode (language server or MCP adapter), where Rust leaves more headroom. Go can narrow the gap with `GOMEMLIMIT`, at the cost of tuning.
- Types. Enums with exhaustive matching fit the lifecycle states and diagnostic codes of ADR 0001, where a missed state is a bug.
- Size. The binary is 3.5 times smaller.

Go wins on build simplicity: cross-compilation needs no setup, and builds are faster. That cost is paid once, in the release pipeline of [ADR 0003](0003-distribution.md).

### Confirmation

Two steps had to pass before acceptance. Both passed on 2026-11-07.

1. CI cross-builds the 7 targets of ADR 0003 without manual steps. `scripts/cross-build.sh` holds the target list and is the only build recipe; `.github/workflows/build.yml` runs it once per target on `ubuntu-24.04` with Rust 1.99.1, zig 0.17.0 (from a community mirror, checked against a pinned SHA-256) and cargo-zigbuild 0.24.0. One Linux host builds all 7: `aarch64-apple-darwin` and `x86_64-apple-darwin` (minimum macOS 11.0, no Apple SDK), `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` (glibc 2.17), `x86_64-unknown-linux-musl` (static), `x86_64-pc-windows-gnu` and `aarch64-pc-windows-gnullvm`. The Windows targets use the GNU ABI so that no MSVC SDK is needed. Proof, measured: on the macOS host the script built all 7 targets of the tool crate in 120 s; the workflow's `run:` steps, copied verbatim, passed in a clean `ubuntu:24.04` container (arm64 in 117 s, and amd64 under emulation in 317 s, for the prototype); the tool's Linux binaries run on Ubuntu 24.04 (the prototype's also on CentOS 7), and its macOS binaries run on macOS 26 (x86-64 under Rosetta). The workflow has not run on GitHub yet, because the branch is not pushed.
2. Git access uses the git command line. [bench/git-access](../../bench/git-access/results.md) performs the inputs of a base-relative check (head listing with blob ids and working-tree changes, base tree listing, blob reads) both ways, with equal output on every run. With git processes started in parallel, gix was 6 to 9 ms faster when 10 blobs are read, and 0 to 26% faster when every Markdown blob is read (synthetic and corpus B, macOS). Most of the difference is process start: `/usr/bin/git` on macOS is an `xcrun` shim that costs 11.3 ms per process, the git binary itself 4.0 ms. gix would add 111 crates, grow the binary from 0.44 to 3.65 MB and the clean build from 2 to about 30 s, and it shipped 4 breaking 0.x releases in 2 months. The CLI also behaves exactly like the user's git.

Profile of the real tool (`SPEC_DNA_TRACE=1`), measured on corpus B in an incremental check with 9 changed documents (median of 9 runs, load average 7): about 43 ms go to the parallel git listings of the working tree and the base tree; loading the index, parsing, building both corpora and diagnosing take about 10 ms together. The warm path is limited by git process work, which is the same in both languages, not by a language cause, so the choice does not flip. Timings of the tool on every corpus: [bench/tool](../../bench/tool/README.md); corpus A results stay with its owner.

The choice flips to Go only if a later profile shows the warm path limited by a language cause. Revisit gix if measurements on Linux or Windows show git process start dominating the warm path.

The owner asked for a rerun of the prototype benchmark when the 1-minute load average is below 4. Other sessions shared the host, and the load stayed between 4.1 and 20 for about an hour. It dropped to 3.2 at 17:59 and the rerun started then, with the same method. The small and medium corpora ran at load 3.4 to 4.4; during the large corpus the load rose again to 13, and corpus B ran at 12 to 13. Medians in ms, minimum in brackets; raw results in `bench/results/rerun-*.json`:

| Corpus | Load (1 min avg) | Scanner | Full scan | Warm | Incremental (10 changed) | Peak RSS full (MB) |
| --- | --- | --- | ---: | ---: | ---: | ---: |
| Synthetic small | 3.44 to 3.44 | Go | 4.7 (4.4) | 3.4 (3.1) | 4.2 (3.8) | 11.2 |
| Synthetic small | 3.44 to 3.44 | Rust | 3.9 (3.7) | 3.2 (3.0) | 3.9 (3.6) | 3.3 |
| Synthetic medium | 3.44 to 4.37 | Go | 38.3 (36.0) | 13.2 (12.4) | 15.5 (14.8) | 27.7 |
| Synthetic medium | 3.44 to 4.37 | Rust | 34.9 (33.4) | 13.7 (13.2) | 15.6 (15.1) | 7.0 |
| Synthetic large | 4.58 to 13.08 | Go | 755.4 (677.6) | 125.0 (119.9) | 146.3 (142.8) | 215.2 |
| Synthetic large | 4.58 to 13.08 | Rust | 765.4 (658.1) | 134.7 (130.1) | 156.4 (152.4) | 43.7 |
| Corpus B | 13.08 to 12.36 | Go | 42.7 (41.4) | 22.6 (21.6) | 26.9 (24.2) | 22.8 |
| Corpus B | 13.08 to 12.36 | Rust | 29.5 (28.3) | 21.1 (20.0) | 24.7 (23.2) | 12.0 |

Both scanners printed identical counts on every corpus, equal to the generator's ground truth on the synthetic ones, and every mode agreed with the full scan. The picture is the one above: warm and incremental runs tie within 10%, Rust's full scan is faster on corpus B (31% less) and tied on the synthetic corpora, and Go's peak RSS on a full scan is 1.9 to 4.9 times Rust's. Corpus A was rerun too and shows the same ordering; its numbers stay with its owner.

### Consequences

- Good: lower memory and binary size; strong typing for the information model; a mature CommonMark parser with offsets.
- Bad: slower builds as dependencies arrive (the tool's clean release build with 4 direct dependencies takes 13 to 20 s per target in the cross-build, measured), and cross-platform release work.
- Follow-up: measure a true cold cache on a Linux CI runner, where dropping the page cache is allowed, and repeat the benchmark on an idle host before tuning.

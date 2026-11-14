---
status: accepted
---

# 0002. Implementation language

## Context and Problem Statement

The brief prefers Rust or Go. It asks for a choice that comes from evidence, not from preference. The tool must start fast and scan fast in cold and warm runs. It must process changes incrementally and use little memory. It must ship as one native binary on macOS, Linux and Windows.

For evidence, the same minimal scanner exists in both languages ([bench/](../../bench/README.md)). Each scanner walks a tree, parses front matter, links and id tokens, and resolves them. Both scanners use only the standard library, the same algorithm, the same worker-pool parallelism and the same index format.

A test oracle checks that both scanners are correct. On every corpus, both scanners print identical counts. On synthetic corpora, the counts are equal to the ground truth of the generator. The index build, warm and incremental runs agree with the full scan.

An independent review of the first run found prototype artifacts that favored Rust. Go copied every file buffer, and Rust deep-cloned cached entries. Also, the build did not strip the Go binary, and Go always ran first. The fixes for these artifacts came before the run that this decision reports.

## Considered Options

1. Go
2. Rust

## Measurements

Hardware: Apple M4 Max, 16 cores, 128 GB, macOS 26.6.1, APFS SSD. Toolchains: go1.25.4 darwin/arm64 (`-trimpath -ldflags=-s -w`); rustc 1.99.1 (release, LTO, 1 codegen unit). The measurement date is 2026-11-07.

Method: `bench/run.py` exports a snapshot of the corpus (`git archive` for a repository). Then it runs each mode 15 times after 1 warm-up, and it runs startup 20 times. Go and Rust alternate run by run, and the scanner that runs first also alternates. Wall time and peak RSS come from `wait4` per process. Cells show the median in milliseconds, with the minimum in brackets.

- Startup: `scan --version`.
- Full scan: no index. The OS file cache is warm. On macOS, a drop of that cache needs root, so the benchmark did not measure a true cold cache.
- Index build: a full scan that also writes the index.
- Warm: the index is present, and nothing changed. The steps are walk, stat, load the index and resolve.
- Incremental: 10 documents changed after the scanner wrote the index.

Other work also used the host. The first table shows the 1-minute load average on 16 cores. Treat differences under about 20% as ties.

| Corpus | Docs | Files | MB | Load (1 min avg) | Equal / modes agree |
| --- | ---: | ---: | ---: | --- | --- |
| Synthetic small | 60 | 81 | 0.5 | 7.36 to 6.85 | yes / yes |
| Synthetic medium | 1,500 | 1,901 | 11.4 | 6.85 to 6.85 | yes / yes |
| Synthetic large | 20,000 | 24,001 | 159.0 | 18.92 to 22.95 | yes / yes |
| Corpus B | 714 | 2,300 | 22.9 | 22.15 to 20.91 | yes / yes |
| Corpus A | 2,309 | 6,537 | 39.0 | 20.91 to 21.64 | yes / yes |

Corpus A is a mature product repository with mixed languages (anonymized, tracked files at a pinned revision). Corpus B is the public `kubernetes/enhancements` repository at `72907deaa85eae6e208bc95c5f74dd27c1911345`. It has about 700 proposals with YAML metadata.

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

The index size is the same for both scanners. In the order of the table, it is 0.03, 0.68, 9.56, 1.25 and 0.68 MB.

Other measured facts (same host):

| Item | Go | Rust |
| --- | --- | --- |
| Release binary, standard library only, stripped | 1.74 MB | 0.50 MB |
| Clean build of the prototype (empty cache) | 1.6 s | 2.1 s |
| Cross-build to Linux x86-64 from macOS | 1.6 s, no extra setup (`GOOS=linux GOARCH=amd64 CGO_ENABLED=0`) | Works with cargo-zigbuild and zig; see Confirmation |
| Prototype size | 534 lines | 601 lines |

To reproduce the measurements:

```sh
cd bench
python3 gen/gen.py --size large --out work/gen/large
python3 run.py --corpus work/gen/large --label large --runs 15 --out results/large.json
git init -q work/enhancements
git -C work/enhancements fetch -q --depth 1 https://github.com/kubernetes/enhancements 72907deaa85eae6e208bc95c5f74dd27c1911345
python3 run.py --corpus work/enhancements --rev FETCH_HEAD --label corpus-b --runs 15 --out results/corpus-b.json
```

The raw results are in `bench/results/*.json`. The results for corpus A stay with its owner.

## Ecosystem (not measured)

| Need | Go | Rust |
| --- | --- | --- |
| CommonMark parser with source positions | goldmark: block nodes expose `Lines()`, segments whose `Start` and `Stop` slice the source bytes (checked in `ast/ast.go` and `text/segment.go`, 2026-11-07; version not checked) | pulldown-cmark 0.13 (0.13.6 in use), tracks CommonMark 0.31.2, offset iterator; comrak for full GFM |
| YAML front matter | `go-yaml` archived 2025-04-01, continued as `yaml/go-yaml` v4 | `serde_yaml` deprecated; successors yaml-rust2, saphyr |
| TOML configuration | BurntSushi/toml | toml |
| Git trees for base-relative checks | `git` CLI plumbing or go-git | `git` CLI plumbing or gix |
| Release tools | GoReleaser; simple cross-compilation | cargo-dist; cross-compilation needs per-target linkers or native runners |

[ADR 0001](0001-information-model.md) limits front matter to a small YAML subset, so a small custom parser is an option in either language.

## Decision Outcome

Chosen option: Rust, by a small margin.

Latency does not decide. Startup is 2 to 4 ms in both languages. Agent hooks use the warm and incremental runs, and these runs differ by less than 15% on every corpus. Go is slightly faster on the large synthetic corpus, and Rust is slightly faster on the real corpora.

Full scans were 20 to 27% faster in Rust on corpora A and B, under load average 21, and tied on the large synthetic corpus. Every median on corpora A and B is under 125 ms.

Secondary factors decide. The first version of this decision also counted the parser. But goldmark also exposes source byte ranges, so the parser does not separate the languages and is no longer a criterion.

- Memory. With the default garbage collector, the peak RSS of Go on a full scan was 2.5 to 5.5 times that of Rust. It was 78.5 against 14.2 MB on Corpus A, and 237 against 44 MB on the large synthetic corpus. On the warm path, the values are close. Full scans occur in CI, after a branch switch, and in a possible long-lived mode (language server or MCP adapter), where Rust leaves more headroom. Go can make the difference smaller with `GOMEMLIMIT`, but you must tune that setting.
- Types. Enums with exhaustive matching fit the lifecycle states and diagnostic codes of ADR 0001, where a missed state is a bug.
- Size. The binary is 3.5 times smaller.

Go is better for build simplicity: cross-compilation needs no setup, and builds are faster. The project pays that cost once, in the release pipeline of [ADR 0003](0003-distribution.md).

### Confirmation

Two steps had to pass before acceptance. Both passed on 2026-11-07.

1. CI cross-builds the 7 targets of ADR 0003 without manual steps. `scripts/cross-build.sh` holds the target list and is the only build recipe. `.github/workflows/build.yml` runs it once per target on `ubuntu-24.04` with Rust 1.99.1, zig 0.17.0 and cargo-zigbuild 0.24.0. Zig comes from a community mirror, and the workflow checks it against a pinned SHA-256.

    One Linux host builds all 7 targets: `aarch64-apple-darwin` and `x86_64-apple-darwin` (minimum macOS 11.0, no Apple SDK), `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu` (glibc 2.17), `x86_64-unknown-linux-musl` (static), `x86_64-pc-windows-gnu` and `aarch64-pc-windows-gnullvm`. The Windows targets use the GNU ABI, so the build does not need an MSVC SDK.

    Measured proof: on the macOS host, the script built all 7 targets of the tool crate in 120 s. The `run:` steps of the workflow, copied verbatim, passed in a clean `ubuntu:24.04` container. For the prototype, this took 117 s on arm64 and 317 s on amd64 under emulation. The Linux binaries of the tool run on Ubuntu 24.04, and the Linux binaries of the prototype also run on CentOS 7. The macOS binaries of the tool run on macOS 26 (x86-64 under Rosetta). The workflow did not run on GitHub yet, because the branch is still local.
2. Git access uses the git command line. [bench/git-access](../../bench/git-access/results.md) runs the inputs of a base-relative check in both ways. These inputs are a head listing with blob ids and working-tree changes, a base tree listing, and blob reads. The output was equal on every run. With git processes in parallel, gix was 6 to 9 ms faster for a read of 10 blobs. It was 0 to 26% faster for a read of every Markdown blob (synthetic and corpus B, macOS).

    Most of the difference comes from process start. On macOS, `/usr/bin/git` is an `xcrun` shim that costs 11.3 ms per process, and the git binary itself costs 4.0 ms. gix would add 111 crates. It would increase the binary from 0.44 to 3.65 MB and the clean build from 2 to about 30 s. It also shipped 4 incompatible 0.x releases in 2 months. The CLI also behaves exactly like the user's git.

This profile of the real tool (`SPEC_DNA_TRACE=1`) comes from an incremental check on corpus B with 9 changed documents. The values are the median of 9 runs, at load average 7. About 43 ms go to the parallel git listings of the working tree and the base tree. The index load, the parse, the build of both corpora and the diagnosis take about 10 ms together.

Git process work limits the warm path, and this work is the same in both languages. A language cause does not limit the warm path, so the choice does not change. [bench/tool](../../bench/tool/README.md) gives the timings of the tool on every corpus. The results for corpus A stay with its owner.

The choice changes to Go only if a later profile shows that a language cause limits the warm path. If Linux or Windows measurements show that git process start is the largest cost of the warm path, examine gix again.

The owner asked for a rerun of the prototype benchmark when the 1-minute load average is below 4. Other sessions shared the host, and the load stayed between 4.1 and 20 for about an hour. The load decreased to 3.2 at 17:59, and the rerun started at that time with the same method. The small and medium corpora ran at load 3.4 to 4.4. During the run of the large corpus, the load increased again to 13, and corpus B ran at 12 to 13.

The table shows the medians in ms, with the minimum in brackets. The raw results are in `bench/results/rerun-*.json`:

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

Both scanners printed identical counts on every corpus. On the synthetic corpora, the counts were equal to the ground truth of the generator. Every mode agreed with the full scan.

The results show the same pattern as the results above. Warm and incremental runs tie within 10%. The full scan of Rust is faster on corpus B (31% less) and ties on the synthetic corpora. On a full scan, the peak RSS of Go is 1.9 to 4.9 times that of Rust. Corpus A also had a rerun, and it shows the same order. Its numbers stay with its owner.

### Consequences

- Good: lower memory and binary size; strong types for the information model; a mature CommonMark parser with offsets.
- Bad: builds become slower as the project adds dependencies. Measured: the clean release build of the tool with 4 direct dependencies takes 13 to 20 s per target in the cross-build. Cross-platform release work is also necessary.
- Follow-up: measure a true cold cache on a Linux CI runner, which allows a drop of the page cache. Before you tune, repeat the benchmark on an idle host.

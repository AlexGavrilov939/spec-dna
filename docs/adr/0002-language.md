---
status: proposed
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
| Cross-build to Linux x86-64 from macOS | 1.6 s, no extra setup (`GOOS=linux GOARCH=amd64 CGO_ENABLED=0`) | Not tried: needs a Linux linker (for example cargo-zigbuild) or native CI runners |
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
| CommonMark parser with source positions | goldmark (version not checked) | pulldown-cmark 0.13.2 (2026-03), tracks CommonMark 0.31.2, offset iterator; comrak for full GFM |
| YAML front matter | `go-yaml` archived 2025-04-01, continued as `yaml/go-yaml` v4 | `serde_yaml` deprecated; successors yaml-rust2, saphyr |
| TOML configuration | BurntSushi/toml | toml |
| Git trees for base-relative checks | `git` CLI plumbing or go-git | `git` CLI plumbing or gix |
| Release tooling | GoReleaser; trivial cross-compilation | cargo-dist; cross-compilation needs per-target linkers or native runners |

[ADR 0001](0001-information-model.md) restricts front matter to a small YAML subset, so a small own parser is an option in either language.

## Decision Outcome

Chosen option: Rust, by a small margin.

Latency does not decide. Startup is 2 to 4 ms in both. Warm and incremental runs, the path agent hooks use, differ by less than 15% everywhere, with Go slightly ahead on the large synthetic corpus and Rust slightly ahead on the real ones. Full scans were 20 to 27% faster in Rust on corpora A and B, under load average 21, and tied on the large synthetic corpus. Every median on corpora A and B is under 125 ms.

The deciding factors are secondary:

- Memory. With the default garbage collector, Go's peak RSS on a full scan was 2.5 to 5.5 times Rust's (Corpus A 78.5 against 14.2 MB; large synthetic 237 against 44 MB). On the warm path they are close. Full scans happen in CI, after a branch switch, and in a possible long-lived mode (language server or MCP adapter), where Rust leaves more headroom. Go can narrow the gap with `GOMEMLIMIT`, at the cost of tuning.
- Types. Enums with exhaustive matching fit the lifecycle states and diagnostic codes of ADR 0001, where a missed state is a bug.
- Parser. pulldown-cmark's offset iterator gives byte ranges for headings and list items, which section serving and requirement keys need.
- Size. The binary is 3.5 times smaller.

Go wins on build simplicity: cross-compilation needs no setup, and builds are faster. That cost is paid once, in the release pipeline of [ADR 0003](0003-distribution.md).

### Confirmation

Before this ADR is accepted:

1. CI cross-builds the 7 targets of ADR 0003 from the prototype without manual steps.
2. A decision on git access (CLI plumbing or gix) is recorded, measured on the base-relative check of ADR 0001.

The choice flips to Go if step 1 cannot be done without manual steps, or if a profile of the real tool shows the warm path limited by a language cause rather than by the shared design.

### Consequences

- Good: lower memory and binary size; strong typing for the information model; a mature CommonMark parser with offsets.
- Bad: slower builds as dependencies arrive (estimated, not measured), and cross-platform release work.
- Follow-up: measure a true cold cache on a Linux CI runner, where dropping the page cache is allowed, and repeat the benchmark on an idle host before tuning.

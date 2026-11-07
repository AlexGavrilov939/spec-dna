# git-access results

Measured 2026-11-07 for [ADR 0002](../../docs/adr/0002-language.md), confirmation step 2. Method and limits: [README.md](README.md). The raw JSON is not committed; rerun the harness to get it.

Host: Apple M4 Max, 16 cores, 128 GB, macOS 26.6.1, APFS SSD, shared with other work. Toolchain: rustc 1.99.1, release profile with LTO, 1 codegen unit, stripped. git 2.50.1 (Apple Git-155), run as `/usr/bin/git` (the `xcrun` shim) and as the Xcode binary it forwards to. gix 0.90.0 (released 2026-10-22, MSRV 1.88).

Cells show the median of 15 runs, with the minimum in brackets, in milliseconds. Every number is measured unless marked estimated. Treat differences under about 20% as ties.

## Corpora

| Corpus | Tracked files | `.md` files | Blob bytes of the base tree (MB) | Load (1 min avg) | Outputs of all modes equal (changed / md) |
| --- | ---: | ---: | ---: | --- | --- |
| Synthetic medium | 1,901 | 1,500 | 11.5 | 7.77 to 7.63 | yes / yes |
| Synthetic large | 24,001 | 20,000 | 159.1 | 7.63 to 7.79 | yes / yes |
| Corpus B | 2,308 | 722 | 56.6 | 7.79 to 7.59 | yes / yes |

Corpus B is the public `kubernetes/enhancements` repository at `72907deaa85eae6e208bc95c5f74dd27c1911345`. Synthetic corpora come from `bench/gen/gen.py`. Each corpus is a fresh repository built by `run.py prepare`: 10 `.md` files changed in a commit after the base, 10 more changed in the working tree, 3 untracked `.md` files.

## Total per check (measured)

Process wall time for steps 1 to 3, including process start and the summary.

| Corpus | Blobs read | cli | cli-direct | cli-par | cli-par-direct | gix |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Synthetic medium | 10 changed `.md` | 64.0 (58.1) | 28.4 (26.8) | 34.9 (32.7) | 15.8 (14.6) | 9.2 (8.5) |
| Synthetic medium | all 1,500 `.md` | 84.9 (80.1) | 48.7 (46.2) | 55.5 (53.8) | 35.4 (34.0) | 28.0 (27.3) |
| Synthetic large | 10 changed `.md` | 118.5 (117.0) | 84.7 (83.3) | 73.9 (69.5) | 60.7 (57.7) | 51.3 (46.0) |
| Synthetic large | all 20,000 `.md` | 383.6 (369.2) | 345.6 (335.3) | 336.0 (325.8) | 325.9 (311.5) | 307.3 (301.9) |
| Corpus B | 10 changed `.md` | 73.9 (72.0) | 39.8 (38.0) | 38.3 (36.4) | 27.0 (25.9) | 21.1 (20.5) |
| Corpus B | all 721 `.md` | 104.8 (103.2) | 71.2 (67.6) | 69.9 (67.0) | 57.8 (55.6) | 57.6 (56.0) |

`cli` and `cli-par` use `git` from `PATH` (the macOS shim); the `-direct` columns use the Xcode git binary. Corpus B has 722 `.md` files but 721 distinct blobs.

## Per step (measured)

Step 1 is head, step 2 is base, step 3 is blobs. In `cli-par` the first two steps overlap, so their time is shown together. "Open" is `gix::open`. Peak RSS is for the bench process and for its largest git child.

### Synthetic medium

| Blobs | Mode | Open | Step 1 | Step 2 | Step 3 | Steps 1 to 3 | Process wall | Peak RSS self / git child (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| changed | cli | | 20.4 (17.9) | 27.4 (25.2) | 12.9 (12.3) | 60.7 (55.4) | 64.0 (58.1) | 3.0 / 6.2 |
| changed | cli-direct | | 10.3 (9.4) | 10.1 (9.4) | 4.7 (4.2) | 25.1 (23.4) | 28.4 (26.8) | 3.1 / 6.3 |
| changed | cli-par | | 30.7 (28.5), steps 1 and 2 | | 1.2 (1.1) | 31.8 (29.7) | 34.9 (32.7) | 3.0 / 6.3 |
| changed | cli-par-direct | | 11.5 (10.5), steps 1 and 2 | | 1.2 (1.1) | 12.6 (11.5) | 15.8 (14.6) | 3.0 / 6.2 |
| changed | gix | 0.3 (0.3) | 4.8 (4.3) | 0.4 (0.4) | 0.2 (0.2) | 5.7 (5.3) | 9.2 (8.5) | 5.9 / none |
| md | cli | | 20.3 (18.6) | 27.7 (26.0) | 33.2 (30.8) | 81.6 (76.9) | 84.9 (80.1) | 3.2 / 9.2 |
| md | cli-direct | | 11.0 (9.8) | 10.1 (9.2) | 24.0 (22.5) | 45.0 (42.8) | 48.7 (46.2) | 3.2 / 9.3 |
| md | cli-par | | 31.5 (30.5), steps 1 and 2 | | 20.3 (19.9) | 52.0 (50.5) | 55.5 (53.8) | 3.1 / 9.1 |
| md | cli-par-direct | | 11.6 (10.6), steps 1 and 2 | | 20.3 (19.6) | 32.0 (31.0) | 35.4 (34.0) | 3.2 / 9.1 |
| md | gix | 0.3 (0.3) | 4.9 (4.4) | 0.4 (0.4) | 18.5 (18.0) | 24.2 (23.6) | 28.0 (27.3) | 9.8 / none |

### Synthetic large

| Blobs | Mode | Open | Step 1 | Step 2 | Step 3 | Steps 1 to 3 | Process wall | Peak RSS self / git child (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| changed | cli | | 62.5 (60.6) | 37.3 (36.0) | 14.3 (13.7) | 113.6 (112.3) | 118.5 (117.0) | 12.5 / 11.0 |
| changed | cli-direct | | 52.8 (51.2) | 21.1 (20.4) | 5.9 (5.7) | 79.8 (78.0) | 84.7 (83.3) | 12.5 / 11.0 |
| changed | cli-par | | 66.7 (62.1), steps 1 and 2 | | 2.3 (2.1) | 68.9 (64.4) | 73.9 (69.5) | 12.8 / 11.0 |
| changed | cli-par-direct | | 53.7 (50.7), steps 1 and 2 | | 2.2 (2.1) | 56.0 (52.9) | 60.7 (57.7) | 15.3 / 11.0 |
| changed | gix | 0.3 (0.3) | 40.1 (34.7) | 4.0 (3.9) | 0.9 (0.9) | 45.5 (39.9) | 51.3 (46.0) | 15.1 / none |
| md | cli | | 64.4 (58.7) | 37.5 (36.2) | 277.1 (263.4) | 378.4 (364.1) | 383.6 (369.2) | 14.2 / 60.5 |
| md | cli-direct | | 56.3 (51.9) | 21.1 (20.2) | 259.8 (256.3) | 340.5 (330.2) | 345.6 (335.3) | 13.0 / 60.4 |
| md | cli-par | | 70.9 (62.8), steps 1 and 2 | | 259.2 (252.8) | 331.2 (320.7) | 336.0 (325.8) | 12.8 / 60.4 |
| md | cli-par-direct | | 54.1 (51.0), steps 1 and 2 | | 260.8 (253.7) | 320.8 (306.6) | 325.9 (311.5) | 15.2 / 60.4 |
| md | gix | 0.4 (0.3) | 42.6 (38.6) | 4.0 (3.9) | 252.9 (251.8) | 301.2 (296.1) | 307.3 (301.9) | 69.8 / none |

### Corpus B

| Blobs | Mode | Open | Step 1 | Step 2 | Step 3 | Steps 1 to 3 | Process wall | Peak RSS self / git child (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| changed | cli | | 31.0 (29.5) | 26.8 (26.4) | 13.0 (12.7) | 70.7 (68.8) | 73.9 (72.0) | 3.2 / 6.7 |
| changed | cli-direct | | 21.4 (20.2) | 10.4 (9.7) | 4.9 (4.4) | 36.6 (35.0) | 39.8 (38.0) | 3.3 / 6.7 |
| changed | cli-par | | 33.7 (31.6), steps 1 and 2 | | 1.7 (1.5) | 35.3 (33.4) | 38.3 (36.4) | 3.4 / 6.7 |
| changed | cli-par-direct | | 22.0 (20.9), steps 1 and 2 | | 1.7 (1.6) | 23.8 (22.6) | 27.0 (25.9) | 3.4 / 6.7 |
| changed | gix | 0.3 (0.3) | 15.4 (15.1) | 0.9 (0.8) | 0.7 (0.6) | 17.4 (17.1) | 21.1 (20.5) | 7.0 / none |
| md | cli | | 32.8 (31.1) | 27.5 (27.0) | 41.3 (40.6) | 101.4 (99.8) | 104.8 (103.2) | 3.5 / 16.0 |
| md | cli-direct | | 24.7 (21.4) | 10.7 (10.2) | 32.6 (32.1) | 67.8 (64.1) | 71.2 (67.6) | 3.6 / 15.8 |
| md | cli-par | | 37.3 (34.3), steps 1 and 2 | | 29.4 (29.1) | 66.7 (63.5) | 69.9 (67.0) | 3.6 / 15.8 |
| md | cli-par-direct | | 25.7 (22.7), steps 1 and 2 | | 29.5 (29.1) | 54.8 (52.1) | 57.8 (55.6) | 3.6 / 16.0 |
| md | gix | 0.3 (0.3) | 17.0 (15.1) | 0.9 (0.8) | 36.0 (35.6) | 54.1 (52.2) | 57.6 (56.0) | 14.8 / none |

gix's own RSS includes the pages of the memory-mapped pack; in the CLI modes that cost sits in the `cat-file` child.

## Cost of a git process (measured)

`git rev-parse HEAD` in the corpus B repository, 30 runs each after 1 warm-up, alternating, load 12.6:

| Executable | Median | Minimum |
| --- | ---: | ---: |
| `/usr/bin/git` (`xcrun` shim) | 11.3 | 10.7 |
| Xcode git binary, called directly | 4.0 | 3.4 |

The shim costs about 7 ms per process here. The tool can skip it by resolving `$(git --exec-path)/git` once and caching the path. Linux has no such shim.

## Build and size (measured)

| Item | CLI only (default features) | gix only (`--no-default-features --features gix`) |
| --- | ---: | ---: |
| Release binary, stripped | 437,648 bytes (0.44 MB) | 3,650,976 bytes (3.65 MB) |
| Dependencies (`cargo tree -e normal,build`, host target, unique crates) | 0 | 111, of which 50 are `gix-*` |
| Packages in `Cargo.lock` (all targets) | 0 | 139 |
| Clean build, empty target directory, sources already downloaded, 2 runs | 2.1 s, 2.3 s | 31.4 s, 28.2 s |

Load (1 min avg) during the builds: 7.2 to 12.4. gix released 0.87.1, 0.88.0, 0.89.0 and 0.90.0 between 2026-08-24 and 2026-10-22 (from the crates.io index); each minor version of a 0.x crate may break its API.

## gix feature choice (measured, exploratory)

7 runs per variant after 1 discarded, interleaved, median process wall in ms, load 8 to 12. All variants printed identical summaries.

| Corpus, blobs | `sha1`, `status`, `revision` | + `parallel` (chosen) | + `max-performance` (`parallel` and pack caches) |
| --- | ---: | ---: | ---: |
| Synthetic large, changed | 73.2 (step 1: 62.4) | 52.4 (step 1: 41.7) | 52.2 (step 1: 41.1) |
| Synthetic large, md | 325.7 | 301.9 | 302.7 |
| Corpus B, changed | 25.4 | 24.5 | 24.5 |
| Corpus B, md | 59.7 | 56.9 | 56.7 |
| Binary size (bytes) | 3,700,832 | 3,650,976 | 3,684,112 |

## Reading

- All 5 modes printed byte-identical summaries on every run of every corpus, for both blob selections.
- Warm path (10 changed blobs): gix is fastest everywhere. The best CLI variant, `cli-par-direct`, is 6.6, 9.4 and 6.0 ms slower (medium, large, corpus B). Through the macOS shim (`cli-par`) the gap is 25.7, 22.6 and 17.3 ms. The sequential `cli` as first specified is 54.8, 67.2 and 52.9 ms slower.
- Cold path (all `.md` blobs): reading and inflating blobs dominates. gix is ahead of `cli-par-direct` by 0.2 ms (0%) on corpus B, 18.7 ms (6%) on the large corpus and 7.4 ms (26%) on the medium one. On corpus B, `git cat-file` alone reads the blobs faster than gix (29.5 against 36.0 ms).
- Step 1 (index and status) takes 4.8, 40.1 and 15.4 ms in gix against 10.3, 52.8 and 21.4 ms in `cli-direct`, which includes about 4 ms of process start. On the large corpus gix's status itself is faster.
- The rest of the CLI gap is likely process start: about 4 ms per direct git process here, with 2 to 3 process lifetimes on the critical path of `cli-par` (estimated, not profiled).
- gix costs 111 crates, an 8.3 times larger binary (3.65 against 0.44 MB) and a 12 to 15 times longer clean build.

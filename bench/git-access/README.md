# git-access

Benchmark for [ADR 0002](../../docs/adr/0002-language.md), confirmation step 2: should the tool read git through the `git` CLI or in-process through the [gix](https://crates.io/crates/gix) crate? It is a prototype, not the spec-dna tool. Results: [results.md](results.md).

| Path | Purpose |
| --- | --- |
| `src/main.rs` | The benchmark binary, modes `cli`, `cli-par` and `gix` |
| `run.py` | Harness: `prepare` builds a test repository from a snapshot, `measure` runs and compares the modes |
| `results.md` | Results for the synthetic corpora and the public corpus. `measure --out` writes the raw JSON, which stays under the ignored `bench/work/` |

## What it measures

One binary reads the git inputs that `check --base <rev>` of [ADR 0001](../../docs/adr/0001-information-model.md) needs on a working tree:

1. Head: every index path with its blob id, the tracked files changed in the working tree, and the untracked files that are not ignored.
2. Base: resolve `<rev>`, find the merge base with `HEAD`, and list every path and blob id of its tree.
3. Blobs: read base blob contents. `changed` reads the `.md` blobs of the base tree whose path is missing from the index or has another id there (10 on the prepared corpora: a warm cache). `md` reads every `.md` blob of the base tree (a cold cache).

Modes:

| Mode | How |
| --- | --- |
| `cli` | `git ls-files -s -z` and `git status --porcelain=v2 -z --untracked-files=all --no-renames` at the same time, then `git merge-base HEAD <rev>`, then `git ls-tree -r -z --full-tree <base>`, then one `git cat-file --batch` with the ids on stdin. Every process runs with `GIT_OPTIONAL_LOCKS=0`, so the index is never written |
| `cli-par` | The same processes, each started as early as its input allows: `ls-files`, `status`, `merge-base` and `cat-file` at once, `ls-tree` when `merge-base` ends |
| `gix` | In-process: `gix::open`, the index, `status` (index against working tree, untracked files listed one by one, renames off), `rev_parse_single`, `merge_base`, a breadth-first tree traversal, `find_object` per blob |

Both CLI modes run `git` from `PATH`, or the executable in `GIT_ACCESS_GIT`. On macOS `/usr/bin/git` is an `xcrun` shim that adds about 7 ms per process (see results), so the harness also runs both CLI modes with the real binary (`cli-direct`, `cli-par-direct`).

Every mode prints the same summary on stdout: counts, the merge base, and checksums of the sorted path and blob id pairs, of the status lists, and of the blob bytes. The harness fails a corpus unless every run of every mode prints identical bytes. Step times and peak RSS (`getrusage`, for the process and for its largest git child) go to stderr.

gix features: `default-features = false` plus `sha1`, `status`, `revision` and `parallel`. The first three are the minimum that compiles the calls above. `parallel` lets `status` check modified files and walk the directory tree on separate threads. The pack caches of `max-performance` gave no measurable gain, so they are left out.

The `cli` feature (default) has no dependencies. Build a CLI-only binary with the default features, and a gix-only binary with `--no-default-features --features gix`.

## Run

You need Rust 1.88+ (edition 2024), git and Python 3.10+. Unix only.

```sh
cd bench
python3 gen/gen.py --size medium --out work/ga/src/medium
python3 git-access/run.py prepare --source work/ga/src/medium --rev - --out work/ga/medium

T=work/target-git-access
cargo build --release --manifest-path git-access/Cargo.toml --target-dir $T
mkdir -p work/ga/bin && rm -f work/ga/bin/git-access-*
cp $T/release/git-access work/ga/bin/git-access-cli
cargo build --release --manifest-path git-access/Cargo.toml --target-dir $T --no-default-features --features gix
cp $T/release/git-access work/ga/bin/git-access-gix

python3 git-access/run.py measure --repo work/ga/medium --label medium \
  --cli work/ga/bin/git-access-cli --gix work/ga/bin/git-access-gix \
  --git-direct "$(git --exec-path)/git" --runs 15 --out work/ga/results/medium.json
```

Remove a binary before copying a new one over it: on macOS, overwriting a signed binary in place gets it killed at its next start.

`prepare` never writes to the source. A git source is exported with `git archive <rev>`; with `--rev -` a directory is copied. In the new repository it commits the snapshot (tagged `bench-base`), changes 10 `.md` files in a second commit, packs with `git gc`, changes 10 other `.md` files in the working tree and adds 3 untracked `.md` files (one in a new nested directory). It then refreshes the index once, after a pause, so no entry is racily clean.

`measure` runs 1 warm-up per mode, then `--runs` runs per mode, interleaved run by run, rotating which mode goes first.

## Limits

- One host (macOS, Apple Silicon), shared with other work. Linux and Windows are not measured; their process start costs differ.
- The OS file cache is warm. Repositories are freshly packed into one pack with a commit graph; loose objects and many packs are not measured.
- The prepared corpora have no ignored files, submodules, sparse checkout, conflicts, fsmonitor or untracked cache. A small hand-made repository with a deletion, a staged-only change, an executable-bit change, ignored files, a symlink and non-ASCII paths gives identical output in both modes, but it is not timed.
- `git status` also compares `HEAD` with the index, which the check does not need. `git diff-files` with `git ls-files --others --exclude-standard` might be cheaper; not measured.
- Step times of `cli-par` cannot be split, because the steps overlap.
- Process wall time includes the summary (sorting and checksums), the same code in every mode.
- gix's peak RSS includes the pages of the memory-mapped pack it touched; for the CLI modes that cost shows up in the git child instead.
- SHA-1 repositories only.

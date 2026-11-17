# git-access

Benchmark for [ADR 0002](../../docs/adr/0002-language.md), confirmation step 2. It compares two ways for the tool to read git: through the `git` CLI, or in-process through the [gix](https://crates.io/crates/gix) crate. It is a prototype, not the spec-dna tool. Results: [results.md](results.md).

| Path | Purpose |
| --- | --- |
| `src/main.rs` | The benchmark binary, with the modes `cli`, `cli-par` and `gix`. |
| `run.py` | The harness. `prepare` builds a test repository from a snapshot. `measure` runs and compares the modes. |
| `walk.sh` | A shell script for the branch walk of `spec-dna check` with `approval = "branch"` ([ADR 0012](../../docs/adr/0012-approval-in-the-branch.md)). See [The walk of the branch](#the-walk-of-the-branch). |
| `results.md` | Results for the synthetic corpora and the public corpus. `measure --out` writes the raw JSON. The JSON stays in `bench/work/`, which git ignores. |

## What it measures

One binary reads the git inputs that `check --base <rev>` of [ADR 0001](../../docs/adr/0001-information-model.md) needs on a working tree:

1. Head: every index path with its blob id, the tracked files changed in the working tree, and the untracked files that git does not ignore.
2. Base: resolve `<rev>`, find the merge base with `HEAD`, and list every path and blob id of its tree.
3. Blobs: read the contents of base blobs. `changed` reads each `.md` blob of the base tree whose path is not in the index or has a different id there. On the prepared corpora, this is 10 blobs: the warm-cache case. `md` reads every `.md` blob of the base tree: the cold-cache case.

Modes:

| Mode | How |
| --- | --- |
| `cli` | `git ls-files -s -z` and `git status --porcelain=v2 -z --untracked-files=all --no-renames` at the same time, then `git merge-base HEAD <rev>`, then `git ls-tree -r -z --full-tree <base>`, then one `git cat-file --batch` with the ids on stdin. Every process runs with `GIT_OPTIONAL_LOCKS=0`, so git never writes the index. |
| `cli-par` | The same processes. Each process starts as early as its input allows: `ls-files`, `status`, `merge-base` and `cat-file` at once, and `ls-tree` when `merge-base` ends. |
| `gix` | In-process. It uses `gix::open`, the index and `status` (index against the working tree, untracked files listed one by one, renames off). It also uses `rev_parse_single`, `merge_base`, a breadth-first tree traversal, and `find_object` for each blob. |

Both CLI modes run `git` from `PATH`, or the executable in `GIT_ACCESS_GIT`. On macOS, `/usr/bin/git` is an `xcrun` shim that adds about 7 ms to each process (see results). Thus, the harness also runs both CLI modes with the real binary (`cli-direct`, `cli-par-direct`).

Every mode prints the same summary on stdout: counts, the merge base and checksums. The checksums are of the sorted path and blob id pairs, of the status lists, and of the blob bytes. The harness fails a corpus unless all runs of all modes print identical bytes. Step times and peak RSS (`getrusage`, for the process and for its largest git child) go to stderr.

gix features: `default-features = false` plus `sha1`, `status`, `revision` and `parallel`. The first three are the minimum set that compiles the calls above. `parallel` lets `status` check modified files and walk the directory tree on separate threads. The pack caches of `max-performance` gave no measurable gain, so the build does not use them.

The `cli` feature (default) has no dependencies. Build a CLI-only binary with the default features. Build a gix-only binary with `--no-default-features --features gix`.

## Run

You need Rust 1.88+ (edition 2024), git and Python 3.10+. The benchmark runs only on Unix.

```sh
cd bench
python3 gen/gen.py --size medium --out work/ga/src/medium
python3 git-access/run.py prepare --source work/ga/src/medium --rev - --out work/ga/medium

T=work/target-git-access
cargo build --release --manifest-path git-access/Cargo.toml --target-dir $T --example git-access
mkdir -p work/ga/bin && rm -f work/ga/bin/git-access-*
cp $T/release/examples/git-access work/ga/bin/git-access-cli
cargo build --release --manifest-path git-access/Cargo.toml --target-dir $T --example git-access --no-default-features --features gix
cp $T/release/examples/git-access work/ga/bin/git-access-gix

python3 git-access/run.py measure --repo work/ga/medium --label medium \
  --cli work/ga/bin/git-access-cli --gix work/ga/bin/git-access-gix \
  --git-direct "$(git --exec-path)/git" --runs 15 --out work/ga/results/medium.json
```

Remove the old binary before you copy a new one to the same path. On macOS, if you overwrite a signed binary, macOS stops it at its next start.

`prepare` never writes to the source. It exports a git source with `git archive <rev>`. With `--rev -`, it copies a directory. In the new repository, it does these steps:

- It commits the snapshot, with the tag `bench-base`.
- It changes 10 `.md` files in a second commit.
- It packs with `git gc`.
- It changes 10 other `.md` files in the working tree.
- It adds 3 untracked `.md` files (one in a new nested directory).
- After a pause, it refreshes the index one time, so that no entry is racily clean.

`measure` runs 1 warm-up for each mode, then `--runs` runs for each mode. The modes alternate in each round, and the first mode of each round changes.

## The walk of the branch

`walk.sh` measures the cost of the walk that `check` runs with `approval = "branch"`. This is not the benchmark of the prototype above: it runs the real `spec-dna` binary.

```sh
cargo build --release
bash bench/git-access/walk.sh [binary] [files] [commits] [dir]
```

With no argument, it uses `target/release/spec-dna`, a fixture of 2,000 files and a branch of 500 commits (2 edited files in each), in a folder from `mktemp -d` that it deletes at the end. The script builds the fixture, then it does these steps:

- It times the git forms that read the paths of the commits: one `diff-tree` for each commit (the form of 0.7.0 part A), one `diff-tree --stdin`, and one `log --name-status`. It also checks that the 2 first forms give the same paths.
- It runs `check --base main` once, and stops if the exit code is not 0 or 1. A pin that differs from the binary gives exit code 3, which is fast and says nothing.
- It times `check --base main` with `approval = "branch"` and with `approval = "base"` (no walk). The 2 modes alternate in each of 5 rounds, so a change of the load hits both. It prints the median of each and their ratio. The target is a ratio of 2 or less on one machine. A time in ms depends on the host, and the ratio does not.
- It counts the git processes that one `check` starts, from `GIT_TRACE2_EVENT`.

Use a release build. The results are in [results.md](results.md#the-walk-of-the-branch).

## Limits

- The results come from one host (macOS, Apple Silicon), and other work used the host at the same time. The benchmark does not measure Linux and Windows. Their process start costs are different.
- The OS file cache is warm. The harness packs each repository into one new pack with a commit graph. The benchmark does not measure loose objects or many packs.
- The prepared corpora have no ignored files, submodules, sparse checkout, conflicts, fsmonitor or untracked cache. A small hand-made repository gives identical output in both modes, but the benchmark does not time it. This repository has a deletion, a staged-only change, an executable-bit change, ignored files, a symlink and non-ASCII paths.
- `git status` also compares `HEAD` with the index, but the check does not need this comparison. `git diff-files` with `git ls-files --others --exclude-standard` can be cheaper. The benchmark does not measure this.
- The benchmark cannot split the step times of `cli-par`, because the steps overlap.
- Process wall time includes the summary (sort and checksums), which is the same code in every mode.
- The peak RSS of gix includes the pages of the memory-mapped pack that it touched. For the CLI modes, that cost is in the git child.
- The benchmark uses only SHA-1 repositories.

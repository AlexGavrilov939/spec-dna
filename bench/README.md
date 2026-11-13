# bench

This directory has the measurements for the decisions, and the measurements of the tool:

| Directory | What it measures |
| --- | --- |
| [tool/](tool/README.md) | The `spec-dna` binary: full, cold, warm and incremental `check`, and `context`. |
| [retrieval/](retrieval/README.md) | Recall, precision, size and latency of `spec-dna context` against tasks with known required sections. |
| [git-access/](git-access/README.md) | Git access through the command line, compared with gix ([ADR 0002](../docs/adr/0002-language.md), confirmation step 2). |
| `go/`, `rust/`, `run.py` | The scanner prototypes of the language decision. The text below describes them. |

The rest of this page describes the prototype code for the language decision in [ADR 0002](../docs/adr/0002-language.md). It is not the spec-dna tool.

There are two versions of the same minimal scanner: Go (`go/`) and Rust (`rust/`). Both versions use only the standard library and follow the same rules:

1. Walk the tree. Skip `.git`, `node_modules` and any `--exclude NAME` directory. Do not follow symlinks.
2. For each `.md` file, read the YAML front matter between `---` lines and take `id`. If the front matter has no id, take the `PREFIX-NUM` token at the start of the file name (for example, `ADR-12-cache.md` gives `ADR-12`).
3. Skip fenced code blocks and inline code spans. Collect inline link targets (the text after `](`) and id tokens `PREFIX-NUM` in the remaining prose. Do not scan link destinations for ids.
4. Remove `#fragment` and `?query` from internal links, and decode `%XX`. Then resolve the links against the walked files and directories. Count id references only for prefixes that a document defines.
5. With `--index FILE`, reuse the parsed data of a file if its size and modification time did not change. If the run parsed a file, rewrite the index atomically.

The output is a fixed list of counts. `--list` adds the broken links and the unresolved ids, in sorted order.

## Files

| Path | Purpose |
| --- | --- |
| `go/main.go` | Go scanner. |
| `rust/src/main.rs` | Rust scanner. |
| `gen/gen.py` | Deterministic generator of synthetic corpora (`small`, `medium`, `large`). Writes `expected.json` with the counts that a correct scanner reports. With `--model spec-dna`, it writes a corpus in the ADR 0001 model, a `spec-dna.toml`, and `tasks.json` with the retrieval ground truth. |
| `run.py` | The harness. It takes any corpus path and makes a snapshot of it. It measures startup, full scan, index build, warm runs and incremental runs. |
| `results/` | Results for the synthetic corpora and the public corpus: prototypes (`small`, `medium`, `large`, `corpus-b`), the tool (`tool-*`) and retrieval (`retrieval-*`). |

## Run

You need Go 1.25+, Rust 1.88+ (edition 2024) and Python 3.10+. The harness works only on Unix, because it uses `os.wait4` for the peak memory of each process.

```sh
cd bench
python3 gen/gen.py --size medium --out work/gen/medium
python3 run.py --corpus work/gen/medium --label medium --runs 15 --out results/medium.json
```

`run.py` never writes to the corpus that it measures. It exports a git repository with `git archive` (use `--rev` to pin a commit), and it copies any other directory. The snapshot, the indexes and the touched files are in `--work` (default `bench/work/`, which git ignores).

The result JSON records the hardware, the toolchain versions and the load average. For each mode, it records the median, minimum, maximum and standard deviation of wall time, and the peak RSS. `outputs_equal` is true when Go and Rust print the same counts. `matches_expected` is true when the counts also match the `expected.json` of the generator.

## Limits

- The generator writes only the syntax that the scanners parse. Thus, `matches_expected` shows that both scanners implement the same rules, not that they conform to CommonMark.
- The id rule finds no ids in the public corpus. In that corpus, the name of each proposal is a numbered directory name.
- The index uses the size and modification time of a file as its key. A tool that keeps modification times can hide a change from the index. The real tool uses the git blob id as the key of a tracked file ([ADR 0001](../docs/adr/0001-information-model.md)).
- The full scan runs with a warm OS file cache. On macOS, you must have root access to clear the cache.

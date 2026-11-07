# bench

Prototype code for the language decision in [ADR 0002](../docs/adr/0002-language.md). It is not the spec-dna tool.

The same minimal scanner exists twice, in Go (`go/`) and in Rust (`rust/`). Both use the standard library only and follow the same rules:

1. Walk the tree. Skip `.git`, `node_modules` and any `--exclude NAME` directory. Do not follow symlinks.
2. For each `.md` file: read YAML front matter between `---` lines and take `id`. Without one, take a leading `PREFIX-NUM` token of the file name (for example `ADR-12-cache.md` gives `ADR-12`).
3. Skip fenced code blocks and inline code spans. Collect inline link targets (the text after `](`) and id tokens `PREFIX-NUM` in the remaining prose. Link destinations are not scanned for ids.
4. Resolve internal links against the walked files and directories, after removing `#fragment` and `?query` and decoding `%XX`. Count id references only for prefixes that some document defines.
5. With `--index FILE`, reuse parsed data for files whose size and modification time are unchanged, and rewrite the index atomically when something was parsed.

Output is a fixed list of counts; `--list` adds the broken links and unresolved ids, sorted.

## Files

| Path | Purpose |
| --- | --- |
| `go/main.go` | Go scanner |
| `rust/src/main.rs` | Rust scanner |
| `gen/gen.py` | Deterministic synthetic corpus generator (`small`, `medium`, `large`). Writes `expected.json` with the counts a correct scanner reports |
| `run.py` | Harness. Takes any corpus path, snapshots it, and measures startup, full scan, index build, warm and incremental runs |
| `results/` | Results for the synthetic corpora and the public corpus |

## Run

You need Go 1.25+, Rust 1.88+ (edition 2024) and Python 3.10+. Unix only, because the harness uses `os.wait4` for per-process peak memory.

```sh
cd bench
python3 gen/gen.py --size medium --out work/gen/medium
python3 run.py --corpus work/gen/medium --label medium --runs 15 --out results/medium.json
```

`run.py` never writes to the corpus it measures. A git repository is exported with `git archive` (use `--rev` to pin a commit); any other directory is copied. The snapshot, the indexes and the touched files live under `--work` (default `bench/work/`, ignored by git).

The result JSON records the hardware, the toolchain versions, the load average, and for each mode the median, minimum, maximum and standard deviation of wall time and the peak RSS. `outputs_equal` is true when Go and Rust print the same counts. `matches_expected` is true when they also match the generator's `expected.json`.

## Limits

- The generator emits only the syntax the scanners parse, so `matches_expected` shows that both scanners implement the same rules, not that they conform to CommonMark.
- The id rule finds no ids in the public corpus, whose proposals are numbered directory names.
- The index keys files by size and modification time. A tool that preserves modification times can hide a change from it; the real tool keys tracked files by git blob id ([ADR 0001](../docs/adr/0001-information-model.md)).
- The full scan runs with the OS file cache warm. Dropping the cache needs root on macOS.

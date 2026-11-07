# Tool benchmark

`run.py` measures the `spec-dna` release binary on one corpus. It never writes to the corpus. It makes a copy:

- It exports a git repository with `git archive`, or it copies any other directory.
- It copies the configuration into the copy as `spec-dna.toml`.
- It makes the copy a new repository with one commit.

Each run is one process. Wall time and peak RSS come from `wait4`. Every mode runs 15 times after 1 warm-up.

| Mode | Command | State |
| --- | --- | --- |
| startup | `spec-dna --version` | |
| full | `check --all --no-cache` | No index: the run parses every document. |
| cold | `check --all` | The harness deletes the index before each run: parse, then write the index. |
| warm | `check --all` | The index is present, and nothing changed. |
| warm_base | `check` | Base `HEAD`. The index is present, and nothing changed. The run lists and diagnoses both trees. |
| context | `context <id> --json` | The index is present. |
| incremental | `check` | Base `HEAD`. After the last write of the index, 10 documents changed in the working tree. |

The harness makes sure that full, cold and warm print the same diagnostics (`modes_agree`). It also makes sure that the incremental run gives the same result as a full run of the same state (`incremental_agrees_with_full`).

```sh
cargo build --release
python3 bench/gen/gen.py --model spec-dna --size medium --out bench/work/sd/medium
python3 bench/tool/run.py --corpus bench/work/sd/medium --config bench/work/sd/medium/spec-dna.toml \
  --label synthetic-medium --context <entry id from tasks.json> --out bench/results/tool-synthetic-medium.json
```

## Results (measured 2026-11-07)

Host: Apple M4 Max, 16 cores, macOS 26.6.1, APFS SSD. Other work used the host at the same time (1-minute load average 6 to 12 during the runs). Binaries: spec-dna 0.1.0 (release, LTO, stripped), and git 2.50.1 (Apple Git-155) called as `git` from `PATH`. Each cell shows the median of 15 runs in milliseconds, with the minimum in brackets. Treat results that differ by less than about 20% as equal.

| Corpus | Files / documents | Index (MB) | Startup | Full | Cold | Warm | Warm, base HEAD | Incremental | Context | Peak RSS full / warm (MB) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| Synthetic small | 75 / 72 | 0.06 | 2.5 (2.1) | 19.9 (18.2) | 21.1 (19.3) | 18.9 (17.9) | 22.5 (20.3) | 21.5 (19.6) | 18.6 (17.3) | 5.4 / 5.4 |
| Synthetic medium | 1,197 / 1,194 | 0.88 | 2.6 (2.2) | 38.9 (38.0) | 40.7 (38.8) | 25.6 (23.4) | 30.2 (28.4) | 30.9 (27.7) | 22.1 (20.7) | 11.2 / 9.4 |
| Synthetic large | 11,823 / 11,820 | 8.59 | 2.6 (2.2) | 320.2 (258.4) | 804.2 (321.7) | 90.4 (82.6) | 128.6 (122.2) | 130.9 (121.8) | 63.2 (61.5) | 52.5 / 52.7 |
| Corpus B | 2,310 / 708 | 5.44 | 2.5 (2.2) | 66.0 (64.9) | 74.4 (69.3) | 44.8 (43.3) | 54.6 (52.9) | 62.7 (57.8) | 45.0 (43.1) | 27.6 / 23.8 |

All modes agree on every corpus. The synthetic corpora come from `bench/gen/gen.py --model spec-dna` (seed 1). Corpus B is `kubernetes/enhancements` at `72907deaa85eae6e208bc95c5f74dd27c1911345` with [retrieval/corpus-b.toml](../retrieval/corpus-b.toml). The cold mode on the large corpus has the largest variation (it writes an 8.6 MB index under load). The owner of corpus A keeps the results for that corpus.

Profile (`SPEC_DNA_TRACE=1`, corpus B, incremental check with 9 changed documents, median of 9 runs):

- Git lists the working tree and the base tree in about 43 ms.
- spec-dna loads the index, parses the changed files, builds both corpora and makes the diagnostics in about 10 ms in total.

On macOS, `/usr/bin/git` is an `xcrun` shim that adds about 7 ms to every git process (see [git-access/results.md](../git-access/results.md)).

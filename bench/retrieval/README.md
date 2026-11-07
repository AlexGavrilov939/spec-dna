# Retrieval correctness

`run.py` measures what `spec-dna context` serves against tasks with known required sections. For each task and budget it reports:

| Metric | Meaning |
| --- | --- |
| Recall (full) | Required sections served in full / required sections |
| Recall (listed) | Required sections served in full or listed as a stub (one `spec-dna show` away) / required sections |
| Precision | Required sections served in full / sections served in full |
| Critical | Sections that state a critical constraint, served in full (hand-made oracles only) |
| Size | Characters served, and estimated tokens: ceil(UTF-8 bytes of the text output / 4), the same estimator for every arm |
| Two-step | Characters of the context output plus `show` of each required section it listed as a stub |
| Baseline | Characters of every document that holds a required section, read whole |
| Latency | Median wall time of `spec-dna context`, 3 to 5 runs |

Tasks:

- Synthetic: `bench/gen/gen.py --model spec-dna` writes 40 tasks per corpus. Ground truth follows the context order of [ADR 0001](../../docs/adr/0001-information-model.md): the requirement sections of the entry spec, every section of the active guides whose `applies-to` matches it, the requirement sections of its `depends-on` targets (or the one section holding a referenced key), and the `Decision Outcome` of each accepted decision linked with it. Distractors: overview, rationale and examples sections, notes, changes, rejected and superseded decisions, unrelated specs.
- Corpus B: 5 tasks written by hand in [corpus-b-oracles.json](corpus-b-oracles.json) before any measurement, by reading the KEPs, without running spec-dna. 4 of them need a second KEP.

```sh
cargo build --release
python3 bench/retrieval/run.py --repo <repository with spec-dna.toml> --tasks <tasks.json> [--query] --out <result.json>
```

## Results (measured 2026-11-07)

Same host and binary as [the tool benchmark](../tool/README.md); load average 6 to 10. Means over tasks, sizes as medians.

| Corpus | Budget | Recall full | Recall listed | Precision | Characters | Tokens (estimated) | Two-step characters | Baseline characters | Latency (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Synthetic medium | 4,000 | 0.71 | 1.00 | 0.72 | 3,800 | 998 | 5,204 | 11,115 | 22.5 |
| Synthetic medium | 8,000 | 0.99 | 1.00 | 0.76 | 5,227 | 1,356 | 5,227 | 11,115 | 22.8 |
| Synthetic medium | 12,000 | 1.00 | 1.00 | 0.76 | 5,227 | 1,356 | 5,227 | 11,115 | 22.7 |
| Synthetic large | 4,000 | 0.76 | 1.00 | 0.75 | 3,779 | 993 | 4,552 | 10,485 | 63.2 |
| Synthetic large | 8,000 | 0.99 | 1.00 | 0.78 | 4,537 | 1,183 | 4,537 | 10,485 | 62.9 |
| Synthetic large | 12,000 | 1.00 | 1.00 | 0.78 | 4,537 | 1,184 | 4,537 | 10,485 | 63.3 |
| Corpus B | 4,000 | 0.03 | 0.03 | 0.03 | 3,996 | 1,069 | 3,996 | 122,784 | 42.9 |
| Corpus B | 12,000 | 0.00 | 0.50 | 0.00 | 11,985 | 3,066 | 15,908 | 122,784 | 45.2 |
| Corpus B | 32,000 | 0.03 | 0.65 | 0.02 | 31,980 | 8,053 | 36,184 | 122,784 | 43.0 |

Raw rows, including the missed sections per task: `bench/results/retrieval-*.json`.

What the numbers show:

1. On a corpus in the ADR 0001 model, the graph order is enough: from 8,000 characters every required section is served in full in 79 of 80 tasks, at less than half the characters of reading the relevant documents whole. Precision stays below 0.8 because each document's title section is served too.
2. Corpus B is not in that model: KEPs have no front matter, no requirement keys, and they link to each other through absolute GitHub URLs or through `kep.yaml`. Without `self-urls` (see below), spec-dna treats both as external. Its READMEs run to 78,000 to 194,000 characters, and the required sections sit deep inside them. Without a task description, `context` cannot know which deep section a task needs, so full-text recall stays near 0.
3. The map makes the rest reachable: at 32,000 characters every required section inside the entry KEP is listed (0.65 = the share of required sections that sit in the entry KEP). The other 0.35 are in second KEPs that no relative link reaches.

The first version of the assembly served the whole entry document and filled leftover budget with remaining sections. On the small synthetic corpus it reached the same recall at 12,000 characters but with precision 0.35 and 8,402 characters, and on corpus B it listed 0.06 of the required sections at 12,000 characters, because full text used the budget before any stub. The current version serves normative sections first, lists the rest, and reserves up to half of the budget for the list.

### `self-urls` and `--query` (measured 2026-11-07)

These rows use the same oracles and the same snapshot of corpus B. [corpus-b.toml](corpus-b.toml) now lists the URL prefixes of the repository in `self-urls`. With the harness flag `--query`, each task gives its task text to `context --query`. Load average: 3.6 for `self-urls`, 7.2 for `--query`.

| Corpus | Budget | Recall full | Recall listed | Precision | Characters | Tokens (estimated) | Two-step characters | Baseline characters | Latency (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Corpus B, `self-urls` | 4,000 | 0.03 | 0.03 | 0.03 | 3,996 | 1,069 | 3,996 | 122,784 | 39.6 |
| Corpus B, `self-urls` | 12,000 | 0.00 | 0.50 | 0.00 | 11,985 | 3,066 | 15,908 | 122,784 | 38.4 |
| Corpus B, `self-urls` | 32,000 | 0.06 | 0.65 | 0.03 | 31,990 | 8,055 | 36,182 | 122,784 | 38.6 |
| Corpus B, `self-urls` and `--query` | 4,000 | 0.20 | 0.57 | 0.45 | 3,932 | 1,104 | 6,928 | 122,784 | 44.0 |
| Corpus B, `self-urls` and `--query` | 12,000 | 0.29 | 0.76 | 0.31 | 11,994 | 3,125 | 18,018 | 122,784 | 46.2 |
| Corpus B, `self-urls` and `--query` | 32,000 | 0.47 | 0.96 | 0.20 | 31,993 | 8,114 | 37,845 | 122,784 | 45.5 |

Raw rows: `bench/results/retrieval-corpus-b-self-urls.json` and `bench/results/retrieval-corpus-b-query.json`.

1. `self-urls` alone changes little. The links now reach the second KEPs, but without a query `context` lists only the title section of a linked document.
2. With `--query`, `context` ranks all sections of the entry and of the linked KEPs. At 12,000 characters, 0.76 of the required sections are listed and 0.29 are in full (before: 0.50 and 0.00). At 32,000 characters, 0.96 are listed and 0.47 are in full.
3. For one task, full recall does not always increase with the budget. The packing is greedy: a large section with a high rank can use the space of 2 smaller required sections.
4. Without `--query`, the output did not change. A new run of the default arm on corpus B and on the synthetic corpora gave the same rows as the table above.
5. The first version of the ranking put the title section of the entry first. In task b1 that section has 2,904 characters of checklist and table of contents, and mean full recall fell to 0.13 at 8,000 and 0.10 at 12,000 characters. Now only sections that a target names come first. The oracles did not change.

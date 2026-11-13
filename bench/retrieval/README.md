# Retrieval correctness

`run.py` measures what `spec-dna context` serves for tasks with known required sections. For each task and budget, it reports these metrics:

| Metric | Meaning |
| --- | --- |
| Recall (full) | Required sections served in full / required sections |
| Recall (listed) | Required sections served in full or listed as a stub (one `spec-dna show` away) / required sections |
| Precision | Required sections served in full / sections served in full |
| Critical | Sections that state a critical constraint, served in full (hand-made oracles only) |
| Size | Characters served, and estimated tokens: ceil(UTF-8 bytes of the text output / 4), the same estimator for every arm |
| Two-step | Characters of the context output, plus the `show` output of each required section that the context output listed as a stub |
| Baseline | Characters of every document that holds a required section, read in full |
| Latency | Median wall time of `spec-dna context`, 3 to 5 runs |

Tasks:

- Synthetic: `bench/gen/gen.py --model spec-dna` writes 40 tasks per corpus. The ground truth follows the context order of [ADR 0001](../../docs/adr/0001-information-model.md):

  - the requirement sections of the entry spec
  - every section of the active guides whose `applies-to` matches the entry spec
  - the requirement sections of its `depends-on` targets (or the one section that holds a referenced key)
  - the `Decision Outcome` of each accepted decision linked with the entry spec

  The distractors are overview, rationale and examples sections, notes, changes, rejected and superseded decisions, and unrelated specs. The context assembly was changed after the first measurement on these tasks, and the same generator wrote the tasks and the corpus. Thus the synthetic numbers show only that the tool follows its own model. They are not a measure of retrieval quality.

- Corpus B: 5 tasks in [corpus-b-oracles.json](corpus-b-oracles.json), written by hand from the KEPs before any measurement and without a run of spec-dna. 4 of them need a second KEP.
- Corpus D (rust-lang/rfcs): 3 tasks in [corpus-d-oracles.json](corpus-d-oracles.json), written by hand without a run of spec-dna. Commit `073bfde` added them with their SHA-256 hash, before the first measurement on corpus D. 2 of them need a second RFC. [corpus-d.toml](corpus-d.toml) is the configuration.

```sh
cargo build --release
python3 bench/retrieval/run.py --repo <repository with spec-dna.toml> --tasks <tasks.json> [--query] --out <result.json>
```

## Results

### Headline: public corpora, oracles written before the measurement (measured 2026-11-08)

These are the numbers to quote. Nothing was tuned on corpus D. Corpus B was the only public corpus during the work on `--query`, and its oracles did not change. Binary: commit `df64974` (session 3 fixes). Host: Apple M4 Max, shared, load average 6.7 to 9.4. Each row is the mean over the tasks. The sizes are medians.

| Corpus | Arm | Budget | Recall full | Recall listed | Critical full | Precision | Characters | Tokens (estimated) | Two-step characters | Baseline characters | Latency (ms) |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| B (5 tasks) | default | 4,000 | 0.03 | 0.03 | 0.03 | 0.03 | 3,996 | 1,069 | 3,996 | 122,784 | 37.8 |
| B (5 tasks) | default | 12,000 | 0.00 | 0.50 | 0.00 | 0.00 | 11,985 | 3,066 | 15,908 | 122,784 | 36.6 |
| B (5 tasks) | default | 32,000 | 0.06 | 0.65 | 0.06 | 0.03 | 31,990 | 8,055 | 36,182 | 122,784 | 38.9 |
| B (5 tasks) | `--query` | 4,000 | 0.20 | 0.57 | 0.20 | 0.45 | 3,932 | 1,104 | 6,928 | 122,784 | 40.4 |
| B (5 tasks) | `--query` | 12,000 | **0.29** | **0.76** | 0.29 | 0.31 | 11,994 | 3,125 | 18,018 | 122,784 | 40.5 |
| B (5 tasks) | `--query` | 32,000 | 0.47 | 0.96 | 0.47 | 0.21 | 31,993 | 8,114 | 37,845 | 122,784 | 43.3 |
| D (3 tasks) | default | 4,000 | 0.00 | 0.34 | 0.00 | 0.00 | 3,917 | 1,047 | 6,947 | 89,751 | 17.8 |
| D (3 tasks) | default | 12,000 | 0.05 | 0.68 | 0.00 | 0.03 | 11,969 | 3,060 | 18,456 | 89,751 | 17.8 |
| D (3 tasks) | default | 32,000 | 0.68 | 0.77 | 0.58 | 0.17 | 31,968 | 8,043 | 31,998 | 89,751 | 18.2 |
| D (3 tasks) | `--query` | 4,000 | 0.11 | 0.63 | 0.00 | 0.28 | 3,939 | 1,120 | 12,463 | 89,751 | 22.8 |
| D (3 tasks) | `--query` | 12,000 | **0.19** | **0.82** | 0.17 | 0.30 | 11,917 | 3,115 | 18,099 | 89,751 | 22.5 |
| D (3 tasks) | `--query` | 32,000 | 0.56 | 0.87 | 0.67 | 0.33 | 31,983 | 8,117 | 33,240 | 89,751 | 22.7 |

Raw rows, with the missed sections for each task: `bench/results/retrieval-corpus-b-s3.json`, `retrieval-corpus-b-query-s3.json`, `retrieval-corpus-d.json` and `retrieval-corpus-d-query.json`.

What the numbers show:

1. On long proposals without requirement keys, `context` is a map, not a final answer. At 12,000 characters with `--query`, it serves 0.19 to 0.29 of the required sections in full and lists 0.76 to 0.82. An agent then needs one `show` call for each listed section (the two-step column). That is 15% to 20% of the characters of the relevant documents read in full.
2. Corpus D repeats the pattern of corpus B. The result on corpus B is thus not only a result of the work on that corpus.
3. Task d2 never lists the sections of its second RFC. The entry RFC links to it only through the published site (`rust-lang.github.io/rfcs/<number>-<slug>.html`). That URL does not contain the repository path, so `self-urls` cannot map it. A URL rewrite rule could map it. This is a finding, not a change: a change now would tune the tool on these oracles.
4. The fixes of session 3 did not change the corpus B rows. The KEPs have no requirements, so the requirement-first order does not apply to them.
5. Critical full recall follows full recall. No arm serves every critical section in full on any public task at 12,000 characters.

### Synthetic corpora (measured 2026-11-08, after tuning)

The binary is the same. These rows check that the tool follows the ADR 0001 model. They are not the headline, because the assembly was tuned on this generator (see "Tasks").

| Corpus | Budget | Recall full | Recall listed | Precision | Characters | Tokens (estimated) | Two-step characters | Baseline characters | Latency (ms) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Synthetic medium | 4,000 | 0.73 | 1.00 | 0.74 | 3,780 | 993 | 5,204 | 11,115 | 18.2 |
| Synthetic medium | 8,000 | 1.00 | 1.00 | 0.76 | 5,227 | 1,356 | 5,227 | 11,115 | 18.1 |
| Synthetic medium | 12,000 | 1.00 | 1.00 | 0.76 | 5,227 | 1,356 | 5,227 | 11,115 | 18.4 |
| Synthetic large | 4,000 | 0.77 | 1.00 | 0.76 | 3,779 | 993 | 4,455 | 10,485 | 61.7 |
| Synthetic large | 8,000 | 0.99 | 1.00 | 0.78 | 4,537 | 1,183 | 4,537 | 10,485 | 60.9 |
| Synthetic large | 12,000 | 1.00 | 1.00 | 0.78 | 4,537 | 1,184 | 4,537 | 10,485 | 60.5 |

Raw rows: `bench/results/retrieval-synthetic-medium-s3.json` and `retrieval-synthetic-large-s3.json`. The small change against session 2 at 4,000 characters probably comes from the requirement-first order.

### History (measured 2026-11-07)

The host and the binary are the same as in [the tool benchmark](../tool/README.md). The load average was 6 to 10. The table shows means over the tasks, and medians for the sizes.

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

Raw rows, with the missed sections for each task: `bench/results/retrieval-*.json`.

What the numbers show:

1. On a corpus in the ADR 0001 model, the graph order is sufficient. From 8,000 characters, the output has every required section in full in 79 of 80 tasks. This output has less than half the characters of the relevant documents read in full. Precision stays below 0.8, because the output also has the title section of each document.
2. Corpus B is not in that model: KEPs have no front matter and no requirement keys. They link to each other through absolute GitHub URLs or through `kep.yaml`. Without `self-urls` (see below), spec-dna treats both kinds of link as external. Its READMEs have 78,000 to 194,000 characters, and the required sections are deep inside them. Without a task description, `context` cannot know which deep section a task needs. Thus, full-text recall stays near 0.
3. The map makes the rest reachable. At 32,000 characters, the output lists every required section inside the entry KEP (0.65 = the share of required sections that are in the entry KEP). The other 0.35 are in second KEPs that no relative link reaches.

The first version of the assembly served the full entry document, and then filled the rest of the budget with the other sections. On the small synthetic corpus, it got the same recall at 12,000 characters, but with precision 0.35 and 8,402 characters. On corpus B, it listed 0.06 of the required sections at 12,000 characters, because full text used the budget before any stub. The current version serves normative sections first and lists the rest. It keeps up to half of the budget for the list.

#### `self-urls` and `--query` (measured 2026-11-07)

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

1. `self-urls` alone gives only a small change. The links now reach the second KEPs. But without a query, `context` lists only the title section of a linked document.
2. With `--query`, `context` ranks all sections of the entry and of the linked KEPs. At 12,000 characters, the output lists 0.76 of the required sections and serves 0.29 in full (before: 0.50 and 0.00). At 32,000 characters, it lists 0.96 and serves 0.47 in full.
3. For one task, full recall does not always increase with the budget. The packing is greedy: a large section with a high rank can use the space of 2 smaller required sections.
4. Without `--query`, the output did not change. A new run of the default arm on corpus B and on the synthetic corpora gave the same rows as the table above.
5. The first version of the ranking put the title section of the entry first. In task b1, that section has 2,904 characters of checklist and table of contents. Mean full recall fell to 0.13 at 8,000 characters and to 0.10 at 12,000 characters. Now only sections that a target names come first. The oracles did not change.

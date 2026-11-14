# Evidence

This page gives the results of the agent experiments that shaped spec-dna, and their limits. The raw data of the experiments is not in this repository: one of the corpora is a private product repository. The benchmarks of the tool itself are in [bench/](../bench/README.md).

## Method

- An agent (Claude Code with Claude Opus or Sonnet) did the same task in 2 or more arms. One arm had no spec-dna, and the other arms had spec-dna in different configurations.
- Each arm had 3 to 6 runs of each task, on the same model and harness.
- An oracle for each task listed the critical constraints. A separate agent wrote it before the first run.
- 2 blind graders checked each run. A run was correct only if no grader found a missed critical constraint or a false claim.
- The main measures were the number of correct runs, the input tokens, and the cost of one correct result. This cost is the cost of all runs of an arm, divided by its correct runs.

## Results

| Question | Setup | Result |
| --- | --- | --- |
| Does the gate keep accepted decisions intact? | Synthetic project, 5 tasks, 2 arms, 3 runs each | With spec-dna: 15 of 15 runs correct. Without it: 6 of 15. The agents edited the frozen text of accepted decisions 7 times and left 3 stale code citations. The cost of one correct result was about half ($1.02 against $2.03, estimated from token prices). |
| Does the gate keep the documents small and consistent as the project grows? | Synthetic project grown by 80 agent tasks in each of 3 arms | All 240 tasks passed. With the gate, the documents were about one third smaller, and drift and broken code citations occurred about half as often. The tasks cost about 14% less. The accuracy of spot questions about the project did not change by more than the noise (118, 114 and 113 of 120). |
| Does it help on a mature real repository? | A private product repository with about 2,300 Markdown documents; real and replayed changes | The light configuration has the gate as a hint, the skill, and the project conventions first. With it, agents gave more correct results in 2 of 3 comparisons and the same number in the third. The cost of one correct result stayed about the same. With the gate, the agents made no more edits of the outcome of accepted decisions. |

## What is not proven

- spec-dna does not reduce tokens by itself. On the synthetic project, the median input of a run was 42% to 101% higher with the tool. The gain is in correct results per cost, not in tokens.
- A split of large documents into smaller ones gave no gain on the real repository. The agents seldom opened the new documents.
- Many differences depend on 1 or 2 tasks. Most confidence intervals of cost include no change.
- No team has used spec-dna in daily work over weeks yet. All measurements used snapshots.

## Limits

- One real repository and 2 synthetic projects.
- One model family for the agents and the graders, so the grading is not fully blind.
- 3 to 6 runs for each cell.
- Agents wrote the tasks and the oracles.
- Dollar figures are estimates from token counts and list prices.

---
status: draft
---
# Protocol 2 of the model-team measurement

## Files

- `docs/adr/0033-protocol-2-of-the-model-team-measurement.md` The protocol, fixed before any paid run.
- `bench/team2/` The recipes, the tasks, the oracle, the tools and the self-test.
- `.claude/agents/ownership.md` The owner of `bench/team2/`.
- `docs/design/0.8.1-note.md` The decision on the advisor of higher-risk changes.
- `docs/team-measurement-2.md` The results page.
- `docs/README.md` The map of the docs.
- `bench/results/evals/` The records of the runs.

## Order

- [ ] t1: Prove the 5 tasks on the stub agent: the start tree fails part 1, the solution with the defects passes part 1 and fails part 2, and the reference passes both. Files: `bench/team2/`. Verify: `python3 bench/team2/selftest.py --write` passes, and each task has tests in both parts.
- [ ] t2: Write ADR 0033 and make it accepted on the word of the owner. Files: `docs/adr/0033-protocol-2-of-the-model-team-measurement.md`. Verify: `cargo run --quiet -- check` gives no error.
- [ ] t3: Run the dry run of the 3 arms on the stub agent, and the 3 paid probes. Apply the gate of the ADR. Files: `bench/team2/limits.json`. Verify: the numbers of the probes and the gate are in the report.
- [ ] t4: Run the batch of 45 runs, one batch at a time, with the back-off of the runner. Files: `bench/results/evals/`. Verify: `python3 bench/team2/analyze.py` reads the result file.
- [ ] t5: Write the results page and the map of the docs, and write the decision on the advisor into the note of 0.8.1. Files: `docs/team-measurement-2.md`, `docs/README.md`, `docs/design/0.8.1-note.md`, `.claude/agents/ownership.md`. Verify: `pre-commit run --all-files` passes.
- [ ] t6: Write the report in Russian, and the archive of `origin/master`. Files: none. Verify: the 2 files are on the Desktop.

## Risks

- A run costs more than the cap allows. The gate of the ADR stops the work before the batch and reports it.
- Another session of Claude Code uses the same account. The runner backs off on a refusal and never counts such a run.
- A run reads the later history, the fixed code or the oracle. The transcript counts show it, and the ADR says how the verdict treats such a run.

## Proof

`spec-dna check --base origin/master` gives no error. The self-test passes. The result file of the batch and its records are committed, and the page gives the 2 verdicts with their intervals.

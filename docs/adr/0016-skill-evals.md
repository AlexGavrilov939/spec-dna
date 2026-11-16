---
status: accepted
---

# 0016. Evals that measure each skill against no skill

## Context and Problem Statement

0.7.0 ships a pack of 11 skills ([D2](../design/0.7.0.md#d2-the-11-skills)). The description of each skill that is on is in each request of an agent, so each skill costs bytes. A gate checks the artifact of some skills. No check shows what a skill adds, such as the order of a test and its code. Only a measurement can show that gain.

The runner of [ADR 0008](0008-harness-and-evals.md) reports the pass rate of the evals of a project. It does not compare a setup with a skill against the same setup without it. How does spec-dna decide which skills are on by default, and how can a project repeat the decision on its own setup?

Constraints:

- Each byte of always-on text needs a reason ([section 1](../design/0.7.0.md#1-evidence-and-sources) and [D3](../design/0.7.0.md#d3-the-disciplines) of the design).
- The runner of ADR 0008 stays: the standard library, git, `claude`, the cost cap, and no prompt or output text in the result. The measurement adds no dependency and no language.
- The working tree of the person never changes.
- A verdict comes from a committed file, so a reader can check it.
- The shipped evals use small fixtures with no consumer data (rule zero).

Evidence (the data of earlier versions stays outside this repository):

- Corpus A (mature product, about 2.3k documents): the gate against the base gave a measured gain. Edits of the locked `Decision` text of accepted ADRs stopped when the gate ran.
- The test of a short `CLAUDE.md` core on corpus A: the difference in correct runs and the ratio of the costs per correct run both have intervals that include the null.
- The synthetic project: the cost of a correct result was 1.02 US dollars with the tool, against 2.03 US dollars without it. The input tokens with the tool were 1.4 to 2.0 times higher. Thus the gain is the cost per correct outcome, not fewer tokens.
- Many wins of the earlier sessions held on only 1 or 2 tasks.
- The earlier sessions used one model family for the agents and the graders, so the blindness was partial. The pack uses no model grader.
- Superpowers turns its skills on in each session. Its own release notes claim fewer tokens, and the sources give no independent benchmark (design, section 1).

## Considered Options

1. No measurement: each skill is on, as in Superpowers. The only evidence is the claim of the author.
2. Measure bundles of skills, for example the whole pack against no pack.
3. Measure each skill against no skill: 2 arms on the same evals, an interval of evals and a fixed rule for the verdict.
4. An LLM judge only: a model grades the output of each arm.
5. Measure in production use: the indicators of the projects that use the pack.

## Decision Outcome

Option 3. Each skill has 5 evals, and each eval runs with the skill and without it. A skill that is off in D2 is on by default only when its result meets the rule below. The design gives the detail in [D32 to D36](../design/0.7.0.md#d32-what-the-set-measures).

### The set and the arms

- The shipped set has 5 evals for each of the 11 skills: 55 evals. Each eval file has the new optional key `skill`, which names one skill of D2.
- The runner runs each eval in 2 arms, in pairs of the same eval and the same run number. In `with`, the temporary copy has the skill folder, also when the configuration has the skill off. In `without`, the copy has no skill folder and no discipline line. It has no name of the skill in the adapter skill and in the `AGENTS.md` line. The arm also denies `Bash(spec-dna skill*)`, so the agent cannot read the text.
- ADR 0008 already runs each eval in a temporary folder, so the working tree never changes.
- The unit of the verdict is one run, and the pass rate of a skill is the share of its runs that pass. The result for each skill has the runs, the passes, the pass rate, the cost and the cost per correct outcome. The 4 runs of one eval are not independent, so the bootstrap resamples the 5 evals first, and then the runs of each eval (2,000 resamples, seed 24). The result has the difference of the pass rates and the ratio of the costs per correct outcome, each with a 90% interval, and a verdict. The ratio is undefined when a resample has no pass in an arm, and rule 2 is then not met. Each check is a command of the pack. The runner uses no model grader.
- `next` does not name an event skill, so the eval of `debug` measures the start of the skill from its description ([D9](../design/0.7.0.md#d9-debug-has-no-stage)).

### The rule for a default

A skill is on by default when the committed result of the release shows one of these:

1. Gain: the pass rate with the skill is at least 0.20 above the pass rate without it. The lower end of the interval is above 0.
2. Lower cost: the pass rate with the skill is not more than 0.05 below the pass rate without it. The cost per correct outcome with the skill is at most 0.90 of the cost without it. The upper end of the interval of this ratio is below 1.

The verdict is `off` when the upper end of the interval of the difference is below 0. Else it is `inconclusive`. A skill that is off in D2 and has one of these verdicts stays off. A skill that is on in D2 stays on unless its verdict is `off`. A verdict `inconclusive` does not turn it off, and the release notes name each skill with no verdict `on`.

The reasons:

- The threshold of 0.20 matches the size of the sample. A simulation with 20 runs for each arm and a true pass rate of 0.8 against 0.6 gave the verdict `on` in about 36% of the trials, with a bootstrap of single runs. The bootstrap of evals gives fewer. Thus only a large gain gives `on`, and the build repeats the simulation on the real bootstrap before the pack ships. A test of the earlier sessions gave an interval that includes 0 for a small gain.
- Rule 2 uses the cost per correct outcome, because the earlier sessions measured the gain of the tool there, not in tokens.
- The rule does not give the verdict `on` for the short-core test of corpus A. The interval of the difference includes 0, and the interval of the cost ratio includes 1.00.
- An `inconclusive` skill that is off in D2 stays off. A skill that is on costs bytes in each request, and a result inside the noise does not pay for them. A skill that is on in D2 stays on until a result says `off`. The full run measures the 7 skills before the tag (Q3).
- A gate checks the artifact of 5 skills that are on in D2, so they can be on before the first result. `brainstorm` and `review` are on because 0.6 had them, and the full run measures them. The 4 skills that are off have no such check.

### Cost

The estimate is 55 evals x 2 arms x 4 runs, which is 440 runs. Lab sessions 8 and 16 cost about 1 US dollar for each run, so the full run costs about 440 US dollars. The pilot runs the 20 evals of the 4 skills that are off, which is 160 runs. It adds 40 runs for the line of `AGENTS.md`, with and without it. The pilot then has 200 runs and costs about 200 US dollars, and the full run has 480 runs and costs about 480 US dollars. The default of `max-total-usd` is 20, so a run needs `--max-total-usd`. These numbers are estimates. `max-total-usd` stops a run above its cap, as in 0.6. [Q3](../design/0.7.0.md#14-owner-questions) asks the owner for the budget.

### Commands and SD507

- `spec-dna eval pack` writes the shipped evals and their small fixtures to `<[evals] dir>/pack/`. It writes each file once. `eval run` without `--ab` does not read this folder, because a JSON file in a subfolder is not an eval.
- `spec-dna eval run --ab [<skill>|all]` reads the pack folder and checks each file with the rules of SD506 (an invalid file gives exit code 3). It runs the 2 arms and writes `<[evals] dir>/results/skills-<date>.json`. The steps and the cost cap of ADR 0008 apply to each run.
- `spec-dna eval verdict` prints the verdict of each skill. It reads the newest project result file, and it falls back to the table of the release that the binary holds. It never starts an agent.
- SD507 `skill-unproven` reports a skill that `[harness] skills` enables when neither the project result nor the table of the release gives it the verdict `on`. It is off in `lite` and `standard`, and a warning in `strict`. A skill that is on in D2 is exempt.

SD507 is never an error. Thus the rule of [ADR 0003](0003-distribution.md) holds: a new rule starts as a warning in a minor release.

### Limits

- One model family runs the agents, as in the earlier sessions. The `without` arm cannot remove what the model learned in training. The pack has no arm with another plugin installed.
- A result is valid for the model and the Claude Code version that the file records. A new model needs a new run.
- No result is a proof for another project. A project can run `eval pack` and `eval run --ab` on its own setup.
- 20 runs for each arm show only a large gain. `next` still prints the name of the skill in the `without` arm, so the arm measures the absence of the text.

### Rejected

- Option 1, each skill on with no measurement: each skill that is on costs bytes in each request. The short-core test shows that text with no check can give no firm gain. A claim of the author is not a measurement.
- Option 2, bundles: the result of a bundle does not show which skill gives the gain. A skill with a loss can hide in a bundle with a gain. D2 sets the default of each skill, so the measurement must be for each skill too.
- Option 4, an LLM judge only: the verdict changes from run to run, and each verdict costs a model call (ADR 0008, "Rejected"). A check of the project gives the same result for the same files.
- Option 5, production use: it has no arm without the skill, and 2 projects differ in more than one skill. spec-dna keeps no prompt or output text, and the data of a project stays in the project. The result also comes after the release, so it cannot set the default of the release.
- A threshold below 0.20, such as 0.10: 20 runs for each arm cannot separate it from noise.
- A skill that is on until an eval shows a loss, as a general rule: a skill with no result then keeps its bytes in each request. The 7 skills that are on in D2 are the exception, and the full run measures them.

### Consequences

- Good: each skill that is on by default has a gate on its artifact or a result. A reader can check the verdict in a committed file.
- Good: the rule uses the cost per correct outcome, which is where the earlier sessions found the gain of the tool.
- Good: a project can repeat the measurement on its own setup with the same commands.
- Bad: the full run costs about 480 US dollars and the pilot about 200 US dollars. Each new model needs a new run.
- Bad: an `inconclusive` result keeps a skill off. A skill with a true gain below 0.20 cannot show the gain, so it stays off.
- Bad: the pilot and the full run both give 20 runs for each arm and skill (5 evals x 4 runs). A true gain of 0.20 gives the verdict `on` in about 36% of the trials or fewer, so a smaller gain stays `inconclusive`. The rule is strict on purpose.
- Bad: the `without` arm removes the folder, the lines and the command, but a model can still apply the idea from its training. The gain of the arm is a lower bound.
- Neutral: `eval run` without `--ab` keeps the behavior of ADR 0008.

## Amends

ADR 0008 is accepted, and its text is locked. Read its section "Evals" with these changes:

- An eval file has the new optional key `skill`. Its value names one skill of D2.
- `eval run` gets the option `--ab`. It runs each selected eval in the arms `with` and `without`, and it writes the result of D32.
- `eval pack` and `eval verdict` are new subcommands. SD507 is a new code.

## Amendments

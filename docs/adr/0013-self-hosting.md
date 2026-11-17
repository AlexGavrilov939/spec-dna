---
status: accepted
---

# 0013. The checkout of spec-dna itself: the tree binary

## Context and Problem Statement

[ADR 0003](0003-distribution.md) makes the pin exact: a binary whose version differs from the pin refuses to run a gate. In the checkout of spec-dna itself, this rule conflicts with a version bump. A binary that this tree builds has the version of `Cargo.toml`, and the release pull request changes that version. The pin then lags behind it. How can CI check this repository with the binary of its own tree, and keep the exact pin for each other repository?

The conflict has 3 parts:

- A release bumps `Cargo.toml`, and the pin must equal that version. But SD503 reports a change of `spec-dna.toml` together with other files. Thus the bump and the pin cannot be in one pull request.
- The pin moves only in its own pull request after the release. Until then, the tree binary refuses the older pin with exit code 3, so CI cannot check this repository with it.
- 0.6.1 has no CI job that checks this repository. Thus no pull request runs the gates of spec-dna on its own documents.

Constraints:

- A release binary in each other repository keeps the exact pin of ADR 0003.
- The mode works in the release that adds it. The pinned binary of the previous release still reads `spec-dna.toml` of this repository.
- The result of a check does not depend on the machine ([ADR 0006](0006-change-lifecycle.md), "Constraints"). Thus the mode reads only the files of the checkout.
- The change adds no dependency and no tool.

Evidence (the data stays outside this repository):

- A lab test: the 0.6.1 binary exits with code 3 on an unknown top-level table `[dev]`, with the message `unknown key dev` ([D18](../design/0.7.0.md#d18-the-tree-binary)).
- Thus a key cannot start a mode in the release that adds it. The pinned binary of the previous release stops on the key first.
- The change of 0.6.1 had 4 pull requests (#34, #35, #37, #40). The last one, #40, only moved the pin ([Evidence from earlier versions](../design/0.7.0.md#evidence-from-earlier-versions)).

## Considered Options

1. Keep the pull request of the pin after each release.
2. A table `[dev]` or a key in `spec-dna.toml`.
3. An environment variable.
4. The pin value `dev`, or a range of versions.
5. A flag `--allow-newer`.
6. No pin in this repository.
7. A tree binary: the binary derives the mode from 2 facts of the checkout.

## Decision Outcome

Option 7. The design page gives the detail in [The checkout of spec-dna itself](../design/0.7.0.md#7-the-checkout-of-spec-dna-itself), D18 to D20. This ADR gives the rules and their reasons.

### The tree binary

The binary is a tree binary when both facts are true:

1. The file `Cargo.toml` at the top level of the git repository that holds `spec-dna.toml` has `name = "spec-dna"`.
2. Its `version` is equal to the version of the binary that runs.

- A tree binary changes one rule only: SD503 (the next section). Each other rule of the pin stays. A pin that differs from the version of the binary stops `check`, `finish` and `eval run` with exit code 3, as in 0.6.1.
- A release binary in another repository never finds this file with this name. Thus it keeps the SD503 rule and the exact pin of ADR 0003.
- The mode needs no key. Both facts are already in the checkout, so the pinned binary of the previous release reads the same `spec-dna.toml` with no error.

### The pin line and SD503

- In a tree binary, SD503 does not count a change of the line `spec-dna = "..."` of `spec-dna.toml` ([D19](../design/0.7.0.md#d19-the-pin-line-and-sd503)). A change of any other line counts as before.
- Thus the release pull request changes `Cargo.toml` and the pin together, and the tree binary equals the pin. The pull request of the pin after the release ends. A local run between a commit of the version and a commit of the pin stops with exit code 3, so a person makes one commit.
- To find a change of the pin line only, SD503 compares the base text and the head text of `spec-dna.toml`. It does this only in a tree binary.

### The self-check in CI

- CI gets a job that builds the tree binary and runs `check --base origin/master` on this repository, with `fetch-depth: 0` ([D20](../design/0.7.0.md#d20-the-self-check-in-ci)).
- The full history also serves the approval rule of [ADR 0012](0012-approval-in-the-branch.md).
- The job is slice 1 of the build ([The slices](../design/0.7.0.md#the-slices)). Thus each later pull request of 0.7.0 runs the new rules on the documents of this repository.

### Rejected

- Option 1, the pull request of the pin: it is the conflict itself. Each release costs one more pull request.
- Option 2, a table `[dev]` or a key: the lab test shows exit code 3 in the 0.6.1 binary. The pinned binary of the previous release then stops on this repository.
- Option 3, an environment variable: it is hidden state. Its value differs between a laptop and CI, so one commit can give 2 results.
- Option 4, a pin `dev` or a range: `[spec-dna-sha256]` names the assets of one version, so a range has no sums. A binary of an earlier version also prints a wrong install message for such a pin.
- Option 5, `--allow-newer`: each caller must remember it, in CI, in the hooks and in each command of an agent.
- Option 6, no pin: the checkout loses the reproducible pin of ADR 0003.

### Consequences

- Good: CI checks each pull request of this repository with the tree binary, from slice 1 of the build.
- Good: a release needs one pull request less, because the pin moves with the version.
- Good: the mode needs no key, no environment variable and no flag. The pinned binary of the previous release reads the same configuration.
- Bad: a fork or a copy that has this `Cargo.toml` gets the same mode. This is acceptable, because the pin protects reproducibility, not security.
- Bad: a person can make the 2 facts true in another repository. The result is only that SD503 ignores a pin-only edit there.
- Bad: in a tree binary, SD503 reads the content of `spec-dna.toml`. `docs/diagnostics.md` says that SD503 reads paths only, so the build changes that text.
- Neutral: a release pull request cannot hold the sums of a release that does not exist yet. This repository has no table `[spec-dna-sha256]`. If it adds one, the sums need a pull request of their own, because SD503 counts each line other than the pin.
- Neutral: `init` and `upgrade` keep the rules of ADR 0003 and ADR 0009.

## Amends

ADR 0001 and ADR 0003 are accepted and locked, so this ADR does not edit them. These rules replace the named text:

- ADR 0003 stays as it is: the pin is exact, and a binary whose version differs from the pin refuses to run a gate. This ADR changes the SD503 rule for the pin line only.
- ADR 0001, amendment "hygiene functions (0.3.0)", item "Configuration changes": in a tree binary, a change of the line `spec-dna = "..."` alone is not a configuration change.
- `.claude/agents/ownership.md`, row "Release": the pin moves in the release pull request. `CONTRIBUTING.md`, section "Releases": the release pull request moves the pin together with the version in `Cargo.toml` and `Cargo.lock`. The last paragraph, which moves the pin in a pull request of its own, ends. Slice 1 of the build writes the new steps.

## Amendments

### 2026-11-17

See ADR 0019.

---
status: proposed
---

# 0032. Acceptance in one pass

## Context and Problem Statement

[ADR 0025](0025-independent-acceptance.md) gives each change with a plan one acceptance run after the review. Part A of 0.8.0 used it on 2 of its own slices. One slice had 8 runs of about 20 minutes and 200k tokens each, and each run said `rejected`. Each code edit made the record stale, also the fix of a `nit`, and nothing stopped the loop.

On 2026-11-21 the owner asked for 5 changes:

- a record only for a `higher-risk` change in `standard`
- one pass that is also the review
- a stop after 2 rejected records
- the texts of 0.7.1 in `lite`
- the agent roles of `[owners]` only

How does the gate keep these rules when an agent can write each file of the branch?

Constraints: `check` has no network ([ADR 0006](0006-change-lifecycle.md)). The trust root stays the person who merges or approves. A branch must not lower a rule with its own commits (Q43 of the design). `lite` changes in nothing. No release has the record of part A.

## Considered Options

- For the count: (1) the commits of the branch; (2) a counter in the record; (3) an archive file for each rejected record.
- For the word of the owner: (a) a new approval of the plan; (b) a new command or a trailer.
- For one pass: (i) `accept` merges into the skill `review`; (ii) 2 skills.
- For the fix of a note: (x) a follow-up that binds the new tree after a short check. The other options are (y) no check, and (z) a new full run.

## Decision Outcome

Options 1, (a), (i) and (x). [Section 15 of design 0.8.0](../design/0.8.0.md#15-acceptance-in-one-pass) (D70 to D79) gives the detail.

- The need. `[change] acceptance-for` (`none`, `higher-risk`, `all`; the default is `all` in `strict`, else `higher-risk`) or a matched role of `[owners]` asks for a record. Each value is the stricter of the head and the base configuration. The class is `higher-risk` when the head, the base or a commit of the branch says so. It is also `higher-risk` when no class is set, or when SD917 says so.
- One pass. The skill `review` holds the steps of the record, and the agent `acceptor` reviews and writes it. `next` has one item.
- Format 2: `roles`, the severities `blocker` and `note`, and `[[follow-up]]`. Only a reason gives `rejected`, and in one life of a record `rejected` never becomes `accepted`.
- The follow-up. An agent that did not write the fix reads its diff with the requirements, and runs `Proof`. The whole chain has a diff of at most 16,384 bytes. The brief hashes the object ids, not the diff text.
- The stop rule. The commits of the branch count the rejected records. After 2, `next` names the owner. With `branch`, SD941 needs an approval commit of the plan after the second. With `base`, the merge decides.
- The friction. `next` prints a quoted prompt that the acceptor runs. With the hook `protect-approvals`, rule D reads the command text and denies a commit of the acceptor with another path.
- `lite`. The text above the marker line of each generated file has the bytes of 0.7.1.
- `[owners]`. A person entry gives a warning, not exit code 3. One entry answers each matched role.

### Rejected

- Option 2: an agent can edit the counter. Option 3: an agent can delete a file that the branch added.
- Option (b): a trailer is text of an agent, and a command adds surface.
- Option (ii): one more description in each request, and 2 steps to learn.
- Option (y): no agent reads the fix. Option (z): the cost of part A.
- A `SubagentStart` hook for the brief, and `isolation: worktree` for the acceptor (design, D75).

### Consequences

- Good: in `standard`, a `routine` change has the flow of 0.7.1. A fix of a note costs a short check. A change has at most 2 full runs before the owner decides.
- Bad: with `base`, or with no hook, the gate cannot stop a third run or prove that a record commit is alone.
- Bad: a rewrite of the history of the branch hides rejected records from the count.
- Bad: nothing in spec-dna stops a shell call of the acceptor that writes outside its worktree.
- Bad: the profile selects 2 more things at run time.

## Amends

ADR 0017, ADR 0025 and ADR 0027 are accepted and locked, so this ADR does not edit them. When the owner accepts this ADR, these rules apply in place of the named text:

- ADR 0017, "Decision Outcome", "At run time, the profile sets the severity of each code". The profile also selects the text of each generated file and the default of `acceptance-for`.
- ADR 0025, "Context and Problem Statement", "each change that has a plan needs an independent acceptance": only a change of the need above.
- ADR 0025, "When a record is needed", item 2: the need above. The sentence "A plan is also new when `base..HEAD` added it or when no commit added it" changes. The new text is: when the tree of the lifecycle base does not have it, or when no ancestor of the base added it. The code reads the base tree, not `base..HEAD`.
- ADR 0025, "When a record is needed", item 4: one entry with `roles` answers each role that matches a role path, with no `--role`. A matched role needs a record, also for a `routine` change.
- ADR 0027, "Owners by path", items 3 to 5: one entry with `roles` answers each role that matches a role path, with no `--role`. A matched role needs a record, also for a `routine` change.
- ADR 0025, "The record", items 2, 3 and 5: format 2 and the brief of format 2. "no `important` finding" becomes "no `blocker` and no rejected follow-up". The acceptor runs the prompt of `next`.
- ADR 0025, "The tree of the record", item 2: the hash compares with the `tree` of the last follow-up.
- ADR 0025, "`finish`", item 2: `finish` also stops on SD941.
- ADR 0025, "Not the author", item 2, and "Who writes", item 1: the skill `review` gives the prompt. Rule D guards the commit where the hook exists.
- ADR 0025, "Consequences", "each change costs one more agent run": one pass, only for a change of the need above.
- ADR 0027, "Consequences", "each matched role costs one more acceptance run": one pass, only for a change of the need above.
- ADR 0025, "Amends", the item on ADR 0023: the set of D2 has no skill `accept`.
- ADR 0027, "Owners by path", item 1: a login or an e-mail gives a warning, not exit code 3.
- ADR 0027, "Deviations", item 4: `finish` prints the entries of the record.

## Amendments

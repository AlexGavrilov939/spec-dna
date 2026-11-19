---
status: accepted
---

# 0019. Limits of 0.7.0 part A

## Context and Problem Statement

The review of 0.7.0 part A left 4 open items in the rules of [ADR 0012](0012-approval-in-the-branch.md) and [ADR 0013](0013-self-hosting.md):

- SD503 compares the text of `spec-dna.toml` byte by byte. A working tree with CRLF line ends and a base with LF gives a change of each line, so a pin-only change counts.
- A `Cargo.toml` that is a symbolic link gives 2 answers: the working tree follows the link, and the index holds the text of the link.
- A branch that approves a plan, adds code and sets the plan back to `draft` passes with `approval = "branch"`. The same net diff gives SD910 with `approval = "base"`.
- The walk runs one `git diff-tree` for each commit. A branch of 500 commits takes 2.9 to 7.3 seconds of each `check` ([bench/git-access](../../bench/git-access/results.md)).

ADR 0012 is at its hard budget, and ADR 0013 is over its soft budget. Thus this ADR holds the new rules.

## Considered Options

1. Keep the rules, and state the limits.
2. Close each limit with the narrowest rule that gives one answer.

## Decision Outcome

Option 2.

- The compare of the pin line reads `\r\n` as `\n` in both texts. The TOML meaning of the 2 texts is the same.
- A `Cargo.toml` that is a symbolic link makes no tree binary, in the working tree and with `--staged`. SD503 then counts the pin line.
- An approval from an approval commit of the branch clears work only while the artifact keeps its approval status at the head. Else SD910 gives reason 9: a later commit took the approval back. Work after that commit, and before a new approval commit, also gives reason 9. An artifact that the head does not have, such as a folder that `finish` deleted, keeps its approval. An approval from the base tree keeps the rule of 0.6, and SD401 reports the move back. The message names the first commit that changed the status after the approval commit. If only the working tree (or the index) set it back, the message names the working tree.
- The walk reads the paths of all commits of the branch with a fixed number of git processes: one `log` for the commits, one `diff-tree --stdin` for the paths, and 2 more for the merge base and the shallow test. The parse of the paths uses `-z` and the place of each record, so no path is read as a commit header.

### Rejected

- Option 1 for the plan set back to `draft`: the branch rule then passes where less history fails.
- Void an approval commit when a later commit changes the status: a plan that the person approves again after a rework then makes each earlier code commit a finding, and only a rewrite of the history clears it.
- SD401 for a move back in the walk: the same rewrite, and SD401 compares 2 trees.
- A read of `git diff` for the CRLF case: one more git call for a case that a replace of bytes solves.
- Follow a linked `Cargo.toml` with `--staged`: the target can be outside the repository.

### Consequences

- Good: the branch rule never passes where the base rule fails for the same net diff.
- Good: the walk costs no git process for each commit.
- Bad: a branch that sets an approved artifact back, and keeps its work, now fails.
- Neutral: SD910 keeps its code and its severity. Its message names one reason of 9.

## Amends

ADR 0012 and ADR 0013 are accepted and locked, so this ADR does not edit them. These rules apply in place of the named text:

- ADR 0012, "Message and output": SD910 names one reason of 9. Reason 9 is a later commit of the branch that took the approval back.
- ADR 0013, "The pin line and SD503": the compare reads `\r\n` as `\n`. "The tree binary": a `Cargo.toml` that is a symbolic link makes no tree binary.

## Amendments

### 2026-11-19

In each binary, SD503 does not count `spec-dna.toml` when its text differs from the base text only in the pin line and in the table `[spec-dna-sha256]`, and the base pins an earlier version. This is the edit of `spec-dna upgrade` for a release that needs no other edit of the configuration. A hook that runs `check` refuses each commit while the pin is older than the binary, so the upgrade is the first commit of a branch. With this rule, the same branch can also hold other work. An edit of another line counts as before. The pin stays exact (ADR 0003). SD913 does not report the files that SD503 does not count. The feedback of a second project gave the reason (design 0.7.1 part B, B3).

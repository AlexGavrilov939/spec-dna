---
status: approved
---
# Fixes of the change lifecycle: finish, scope, the acceptor, layers and small items: spec

## Delta

None. This change adds, changes or removes no requirement, because this repository has no layer of requirements. It narrows the findings of SD913 and SD915, and changes the texts of SD401 and SD939. It changes the output of `next`, `accept`, `layer add` and `upgrade`, the key `reports_read` of `stats --json`, the key `skill_updated` of `layer add --json`, the generated file `acceptor.md` with no hook, the skill `execute`, the plan template and the title rule. It adds no diagnostic code, no configuration key and no new exit code. The [design](../../design/0.8.1-a-lifecycle-and-harness.md) states the contract in D87, D88 and D90 to D93, and its Q8 gives the fix of the fallback of `accept`.

## Interfaces

- None: no interface document describes these commands.

## Decisions

- [[adr/0006-change-lifecycle]]
- [[adr/0012-approval-in-the-branch]]
- [[adr/0024-test-report-as-evidence]]
- [[adr/0027-owners-invariants-and-write-back]]
- [[adr/0032-acceptance-in-one-pass]]

## Concerns

- [x] D87 accepts a ticked box as the proof that the work is done. Resolved: the rule of 0.6 had the same limit, because one edit of a path cleared it. The rule only relaxes, so no pull request that 0.8.0 passes fails, and the rule of ADR 0003 for a new warning does not apply.
- [x] With the code and the edits of requirements in the first pull request, no check reads the evidence of these requirements. Resolved: the guide asks for the edits of requirements in the pull request of `finish`, and the design states the limit.
- [x] The cause of the failed commit of the acceptor is likely, not proved (Q7 of the design). Resolved: the fix covers both candidates, the repeated id in the plan and the repeated id in the record, so it does not wait for the data of the owner.
- [x] D88 adds 89 bytes to the body of `execute`, from 2,174 to 2,263 bytes of a limit of 2,500. The page of the harness additions proposes 326 more bytes. Resolved: this change adds only the 89 bytes, and the build of the additions cuts other text.
- [x] D93 item 2 gives 4 texts of `acceptor.md`, not 2: the hook and the key `advisor` are independent. The cases of the key against the default of the profile, and the test that compares the 2 golden files, change. Resolved: the file of `strict` keeps the hash `f541cad357de8926`, and the plan matches the golden files of the acceptor with one pattern.
- [x] D92 says that its text goes into the page of the test reports of the change `test-tag-bind`. Resolved: the rule of `stats` is a detail of one command, so it goes into the section `stats` of `docs/commands.md`, and the page links that section. Thus neither change waits for the other.
- [x] The fix of the fallback of `accept` changes `paths` of a record entry that an open branch added with the old fallback, so SD928 asks for a new pass. Resolved: the owner chose this fix (Q8), and an entry of the base is not computed again.
- [x] The design gives the class `routine` to the slices A2, A3, A5 and A6. Resolved: the change is `higher-risk` as a whole, because A1 changes when the error SD915 passes.
- [x] The changes `lifecycle-fixes` and `test-tag-bind` edit some of the same files. Resolved: no contract of one change depends on the other, so the change that merges second rebases.

## Open questions

- [x] May a new layer of new types merge with its first documents in one pull request (Q1 of the design)? Answer of the owner (2026-11-24): (a) no. SD503 keeps each configuration change apart, and the path has 2 pull requests.
- [x] Does the fix of the fallback of `accept` go into 0.8.1 (Q8 of the design)? Answer of the owner (2026-11-24): (a) yes, in this change, with a test of a squash commit.
- [x] Which candidate cause of D88 acted (Q7 of the design)? Answer: the owner sends the code and the message of the failed commit. The fix covers both candidates, so this change does not wait for the answer.

---
status: proposed
---

# 0025. Independent acceptance and the acceptance record

## Context and Problem Statement

The agent that writes the code also says that the code is done. `finish` then applies the delta and deletes the folder of the change, so no file keeps what was tried. The owner decided on 2026-11-18: each change that has a plan needs an independent acceptance. An agent in a new session, not the author, tries to break the change and writes a record, and `finish` does not pass without a valid record. On 2026-11-19 the owner allowed an acceptor that runs as a subagent of the building session, on one condition: the builder never writes its brief. The owner also turned the acceptance off in the profile `lite`.

Where does the record stay after `finish`, what can "not the author" mean in a deterministic gate, and who is the root of the trust?

Constraints:

- `check` has no network and no model. A skill is never a precondition of a gate ([ADR 0006](0006-change-lifecycle.md) and [ADR 0001](0001-information-model.md)).
- No committed registry: 2 branches that edit one file give a conflict (ADR 0001, "Concurrency").
- A finished change leaves no file in the hot path. Agents search the repository, and an old plan is a search hit next to the current truth (ADR 0006, "Rejected", option 1).
- A squash merge keeps the net diff, and the title of the pull request replaces the commit messages.
- An upgraded repository with no open change gets no new diagnostic.

Evidence ([design 0.8.0](../design/0.8.0.md#1-evidence-and-sources)):

- A path in the folder of a change needs no approved plan for SD910 (F7).
- When the diff deletes the folder, each check reads the artifacts from the base tree (F8). A file written in the pull request of `finish` is then not in the folder that the checks read.
- A hook gets a session id and an agent id, and a transcript is a local file (F12). A subagent has the session id of its caller, and no tool gives it its agent id (F23). CI has neither, so an id in a record is a claim.
- `finish --json` prints the `step` of a stop, and 6 on success (F6).

## Considered Options

For the place of the record after `finish`:

1. One file per change in a hidden folder outside the roots: `.spec-dna/accepted/<slug>.toml`.
2. One append-only log file.
3. The commit message of `finish`.
4. A document of a new type in a layer with no `context`.

For "not the author":

- (a) Claims in the record, with 2 checks that git can show: distinct sessions, and the record in commits of its own.
- (b) A session trailer on each commit of the builder, from a hook of the agent, compared with the record.
- (c) The author e-mail of the commits.
- (d) No check.

For the input of the acceptor:

- (i) A new session that starts from the slug only.
- (ii) A subagent with a prompt that the builder writes.
- (iii) A subagent or a new session that gets the output of `spec-dna accept <slug> --brief`, unchanged. The record keeps the hash of that brief.

## Decision Outcome

Option 1, option (a) and option (iii). The design page gives the detail in [D3](../design/0.8.0.md#d3-the-acceptance-record).

### When a record is needed

- The acceptance is on in `standard` and `strict`, and off in `lite`: there, each record code is off and `finish` needs no record. SD927 to SD940 are in the new rule group `acceptance`, so no group rule of 0.7.1 changes them.
- Each change that `finish` applies needs one record when the acceptance is on and its plan is new. A plan is new when an ancestor of the lifecycle base added it at a pin of 0.8.0 or later (`git log -1 --diff-filter=A <base>`, as B5 of [design 0.7.1 part B](../design/0.7.1-b.md#b5-a-plan-of-an-earlier-version-and-sd923)), when `base..HEAD` added it, or when no commit added it. Thus a branch cannot set the pin back for one commit to skip the record. In a shallow clone, SD927 names the cause. `next` names `accept` only for such a change.
- A closed change and a small fix need none.
- A `routine` change needs 1 acceptor. A `higher-risk` change needs `[change] higher-risk-acceptors` (default 1). Each role of `[owners]` that matches a path of `paths` needs an acceptor with that role ([ADR 0027](0027-owners-invariants-and-write-back.md)). One entry can count for both.

### The record

- One TOML file: `acceptance.toml` in the folder of the change, then `.spec-dna/accepted/<slug>.toml` (or `<slug>-2.toml`) after `finish`.
- The fields: `format`, `change`, `scope`, `paths`, `tree`, `result` (`accepted` or `rejected`), `acknowledged-by`, and the tables `[[acceptor]]` (role, brief, session, agent, model, result), `[[proof]]`, `[[invariant]]`, `[[deviation]]`, `[[attempt]]` and `[[finding]]` (acceptor number, severity, path, line, text). 2 acceptors can have the same role, so an attempt or a finding names the number of its acceptor.
- The rules: no unknown key, at most 16,384 bytes, no answer `pending`, at least one attempt for each acceptor, and a path for each finding. The top `result` is `accepted` only when each acceptor accepted, no `important` finding exists, no invariant is `violated` and no deviation is `rejected`.
- `spec-dna accept <slug> [--role <name>] [--session <id>]` writes the skeleton with the computed fields. A person can write the same file by hand. The gate reads the file, not the skill.
- `spec-dna accept <slug> --brief [--role <name>]` prints the brief: the task and the rules of the record (fixed text of the format), the intent, the spec and the plan, `scope`, `paths` and `tree`, the invariants, the deviations, and the roles of `[owners]` that match. It is a function of the committed state only: both forms refuse (exit code 1) while the folder, except the record, or `spec-dna.toml` has an uncommitted edit. Its last line is the hash of the text before it. The builder passes the output unchanged, and the acceptor copies the hash into `brief` of its entry.

### The tree of the record

- The record stores its scope: `scope` holds the paths and globs of the scope of the plan and of `[change] shared-paths`. `paths` holds the code paths of [ADR 0024](0024-test-report-as-evidence.md) of the diff of the change against the lifecycle base that `scope` matches, sorted. Thus open changes do not change the list. When the diff has no code path in `scope`, `accept` takes the code paths since the commit that added the plan. `tree` is the hash of these paths only.
- The record is stale (SD929) when the hash differs, or when the diff of the change has a code path that `scope` matches and `paths` does not list. `finish` and the checks after it read `scope` and `paths` from the record, not from the plan, so an edit of `Files` during the build changes nothing.
- In CI, the checkout is the test merge of the branch into the base. A base commit that edits a listed path is a real conflict of meaning, and the record is stale. A base commit to other paths, or a Markdown edit, changes nothing. Code of an earlier pull request of the change is not listed, so the acceptance runs in the last pull request with code.
- `check` checks only the record and its commits (SD928, SD931, SD932) where the diff adds or changes it. It checks the presence (SD927), the tree (SD929), the result (SD930), the count of acceptors (SD933), the acknowledgement (SD934) and the answers (SD938) only in 2 cases. One case is a diff that deletes the folder, as SD915 and SD916. The other case is a change that the branch added and deleted. Thus the first of 2 acceptors can commit. SD927 needs the archive file in the head, so a folder deleted with no `finish` gives SD927, also when the base folder holds a record. A later change never makes an old record stale.

### `finish`

- `finish` checks the record in step 5, after the reports (part 5.1) and before the evidence (part 5.4). The step numbers of 0.7.1 and the success value 6 of `finish --json` keep their meaning.
- It stops on each record code that is not off: an absent, invalid, stale or rejected record (SD927 to SD930), SD931, an absent acceptor (SD933), SD936 to SD940, and with `branch` SD932 and SD934.
- It adds the archive file to its writes. A diff that changes or deletes an archive file gives SD935.

### "Not the author"

- spec-dna checks 3 signs, and none is a proof:
  - SD928 computes the brief of each entry again from the tree of its adding commit: the oldest commit of the latest unbroken run of commits that hold an entry with the same role, session and brief. A different hash gives SD928. No compare runs for an entry of the base, on an archive file or on the default branch.
  - 2 entries with the same `session` and the same `agent` give SD931. `unknown` matches no value, and `accept` writes `agent = "unknown"`. Thus SD931 fires only on a value written by hand, and it cannot catch a forger.
  - With `approval = "branch"`, a commit that adds or edits the record changes no other path (SD932). The commit of the result of `finish` is the exception.
- The skill `accept` tells the builder to pass the brief with no word of its own. The agent `acceptor` works from the brief and edits no code.
- spec-dna cannot show that the acceptor is not the builder, which model ran, which text the acceptor read, or that it ran in a new session. Anyone can compute the brief and its hash, so the hash does not show that the acceptor got the brief. No profile asks for a new session, because the gate cannot check it.

### Who writes, and who acknowledges

- The acceptor writes the record and commits it alone, in each approval mode. The builder never edits it. An agent never adds an entry to `acknowledged-by`. The rule of the skills says so, and the hook `protect-approvals` denies such a commit. Rule A of the hook already keeps `spec-dna approve` for a person.
- The record needs an acknowledgement when it holds a deviation:
  - `base`: the merge of the pull request of `finish`. `strict` uses `base`.
  - `branch`: `spec-dna approve <slug> acceptance` adds the e-mail of its author to `acknowledged-by` and commits that file alone. The walk counts the entry only when that commit added it, its author has that e-mail, and no later commit changes the record, except another such commit or the commit of `finish`, which moves the same bytes to the archive. Any other commit that changes the record and keeps an entry gives SD932, so the approver approves again. Thus an approver never acknowledges a text that changed after the approval.
- The persons of `[owners]` and the mode `review` of [ADR 0026](0026-approval-through-the-code-host.md) moved to 0.8.1.

### The trust root

A forged record costs one file and one commit. The root of the trust is the person who acknowledges: the merge or the approval commit. The evals of the skill `accept` measure the catch rate and the cost. The record makes the work of the acceptor visible, binds it to the code, and keeps it.

### Rejected

- Option 2: each pull request appends to one file, so 2 branches conflict (ADR 0001).
- Option 3: the squash title replaces the message, `finish` must validate the record before a commit exists, and `check` cannot read a message on the default branch.
- Option 4: a new type changes the type table, the layers and the templates, and a Markdown document is a search hit. The default search tools of agents skip a hidden folder.
- Option (b): it needs a hook and a trailer of the builder. The owner can add it later (design, Q10).
- Option (c): an agent often commits with the e-mail of its owner ([ADR 0018](0018-approval-by-a-person.md)), so each honest change would fail.
- Option (d): the owner asked for a check and its limits.
- Option (i) as the only way: one more session start for each change. The owner allowed a subagent.
- Option (ii): the builder frames the review in its own words, and nothing shows it.
- A record for each acceptor in its own file: 2 files for one change, and a second place to read.
- A new step number for the record: `finish --json` prints the step, so a new number changes its meaning.

### Consequences

- Good: each change keeps one record of what was tried, with its findings, after `finish`.
- Good: an edit of the code after the acceptance stops `finish`.
- Good: the record is the carrier of the deviations, the invariant answers and the proof states, so no other store exists.
- Bad: each change costs one more agent run. A `higher-risk` change in `strict` costs 2.
- Bad: the session, the agent and the input of the acceptor are claims or signs, and a forged record passes the gate. Only a person stops it.
- Bad: the brief costs about 10 KB of output in the context of the builder for each acceptor.
- Bad: the archive grows by one file of 1 to 4 KB per change. `git grep` still finds it.
- Neutral: a repository of 0.7.1 with no open change gets no new diagnostic. An open change with a plan of 0.7.1 on the base finishes with no record.

## Amends

ADR 0006, ADR 0012, ADR 0018 and ADR 0023 are accepted and locked, so this ADR does not edit them. When the owner accepts this ADR, these rules apply in place of the named text:

- ADR 0006, "Finish", the sentence "`spec-dna finish <slug>` checks the change, applies the delta, checks the evidence and deletes the folder": it also checks the acceptance record and moves it to `.spec-dna/accepted/`.
- ADR 0012, "The rule", condition 3 of an approval commit: a commit can also add the e-mail of its author to `acknowledged-by` of `acceptance.toml`, and change nothing else.
- ADR 0018, "Decision Outcome", the hook `protect-approvals`: it also denies a `git commit` that adds an entry to `acknowledged-by` of a record in a change folder, except the commit of `finish`, which moves the same bytes to the archive. The rule of the skills also says "never add an entry to `acknowledged-by`".
- ADR 0023, the 7 skills of D2 of 0.7.0: the set also holds the skill `accept`, in the profiles where the acceptance is on (design 0.8.0, Q19).

## Amendments

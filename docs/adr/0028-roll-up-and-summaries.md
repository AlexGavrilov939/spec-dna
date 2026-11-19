---
status: accepted
---

# 0028. Roll-up: summaries with pinned hashes

## Context and Problem Statement

The gate keeps the requirements linked and checked, but nothing above them stays true. An agent that changes a requirement of the area `valves` gets no short text of that area. The `system` document can still describe the old behavior, and no check finds it.

[ADR 0001](0001-information-model.md) reserves the suffix `@<hash>` of a reference for suspect links: "A stored suspect link keeps the hash of the target at review time, as in Doorstop." spec-dna parses the suffix and ignores it.

The owner decided for 0.8.0:

- An area has a summary that stores the hashes of the requirements that it rests on.
- A summary is stale when a requirement changes. This is an error in `standard` and `strict` and a warning in `lite`. On 2026-11-19 the owner turned each new mechanism of 0.8.0 off in `lite`, so the roll-up is off there.
- The `system` and `product` documents rest on the summaries.
- A project splits or condenses an area that is over its budget.
- `context` reads from the top down.

How does spec-dna store the pins, find staleness, and keep the cost of each change small, with no model in `check`?

Constraints:

- `check` has no network and no model. It cannot read the meaning of a text.
- A clean 0.7.1 repository passes `check` after `upgrade` alone. An absent key keeps the behavior of 0.7.1.
- `place_type` gives a type by folder only for a layer of one type. A second type in the layer `requirements` removes the place type of each current document there.
- `finish` and `mv` stop on each new error.
- Each byte that `context` serves has a cost in each request.

## Considered Options

The place of a summary:

1. A new type `summary` in a new layer `summaries`, whose paths mirror the area folders.
2. A second type in the layer `requirements`, with a file such as `summary.md` in each area folder.
3. No new type: a `system` document for each area.

The form of the pins:

- A. A front matter list, one pin for each requirement (`<doc-id>#<key>@<hash>`) or lower document (`<doc-id>@<hash>`).
- B. One combined hash of all requirements of the area.
- C. Links with the suffix `@<hash>` in the body.

## Decision Outcome

Option 1 with form A.

### The type and the levels

- `summary` is the 16th type. It has no status, no required section and no requirement key, and it is not normative. Its budget is 3,000 and 6,000 characters.
- The default graph gets the layer `summaries`: folder `summaries`, `keep = "forever"`, `context = "always"`, no edges. The implied graph of a project with no `[layers]` table does not get the layer. A folder `summaries/` of 0.7.1 can hold documents of another type. `init` writes the default graph for a project of 30 documents or more, so such a new project gets the layer in `standard` and `strict`. `spec-dna layer add summaries` adds the layer to another project.
- `<summaries folder>/<area>.md` is the summary of the folder `<requirements folder>/<area>/`. A folder of any depth is an area. Thus a split area gets a deeper summary, and the levels follow the folders.
- A `system` or `product` document with the field `rests-on` is the top. A `system` document rests on the whole layer `requirements`. A `product` document rests on the `system` documents with `rests-on`, or on the whole layer when there is none.

### The pins

- The front matter field `rests-on` lists the pins. A pin is not in the body, so it is not in the hash of the body, not in a budget and not in `context`.
- The frontier of a document is the set of items that it must pin: each active requirement in its scope. The summary of the highest sub-area that has one replaces the requirements of that sub-area. An active requirement is one that SD608 reads (status `active`).
- A pin to a requirement matches when `hash_matches` accepts its hash. A pin to a document matches when its hash is the `frozen_hash` of the document. This hash covers the text outside the amendment, history and live sections, without link destinations. No new hash definition.
- A pin resolves as a reference: document aliases, key aliases, the section aliases of a split and the legacy ids.
- A pin names only an item below its document, so no cycle can pass `check`.

```md
---
rests-on:
  - requirements/valves/closing#close-on-timeout@3f2a9c1b7e0d4a11
  - summaries/valves/drives@9a1c44e0d2b7f6a3
---
```

### The codes

All codes default to Warning. A new rule group `rollup` holds SD641 to SD645, and a new rule group `area-budget` holds SD705 to SD707. Thus no group rule of 0.7.1, such as `budgets`, changes them. The profile table sets these severities:

| Code | Finding | lite | standard | strict |
| --- | --- | --- | --- | --- |
| SD641 | A pin no longer holds: the text changed, the target was removed or is not active, or the target left the frontier | off | error | error |
| SD642 | An item of the frontier has no pin | off | error | error |
| SD643 | A pin has a bad form, is ambiguous, names an item of the same or a higher level, or sits in a type other than `summary`, `system` and `product` | off | error | error |
| SD644 | A summary whose path names no area folder | off | error | error |
| SD645 | Against the base, the set of pins (resolved targets and hashes) changed and the text did not, and no change finished in the diff edited the text | off | warning | warning |
| SD705 | An area over its soft budget | off | warning | error |
| SD706 | An area over its hard budget: new, under it in the base, or grown by more than `growth` | off | error | error |
| SD707 | An area over its hard budget in the base, grown by `growth` or less | off | warning | error |

Each stale pin is one diagnostic with the pin as its target, so the gate sees a second stale pin as new.

### The refresh and its guard

`spec-dna refresh <id>... [--holds]` and `spec-dna refresh --all` write `rests-on` as the frontier, with the current hashes and canonical ids. The guard: `refresh` refuses a document (exit code 1) that has SD641 or SD642 and whose `frozen_hash` equals the one in the base. `--holds` writes the pins anyway, and SD645 shows the claim in the diff. `--all --holds` is a usage error.

The guard turns a refresh with no text change into a named claim. It does not check that a new text is true: a one-word edit passes the guard. Only a reader checks meaning: the acceptor or the owner.

A summary is not an artifact of a change, so no approval mode asks for a status. An edit of its text and a claim with `--holds` are in the diff of the pull request. The merge (`base`) covers that diff. In `branch` mode, a claim has only SD645. The acceptance record does not carry the claim in 0.8.0.

### `finish`, `next` and `mv`

- The text of a requirement changes only when `finish` applies the delta, so no pin of an area summary is stale during the build. In the build, `next` prints a note that names the due roll-up documents: each summary, `system` or `product` document whose frontier holds a target of the delta. The agent edits the text of each one whose meaning changes. The level above pins that text, so that level is stale at once. The agent updates that level in the build. The acceptor reads these edits.
- `spec-dna finish <slug> [--holds <id>...]`. In step 5, after the evidence and before the archive of the record, one function re-pins each due roll-up document whose text the change edited. The compare uses the tree before the commit that added the plan. Thus it counts an edit that an earlier pull request of the change merged. The function also re-pins each due document that `--holds` names. That option is a claim that the text still holds, and the diff and SD645 show the claim. The function prints the other due documents. It never stops, so it has no step number. The last check of step 5 does not count SD641 and SD642 of the printed documents.
- After `finish`, `check` reports each printed summary as stale (SD641) until an agent edits the summary and runs `refresh`, or runs `refresh --holds`. The fix costs one more commit.
- SD913 does not report a roll-up document, as it does not report a path of `shared-paths`.
- The preservation check of `mv` does not count SD641 and SD642. `mv` prints the summaries whose frontier changed. A move of an area folder also moves its summaries.

### The budget of an area

The size of an area is the body characters of its documents that are not inactive. A sub-area with a summary counts as its summary. `[budgets.area]` holds `soft` and `hard` in characters. The area budget is a mechanism of its own. It is not a part of the roll-up. The table is its key, and with no table it checks nothing. The rule against the base is the rule of SD702 to SD704. The fix is a split into sub-areas, each with a summary, or a shorter text.

`init` writes the table: `standard` 60,000 and 120,000, `strict` 40,000 and 80,000. In `lite`, `init` writes no table and no layer `summaries`. 120,000 characters are about 30,000 tokens. An agent can still read an area of this size in one request.

### `context`

A new tier `summary` comes after the conventions and before the rules. It holds the chain of the request, from the top down: `product`, `system`, then the summaries from the shallow folder to the deep one. Its full text uses at most a quarter of the budget. A stale document has the word `stale` in its header. With no roll-up document, the output is the output of 0.7.1.

### What the gate cannot check

- The meaning of a summary. A wrong text with current pins passes, and `--holds` can be false.
- Rules, interfaces and decisions. They have no roll-up in this version.
- SD645 needs a base. The guard compares with the base, or with `HEAD` when there is none. Thus after one edit of a summary in a branch, each later refresh of that summary in that branch passes.

### Rejected

- Option 2: each current document of the layer `requirements` loses its place type and gets SD303.
- Option 3: no place for the area, and no budget of its own.
- Form B: a new hash definition, and no name of the changed requirement without a base. A move inside the area would change the hash, unless the hash drops the ids.
- Form C: the pins would be in the body. Thus each refresh would change the hash that the level above pins, and each change would go up to the top. They would also cost bytes in `context`.
- A refresh with no guard: one command would silence SD641.
- A refresh that always needs a text change. A correct summary would get a needless edit, and that edit would go up to the next level.

### Consequences

- Good: the suspect links of ADR 0001 work with no new hash and no format change.
- Good: a typical change has 1 rewrite of at most 6,000 characters and 1 or 2 commands. A refresh never changes the hash of the level above, `--holds` stops the chain, and the budget keeps the depth small.
- Good: a project with no summary sees no change.
- Bad: one front matter line of about 60 bytes for each requirement of an area.
- Bad: an agent can write `--holds` and read nothing. SD645 makes it visible, not false.
- Bad: a due summary that the agent neither edits nor names in `finish --holds` is stale after `finish`. The pull request then needs one more commit.
- Bad: the acceptor does not see a claim of `finish --holds`. Only the diff and SD645 show it.
- Neutral: the area becomes a folder of any depth. `--group` keeps its first-folder rule.

## Amends

ADR 0001, ADR 0005 and ADR 0017 are accepted and locked, so this ADR does not edit them. These rules apply in place of the named text:

- ADR 0001, "Change-risk classes": spec-dna compares the suffix `@<hash>` of the pins of `rests-on`. A suffix on a link in the body stays parsed and ignored. The model has a 16th type, `summary`.
- ADR 0005, the default graph: it has the layer `summaries`. The implied graph does not.
- ADR 0017, "What `init` writes": `init` writes `[budgets.area]` for `standard` and `strict`.

## Amendments

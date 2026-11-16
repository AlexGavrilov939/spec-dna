---
status: accepted
---

# 0015. The known limits of 0.6.1

## Context and Problem Statement

0.6.1 shipped with a list of known limits. In 4 of them, a command gives a wrong result, or a message that does not tell the person what to fix:

- A paragraph under `Files` does not mean "no files". `next` then asks for paths that the work does not have.
- `index` on a layer with a status stops with SD304 and a message with an empty target.
- 2 area names that differ only in case plan 2 folders, and a file system that ignores case merges them.
- A plain `git mv` of an intent out of the intent folder writes no alias, so `log` reads it as a closed change.

Which contract closes each limit with the smallest change, and keeps each current corpus valid where it can?

Constraints:

- [ADR 0001](0001-information-model.md): ids and hashes stay. The cache is disposable, and it keys a tracked file by its git blob id. `PARSER_VERSION` in `src/parse.rs` removes old entries when the parsed data changes.
- [ADR 0003](0003-distribution.md): a new rule starts as a warning in a minor release. A fix to an error rule that exists can change results in a minor release.
- [ADR 0006](0006-change-lifecycle.md): `log` reads git, and the deletion of `intent.md` from the intent folder ends a change.
- A failure with no proven cause gets a reproduction by `failure-analyst` before a fix (`.claude/agents/ownership.md`, "Escalation").
- A message is enough when the person can fix the input. A new diagnostic code adds a row, a severity and a `[rules]` key.

Evidence:

- These 4 items are 4 of the 6 "Known limits" of 0.6.1 in `CHANGELOG.md`. The fifth is a commit of `finish` that adds a near copy of an intent, with its id as an alias. Conditions 2 and 3 of D31 close it, except for a folder that holds only `intent.md`, where the alias stays a gap. The sixth is `layer add` with a `types` list on many lines, and it stays.
- The reproduction of [item 9 of 0.6.1](../design/0.6.1.md#9-a-routine-change-with-no-files) had the text "None: this change is research." under `Files`. D23 accepted a list item or a paragraph. The build dropped the paragraph, because the parser kept list items only ([changes during the build](../design/0.6.1.md#changes-during-the-build)).
- `PARSER_VERSION` is 13 in 0.6.1.
- D42 of 0.6.1 compares the target paths of `upgrade` in lowercase ASCII, because macOS and Windows fold the case of a name. The open item of the build names the stop of `index` and the case of area names.
- In 0.6.1, a move out of the intent folder needs a rename record and the old id in `aliases` ([D9 and D10](../design/0.6.1.md#3-log-and-a-move-out-of-the-intent-layer)). In a scratch repository, git paired a deletion by `finish` with a research note that copied the intent (`R078`).

## Considered Options

1. A paragraph under `Files`. The options: keep the limit, read each paragraph as "no files", add a diagnostic code, or read the first paragraph.
2. `index` on a layer with a status. The options: keep the limit, write no status, exempt a generated index from SD304, or a contract with a fix after a proven cause.
3. Area names that differ only in case. The options: keep them, change each name to lowercase with no message, give a warning, or give an error for a pair.
4. A plain `git mv` of an intent. The options: require the alias, check the move, write the alias in a hook, or read a rename that git finds as a move.

## Decision Outcome

For each limit, the last option. The design holds the detail: [D28](../design/0.7.0.md#d28-a-paragraph-under-files), [D29](../design/0.7.0.md#d29-index-on-a-layer-with-a-status), [D30](../design/0.7.0.md#d30-area-names-that-differ-only-in-case) and [D31](../design/0.7.0.md#d31-git-mv-of-an-intent-with-no-alias). This ADR gives the rules and their reasons.

### A paragraph under `Files`

- The parser keeps the first paragraph of the section `Files`. `PARSER_VERSION` is 14, so spec-dna rebuilds the cache once.
- A paragraph with the first word `none` (any case), followed by `:`, `.` or the end of the paragraph, means the same as `- none`. The word rule of [Files and paths](../types.md#files-and-paths) stays, so `nonexistent` is not the word.
- As D23 of 0.6.1 said first, the section that states "no files" holds one item only: the list item, or the paragraph.
- Another paragraph gives this line in `next`, `status` and the messages of SD913 and SD915: "the section `Files` has a paragraph and no list item; write the paths as a list, or write `- none`".

Reason: a person writes prose first, as the reproduction of 0.6.1 shows. D23 of 0.6.1 accepted this form, and the build dropped it only for the cost of the parser. The cost is one parsed field and one rebuild of the cache.

### `index` on a layer with a status

- `index` never writes a document that `check` rejects.
- Assume that the types of the layer need a status, and the index document has none. Then `index` writes the first status of the type that the place gives. `mv` uses the same rule since 0.6.1 (D6).
- If `index` cannot name such a status, it stops with exit code 2 and names the layer. `history` stops in the same way when no layer has the type `research`.
- The cause of the empty target of SD304 is not proven. `failure-analyst` proves it with a reproduction first ([slice 7](../design/0.7.0.md#13-the-test-plan-and-the-build)). This ADR states the contract only, so the fix follows the cause.

Reason: a generated file must pass the check that reads it. One rule for the status of a place serves both `mv` and `index`.

### Area names that differ only in case

- The configuration reader compares the names of `areas` in lowercase ASCII. 2 names that differ only in case are an error (exit code 3).
- `new`, `mv`, `import --areas` and `upgrade --layout --areas` use the same compare.
- A project with such a pair gets the error at the first run, and the person renames one area.
- `upgrade --layout --areas` writes the areas into `spec-dna.toml`, and its proof parses the new configuration (ADR 0009). Thus a pair stops that plan with exit code 1, and the command writes nothing.

Reason: macOS and Windows fold the case of a name, so a clone there merges the 2 folders. The key `areas` already rejects a name that appears twice ([config](../config.md#keys)). The compare in lowercase extends that error rule, and ADR 0003 allows such a fix in a minor release. D42 of 0.6.1 uses the same compare, so the 2 rules agree.

### A plain `git mv` of an intent

- `log` and `stats --sdlc` read a rename of `intent.md` out of the intent folder as a move when 3 conditions are true:
  1. Git rename detection (`-M`, a similarity of 50% or more) finds the rename.
  2. The new file is identical to the old file, or it has the old id as an alias.
  3. The same commit deletes no other file of the change folder.
- A plain `git mv` gives an identical file, so it is a move with no alias. A near copy with no alias is not a move, and a `finish` commit that deletes the other files of the folder is not a move.
- The alias keeps its other task: it keeps the old id valid for links (SD101).
- A move that changes more than half of the text still ends the change, as in 0.6.1.

Reason: a person can use `git mv` in place of `spec-dna mv`. Git already records the rename, so `log` reads it with no second fact. The 0.6.1 rule reported a closed change that did not close.

### Rejected

- Each paragraph read as "no files": a plan with prose under `Files` then lists no file, and the scope check SD913 stops with no notice.
- Keep the limit of the paragraph: `next` and `status` ask the person for paths that the work does not have.
- A new code for a paragraph: the message of `next`, `status`, SD913 and SD915 is enough. A code adds a row, a severity and a `[rules]` key for one input form.
- Keep the limit of `index`: a person cannot index a layer with statuses, and the message names no target.
- An index with no status: `index` then writes a document that `check` rejects.
- An exemption of a generated index from SD304: `check` then has a second status rule for one kind of file.
- A fix of `index` before the cause is proven: the project rule asks for a reproduction first. A fix of a guessed cause can hide the real defect.
- Area names as they are: a file system that ignores case merges the 2 folders, and the documents of 2 areas share one folder.
- Lowercase names with no message: spec-dna changes a name that a person chose, and each id under the area, with no review.
- A warning for a pair: the merge occurs at the clone, before a check runs. The rule extends an error rule that exists, so ADR 0003 allows an error.
- The alias as a condition, the rule of 0.6.1: this is the stated limit. A plain `git mv` gives a closed change that did not close.
- A new check of the move: it asks the person for a fact that git already records, and it adds a code.
- A hook that writes the alias: it runs only on a machine that has the hook, so the result depends on the machine (ADR 0006).

### Consequences

- Good: each limit gets a contract and a test ([test plan](../design/0.7.0.md#13-the-test-plan-and-the-build)). No diagnostic code and no key is new.
- Good: the prose that a person writes first under `Files` now works, and `index` writes no file that `check` rejects.
- Bad: `PARSER_VERSION` 14 makes each project rebuild its cache once.
- Bad: the cause of the stop of `index` is not proven. If the reproduction shows another cause, the build records the change of D29 in section 15 of the design.
- Bad: a current project with 2 area names that differ only in case stops with exit code 3. The person renames one area and moves its documents with `spec-dna mv` before other work.
- Bad: a person who moves one file of a folder and deletes the other files in the same commit makes a closed change. The rule reads it as an end.
- Bad: a `git mv` and a large edit in one commit still end the change.
- Neutral: the limit of `layer add` with a `types` list on many lines stays as it is.

## Amends

This ADR amends no locked text. ADR 0001, ADR 0003 and ADR 0006 stay as they are. The design of 0.6.1 is accepted, so read it with these changes:

- D23 and its entry in "Changes during the build": a paragraph under `Files` again states "no files" when its first word is `none`. The section still holds one item only.
- D9, condition 2, and the sentence of D10 that a plain `git mv` still ends the change. For a rename that git finds, the alias is no longer a condition. D8 stays: a move out that keeps the id as an alias gives no entry.
- D42: the compare in lowercase ASCII also applies to the names of `areas`, in the configuration reader, `new`, `mv` and `import`.
- The item "Open" of "Changes during the build": D29 and D30 replace it.

These change sets of `.claude/agents/ownership.md` apply: "Hash definition or parsed data" (D28), "Move, split or generated index" (D29), "Config key" (D30) and "Git access" (D31). If the new field of D28 goes into the cache file, "Cache layout" applies too. "Command, option, help text or exit code" applies to the new messages and to the exit code of `index`.

## Amendments

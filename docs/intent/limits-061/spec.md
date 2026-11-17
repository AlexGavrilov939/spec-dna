---
status: draft
---
# Limits of 0.6.1: index, area names and a plain move: spec

## Delta

None. This change adds, changes or removes no requirement. It changes the behavior of `index`, the configuration reader, `new`, `mv`, `init`, `log` and `stats --sdlc`. The rules are in [the design](../../design/0.7.0.md), D29 to D31 and section 15, and the docs of the build state them.

## Interfaces

None. No interface document describes the CLI or the configuration. The surface changes in this way.

`index` writes `status: <first status of the type>` when the type of its place has statuses and the file has none. It keeps a status that the file has, and it writes no line when the root gives the status. If the place gives no type, `index` stops with exit code 2 and writes nothing. The message is "the place `<dir>` gives no type; index a folder of a layer, or set `default-type` of the root". This was exit code 1.

A pair of names in `areas` that differ only in case is exit code 3, and the message names both. The schema description of `areas` says so. `new` and `mv` stop with exit code 2 and write nothing for an area that differs from a name of `areas` only in case. The area of a path is its first folder under the folder of the requirements layer. The message names the name of `areas`. The plan of `upgrade --layout --areas` stops with exit code 1 for a pair in its file, as for the other stops of `--areas`. `init` proposes the first name of such a pair (the most documents, then the name), and the preview names the other folder in a note.

`log` and `stats --sdlc` read a rename of `intent.md` out of the intent folder as a move when 3 conditions are true. First, the `-M` diff of the commit against its first parent has an `R` record from the deleted `intent.md`. Second, the record is `R100`, or the new file has the old id in `aliases`. Third, the same diff has no other `D` or `R` record from a file of the change folder. The `--json` shapes do not change.

## Decisions

- [[adr/0015-limits-of-0-6-1]]
- [[adr/0006-change-lifecycle]]
- [[adr/0003-distribution]]

## Concerns

- [x] The cause of D29 must be proven before the fix. Resolved: the reproduction of `failure-analyst` of 2026-11-17 proves it. `registry::render` writes no `status` line, and the `index` handler never resolves the type of its place. The empty target comes from the guard message of the write, and `check` is correct.
- [x] `templates::first_status` names a status for each type with statuses, so the stop of D29 cannot occur for SD304. Resolved: the stop applies when the place gives no type. It names the folder, because the place has no layer.
- [x] `finish` also calls `registry::render` for the refresh of an index. Resolved: one function gives the status for both callers, so `finish` never writes an index that `check` rejects. When the place gives no type, the refresh keeps the front matter as it is, and `finish` does not stop.
- [x] D30 names `new` and `mv`, but they do not read `areas` today. Resolved: each stops for an area that differs only in case, and with an empty `areas` they check nothing.
- [x] `init` can detect 2 folders that differ only in case and write a configuration that its reader rejects. Resolved: it proposes one name and gives a note.
- [x] ADR 0015 says that the proof of `upgrade --layout --areas` stops a pair with exit code 1. But the plan of `upgrade` parses the new configuration before the proof, so the reader alone gives exit code 3. Resolved: the plan of `--areas` stops a pair with exit code 1, as it stops an area that is not one folder name. The plan of `import --areas` does the same (change `import`).
- [x] Does a `git mv` of the whole folder out of the intent folder end the change? Resolved: yes. A rename of another file of the folder is a removal for condition 3, because the folder leaves the layer.
- [x] Does the rule add a git process? Resolved: no. `moved_out` reads the records of the one `git diff-tree` that it runs now, and `log` and `stats --sdlc` share it.

## Open questions

- none.

## Tests

- D29: `index` of a rule folder writes `status: draft`, and of a decision folder writes `status: proposed`, with `check --all` clean. The same in a root of one type. `--replace` over a file with no status writes the status, and over a file with `status: active` keeps it. A root with `default-type` and `status` gets no line. A place with no type gives exit code 2 and no file. The refresh of `finish` writes the status.
- D30: a pair in `areas` gives exit code 3. `new` and `mv` with `Valves` for the area `valves` give exit code 2 and change nothing. `upgrade --layout --areas` with a pair gives exit code 1. `init` on 2 folders that differ only in case proposes one name.
- D31: a plain `git mv` of `intent.md` gives no entry in `log` and no life in `stats --sdlc`. A near copy with no alias gives an entry. A near copy with the alias in a commit of `finish` that deletes the other files gives an entry. A `git mv` of the whole folder gives an entry. A folder with only `intent.md` and a near copy with the alias gives no entry (the limit that stays).

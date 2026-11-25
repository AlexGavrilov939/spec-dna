---
status: accepted
class: higher-risk
---
# A tag binds the test under it

## Intent

A `covers:` tag proves a requirement only through the test directly under it, or through the case that `case:` names in full (design D82). A case fits the test file of the tag by its `file` or by its `classname`. A bind by the name of the case only still counts, and the new warning SD621 reports it. A tag in a file outside the code tree binds no case (D85). `show` gives the test under each tag. `show --reports` and `check --reports --json` give the bind of each tag in the new field `binds`. The new page `docs/test-report.md` holds the reference of the test reports. The hook that runs the tests also runs `spec-dna evidence stamp`, and `init` and `upgrade` print a note for a test hook that does not stamp (D89).

## Why now

In a real project, a tag above an import line bound a case of another file through the word `it`, so requirements read as proven by tests that do not check them. The design measured the same false proof on a fixture (E22). A test file that git does not track can prove a requirement, also after an edit, with no SD614 (issue 113). A pre-commit hook that writes a report makes the stamp stale at each commit, so `check --reports` gives SD614.

## Out of scope

- The place of the tag in the text of the skill `tdd` (slice A7 of the design). A change of that text needs a paid rerun of the skill evals (ADR 0016), so a later change does it.
- A bind of a group (`describe`, `class`) to each of its cases.
- A new field or a new format of the acceptance record: the format stays 2.
- The source layers of the roll-up (D84) and the note of an old plan (D86).

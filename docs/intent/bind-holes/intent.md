---
status: accepted
class: higher-risk
---
# A failed test case never proves a requirement, a case of another language never fails a test, and 2,000 tags on one title add at most 1 s

## Intent

The bind of a `covers:` tag reads the identity of each case of a test report: its locator (the `file`, or the `classname` as its suite) and its head. A word as a `classname`, and `.`, are relative names, so they never leave the failed case of the test under the tag out of its set (B2). A full locator that names no reading of the tag file makes the case another test, so a failed case of another language no longer fails a passed test (B3). The built name of a call gets the forms that jest-junit and Vitest write with their defaults. The work of a tag no longer grows with the cases of another file or another full name (B4). The change also closes the 4 notes of the acceptor of part A, and it makes SD621 an error in `strict` only where a documented setting of the runner gives a strong bind. Tests run the bind on 24 reports that Vitest, jest with jest-junit, pytest, cargo-nextest, gotestsum and go-junit-report wrote, and a script of the repository writes these reports again. The change is for each project that turns the test reports on.

## Why now

The owner asked to close each hole of the bind before the release candidate of 0.8.1 (items 2 to 7 of part B, word of 2026-11-25). Today a failed test can read as proven through a `classname` of one word, in `standard` with only a warning. A lab run of the real reports also found a silent false proof through the `.` of gotestsum `relative` in a tree of 2 Go modules.

## Out of scope

- The `name` of the `testsuite` element (Q2 of the design).
- Reports of real runners for Playwright, Surefire, PHPUnit, RSpec and .NET.
- A bind of a tag above a group, and a new row of a runner.
- The heavy variant of the load L7 of part A.

---
name: spec-dna-tdd
description: In a spec-dna project, when a requirement has no evidence, write a test that fails before the code.
---

When to use: A requirement has no evidence, or the change adds behavior.

Commands:

1. Write a test for the requirement. Write `covers: <doc-id>#<key>` in the test.
2. Run the test before you write the code. Read its output. It must fail, for the reason that the requirement gives.
3. Write the smallest code that makes the test pass. Run the test again, then the other tests.

Artifact: A test with a `covers:` tag, run red before the code.

Check: `spec-dna check --base origin/HEAD` gives no SD608 and no SD916. SD608 is a requirement with no evidence. SD916 is a delta requirement with no `covers:` tag. They check the tag, not the order.

If the check fails: Add the tag, or fix its id. If the test passes before the code, it proves nothing: change it until it fails.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:44c5d55c6e6ffbb2 -->

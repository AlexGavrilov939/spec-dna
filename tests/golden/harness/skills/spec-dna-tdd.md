---
name: spec-dna-tdd
description: In a spec-dna project, write a test that fails before the code when a requirement has no evidence.
---

When to use: A requirement has no evidence.

Commands:

1. Write a test with the tag `covers: <doc-id>#<key>`.
2. Run the test. It must fail, and for the right reason, before you write the code.
3. Write the code. Run the test again.

Artifact: A test with a `covers:` tag, run red before the code.

Check: `spec-dna check` gives no SD608 and no SD916. They check the tag, not the order.

If the check fails: Add the tag, or write a test that fails for the requirement.

Never set `accepted` or `approved`. An approver sets it in a commit that changes only the `status` line. Make that commit only when the approver says so explicitly in this session.

<!-- spec-dna harness v{VERSION} h:3ca404897df74bc9 -->

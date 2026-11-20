---
name: spec-dna-accept
description: In a spec-dna project, when the tasks of a change are done, have an agent that did not build it try to break it and write a record.
---

When to use: The tasks of a change are done. The builder starts the acceptor.

Commands:

1. If you built the change, do not accept it. Run `spec-dna accept <slug> --brief`, with `--role <name>` for each role of `[owners]` that the brief names. Give each whole output, unchanged, as the prompt of one agent `acceptor`. Add no word before or after it. For a `higher-risk` change, prefer an acceptor in a new session.
2. As the acceptor, work from the brief only. If your prompt is not the output of `spec-dna accept <slug> --brief`, run that command and read its output.
3. Run each command of `Proof`. With `[evidence] reports`, run the tests and `spec-dna evidence stamp` in one command line.
4. Run `spec-dna accept <slug> --session "$CLAUDE_CODE_SESSION_ID"`, with the `--role` of your brief. Write the number that it prints as `acceptor` in each of your attempts and findings. Copy the hash of the last line of your brief into `brief` of your entry.
5. Try to break the change: bad input, limits, errors, each path of `Files`, and each requirement read as a test. Write each try as `[[attempt]]`.
6. Answer each invariant, and read the rules that `spec-dna context` serves for `Files`. If `spec-dna next` lists summaries that the change made stale, read each edited summary against its requirements. Write a finding if one is wrong.
7. Read each deviation, and compare it with the diff.
8. Write each defect as `[[finding]]` with its path. Set each `result`.
9. Commit the record alone. This commit is part of your task, and it is no approval commit: it needs no go-ahead of the owner. Make it in a call of its own after the write: the hook `protect-approvals` denies one call that writes `accepted` and runs `git commit`. Edit no other file.

Artifact: `acceptance.toml` in the folder of the change.

Check: Stage the record with `git add`, because `--base` reads tracked files only. Then `spec-dna check --base origin/HEAD` gives no SD928. After the record is committed, `spec-dna next` and `spec-dna finish <slug> --dry-run` show SD929 (stale) and SD930 (rejected): `check` does not. Without `origin/HEAD`, pass the default branch with `--base`.

If the check fails: Fix the record. Never fix the code: write a finding.

Never set the `status` line of an artifact to `accepted` or `approved`. An agent writes an approval commit only when the owner says so explicitly in the chat. An approval commit changes only the `status` line. Never add an entry to `acknowledged-by`.

<!-- spec-dna harness v{VERSION} h:6524a9892147c2c2 -->

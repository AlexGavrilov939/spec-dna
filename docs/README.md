# spec-dna documentation

This page lists each page of the documentation. The [README](../README.md) gives the short overview. Each page has one kind: it teaches, it explains, it gives the steps of a task, or it gives the reference.

## Start

- [Quickstart](quickstart.md): install, the first run of `init`, daily use, the CI job, upgrade and uninstall.
- [Demo](demo.md): the steps of the quickstart on a small example project.
- [Examples](../examples/README.md): 3 scripts that run spec-dna in a scratch repository, with their real output.
- [Troubleshooting](troubleshooting.md): the cause and the fix of the frequent stops of a new user.

## Concepts

- [Concepts](concepts.md): how spec-dna works. Types, ids, layers, the gate, the change lifecycle, context and the harness.
- [Glossary](glossary.md): each term and its meaning.

## Guides

- [One change from start to finish](guides/change.md): the intent, the spec, the plan, the code and `finish`.
- [Adopt spec-dna in an existing repository](guides/existing-repo.md): `init` on the current folders, the baseline and the gate.
- [Upgrade from 0.5 to 0.6](guides/upgrade-0.5.md): the renames, `spec-dna upgrade` and the first pull request.
- [Upgrade from 0.6 to 0.7](guides/upgrade-0.6.md): the new skill names, the pull request of the pin, the parts of 0.7.0 and the new checks.

## Reference

- [Commands](commands.md): each command, its options and the exit codes.
- [Configuration](config.md): each key of `spec-dna.toml`.
- [Document types](types.md): the types, their places, statuses and templates.
- [Diagnostics](diagnostics.md): each diagnostic code and its fix.
- [Profiles](profiles.md): the severity of each code in each profile.
- [Test reports](test-report.md): the bind of a `covers:` tag to a test case, the code tree and the stamp, the setup of a hook, the report formats and the proof levels.
- [init](init.md): what `init` detects and asks.
- [Agents](agents.md): how spec-dna works with each coding agent, and what each adapter adds.
- [Claude Code harness](harness.md): the roles, skills, hooks and `REVIEW.md` of the Claude Code adapter.
- [Evals](evals.md): the eval files, `eval run`, its cost limits and the CI job.
- [Transcripts](transcript.md): the token and read counts of agent sessions.

## Project

- [Evidence](evidence.md): the experiments and their limits.
- [Model team measurement](team-measurement.md): the pilot of the measurement of the model team, and why the batch did not run.
- [Model team measurement, protocol 2](team-measurement-2.md): the probes and the batch on real slices from the history, and the result.
- [Decisions](adr/): the ADRs of the project.
- [Designs](design/): the design of a version, with the deviations of each step.
- [Design note of 0.8.1](design/0.8.1-note.md): the start of the design of 0.8.1, with the parts that moved from the design of 0.8.0.
- [Design of 0.8.1, part A: evidence and context](design/0.8.1-a-evidence-and-context.md): the binding of a test tag, the summary tier of `context`, the pins of the roll-up and the untracked tests.
- [Design of 0.8.1, part A: lifecycle and harness](design/0.8.1-a-lifecycle-and-harness.md): the friction of the change process, the context meter, the default harness and the eval runner.
- [Harness additions of 0.8.1](design/0.8.1-harness-additions.md): the design of 5 harness additions: the subagent card, the limit of failed fixes, lessons, the cost report and `outline`.
- [Design of 0.8.1, part B](design/0.8.1-b.md): the summaries of the areas in `context` for a source file (D101).
- [Design of 0.8.1, part B: the holes of the bind](design/0.8.1-b-binding.md): the identity of a test case, many tags on one title, the notes of the acceptor, the level of SD621 in `strict`, and the tests on reports of real runners.
- [Research matrix](research/matrix.md): standards and prior art.
- [Changelog](../CHANGELOG.md), [Contributing](../CONTRIBUTING.md), [Security](../SECURITY.md).

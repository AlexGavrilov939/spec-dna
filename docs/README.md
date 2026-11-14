# spec-dna documentation

This page lists each page of the documentation. The [README](../README.md) gives the short overview. Each page has one kind: it teaches, it explains, it gives the steps of a task, or it gives the reference.

## Start

- [Quickstart](quickstart.md): install, the first run of `init`, daily use, the CI job, upgrade and uninstall.
- [Demo](demo.md): the steps of the quickstart on a small example project.

## Concepts

- [Concepts](concepts.md): how spec-dna works. Types, ids, layers, the gate, the change lifecycle, context and the harness.
- [Glossary](glossary.md): each term and its meaning.

## Guides

- [One change from start to finish](guides/change.md): the intent, the spec, the plan, the code and `finish`.
- [Adopt spec-dna in an existing repository](guides/existing-repo.md): `init` on the current folders, the baseline and the gate.
- [Upgrade from 0.5 to 0.6](guides/upgrade-0.5.md): the renames, `spec-dna upgrade` and the first pull request.

## Reference

- [Commands](commands.md): each command, its options and the exit codes.
- [Configuration](config.md): each key of `spec-dna.toml`.
- [Document types](types.md): the types, their places, statuses and templates.
- [Diagnostics](diagnostics.md): each diagnostic code and its fix.
- [Profiles](profiles.md): the severity of each code in each profile.
- [init](init.md): what `init` detects and asks.
- [Agent harness](harness.md): the roles, skills, hooks and `REVIEW.md` that `init` writes for Claude Code.
- [Transcripts](transcript.md): the token and read counts of agent sessions.

## Project

- [Evidence](evidence.md): the experiments and their limits.
- [Decisions](adr/): the ADRs of the project.
- [Designs](design/): the design of a version, with the deviations of each step.
- [Research matrix](research/matrix.md): standards and prior art.
- [Changelog](../CHANGELOG.md), [Contributing](../CONTRIBUTING.md), [Security](../SECURITY.md).

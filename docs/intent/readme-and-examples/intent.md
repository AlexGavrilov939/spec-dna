---
status: accepted
class: routine
---
# Readme and examples

## Intent

Make the GitHub page of spec-dna show its value on the first screen. Give a new user a first result in 1 minute. The change is for a developer who sees the repository for the first time:

- A new `README.md`. It starts with real output of `check`, gives a path to try the tool in a scratch repository, compares spec-dna with spec frameworks, and states the limits.
- 3 examples in `examples/`: a first project, an existing repository with old problems, and one change from the intent to `finish`. Each example is a script that makes a scratch repository and runs the real commands. A test runs each script and compares each `console` block of the pages with the real output.
- A logo and a social preview image, each with a light and a dark variant where GitHub can show both.
- A troubleshooting page for the frequent stops of a new user.
- Recommendations for the owner, outside the repository: the GitHub description, the topics and the launch text.

The owner's brief is the acceptance of this intent. The text treats 0.6.1 as the current release.

## Why now

The repository is public, and 0.6.1 is the first release with the stable names of 0.6. The current README is long and dense. Its first screen shows no output, and its quick start has no step that shows a result.

## Out of scope

- A change of the behavior or the defaults of a command. The findings about `init` go to the owner.
- The 11 files that the 0.6.1 branch edits: `CHANGELOG.md`, and `commands.md`, `concepts.md`, `config.md`, `diagnostics.md`, `glossary.md`, `init.md`, `profiles.md`, `types.md`, `guides/change.md` and `guides/upgrade-0.5.md` in `docs/`.
- GitHub settings, a release, a push and a pull request. The owner approves or does each of them.
- A documentation website.

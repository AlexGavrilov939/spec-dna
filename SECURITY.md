# Security policy

## Supported versions

spec-dna is at version 0.x. Only the latest release gets security fixes.

## Report a vulnerability

Do not open a public issue for a security problem. Use the private report form of GitHub: [Report a vulnerability](https://github.com/AlexGavrilov939/spec-dna/security/advisories/new).

Give the version (`spec-dna --version`), your platform, the steps to reproduce the problem, and its effect. The maintainer answers in the private advisory. When a fix is available, the release notes and a GitHub security advisory name the problem.

## Scope

spec-dna reads the files of a local git repository and runs the `git` command line. It does not use the network. These problems are in scope:

- A document, a `spec-dna.toml` or a git object that makes spec-dna run a program, or read or write a file outside the repository.
- A symbolic link that makes spec-dna read a file outside the repository.
- `spec-dna transcript` output that contains prompt, command or file text. This command must print counts and ids only.
- A release binary that does not match its published SHA-256 sum or its build-provenance attestation.

## Verify a release binary

Each release has the file `SHA256SUMS` and a GitHub artifact attestation for each binary. [docs/quickstart.md](docs/quickstart.md#install) gives the commands to verify them.

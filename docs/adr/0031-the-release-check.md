---
status: accepted
---

# 0031. The release check

## Context and Problem Statement

[ADR 0003](0003-distribution.md) publishes each release with build-provenance attestations. On the day of 0.7.1, the check `gh attestation verify` printed nothing, and the person read the result as a pass. The cause, proved by the failure analysis:

- The attestations exist: the release workflow made them, and their 9 subjects equal the digests of the 9 assets.
- GitHub CLI 2.101.0 prints nothing on success when its standard output is not a terminal.
- The call wrote its output to a file and ended with `; echo ok`, so the exit code was lost.
- No script and no workflow step verifies the assets. Only 2 pages of the docs name the command.

How does each release prove its attestations with a check that always reports a failure?

Constraints:

- The binary has no network, and this check adds no dependency to it.
- The repository has immutable releases. After the publication, nobody can change an asset.
- The exit codes of `gh`: 0 verified, 1 a failure (absent, policy, network), 4 no login.

## Considered Options

1. A step in the checklist only: a person runs the command and reads the output.
2. A script that checks each asset, a workflow step that runs it before the publication, and the same script after the publication.
3. A check inside the binary.

## Decision Outcome

Option 2.

### The script

`scripts/verify-release.sh <tag> <dir>`, or `scripts/verify-release.sh --download <tag>`, which first downloads the assets of the release into a temporary folder. The script:

1. Needs exactly the assets of the version in the folder: the binary of each target of `scripts/cross-build.sh --list --all`, `SHA256SUMS` and `spec-dna-pin.toml`. A file too many or too few fails before `gh` runs.
2. Checks each binary against `SHA256SUMS`.
3. Runs, for each asset, `gh attestation verify <asset> --repo <repo> --signer-workflow <repo>/.github/workflows/release.yml --source-ref refs/tags/<tag> --deny-self-hosted-runners --format json --jq length`. It keeps the exit code and the output apart. It needs exit code 0 and an integer of 1 or more. It never chains a command with `;`, and it never ends a command with `|| true`.
4. Checks each asset, names each failure with its reason, and fails at the end when one asset failed.
5. Prints `verified <asset>: <n> attestation(s)` for each asset, and at the end `verify-release: <n> of <n> assets verified for <tag>`.

Exit codes: 0 each asset is verified, 1 a failure, 2 a usage error.

### The workflow

The release job creates the release as a draft and runs `scripts/verify-release.sh` on the local assets (the same digests as the attestations). Only then does it publish the draft. A failure keeps the draft, so nothing is published. The job keeps its permissions.

### The checklist and the docs

- The checklist of `release-engineer` gets a step after the publication: run `scripts/verify-release.sh --download v<version>` and give the whole output. No final line `<n> of <n>`, or an exit code other than 0, is a failed release.
- The pages that name `gh attestation verify` say that the command prints nothing when its output goes to a file or a pipe. They tell a person to check its exit code.

### The test

A test puts a stub `gh` first on `PATH`. The asset names come from the targets of `src/release.rs`. These cases fail: a stub that exits 0 with no output (the defect of 0.7.1), a stub that prints 0 and a stub that exits 1 for one asset. An absent asset, an extra file and a wrong sum also fail. A stub that prints 1 passes. The stub records its arguments. Each call has the signer workflow, the source ref and `--deny-self-hosted-runners`.

### Rejected

- Option 1: the defect of 0.7.1 was a step of a person with a lost exit code.
- Option 3: a network call in the binary, against ADR 0003 and the offline rule of `check`.
- A check after the publication only: an immutable release cannot be fixed, so the check must stop the publication.
- An offline check with a downloaded bundle and a trusted root. The release does not publish the bundle, and the check needs one online fetch first.
- A scheduled job that checks each release again: one more workflow for a result that the release job already proves.

### Consequences

- Good: a release with an asset that has no attestation of this workflow and this tag is never published.
- Good: the check of a person and the check of the job run the same script.
- Bad: the release job makes 9 more calls to the API, about 10 seconds.
- Neutral: the check proves the provenance of the assets. It does not prove that the source of the tag is safe.

## Amends

ADR 0003 is accepted and locked, so this ADR does not edit it. This rule applies in addition to its section on phase 2. The release workflow verifies the attestation of each asset with `scripts/verify-release.sh` before it publishes the release.

## Amendments

# Example: an existing repository

This example adds spec-dna to a repository that already has documents in its own folders, and old problems in them. No document moves, and nobody has to fix the old problems first. Read it before you adopt spec-dna in a mature repository.

## Run it

```sh
examples/existing-repo/run.sh
```

You need `spec-dna` on your `PATH`, git 2.32 or later, bash and perl. [The list of examples](../README.md#run-an-example) gives the details.

## 1. Map the folders

The repository has decisions in `docs/adr/`, specs in `docs/specs/` and a runbook in `docs/runbooks/`. A root maps each folder to one type, so the folders keep their names:

```toml
[[roots]]               # each folder keeps its name and gets one type
path = "docs/adr"
type = "decision"

[[roots]]
path = "docs/specs"
type = "requirements"
```

The script gives the roots in an answers file of `init`. In an interactive `init`, you accept, drop or edit each folder that `init` detects. Check the proposed types: `init` can propose one type for all folders under `docs/`. [The adoption guide](../../docs/guides/existing-repo.md) gives the steps.

```console
$ spec-dna init --answers ../answers.toml
spec-dna init: brownfield repository; profile standard; 3 roots (see spec-dna.toml)
...
```

## 2. Take the baseline

`check --all` reports each problem in the documents once. The runbook has no `status`, and it links to a page that does not exist:

```console
$ spec-dna check --all
docs/runbooks/deploy.md:1: error SD304 status-invalid: type `procedure` needs a `status` (draft, active, deprecated, superseded, retired)
docs/runbooks/deploy.md:3: error SD102 unresolved-link: link target `release-checklist.md` is not in the git tree (resolved to `docs/runbooks/release-checklist.md`)
docs/specs/billing.md:8: warning SD608 requirement-unverified: `billing#charge-once` is active but no test covers it (`covers:` tag or `verified-by`)
...
spec-dna check: 3 documents; 2 errors and 1 warning
```

You do not have to fix these problems now. Fix each one when you edit its document for another reason.

## 3. Gate the next change

A branch adds a requirement. The gate compares the branch with `main`. It reports only the problems that the branch adds, so the old errors do not stop it:

```console
$ spec-dna check --base main
docs/specs/billing.md:9: warning SD608 requirement-unverified: `billing#refund-window` is active but no test covers it (`covers:` tag or `verified-by`)
...
spec-dna check: 3 documents; 0 new errors and 1 new warning against cffeaef098ad (2 errors and 2 warnings in total)
```

The next commit adds a link to a page that does not exist. Now the gate stops the branch with exit code 1:

```console
$ spec-dna check --base main
docs/specs/billing.md:9: error SD102 unresolved-link: link target `refund-policy.md` is not in the git tree (resolved to `docs/specs/refund-policy.md`)
...
spec-dna check: 3 documents; 1 new error and 1 new warning against cffeaef098ad (3 errors and 2 warnings in total)
```

In CI, run the same command against the base branch of the pull request. `init` can write the job ([quickstart](../../docs/quickstart.md#ci)).

## If it fails

| Message | Cause | Fix |
| --- | --- | --- |
| `SD303 type-invalid` for each document | A folder has no type: no root maps it, and it is not the folder of a layer | Add a `[[roots]]` table for the folder, or a `type:` line to the document |
| `SD304 status-invalid` for each document of a root | The type needs a `status`, and the old documents have none | Add a `status` line, or set `status` in the `[[roots]]` table of the folder. `status-map` maps old status words ([docs/config.md](../../docs/config.md#keys)) |
| The output differs from this page | Your binary has another version (`spec-dna --version`) | Install the current release |

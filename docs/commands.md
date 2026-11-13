# Commands

This page gives each command of spec-dna, its options and the exit codes. [quickstart.md](quickstart.md) gives the steps in the sequence of a new project.

## Summary

```sh
spec-dna init                       # detection, 8 questions, a preview, then spec-dna.toml and the adapters
spec-dna init --dry-run             # the preview only (also --non-interactive, --answers <file>, --reconfigure)
spec-dna new decision psp-choice    # a new document from the template of its type
spec-dna context src/gateway.rs     # the sections that cite a code file
spec-dna check                      # new diagnostics against HEAD; exit 1 on a new error
spec-dna check --base origin/master # in CI: new diagnostics against the merge base
spec-dna check --all                # every diagnostic
spec-dna check --lint               # also list each prose lint warning
spec-dna context auth/session --path src/gateway.rs
spec-dna context auth/session --query "renew a session after a password change"
spec-dna show auth/session#token-expiry
spec-dna mv auth/session auth/sessions          # the old id becomes an alias
spec-dna mv auth/session#token-expiry auth/tokens
spec-dna mv auth/session#renewal auth/session#token-expiry auth/tokens   # several keys
spec-dna mv auth/session#mfa auth/second-factor  # a whole section into a new spec
spec-dna mv guides/deploy#rollback runbooks/rollback --type runbook   # a section of a guide into a new runbook
spec-dna history decisions/cache docs/history/cache.md   # the amendment and history sections into a reference document
spec-dna index docs/incidents --replace         # a generated index in place of a hand-kept registry
spec-dna mv guides/old guides/new --stub        # a stub at the old path for tools that read by path
spec-dna transcript ~/.claude/projects/<project> --expect auth/session
spec-dna --help                     # commands and options (-h); --version (-V) prints the version
spec-dna mv --help                  # the help of one command
```

Each command accepts `--json`. spec-dna does not use the network. It reads the documents with local `git` commands.

A target is a document id, an alias such as `ADR-12`, `<id>#<anchor or key>`, or a repository path.

## init

```text
spec-dna init [--non-interactive | --answers <file>] [--reconfigure] [--dry-run] [--root <dir>]
```

`init` examines the repository and finds these items:

- the document roots and their types, the ids, the requirement keys and the statuses;
- the languages of the headings;
- the vendored, generated and large reference directories;
- the budgets, the stack, the tests, the agent files, CI, the hooks and the default branch.

Then `init` asks at most 8 questions, with the found values as defaults. It shows the full `spec-dna.toml` and the files that it will create or change. It writes them after you confirm. `--dry-run` shows this preview only. Without a terminal, `init` uses the defaults. `init` never edits a document. [init.md](init.md) gives the details.

`init` writes these adapters:

- one line in `AGENTS.md`, if the file exists or if neither file exists;
- one line in `CLAUDE.md`, unless `CLAUDE.md` imports such an `AGENTS.md`;
- the skill `.claude/skills/spec-dna/SKILL.md`.

The line never imports another file. Each adapter has the version of its generator and a hash. `check` reports a manual edit or an old version. A second run of `init` updates the adapters. It also replaces the blocks of many lines that earlier versions wrote.

If `spec-dna.toml` exists, `init` updates only the adapters. With `--reconfigure`, it asks the questions again. `init` does not run if the pin in `spec-dna.toml` is a different version.

At the end, `init` prints the bytes that the adapters add to the prompt prefix of each agent request (`--json`: `prefix_bytes`). It also prints the `git add` command for the files that it wrote.

## check

```text
spec-dna check [--base <rev>] [--all] [--lint]
```

`check` runs the controls of ADR 0001 on the working tree. With `--base`, the base is the merge base of `<rev>` and `HEAD`. Without `--base`, the base is `HEAD`.

- Without `--all`, `check` reports only the diagnostics that are new against the base.
- With `--all`, it uses the same base, but it reports each diagnostic and fails on any error.
- Without `--base`, `check` also reads the untracked documents that git does not ignore.
- With `--base`, it reads only tracked files, as CI does.

A change can remove or rename a path or an identifier that a document cites. Then `check` gives a warning that names the document (SD606, SD607). In an accepted decision, an amendment section that cites the new path or identifier clears these warnings.

The text output gives the errors first, then SD606 and SD607, then the other warnings. Each prose lint code (SD801 to SD805) gets one line with a count. `--lint` lists each of these warnings. Another warning code with more than 10 diagnostics also gets one line with a count. At the end, the output gives the fix of each listed code once.

The JSON output lists each diagnostic. It also lists each document with its number of recognized requirements. The profile sets the severity of each code ([profiles.md](profiles.md)). [diagnostics.md](diagnostics.md) gives each code.

## context

```text
spec-dna context <target>... [--path <file>]... [--budget <chars>] [--query <text>] [--include-inactive]
```

`context` prints the sections that an agent needs, in this order:

1. The entry documents or sections.
2. For a code path: the sections that cite the file by its path, or by an identifier that is in the file ([diagnostics.md](diagnostics.md#code-citations)). A code path is a target or a `--path`, and it can be an untracked file. The documents do not need `applies-to`.
3. The requirement sections and the summary of the best documents that cite the file.
4. The documents whose `applies-to` matches an entry or a path, and the sections that cite only a directory of the path.
5. The `depends-on` targets at depth 1, and the linked accepted decisions as summaries.
6. The other sections.

`context` serves at most 8 documents and 5 summaries that cite a file. It lists the other sections that cite the file. It lists a change or a note that cites the file, but it does not serve that document.

In a document with requirements, the requirement sections come first, then the title section. `context` lists the other sections of that document.

`context` does not serve a superseded, retired or rejected document. It lists the document with its status and its successor. A target that names the document, or `--include-inactive`, makes `context` serve it.

If a section does not fit the budget, `context` lists its id, hash and size. Each section has its id and hash.

With `--query` (the task in plain words), the sections that match its words come first. BM25 ranks them, without a network or a model. Use this option for long documents without requirement lists.

## show

```text
spec-dna show <id>[#<anchor>]
spec-dna show --conventions <type>
```

`show <id>` prints a document, a section with its subsections, or one requirement, with ids and hashes. For a spec or a contract, it also prints these items:

- the number of recognized requirements (`--json`: their keys);
- the evidence of each requirement: its `verified-by` paths and the `covers:` tags of the tests.

`show --conventions <type>` prints the conventions for documents of the type. These are the sections of project documents that `[conventions]` in `spec-dna.toml` names. The output also gives the names of the amendment and live sections of the type. When a target of `context` is a document of that type, `context` serves the same sections first.

## new

```text
spec-dna new <type> <id> [--root <dir>]
```

`new` writes a new document from the template of its type ([types.md](types.md)). In a mixed root, an id without a namespace gets the namespace of its type, such as `decisions/<id>`.

## mv

```text
spec-dna mv <from> <to> [--stub]
spec-dna mv <doc-id>#<key or anchor>... <doc-id or path> [--type <type>]
```

The first form moves a document with `git mv`. It adds the old id to the `aliases` of the document, and it changes the relative links to the document. `--stub` leaves a stub at the old path for tools that read a file by its path. The stub has the front matter `moved-to`, no id and no budget.

The second form splits a document of any type. It moves requirements, or whole sections with their subsections, into one of these documents:

- an existing document of any type;
- a new document. `mv` writes its front matter and its title. Its type is the type of its root, `--type`, `reference` for the history of a decision, or the type of the source.

These rules apply to a split:

- Requirements move only into a spec, a contract or an invariant.
- Each moved section of `HEAD` keeps its old id as an alias: `<old-id>#<anchor>` in the `aliases` of the destination.
- Each moved requirement whose id is in `HEAD` gets an inline marker `<!-- aliases: ... -->` with that id. A key that is new in the uncommitted change gets no alias.
- The rows of `companion-sections`, such as an acceptance map, move with their requirements. If a row names a key that moves and a key that stays, `mv` refuses the move.
- Each moved section leaves its heading in the source, with a one-line pointer to its new place.
- Links to a moved key or section follow it. This rule also applies in accepted decisions, because a link destination is not frozen text.
- A decision keeps its frozen text. Only its amendment and history sections move, into a reference document. `mv` lists the moved amendments that can change the outcome. These amendments are candidates for a new decision that supersedes the old one.

Before `mv` changes a file, a preservation check compares the result with the current tree:

- each requirement has the same hash (without link destinations);
- each requirement id resolves to the same requirement;
- the frozen text of an accepted decision does not change;
- there is no new error.

If a check fails, `mv` changes nothing. It names each requirement whose text would change, and it exits with code 1.

## history

```text
spec-dna history <doc-id> [<to>]
```

`history` moves the history sections of a document into a reference document. The history sections are the `history-sections`, and the amendment sections of a decision. `history` leaves a pointer for each section, and it adds a backlink. The default destination is `<name>-history.md` next to the document, in a mixed root.

## index

```text
spec-dna index <dir or file> [--replace]
```

`index` writes a generated index of the documents under a directory. The index has one row for each entry, with its name, title and status. `--replace` replaces a file that a person keeps manually. Then the links to a section of the old file that names an entry go to the entry. `check` reports a stale or manually edited index (SD610).

## transcript

```text
spec-dna transcript <file or dir>... [--expect <id>]... [--format <name>]
```

`transcript` reads agent transcripts. The first format is Claude Code JSONL. For each session and episode, it reports these counts:

- the input, cache-creation, cache-read and output tokens;
- the tool calls;
- the documents and sections that the agent read.

It prints counts and ids only. It never prints the text of a prompt, a command or a file. [transcript.md](transcript.md) gives the details.

## Exit codes

| Code | Meaning |
| --- | --- |
| 0 | Success. For `check`: no new error (with `--all`: no error) |
| 1 | `check` found a new error (with `--all`: any error), or the preservation check of `mv`, `history` or `index` refused the change |
| 2 | Usage error |
| 3 | Configuration error: no valid `spec-dna.toml`, a different version pin, an unknown base revision, or no git repository |
| 4 | Git or file system error |
| 5 | A target names no document, section or requirement |

## Cache

spec-dna keeps the parsed documents in a cache at `<git-dir>/spec-dna/index-v1.bin`, with the git blob id as the key. Thus, the cache never goes into a commit, and it stays after a branch switch. On each run, spec-dna parses the files that differ from the git index, and the untracked files. It writes the cache to a temporary file and then renames that file. You can delete the cache at any time, or use `--no-cache`.

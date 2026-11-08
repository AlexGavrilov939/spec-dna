# Transcripts: `spec-dna transcript`

`spec-dna transcript` reads the transcripts of agent sessions. It reports, for each session and each episode, the token usage, the tool calls and the documents that the agent read. An A/B measurement uses it to compare two arms with real usage instead of document sizes.

```sh
spec-dna transcript ~/.claude/projects/<project>/<session>.jsonl
spec-dna transcript <dir> --expect auth/session --expect ADR-12 --json
```

An argument is a file or a directory. A directory gives every `.jsonl` file below it, so the files of subagents are included. In a directory, a file that no adapter reads (for example a workflow journal) gives a warning and the command continues. A file that you name must be a transcript. A file that two arguments give is read once. Run the command in a repository with `spec-dna.toml`. Then the report maps file reads to document ids and sections. Outside such a repository, the report has token and tool counts, and the ids in spec-dna output only.

## Privacy

A transcript can contain secrets: prompts, commands, file contents and tool output. The command prints counts, tool names, model names, times, session and agent ids, and spec-dna document and section ids. It never prints prompt, command, path or file text. It reads that text in memory to find document reads, and then drops it. A test with secret text in every field checks this rule.

## Terms

| Term | Meaning |
| --- | --- |
| Session | All records with the same session id, from all files. The main agent and its subagents are in one session. |
| Agent | `main`, or the id of a subagent. |
| Episode | The records of one agent from one prompt to the next prompt of that agent. A prompt is a message that is not a tool result, not a meta message and not a compaction summary. Its origin is `human`, or the kind that the transcript records, such as `task-notification`. The report leaves out an episode without requests and without tool calls. |
| Request | One model request. A transcript can repeat the usage of a request on several lines, and the file of a forked subagent can repeat records of its parent. The command counts each request, prompt, tool call and tool result once, by its id, and reads the files of main agents first. A message that the agent writes itself (`<synthetic>`) is not a request. |

## Metrics

| Metric | Source | Kind |
| --- | --- | --- |
| `tokens.input`, `cache_creation`, `cache_read`, `output` | The usage record of each request | Measured |
| `requests`, `tool_calls`, `tools`, `errors` | Tool calls and their results | Measured |
| `searches` | Search tools, and shell commands that start with `grep`, `rg`, `find`, `ag` or `fd` (not after a `\|`) | Measured for tools, heuristic for shell commands |
| `documents`, `doc_reads`, `repeated_reads` | See "Document reads" | Measured for read tools and spec-dna output, heuristic for shell commands |
| `spec_chars` | Characters of the tool results that the report gives to documents | Measured for read tools; estimated for a shell result that names more than one document (its size is split evenly) |
| `spec_tokens_estimated` | `ceil(spec_chars / 4)` | Estimated. The same estimator applies to every arm. |
| `duration_ms` | First and last record time of the episode | Measured |
| `expected` (with `--expect`) | `read`: the expected documents that the agent read (a read of the front matter only does not count). `first_relevant_ms`: the time from the start to the first such read | Measured |
| `document_edits` | Edit and write tools on document paths | Measured |

The usage of a request includes the full prompt prefix, so `cache_read` grows with the length of a session. Spec-related tokens are thus an estimate of the document text that entered the context. They are not a share of the billed input.

## Document reads

The report gives a tool result to a document in these cases:

1. A file read tool reads a document path. If the read has a line range, the report lists the sections that the range overlaps. A range that covers only the front matter shows as `front matter only`. A read without a range is a full read.
2. A reader program in a shell command (`cat`, `head`, `tail`, `sed`, `less`, `more`, `bat`, `nl`, `awk`, `grep`, `egrep`, `rg`, `ag`, `cut`, `diff`) gets a document path, for example `sed -n 1,40p docs/a.md`. The report counts a full read. A `>` target, `git add` or `wc` is not a read. If the command names several documents, each gets an equal share of the result size.
3. A shell command runs `spec-dna`, and its result contains section headers (`<!-- <id> h:<hash> ... -->`). Each served section counts. A listed stub does not count. With a configuration, only ids that name a document count. Without one, only strings with the form of an id count (letters, digits, `-`, `_`, `.`, `/`, `#`). Thus other text with the same shape cannot reach the report.

A path maps to a document in this order: the path relative to the repository root, the path relative to the working directory of the agent (a relative path starts there), and then the longest tail of the path with at least 2 segments that is a document path. Thus a session in an exported copy of the repository maps to the same ids.

## Adapters

An adapter converts one transcript format into a small set of records: prompt, usage, tool call and tool result. The analysis does not depend on the format. The first line that parses selects the adapter of a file; `--format <name>` selects one for all files.

| Adapter | Format |
| --- | --- |
| `claude-code` | Claude Code JSONL. Subagents write their own files, with `isSidechain` set. The adapter reads the file read, edit, search and shell tools by their names. |

To add a format, implement the `Adapter` trait in `src/transcript.rs` and add it to `ADAPTERS`. Map the tools of the agent to the targets `Read`, `Write`, `Search` and `Shell`.

## Limits

- A shell command that reads a file through a variable, a glob or a wrapper program is not mapped.
- A read of a long file that a tool cuts without a line range counts as a full read.
- The format of agent transcripts can change without notice. A test with synthetic transcripts covers the fields that the adapter reads.

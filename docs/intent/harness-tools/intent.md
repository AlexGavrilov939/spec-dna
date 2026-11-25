---
status: draft
class: higher-risk
---
# `transcript --cost` gives the USD of a change, and `outline` gives the line ranges of the definitions of a code file

## Intent

`spec-dna transcript <file or dir>... --cost [--change <slug>] [--prices <file>]` sums the list price of agent transcripts in USD, by change, model and role (item 21 of the [harness additions of 0.8.1](../../design/0.8.1-harness-additions.md)). It uses a price table in the binary, gives a lower limit for the advisor, and keeps the privacy rule of `transcript`. `spec-dna outline <path>... [--json]` prints the definitions of a source file with their line ranges (item 22). Line rules for Rust, Python, Go, JavaScript and TypeScript, and Java find them, with no new dependency. The adapter skill and `Explore` of `standard` and `strict` tell an agent to run `outline` before it reads a long code file. The change is for the person who pays for a change, and for the agent that must read one part of a long file.

## Why now

The owner answered Q10 to Q16 of the design with the recommended option of each (word of 2026-11-25), and asked for the build in 0.8.1. Today a person prices each model by hand and cannot tell which session belongs to a change. An agent reads a whole long file, and the file stays in each later request. The 2 agent texts must be final before the paid reruns of `execute` and `review` in this part.

## Out of scope

- The card, the return states, the fix limit and the lessons (items 18 to 20): the change `harness-reports`.
- tree-sitter (Q14 b) and more language families (Q15 b).
- A sentence in the line of `AGENTS.md` and `CLAUDE.md`, or in the context meter (Q16 b and c).
- A command `cost` (Q10 b), a table `[prices]` in `spec-dna.toml` (Q11 b), the branch of a transcript and a session trailer (Q12 b and c).
- An invoice: a fixed fee, a discount, and a price for a tier, a speed or a long request.
- The texts of `lite`.

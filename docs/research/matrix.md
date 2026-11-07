# Research matrix: standards and prior art

- Date: 2026-11-07. All sources accessed on that date.
- Scope: which standards and tools spec-dna adopts, adapts, cites or rejects.
- Method: primary sources first (standards bodies, official repositories, release pages). A claim marked **unverified** was not confirmed on a primary source. iso.org, incose.org and asd-ste100.org returned HTTP 403 to automated fetches, so some statements about them rest on official previews or search snippets.
- Ratings (H high, M medium, L low) are this project's assessment, not measurements. For "complexity" and "adoption cost", L is better.

## 1. Standards

| Standard | Version seen | Applicability | Scalability | Complexity | Maintainability | Machine validation | Agent efficiency | Adoption cost | Verdict |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| BCP 14 (RFC 2119 + RFC 8174) | RFC 2119 1997-03, RFC 8174 2017-05 | H | H | L | H | H (uppercase keyword set) | H (one declaration per corpus) | L | Adopt as the only normative vocabulary |
| EARS (Mavin et al., RE'09) | 2009 paper, 5 patterns | H | H | L | H | M (pattern classification is exact; a mismatch is only a warning) | H (one-line requirements, filterable by trigger) | L | Adopt as requirement sentence syntax |
| MADR | 4.0.0, 2024-09-17 | H | H | L | H | M (status, date, headings) | M (status lets agents skip superseded ADRs) | L | Adopt the minimal template |
| Nygard ADR | 2011-11-15 | H | M | L | H | M | M | L | Conceptual base of MADR |
| Diátaxis | unversioned | M | H | L | H | L (mode is declared, fit needs judgment) | M (agents skip tutorials and explanation) | L | Optional vocabulary (`x-` metadata); not required by ADR 0001 |
| DITA | 1.3 OASIS Standard 2015-12-17; 2.0 still working draft (latest seen 2024-08-26) | M (concepts only) | H | H as XML, M as concepts | M | M (keys, includes, cycles resolve) | H (map plus only needed topics) | H as XML | Adopt concepts: typed topics, maps (generated), keys (ids with aliases). Includes deferred. Reject XML |
| arc42 | template EN 9.1, release 2026.10.19 | M | M | M | M | L (12 headings present) | M | M | Reference only; optional template |
| C4 model | unversioned | M | H | L | H | L (level as an enum) | M | L | Optional `x-` metadata on diagrams |
| ISO/IEC/IEEE 29148 | 2nd ed. 2018-11 current; 3rd ed. at DIS ballot since 2026-09-14 | M | H | M | M | M (ids, attributes, own vague-term list) | L | M (paid text) | Reference only; map own checks to its characteristic names |
| INCOSE GtWR | v4, INCOSE-TP-2010-006-04, 2023-07-01 | M | H | M | M | M (about half the rules are lexical; counts unverified) | L | M (paid text) | Reference only; write own lexical checks, cite rule ids |
| ASD-STE100 | Issue 9, 2025-01-15, 53 rules | L (aerospace maintenance text) | M | H with dictionary | L | L without the dictionary | M (shorter sentences) | H | Adopt own length limits only; never claim compliance |
| CommonMark | 0.31.2, 2024-01-28 | H | H | L (library) | H | H (AST) | H | L | Adopt as the parse target; GFM tables as an extension |
| YAML front matter | 1.2.2, 2021-10-01 | H | H | L | H | H | H (metadata before bodies) | L | Adopt, restricted to a JSON-compatible subset |
| JSON Schema | 2020-12 | H | H | L | H | H | n/a | L | Adopt for front matter and index schemas |
| TOML | 1.1.0, 2025-12-18 | M | H | L | H | H | n/a | L | Adopt for tool configuration, within 1.0 syntax |
| SemVer | 2.0.0 | H | H | L | H | H | n/a | L | Adopt for the tool and the standard |
| OpenAPI, AsyncAPI | OAS 3.2.1 (2026-09-10), AsyncAPI 3.1.0 | M | H | L (delegate) | H | H (existing validators) | M | L | Link and index contracts; do not reimplement validation |
| llms.txt | proposal 2024-09-03, modified 2026-08-10 | M | M | L | H | M (H1 plus link lists) | H (cheap index) | L | Adopt the shape for the served index |
| Docs as code | Write the Docs guide | H | H | L | H | H (CI) | n/a | L | Adopt (the repository already works this way) |

## 2. Licensing constraints

| Source | Terms | What spec-dna may do |
| --- | --- | --- |
| ASD-STE100 Issue 9 | Copyright and EU trade mark 017966390 of ASD; free download through a form; no redistribution (quote of the no-reproduction clause **unverified**) | Ship nothing from it. Do not use the name in rule names. Implement generic sentence and paragraph length limits in own words. Never claim compliance |
| INCOSE GtWR v4 | INCOSE copyright; sold through the INCOSE Store; reproduction and derivative works not automatically granted (from search snippets, **unverified** on the page) | Cite rule ids and characteristic names. Write own rule text and word lists |
| ISO/IEC/IEEE 29148 | Paid; no reproduction in any form without permission (official preview) | Cite characteristic names only |
| EARS | No licence stated; the author's site says "All Rights Reserved" | Implement the grammar as a method; write own documentation; cite the paper |
| BCP 14 | IETF Trust Legal Provisions 5.0, section 3.c: unmodified excerpts with attribution | Quote the boilerplate word for word with attribution |
| MADR | MIT OR CC0-1.0 | Bundle the template |
| Nygard ADR | CC0 1.0 | Bundle |
| C4 model | CC BY 4.0 | Bundle with attribution |
| arc42, Diátaxis | CC BY-SA 4.0 | Ship only as separately licensed files; adapted templates stay BY-SA. Mode names as vocabulary need no copy |
| JSON Schema meta-schemas | BSD-3-Clause OR AFL-3.0 | Bundle |
| CommonMark spec | CC BY-SA 4.0 (spec), BSD (reference implementations) | Use libraries; do not copy spec text |

Consequence: the spec-dna standard is written in its own words under the repository licence. It cites external standards and never bundles proprietary text.

## 3. Specification frameworks

| Tool | Latest release | Licence | Canonical storage | Change model | Deterministic checks | Lifecycle | Footprint |
| --- | --- | --- | --- | --- | --- | --- | --- |
| GitHub Spec Kit | v1.3.2, 2026-11-06 | MIT | `specs/NNN-feature/{spec,plan,tasks,...}.md`, `.specify/memory/constitution.md` | One folder per change; retention is a team convention | None for spec content in core; `analyze` and `converge` are LLM prompts | No archive or supersede command | Python 3.11+, uv; per-agent command files |
| OpenSpec | v1.16.1, 2026-11-07 | MIT | `openspec/specs/<capability>/spec.md` as current truth; `openspec/changes/<name>/` with deltas | Delta specs (ADDED, MODIFIED, REMOVED, RENAMED) merged on archive | `openspec validate --strict --json`: sections, SHALL/MUST, scenario per requirement, delta grammar | `archive` moves changes to a dated folder | TypeScript; binary or Node 22.12+; telemetry on by default |
| BMAD Method | v6.12.4, 2026-10-14 (v7 in alpha) | MIT code, trade-marked names | `_bmad-output/` planning artifacts per initiative | Persona-driven workflow; epics and stories | No gate for spec content; narrow helper scripts | Retrospectives; no supersede metadata | Node 20.12+, Python 3.10+, 30+ skills per tool |

| Tool | Applicability | Scalability at thousands of docs | Complexity | Maintainability | Machine validation | Agent efficiency | Adoption cost |
| --- | --- | --- | --- | --- | --- | --- | --- |
| Spec Kit | M (greenfield features) | L (no merged current spec, per-project `NNN` numbering, no link checker) | M | M (5 releases in 10 days rewrite managed files) | L | M | M |
| OpenSpec | H (brownfield-first) | M (flat namespaces; parallel MODIFIED deltas could drop scenarios, documented by the project; current fix **unverified**) | M | M | H for its own grammar | H (index of spec id, purpose, fetch command) | M |
| BMAD | L (process, not a spec store) | L | H | L (stable line frozen, main is breaking alpha) | L | L (persona prompts) | H |

Other approaches studied:

- Kubernetes KEPs (`kep.yaml` beside `README.md`, about 710 proposals): a typed schema with status and stage enums, cross-field rules, `replaces` and `superseded-by` links, a JSON index, and checks on changed files only. The metadata check fetches a remote file, so it is not hermetic. This is the closest prior art for machine-validated spec metadata.
- AGENTS.md: plain Markdown, nearest file wins, no required fields.
- Agent Skills (`SKILL.md`): 3 disclosure levels (metadata of about 100 tokens, body on activation, files on demand).
- Claude Code: `CLAUDE.md` with `@path` imports (4 hops), `.claude/rules` with `paths:` globs, hooks that inject context capped at 10,000 characters per string.
- Kiro: `requirements.md`, `design.md`, `tasks.md` per feature with EARS; steering files with inclusion modes. Closed source (**unverified**).
- Aider repo map: tree-sitter definitions and references ranked into a token budget (PageRank specifically **unverified**).
- Tessl: public docs now describe a skills registry; former spec-syntax pages return 404, so its spec support is **unverified**.

Also considered, not evaluated in depth (versions not checked):

| Tool or idea | Why it matters | Position |
| --- | --- | --- |
| Doorstop | Requirements as files in git; suspect links store the target's hash | Model for a later blocking R2 check (ADR 0001) |
| Sphinx-needs, StrictDoc, OpenFastTrace | Requirement objects with ids and traceability to code and tests | Confirm the requirement-key approach; each brings its own markup or runtime (Python, Java) |
| lychee | Fast link checker | External links stay out of spec-dna gates; projects can run it |
| Vale | Prose linter with rule packages | Optional external step for style; not a gate dependency |
| Backstage entity references (`kind:namespace/name`) | Namespaced ids across repositories | Shape of the cross-repository reference form |
| Model Context Protocol | A serving surface for agents | A possible later adapter over the same CLI functions |

Mechanisms to reuse instead of rebuild:

1. Agent loading surfaces as they exist: AGENTS.md, `CLAUDE.md` imports, path-scoped rules, skills. spec-dna emits small entry points into them; it does not invent a loader.
2. The OpenSpec diagnostic shape `{severity, code, message, target, fix}` and its `x-` prefix for extension keys.
3. The KEP metadata pattern: typed schema, enums, cross-field rules, supersede links, changed-files-only checks. Keep it offline.
4. The OpenSpec referenced-spec index (id, one-line purpose, fetch command) as the progressive-disclosure unit.

Integration decision: spec-dna stays independent of all three frameworks. None stores a merged current truth with stable ids, supersede links and a link checker at the scale of thousands of documents, and none validates its own documents deterministically except OpenSpec, whose grammar is narrower than the information model spec-dna needs. A later adapter can import OpenSpec capability specs, because their requirement and scenario structure maps to the spec-dna requirement type.

## 4. Context engineering evidence

| Source | Finding | Consequence for spec-dna |
| --- | --- | --- |
| Liu et al., "Lost in the Middle", TACL, arXiv v3 2023-11-20 | Accuracy drops when relevant text sits in the middle of long input, also for long-context models | Serve few, small, ranked units; put the entry point first |
| Chroma, "Context Rot", 2025-07-14 (18 models) | Quality degrades with input length, even on simple tasks; one distractor hurts | Minimum sufficient context, not maximum context |
| NoLiMa, ICML 2025 | 11 of 13 models fall below half their short-context score at 32K tokens without literal overlap | Same |
| Xu et al., ICLR 2024 | Retrieval helps regardless of window size | Retrieval stays necessary |
| Li et al., EMNLP 2024 industry | Long context can beat RAG on quality; RAG is much cheaper; a hybrid keeps quality at lower cost | Budgeted, progressive expansion |
| Anthropic, "Effective context engineering", 2025-09-29 | Smallest high-signal token set; load just in time through identifiers | Index of ids and paths, bodies on demand |
| Gloaguen et al., arXiv 2602.11988 v3 2026-09-29 | Context files do not generally raise success rates and cost over 20% more; repository overviews do not help | Keep always-loaded files tiny; no generated overviews |
| Vercel blog, 2026-01-27 (single vendor) | An 8 KB index in AGENTS.md passed 100%; an optional skill passed 53 to 79% and was not invoked in 56% of cases | A small always-loaded index plus a deterministic fetch command beats optional retrieval. ADR 0001 commits a namespace-level index only, so it does not change with every new document |

## 5. Selected composition

1. Format: CommonMark plus GFM tables; YAML front matter in a JSON-compatible subset; JSON Schema 2020-12 for front matter; TOML for tool configuration.
2. Language: BCP 14 uppercase keywords as the only normative vocabulary. Requirements use EARS patterns. EARS, INCOSE and 29148 use lowercase "shall" with goal semantics for "should"; spec-dna uses BCP 14 meanings and matches EARS trigger words without regard to case.
3. Quality rules: a small own controlled-language lint, described below. All its checks are warnings except where an ADR proves a check exact.
4. Decisions: MADR 4.0.0 minimal.
5. Structure: DITA concepts without DITA XML: typed topics (the 6 document types of ADR 0001), generated maps, and keys (path ids with aliases). Includes are deferred. Diátaxis modes and C4 levels are optional `x-` metadata.
6. Serving: an llms.txt-shaped index; bodies fetched by id through the CLI.
7. Versioning: SemVer 2.0.0 for the tool and the standard; Conventional Commits 1.0.0 in this repository.

### Controlled-language lint

ASD-STE100, INCOSE GtWR and ISO/IEC/IEEE 29148 stay reference only. spec-dna claims compliance with none of them. The lint is written in this project's own words and runs on requirement sentences (the list items with a bold key in a `spec` or `contract`, ADR 0001). Every finding is a warning.

| Check | What it flags | Maps to |
| --- | --- | --- |
| Sentence length | A requirement sentence over 30 words (configurable). A long sentence usually holds more than one requirement | 29148 characteristic "singular"; GtWR R18 (single thought sentence); ASD-STE100 sets its own length limits, used as inspiration only |
| Vague terms | A word or phrase from spec-dna's own list: terms that cannot be tested ("adequate", "appropriate", "fast", "user-friendly", "robust", "efficient", "seamless", "reasonable", "sufficient"), escape clauses ("if possible", "as appropriate", "where practical"), open endings ("etc.", "and so on", "including but not limited to"), and loose quantities ("some", "several", "various", "many", "few") | 29148 characteristics "unambiguous" and "verifiable"; GtWR R7 (vague terms), R8 (escape clauses), R9 (open-ended clauses) |
| Passive voice | A form of "be" followed by a past participle (a word ending in "-ed" or "-en", or a short list of irregular forms), as in "the request SHALL be rejected". A heuristic: it misses some forms and flags some adjectives | 29148 characteristic "unambiguous" (the actor is missing); GtWR R2 (active voice) |

GtWR rule ids are from the v4 rule list as cited in secondary sources; the paid text was not checked (see section 2). The 29148 characteristic names are from the official preview.

## 6. Rejected alternatives

| Alternative | Reason |
| --- | --- |
| Adopt Spec Kit, OpenSpec or BMAD as the base | See section 3: no merged truth with stable ids at scale (Spec Kit), flat namespaces and a narrow grammar (OpenSpec), process without a spec store (BMAD). Spec Kit and BMAD also need Python or Node; OpenSpec has a standalone binary |
| Full ISO/IEC/IEEE 29148 or INCOSE templates | Paid text that cannot be bundled; many mandatory attributes without demonstrated value for agents |
| ASD-STE100 dictionary checks | Dictionary is copyrighted and cannot be shipped; the domain is aircraft maintenance |
| DITA XML toolchain | Heavy toolchain and XML noise in agent context; the useful ideas survive as concepts |
| Full arc42 as a mandatory structure | 12 sections impose ceremony on small projects; CC BY-SA would spread to adapted templates |
| Embeddings, a vector database or LLM re-ranking for retrieval | Non-deterministic, needs a service or network; explicit links and ids come first (brief section 4.2) |
| LLM-based validation as a gate | Non-deterministic; a gate must be reproducible. LLM review stays advisory |
| Gates on human-typed dates (freshness clocks) | Observed in a mature corpus: such gates produced mass false positives and were removed. Freshness comes from content hashes and git, not typed dates |
| Global sequential id counters | Merge conflicts between parallel branches and agents; see ADR 0001 |

## 7. Sources

| URL | What | Version or date seen |
| --- | --- | --- |
| <https://www.rfc-editor.org/rfc/rfc2119> | RFC 2119 | 1997-03 |
| <https://www.rfc-editor.org/rfc/rfc8174> | RFC 8174 | 2017-05 |
| <https://trustee.ietf.org/documents/trust-legal-provisions/tlp-5> | IETF Trust Legal Provisions | 5.0, effective 2015-03-25 |
| <https://alistairmavin.com/ears/> | EARS templates | page "© 2026" |
| <https://github.com/adr/madr> | MADR | 4.0.0, 2024-09-17 |
| <https://www.cognitect.com/blog/2011/11/15/documenting-architecture-decisions> | Nygard ADR | 2011-11-15 |
| <https://diataxis.fr/> and <https://github.com/evildmp/diataxis-documentation-framework> | Diátaxis, licence | unversioned |
| <https://www.oasis-open.org/committees/tc_home.php?wg_abbrev=dita> | DITA 1.3 | OASIS Standard 2015-12-17 |
| <https://dita-lang.org/history> | DITA 2.0 status | working drafts |
| <https://arc42.org/license> and <https://github.com/arc42/arc42-template/releases> | arc42 | EN 9.1, 2026.10.19 |
| <https://c4model.com/> | C4 model | unversioned, CC BY 4.0 |
| <https://cdn.standards.iteh.ai/samples/72089/62bb2ea1ef8b4f33a80d984f826267c1/ISO-IEC-IEEE-29148-2018.pdf> | ISO/IEC/IEEE 29148 preview | 2nd ed. 2018-11 |
| <https://iss.rs/en/project/show/iso:proj:94091> | 29148 3rd ed. stage | DIS 40.20, 2026-09-14 |
| <https://www.incose.org/docs/default-source/working-groups/requirements-wg/gtwr/incose_rwg_gtwr_v4_040423_final_drafts.pdf> | INCOSE GtWR | v4, 2023-07-01 (403, snippet only) |
| <https://www.asd-europe.org/news-media/news-events/news/simplified-technical-english-asd-ste100-issue-9> | ASD-STE100 Issue 9 | 2025-01-15 |
| <https://www.asd-europe.org/standards-specifications/simplified-technical-english/faq-simplified-technical-english-ste/> | ASD-STE100 FAQ | Issue 9 |
| <https://spec.commonmark.org/> | CommonMark | 0.31.2, 2024-01-28 |
| <https://yaml.org/spec/1.2.2/> | YAML | 1.2.2, 2021-10-01 |
| <https://json-schema.org/specification> | JSON Schema | 2020-12 |
| <https://toml.io/en/latest> | TOML | 1.1.0, 2025-12-18 |
| <https://semver.org/> | SemVer | 2.0.0 |
| <https://spec.openapis.org/oas/latest.html> | OpenAPI | 3.2.1, 2026-09-10 |
| <https://www.asyncapi.com/docs/reference/specification/latest> | AsyncAPI | 3.1.0 |
| <https://llmstxt.org/> | llms.txt | proposal 2024-09-03 |
| <https://github.com/github/spec-kit> | Spec Kit repository | v1.3.2, HEAD c43849408ac0 |
| <https://github.com/Fission-AI/OpenSpec> | OpenSpec repository | v1.16.1, HEAD 0fa4a9705c20 |
| <https://github.com/bmad-code-org/BMAD-METHOD> | BMAD repository | v6.12.4, HEAD e734d3ac3c0f |
| <https://github.com/kubernetes/enhancements> | KEP process and tooling | HEAD 72907deaa85e, 2026-11-06 |
| <https://kiro.dev/docs/specs/> | Kiro specs | page updated 2026-10-15 |
| <https://agents.md/> | AGENTS.md | undated |
| <https://agentskills.io/specification> | Agent Skills | unversioned |
| <https://code.claude.com/docs/en/memory> and <https://code.claude.com/docs/en/hooks> | Claude Code memory and hooks | docs current on access date |
| <https://aider.chat/docs/repomap.html> | Aider repo map | undated |
| <https://arxiv.org/abs/2307.03172> | Lost in the Middle | v3 2023-11-20 |
| <https://www.trychroma.com/research/context-rot> | Context Rot | 2025-07-14 |
| <https://arxiv.org/abs/2502.05167> | NoLiMa | v3 2025-07-09 |
| <https://arxiv.org/abs/2310.03025> | Retrieval meets long context | v2 2024-01-23 |
| <https://arxiv.org/abs/2407.16833> | RAG or long context | v2 2024-10-17 |
| <https://arxiv.org/abs/2602.11988> | Evaluating AGENTS.md | v3 2026-09-29 |
| <https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents> | Context engineering | 2025-09-29 |
| <https://vercel.com/blog/agents-md-outperforms-skills-in-our-agent-evals> | AGENTS.md vs skills eval | 2026-01-27 |

//! `spec-dna transcript` reads agent transcripts. For each session and each
//! episode, it reports token usage, tool calls, and the documents that the
//! agent read, as spec-dna ids and sections.
//!
//! A transcript can contain secrets. Thus this module prints counts, names of
//! tools and models, and spec-dna ids only. It never prints prompt, command,
//! path or file text. It reads that text in memory to find document reads,
//! and then discards it. A session id, agent id, origin, model name or tool name
//! that does not match the strict pattern of its field prints as
//! `<redacted>` (`safe`).
//!
//! An adapter converts one transcript format into `Record`s. The analysis does
//! not depend on the format. `ClaudeCode` reads Claude Code JSONL files.

use crate::json::{self, Obj, Value};
use crate::model::{Corpus, Res};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// The text that replaces a value that does not match its pattern.
pub const REDACTED: &str = "<redacted>";

/// A transcript field that the report prints.
#[derive(Clone, Copy, Debug)]
enum Field {
    /// A UUID in lowercase hexadecimal.
    Session,
    /// `main`, `sidechain` or 1 to 32 lowercase hexadecimal digits.
    Agent,
    /// 1 to 32 lowercase ASCII letters and hyphens.
    Origin,
    /// 1 to 64 lowercase ASCII letters, digits and `.`, `-`, `_`, `:`, `@`,
    /// with a letter first.
    Model,
    /// The tool name pattern of the Anthropic API, with a letter first: 1 to
    /// 64 ASCII letters, digits, `_` and `-`.
    Tool,
}

/// Returns the value if it matches the pattern of its field, or `REDACTED`.
/// Thus a malformed or hostile transcript cannot put free text into the
/// output.
fn safe(field: Field, v: &str) -> String {
    let b = v.as_bytes();
    let len = |max: usize| (1..=max).contains(&b.len());
    let hex = |c: &u8| c.is_ascii_digit() || (b'a'..=b'f').contains(c);
    let ok = match field {
        Field::Session => {
            b.len() == 36
                && b.iter().enumerate().all(|(i, c)| match i {
                    8 | 13 | 18 | 23 => *c == b'-',
                    _ => hex(c),
                })
        }
        Field::Agent => v == "main" || v == "sidechain" || (len(32) && b.iter().all(hex)),
        Field::Origin => len(32) && b.iter().all(|c| c.is_ascii_lowercase() || *c == b'-'),
        Field::Model => {
            len(64)
                && b[0].is_ascii_lowercase()
                && b.iter().all(|c| {
                    c.is_ascii_lowercase()
                        || c.is_ascii_digit()
                        || matches!(c, b'.' | b'-' | b'_' | b':' | b'@')
                })
        }
        Field::Tool => {
            len(64)
                && b[0].is_ascii_alphabetic()
                && b.iter()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
        }
    };
    if ok {
        v.to_string()
    } else {
        REDACTED.to_string()
    }
}

/// What a tool call reads or writes.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    None,
    /// A file read. `offset` and `limit` are lines.
    Read {
        path: String,
        offset: Option<u64>,
        limit: Option<u64>,
    },
    Write {
        path: String,
    },
    Search,
    Shell {
        command: String,
    },
}

/// The line range of a file read: first line (1-based), number of lines, and
/// number of lines in the file, if the transcript records them.
pub type Lines = (u64, u64, Option<u64>);

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// A message that starts an episode, with its origin (such as `human`).
    /// `id` identifies the message, so that a copy in another file counts once.
    Prompt { origin: String, id: String },
    /// One model request. Records with the same `request` are one request.
    Usage {
        request: String,
        model: String,
        tokens: Tokens,
    },
    Call {
        id: String,
        name: String,
        target: Target,
    },
    Result {
        id: String,
        text: String,
        /// Characters of the result. A tool can store a long result outside
        /// the transcript, so this can be more than `text` has.
        chars: u64,
        error: bool,
        lines: Option<Lines>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub session: String,
    /// `main`, or the id of a subagent.
    pub agent: String,
    /// Milliseconds since 1970-01-01T00:00:00Z.
    pub time: Option<i64>,
    pub cwd: Option<String>,
    pub event: Event,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tokens {
    pub input: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
    pub output: u64,
}

impl Tokens {
    fn add(&mut self, o: &Tokens) {
        self.input += o.input;
        self.cache_creation += o.cache_creation;
        self.cache_read += o.cache_read;
        self.output += o.output;
    }

    fn sub(&mut self, o: &Tokens) {
        self.input -= o.input;
        self.cache_creation -= o.cache_creation;
        self.cache_read -= o.cache_read;
        self.output -= o.output;
    }

    fn json(&self) -> String {
        Obj::new()
            .num("input", self.input)
            .num("cache_creation", self.cache_creation)
            .num("cache_read", self.cache_read)
            .num("output", self.output)
            .end()
    }

    fn text(&self) -> String {
        format!(
            "input {}, cache creation {}, cache read {}, output {}",
            self.input, self.cache_creation, self.cache_read, self.output
        )
    }
}

/// Converts the records of one transcript format.
pub trait Adapter {
    fn name(&self) -> &'static str;
    /// Returns true if a parsed line has this format.
    fn detect(&self, line: &Value) -> bool;
    /// Appends the records of one parsed line.
    fn records(&self, line: &Value, out: &mut Vec<Record>);
}

pub const ADAPTERS: &[&dyn Adapter] = &[&ClaudeCode];

/// Claude Code writes one JSON object per line. Subagents write their own
/// files with `isSidechain: true`. A streamed response repeats its usage on
/// several lines with the same message id.
pub struct ClaudeCode;

impl Adapter for ClaudeCode {
    fn name(&self) -> &'static str {
        "claude-code"
    }

    fn detect(&self, line: &Value) -> bool {
        line.get("sessionId").as_str().is_some() && line.get("type").as_str().is_some()
    }

    fn records(&self, o: &Value, out: &mut Vec<Record>) {
        let kind = o.get("type").as_str().unwrap_or("");
        if kind != "user" && kind != "assistant" {
            return;
        }
        let Some(session) = o.get("sessionId").as_str() else {
            return;
        };
        let agent = if o.get("isSidechain").is_true() {
            o.get("agentId").as_str().unwrap_or("sidechain").to_string()
        } else {
            "main".to_string()
        };
        let base = Record {
            session: session.to_string(),
            agent,
            time: o.get("timestamp").as_str().and_then(parse_time),
            cwd: o.get("cwd").as_str().map(str::to_string),
            event: Event::Prompt {
                origin: String::new(),
                id: String::new(),
            },
        };
        let msg = o.get("message");
        let push = |out: &mut Vec<Record>, event: Event| {
            let mut r = base.clone();
            r.event = event;
            out.push(r);
        };
        if kind == "assistant" {
            let request = msg
                .get("id")
                .as_str()
                .or_else(|| o.get("requestId").as_str())
                .unwrap_or("");
            let u = msg.get("usage");
            let model = msg.get("model").as_str().unwrap_or("");
            // Claude Code writes `<synthetic>` messages itself: they are not
            // model requests.
            if !request.is_empty() && u != &Value::Null && model != "<synthetic>" {
                push(
                    out,
                    Event::Usage {
                        request: request.to_string(),
                        model: model.to_string(),
                        tokens: Tokens {
                            input: u.get("input_tokens").as_u64(),
                            cache_creation: u.get("cache_creation_input_tokens").as_u64(),
                            cache_read: u.get("cache_read_input_tokens").as_u64(),
                            output: u.get("output_tokens").as_u64(),
                        },
                    },
                );
            }
            for b in msg.get("content").as_arr() {
                if b.get("type").as_str() == Some("tool_use") {
                    let name = b.get("name").as_str().unwrap_or("").to_string();
                    let target = claude_target(&name, b.get("input"));
                    push(
                        out,
                        Event::Call {
                            id: b.get("id").as_str().unwrap_or("").to_string(),
                            name,
                            target,
                        },
                    );
                }
            }
            return;
        }
        let content = msg.get("content");
        let blocks = content.as_arr();
        let results: Vec<&Value> = blocks
            .iter()
            .filter(|b| b.get("type").as_str() == Some("tool_result"))
            .collect();
        if results.is_empty() {
            // A compaction summary continues the episode. It is not a prompt.
            if o.get("isMeta").is_true() || o.get("isCompactSummary").is_true() {
                return;
            }
            let origin = match o.get("origin").get("kind").as_str() {
                Some(k) => k.to_string(),
                None => "human".to_string(),
            };
            let id = o.get("uuid").as_str().unwrap_or("").to_string();
            push(out, Event::Prompt { origin, id });
            return;
        }
        let meta = o.get("toolUseResult");
        for b in &results {
            let text = match b.get("content") {
                Value::Str(s) => s.clone(),
                Value::Arr(parts) => parts
                    .iter()
                    .filter_map(|p| p.get("text").as_str())
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            };
            let mut chars = text.chars().count() as u64;
            let mut lines = None;
            if results.len() == 1 {
                let stored = meta.get("persistedOutputSize").as_u64();
                if stored > chars {
                    chars = stored;
                }
                let f = meta.get("file");
                if f.get("numLines") != &Value::Null {
                    let total = f.get("totalLines");
                    lines = Some((
                        f.get("startLine").as_u64().max(1),
                        f.get("numLines").as_u64(),
                        (total != &Value::Null).then(|| total.as_u64()),
                    ));
                }
            }
            push(
                out,
                Event::Result {
                    id: b.get("tool_use_id").as_str().unwrap_or("").to_string(),
                    text,
                    chars,
                    error: b.get("is_error").is_true(),
                    lines,
                },
            );
        }
    }
}

fn claude_target(name: &str, input: &Value) -> Target {
    let s = |k: &str| input.get(k).as_str().map(str::to_string);
    let n = |k: &str| match input.get(k) {
        Value::Num(v) if *v >= 0.0 => Some(*v as u64),
        _ => None,
    };
    match name {
        "Read" => match s("file_path") {
            Some(path) => Target::Read {
                path,
                offset: n("offset"),
                limit: n("limit"),
            },
            None => Target::None,
        },
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => {
            match s("file_path").or_else(|| s("notebook_path")) {
                Some(path) => Target::Write { path },
                None => Target::None,
            }
        }
        "Grep" | "Glob" => Target::Search,
        "Bash" => match s("command") {
            Some(command) => Target::Shell { command },
            None => Target::None,
        },
        _ => Target::None,
    }
}

/// Parses an RFC 3339 UTC time such as `2026-11-07T21:10:38.029Z`.
pub fn parse_time(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || b[10] != b'T' {
        return None;
    }
    let num = |r: std::ops::Range<usize>| s.get(r)?.parse::<i64>().ok();
    let (y, mo, d) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (h, mi, sec) = (num(11..13)?, num(14..16)?, num(17..19)?);
    let mut ms = 0;
    let mut rest = &s[19..];
    if let Some(frac) = rest.strip_prefix('.') {
        let digits: String = frac.chars().take_while(char::is_ascii_digit).collect();
        rest = &frac[digits.len()..];
        ms = format!("{digits:0<3}")[..3].parse().ok()?;
    }
    if rest != "Z" {
        return None;
    }
    // Days from 1970-01-01 to the date (Howard Hinnant, "days_from_civil").
    let y = if mo <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (mo + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Some(((days * 24 + h) * 60 + mi) * 60_000 + sec * 1000 + ms)
}

/// How a document reached the context.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Via {
    /// A file read tool.
    Read,
    /// A shell command that names the file. Its size is an estimate.
    Shell,
    /// `spec-dna context` or `spec-dna show` output.
    SpecDna,
}

impl Via {
    fn name(self) -> &'static str {
        match self {
            Via::Read => "read",
            Via::Shell => "shell",
            Via::SpecDna => "spec-dna",
        }
    }
}

#[derive(Default)]
struct DocStat {
    reads: u32,
    chars: u64,
    full: bool,
    sections: BTreeSet<String>,
    via: BTreeSet<Via>,
    first_ms: Option<i64>,
}

#[derive(Default)]
struct Episode {
    agent: String,
    origin: String,
    start: Option<i64>,
    end: Option<i64>,
    requests: u32,
    tokens: Tokens,
    models: BTreeSet<String>,
    tools: BTreeMap<String, u32>,
    errors: u32,
    searches: u32,
    /// Shell runs of spec-dna, by subcommand.
    spec_dna: BTreeMap<&'static str, u32>,
    other_reads: u32,
    docs: BTreeMap<String, DocStat>,
    writes: BTreeMap<String, u32>,
}

impl Episode {
    fn touch(&mut self, t: Option<i64>) {
        if let Some(t) = t {
            self.start = Some(self.start.map_or(t, |s| s.min(t)));
            self.end = Some(self.end.map_or(t, |e| e.max(t)));
        }
    }

    fn doc_reads(&self) -> u32 {
        self.docs.values().map(|d| d.reads).sum()
    }

    fn spec_chars(&self) -> u64 {
        self.docs.values().map(|d| d.chars).sum()
    }
}

struct AgentState {
    /// Index into `Session::episodes` of the open episode.
    current: Option<usize>,
    calls: HashMap<String, (String, Target)>,
}

#[derive(Default)]
struct Session {
    adapter: &'static str,
    episodes: Vec<Episode>,
    agents: BTreeMap<String, AgentState>,
    /// Request id to its episode and its last usage.
    requests: HashMap<String, (usize, Tokens)>,
    /// Ids of prompts, tool calls and tool results already counted. A forked
    /// subagent file can repeat records of its parent.
    seen: HashSet<(u8, String)>,
}

/// Collects records and attributes reads to documents.
pub struct Analysis<'a> {
    corpus: Option<&'a Corpus>,
    root: Option<String>,
    sessions: BTreeMap<String, Session>,
}

impl<'a> Analysis<'a> {
    /// `corpus` and `root` (the repository root) map file paths to documents.
    /// Without them, the analysis counts tokens and tools only.
    pub fn new(corpus: Option<&'a Corpus>, root: Option<&std::path::Path>) -> Self {
        Analysis {
            corpus,
            root: root.map(|r| r.to_string_lossy().trim_end_matches('/').to_string()),
            sessions: BTreeMap::new(),
        }
    }

    pub fn feed(&mut self, adapter: &'static str, r: Record) {
        let s = self.sessions.entry(r.session.clone()).or_default();
        s.adapter = adapter;
        let key = match &r.event {
            Event::Prompt { id, .. } => Some((0, id)),
            Event::Call { id, .. } => Some((1, id)),
            Event::Result { id, .. } => Some((2, id)),
            Event::Usage { .. } => None,
        };
        if let Some((kind, id)) = key
            && !id.is_empty()
            && !s.seen.insert((kind, id.clone()))
        {
            return;
        }
        // A request id that the analysis knows replaces its earlier usage, in
        // the episode of its first record (streamed lines and forked copies).
        if let Event::Usage {
            request, tokens, ..
        } = &r.event
            && let Some((old_ei, old)) = s.requests.get(request).copied()
        {
            s.episodes[old_ei].tokens.sub(&old);
            s.episodes[old_ei].tokens.add(tokens);
            s.requests.insert(request.clone(), (old_ei, *tokens));
            return;
        }
        let agent = s
            .agents
            .entry(r.agent.clone())
            .or_insert_with(|| AgentState {
                current: None,
                calls: HashMap::new(),
            });
        let start_new = matches!(r.event, Event::Prompt { .. }) || agent.current.is_none();
        if start_new {
            s.episodes.push(Episode {
                agent: safe(Field::Agent, &r.agent),
                origin: match &r.event {
                    Event::Prompt { origin, .. } => safe(Field::Origin, origin),
                    _ => "unknown".into(),
                },
                ..Episode::default()
            });
            agent.current = Some(s.episodes.len() - 1);
        }
        let ei = agent.current.unwrap_or(0);
        s.episodes[ei].touch(r.time);
        match r.event {
            Event::Prompt { .. } => {}
            Event::Usage {
                request,
                model,
                tokens,
            } => {
                let e = &mut s.episodes[ei];
                e.requests += 1;
                e.tokens.add(&tokens);
                s.requests.insert(request, (ei, tokens));
                if !model.is_empty() {
                    s.episodes[ei].models.insert(safe(Field::Model, &model));
                }
            }
            Event::Call { id, name, target } => {
                let e = &mut s.episodes[ei];
                *e.tools.entry(safe(Field::Tool, &name)).or_default() += 1;
                match &target {
                    Target::Search => e.searches += 1,
                    Target::Shell { command } => {
                        e.searches += shell_searches(command);
                        for sub in spec_dna_runs(command) {
                            *e.spec_dna.entry(sub).or_default() += 1;
                        }
                    }
                    Target::Write { path } => {
                        let c = self.corpus;
                        if let Some(d) = map_path(c, self.root.as_deref(), r.cwd.as_deref(), path)
                            && let Some(c) = c
                        {
                            *e.writes.entry(c.docs[d as usize].id.clone()).or_default() += 1;
                        }
                    }
                    _ => {}
                }
                agent.calls.insert(id, (name, target));
            }
            Event::Result {
                id,
                text,
                chars,
                error,
                lines,
            } => {
                let e = &mut s.episodes[ei];
                if error {
                    e.errors += 1;
                }
                let Some((_, target)) = agent.calls.remove(&id) else {
                    return;
                };
                if error {
                    return;
                }
                attribute(
                    self.corpus,
                    self.root.as_deref(),
                    r.cwd.as_deref(),
                    e,
                    &target,
                    &text,
                    chars,
                    lines,
                    r.time,
                );
            }
        }
    }

    /// Returns the report. `expect` lists the document ids that the task
    /// needs. With them, the report gives the time to the first relevant read
    /// and the share of expected documents that the agent read.
    pub fn report(&self, expect: &[String], as_json: bool) -> String {
        let mut out = String::new();
        let mut sessions_json = Vec::new();
        let (mut n_agents, mut n_episodes, mut n_requests) = (0, 0, 0);
        for (raw_id, s) in &self.sessions {
            let id = &safe(Field::Session, raw_id);
            // An episode without a request or a tool call did nothing, for
            // example a prompt that a local command answered.
            let mut episodes: Vec<&Episode> = s
                .episodes
                .iter()
                .filter(|e| e.requests > 0 || !e.tools.is_empty())
                .collect();
            // The files of a session can come in any order. Sort by start time.
            episodes.sort_by_key(|e| e.start);
            n_agents += s.agents.len();
            n_episodes += episodes.len();
            let mut total = Episode::default();
            for e in &episodes {
                total.requests += e.requests;
                total.tokens.add(&e.tokens);
                total.errors += e.errors;
                total.searches += e.searches;
                for (k, v) in &e.spec_dna {
                    *total.spec_dna.entry(k).or_default() += v;
                }
                total.other_reads += e.other_reads;
                total.models.extend(e.models.iter().cloned());
                total.touch(e.start);
                total.touch(e.end);
                for (k, v) in &e.tools {
                    *total.tools.entry(k.clone()).or_default() += v;
                }
                for (k, v) in &e.writes {
                    *total.writes.entry(k.clone()).or_default() += v;
                }
                for (k, d) in &e.docs {
                    let t = total.docs.entry(k.clone()).or_default();
                    t.reads += d.reads;
                    t.chars += d.chars;
                    t.full |= d.full;
                    t.sections.extend(d.sections.iter().cloned());
                    t.via.extend(d.via.iter().copied());
                    t.first_ms = match (t.first_ms, d.first_ms) {
                        (Some(a), Some(b)) => Some(a.min(b)),
                        (a, b) => a.or(b),
                    };
                }
            }
            n_requests += total.requests;
            let expected = expect_stats(&total, expect);
            if as_json {
                let list = episodes
                    .iter()
                    .enumerate()
                    .map(|(i, e)| episode_json(e, i + 1, expect));
                let mut o = Obj::new()
                    .str("session", id)
                    .str("adapter", s.adapter)
                    .num("agents", s.agents.len())
                    .num("episodes", episodes.len());
                o = o.raw("total", &episode_json(&total, 0, expect));
                if let Some(x) = &expected {
                    o = o.raw("expected", x);
                }
                sessions_json.push(o.raw("episode_list", &json::array(list)).end());
            } else {
                out.push_str(&format!(
                    "session {id} ({}): {}, {}\n",
                    s.adapter,
                    n(s.agents.len() as u64, "agent"),
                    n(episodes.len() as u64, "episode")
                ));
                out.push_str(&episode_text(&total, "  "));
                if let Some(x) = &expected {
                    out.push_str(&format!("  expected: {x}\n"));
                }
                for (i, e) in episodes.iter().enumerate() {
                    out.push_str(&format!(
                        "  episode {} ({}, {}){}:\n",
                        i + 1,
                        e.agent,
                        e.origin,
                        duration(e)
                    ));
                    out.push_str(&episode_text(e, "    "));
                }
            }
        }
        if as_json {
            return format!(
                "{}\n",
                Obj::new()
                    .str("version", crate::adapters::VERSION)
                    .bool("documents_mapped", self.corpus.is_some())
                    .strs("expect", expect)
                    .raw("sessions", &json::array(sessions_json))
                    .end()
            );
        }
        let head = format!(
            "spec-dna transcript: {}, {}, {}, {}{}\n",
            n(self.sessions.len() as u64, "session"),
            n(n_agents as u64, "agent"),
            n(n_episodes as u64, "episode"),
            n(n_requests, "request"),
            if self.corpus.is_none() {
                "; no spec-dna.toml, so file reads are not mapped to documents"
            } else {
                ""
            }
        );
        format!("{head}{out}")
    }
}

fn duration(e: &Episode) -> String {
    match (e.start, e.end) {
        (Some(a), Some(b)) => format!(", {} s", (b - a) / 1000),
        _ => String::new(),
    }
}

fn tools_text(tools: &BTreeMap<String, u32>) -> String {
    let mut v: Vec<(&String, &u32)> = tools.iter().collect();
    v.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    v.iter()
        .map(|(k, n)| format!("{k} {n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn episode_text(e: &Episode, indent: &str) -> String {
    let calls: u32 = e.tools.values().sum();
    let reads = e.doc_reads();
    let mut out = format!(
        "{indent}{}; tokens: {}\n{indent}{} ({}); {}; {}\n{indent}documents: {} in {} ({} repeated); other files: {}; spec-related about {} (estimated)\n",
        n(e.requests, "request"),
        e.tokens.text(),
        n(calls, "tool call"),
        tools_text(&e.tools),
        n(e.errors, "error"),
        if e.searches == 1 {
            "1 search".to_string()
        } else {
            format!("{} searches", e.searches)
        },
        n(e.docs.len() as u64, "document"),
        n(reads, "read"),
        reads.saturating_sub(e.docs.len() as u32),
        n(e.other_reads, "read"),
        n(e.spec_chars().div_ceil(4), "token")
    );
    for (id, d) in &e.docs {
        let what = if d.full {
            "full".to_string()
        } else if d.sections.is_empty() {
            "front matter only".to_string()
        } else {
            d.sections.iter().cloned().collect::<Vec<_>>().join(" ")
        };
        let via: Vec<&str> = d.via.iter().map(|v| v.name()).collect();
        out.push_str(&format!(
            "{indent}  {id}: {}, {what}, via {}\n",
            n(d.reads, "read"),
            via.join("+")
        ));
    }
    if !e.spec_dna.is_empty() {
        let runs: Vec<String> = e.spec_dna.iter().map(|(k, n)| format!("{k} {n}")).collect();
        out.push_str(&format!("{indent}spec-dna commands: {}\n", runs.join(", ")));
    }
    if !e.writes.is_empty() {
        let w: Vec<String> = e.writes.iter().map(|(k, n)| format!("{k} {n}")).collect();
        out.push_str(&format!("{indent}  document edits: {}\n", w.join(", ")));
    }
    out
}

fn episode_json(e: &Episode, index: usize, expect: &[String]) -> String {
    let docs = e.docs.iter().map(|(id, d)| {
        let sections: Vec<String> = d.sections.iter().cloned().collect();
        let via: Vec<String> = d.via.iter().map(|v| v.name().to_string()).collect();
        Obj::new()
            .str("id", id)
            .num("reads", d.reads)
            .bool("full", d.full)
            .strs("sections", &sections)
            .strs("via", &via)
            .num("chars", d.chars)
            .end()
    });
    let tools = e
        .tools
        .iter()
        .map(|(k, n)| Obj::new().str("name", k).num("calls", n).end());
    let writes = e
        .writes
        .iter()
        .map(|(k, n)| Obj::new().str("id", k).num("edits", n).end());
    let models: Vec<String> = e.models.iter().cloned().collect();
    let mut o = Obj::new();
    if index > 0 {
        o = o
            .num("index", index)
            .str("agent", &e.agent)
            .str("origin", &e.origin);
    }
    o = o
        .opt_str("start", e.start.map(format_time).as_deref())
        .raw(
            "duration_ms",
            &match (e.start, e.end) {
                (Some(a), Some(b)) => (b - a).to_string(),
                _ => "null".into(),
            },
        )
        .num("requests", e.requests)
        .strs("models", &models)
        .raw("tokens", &e.tokens.json())
        .num("tool_calls", e.tools.values().sum::<u32>())
        .raw("tools", &json::array(tools))
        .num("errors", e.errors)
        .num("searches", e.searches)
        .raw("spec_dna_commands", &{
            let mut o = Obj::new();
            for (k, n) in &e.spec_dna {
                o = o.num(k, n);
            }
            o.end()
        })
        .num("doc_reads", e.doc_reads())
        .num(
            "repeated_reads",
            e.doc_reads().saturating_sub(e.docs.len() as u32),
        )
        .num("other_file_reads", e.other_reads)
        .num("spec_chars", e.spec_chars())
        .num("spec_tokens_estimated", e.spec_chars().div_ceil(4))
        .raw("documents", &json::array(docs))
        .raw("document_edits", &json::array(writes));
    if index > 0
        && let Some(x) = expect_stats(e, expect)
    {
        o = o.raw("expected", &x);
    }
    o.end()
}

/// With an expected set: the share of expected documents read, and the time
/// from the start to the first read of one of them.
fn expect_stats(e: &Episode, expect: &[String]) -> Option<String> {
    if expect.is_empty() {
        return None;
    }
    // A read that delivers no section (only front matter) is not relevant
    // context.
    let read: Vec<&String> = expect
        .iter()
        .filter(|x| e.docs.get(*x).is_some_and(|d| d.first_ms.is_some()))
        .collect();
    let first = expect
        .iter()
        .filter_map(|x| e.docs.get(x).and_then(|d| d.first_ms))
        .min();
    let ttf = match (first, e.start) {
        (Some(f), Some(s)) => (f - s).to_string(),
        _ => "null".into(),
    };
    Some(
        Obj::new()
            .num("read", read.len())
            .num("of", expect.len())
            .raw("first_relevant_ms", &ttf)
            .end(),
    )
}

/// Formats milliseconds since 1970 as an RFC 3339 UTC time.
fn format_time(ms: i64) -> String {
    let days = ms.div_euclid(86_400_000);
    let rem = ms.rem_euclid(86_400_000);
    // "civil_from_days" (Howard Hinnant).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3_600_000,
        rem / 60_000 % 60,
        rem / 1000 % 60
    )
}

/// Maps a file path from a transcript to a document id. The order of tries:
///
/// - the path relative to the repository root, or to the working directory of
///   the agent (a relative path starts at that directory)
/// - the longest tail of the path, with at least 2 segments, that is a
///   document path
///
/// Thus a run in an exported copy of the repository maps to the same ids.
fn map_path(
    corpus: Option<&Corpus>,
    root: Option<&str>,
    cwd: Option<&str>,
    path: &str,
) -> Option<u32> {
    let c = corpus?;
    let mut path = path.replace('\\', "/");
    if !path.starts_with('/')
        && let Some(abs) = cwd.and_then(|c| crate::model::join(c, &path))
    {
        path = abs;
    }
    let mut tries: Vec<String> = Vec::new();
    for base in [root, cwd].into_iter().flatten() {
        if let Some(rest) = path.strip_prefix(base).and_then(|r| r.strip_prefix('/')) {
            tries.push(rest.to_string());
        }
    }
    if !path.starts_with('/') {
        tries.push(path.trim_start_matches("./").to_string());
    }
    for t in &tries {
        if let Some(&d) = c.by_path.get(t) {
            return Some(d);
        }
    }
    let segs: Vec<&str> = path.split('/').collect();
    (0..segs.len().saturating_sub(1)).find_map(|i| c.by_path.get(&segs[i..].join("/")).copied())
}

/// Returns true if a read covers the whole document. Else it returns the ids of
/// the sections that the line range of the read overlaps.
fn sections_for(
    c: &Corpus,
    d: u32,
    lines: Option<Lines>,
    offset: Option<u64>,
    limit: Option<u64>,
) -> (bool, Vec<String>) {
    let doc = &c.docs[d as usize];
    let (start, count, total) = match lines {
        Some(l) => l,
        None => match (offset, limit) {
            (None, None) => return (true, Vec::new()),
            (o, l) => (o.unwrap_or(1).max(1), l.unwrap_or(u64::MAX / 2), None),
        },
    };
    let end = start.saturating_add(count.max(1)) - 1;
    if start <= 1 && total.is_some_and(|t| end >= t) {
        return (true, Vec::new());
    }
    let chunks = &doc.parsed.chunks;
    let mut out = Vec::new();
    for (j, ch) in chunks.iter().enumerate() {
        let first = u64::from(ch.line);
        let last = chunks
            .get(j + 1)
            .map_or(u64::MAX, |n| u64::from(n.line) - 1);
        if first <= end && last >= start {
            out.push(crate::context::section_id(doc, j));
        }
    }
    let full = out.len() == chunks.len();
    (full, if full { Vec::new() } else { out })
}

#[allow(clippy::too_many_arguments)]
fn attribute(
    corpus: Option<&Corpus>,
    root: Option<&str>,
    cwd: Option<&str>,
    e: &mut Episode,
    target: &Target,
    text: &str,
    chars: u64,
    lines: Option<Lines>,
    time: Option<i64>,
) {
    let note =
        |e: &mut Episode, id: String, via: Via, chars: u64, full: bool, sections: Vec<String>| {
            let d = e.docs.entry(id).or_default();
            d.reads += 1;
            d.chars += chars;
            d.full |= full;
            d.sections.extend(sections);
            d.via.insert(via);
            if d.first_ms.is_none() && (full || !d.sections.is_empty()) {
                d.first_ms = time;
            }
        };
    match target {
        Target::Read {
            path,
            offset,
            limit,
        } => match (corpus, map_path(corpus, root, cwd, path)) {
            (Some(c), Some(d)) => {
                let (full, sections) = sections_for(c, d, lines, *offset, *limit);
                note(
                    e,
                    c.docs[d as usize].id.clone(),
                    Via::Read,
                    chars,
                    full,
                    sections,
                );
            }
            _ => e.other_reads += 1,
        },
        Target::Shell { command } => {
            let commands = shell_commands(command);
            // spec-dna output names its sections in headers. A served section
            // counts as a read. A listed stub does not. Only the output of a
            // spec-dna command counts. Only ids that are documents count.
            // Without a configuration, only ids that have the form of an id
            // count. Any other text can contain a line of the same shape.
            let runs_spec_dna = commands
                .iter()
                .any(|c| program(c).is_some_and(|p| p == "spec-dna"));
            let served: Vec<String> = if runs_spec_dna {
                spec_dna_sections(text)
                    .into_iter()
                    .filter_map(|id| match corpus {
                        Some(c) => match c.resolve(&id) {
                            Res::Found(d, frag, true) => Some(match frag {
                                Some(f) => format!("{}#{f}", c.docs[d as usize].id),
                                None => c.docs[d as usize].id.clone(),
                            }),
                            _ => None,
                        },
                        None => is_plain_id(&id).then_some(id),
                    })
                    .collect()
            } else {
                Vec::new()
            };
            if !served.is_empty() {
                let mut by_doc: BTreeMap<String, Vec<String>> = BTreeMap::new();
                for s in served {
                    let doc = s.split('#').next().unwrap_or("").to_string();
                    by_doc.entry(doc).or_default().push(s);
                }
                let share = chars / by_doc.len() as u64;
                for (doc, secs) in by_doc {
                    note(e, doc, Via::SpecDna, share, false, secs);
                }
                return;
            }
            let Some(c) = corpus else {
                return;
            };
            let docs: BTreeSet<String> = shell_reads(&commands)
                .iter()
                .filter_map(|p| map_path(corpus, root, cwd, p))
                .map(|d| c.docs[d as usize].id.clone())
                .collect();
            if docs.is_empty() {
                return;
            }
            let share = chars / docs.len() as u64;
            for id in docs {
                note(e, id, Via::Shell, share, true, Vec::new());
            }
        }
        _ => {}
    }
}

/// Returns the section ids of spec-dna output headers in a text:
/// `<!-- <id> h:<16 hex digits> ...`.
fn spec_dna_sections(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix("<!-- ") else {
            continue;
        };
        let mut parts = rest.split(' ');
        let (Some(id), Some(h)) = (parts.next(), parts.next()) else {
            continue;
        };
        let hash = h.strip_prefix("h:").unwrap_or("");
        if hash.len() == 16 && hash.bytes().all(|b| b.is_ascii_hexdigit()) && !id.is_empty() {
            out.push(id.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

/// Splits a shell command into simple commands, each a list of words. Quotes
/// keep a word together. `<` and `>` are words of their own.
fn shell_commands(command: &str) -> Vec<Vec<String>> {
    fn flush(word: &mut String, cmds: &mut [Vec<String>]) {
        if !word.is_empty()
            && let Some(last) = cmds.last_mut()
        {
            last.push(std::mem::take(word));
        }
    }
    let mut cmds: Vec<Vec<String>> = vec![Vec::new()];
    let mut word = String::new();
    let mut quote: Option<char> = None;
    for ch in command.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => quote = Some(ch),
            (None, ';' | '|' | '&' | '\n' | '(' | ')') => {
                flush(&mut word, &mut cmds);
                if cmds.last().is_some_and(|c| !c.is_empty()) {
                    cmds.push(Vec::new());
                }
            }
            (None, '<' | '>') => {
                flush(&mut word, &mut cmds);
                if let Some(last) = cmds.last_mut() {
                    last.push(ch.to_string());
                }
            }
            (None, c) if c.is_whitespace() => flush(&mut word, &mut cmds),
            (None, c) => word.push(c),
        }
    }
    flush(&mut word, &mut cmds);
    cmds.retain(|c| !c.is_empty());
    cmds
}

/// Returns the program name of a simple command, after any `NAME=value`
/// assignments, without its directory.
/// The spec-dna subcommands that the report names. Any other word counts as
/// `other`, so the report never prints text from a command.
const SPEC_DNA_COMMANDS: &[&str] = &["init", "check", "context", "show", "mv", "transcript"];

/// Returns the subcommand of each spec-dna run in a shell command.
fn spec_dna_runs(command: &str) -> Vec<&'static str> {
    shell_commands(command)
        .iter()
        .filter(|c| program(c) == Some("spec-dna"))
        .map(|c| {
            // The first word after the program that is not an option.
            let at = c.iter().position(|w| !w.contains('=')).unwrap_or(0);
            let words = &c[(at + 1).min(c.len())..];
            let sub = match words.first().map(String::as_str) {
                Some("-h" | "--help") => "--help",
                Some("-V" | "--version") => "--version",
                _ => words
                    .iter()
                    .map(String::as_str)
                    .find(|w| !w.starts_with('-'))
                    .unwrap_or(""),
            };
            match sub {
                "-h" | "--help" | "help" | "" => "help",
                "-V" | "--version" => "version",
                s => SPEC_DNA_COMMANDS
                    .iter()
                    .find(|k| **k == s)
                    .copied()
                    .unwrap_or("other"),
            }
        })
        .collect()
}

fn program(cmd: &[String]) -> Option<&str> {
    let w = cmd.iter().find(|w| !w.contains('='))?;
    Some(w.rsplit('/').next().unwrap_or(w))
}

/// Programs that print the text of the files that they get.
const READERS: &[&str] = &[
    "cat", "head", "tail", "sed", "less", "more", "bat", "nl", "awk", "grep", "egrep", "rg", "ag",
    "cut", "diff",
];

/// Returns the Markdown paths that a shell command reads: the arguments of a
/// reader program, but not the target of a `>` redirection. The function
/// removes a `:line` suffix (as `grep -n` prints).
fn shell_reads(cmds: &[Vec<String>]) -> Vec<String> {
    let mut out = Vec::new();
    for cmd in cmds {
        if !program(cmd).is_some_and(|p| READERS.contains(&p)) {
            continue;
        }
        for (i, w) in cmd.iter().enumerate() {
            if i > 0 && cmd[i - 1] == ">" {
                continue;
            }
            let w = match w.rsplit_once(':') {
                Some((p, line)) if !line.is_empty() && line.bytes().all(|b| b.is_ascii_digit()) => {
                    p
                }
                _ => w.as_str(),
            };
            if w.ends_with(".md") || w.ends_with(".markdown") {
                out.push(w.to_string());
            }
        }
    }
    out
}

/// Returns true for a string with the form of a document or section id:
/// letters and digits in any script, and `-`, `_`, `.`, `/` and `#`. A path
/// that starts with `/` or `.`, or ends in a Markdown extension, is not one.
fn is_plain_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars().count() <= 200
        && !s.starts_with(['/', '.'])
        && !s.ends_with(".md")
        && !s.ends_with(".markdown")
        && s.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '#'))
}

/// Counts the search programs of a shell command: `grep`, `rg`, `find` and
/// similar at the start of a command of a list. A `grep` after `|` filters the
/// output of another command, so it does not count.
fn shell_searches(command: &str) -> u32 {
    command
        .replace("&&", ";")
        .replace("||", ";")
        .split([';', '\n', '&'])
        .filter_map(|part| {
            let mut words = part.split_whitespace().skip_while(|w| w.contains('='));
            match words.next()? {
                "git" => words.next().filter(|w| *w == "grep"),
                w => Some(w),
            }
        })
        .filter(|w| matches!(*w, "grep" | "egrep" | "rg" | "ag" | "find" | "fd"))
        .count() as u32
}

/// `1 session`, `2 sessions`.
fn n(count: impl Into<u64>, what: &str) -> String {
    let count = count.into();
    format!("{count} {what}{}", if count == 1 { "" } else { "s" })
}

/// Reads transcript files. `format` names an adapter. Without it, the first
/// line that parses selects the adapter of each file.
pub fn read_files(
    files: &[(std::path::PathBuf, bool)],
    format: Option<&str>,
    analysis: &mut Analysis,
) -> Result<(), String> {
    let forced = match format {
        Some(f) => Some(
            *ADAPTERS
                .iter()
                .find(|a| a.name() == f)
                .ok_or_else(|| format!("unknown transcript format `{f}`"))?,
        ),
        None => None,
    };
    // A forked subagent can repeat records of its parent, and the analysis
    // counts a record once. Thus read the files of main agents first, so that
    // a repeated record stays with the agent that made it.
    let mut order: Vec<(bool, usize)> = Vec::new();
    for (i, (f, named)) in files.iter().enumerate() {
        let text = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let text = String::from_utf8_lossy(&text);
        let mut adapter = forced;
        let mut first: Option<bool> = None;
        let mut records = Vec::new();
        for line in text.lines() {
            let Ok(v) = json::parse(line) else {
                continue;
            };
            if adapter.is_none() {
                adapter = ADAPTERS.iter().copied().find(|a| a.detect(&v));
            }
            if let Some(a) = adapter {
                a.records(&v, &mut records);
                if let Some(r) = records.first() {
                    first = Some(r.agent != "main");
                    break;
                }
            }
        }
        if adapter.is_none() {
            // A directory can hold other JSONL files, such as workflow
            // journals. Only a file that the user names is an error.
            if *named {
                return Err(format!(
                    "{}: no transcript adapter reads this file",
                    f.display()
                ));
            }
            eprintln!(
                "spec-dna: skipped {}: no transcript adapter reads this file",
                f.display()
            );
            continue;
        }
        order.push((first.unwrap_or(true), i));
    }
    order.sort();
    for (_, i) in order {
        let f = &files[i].0;
        let text = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let text = String::from_utf8_lossy(&text);
        let mut adapter = forced;
        let mut records = Vec::new();
        for line in text.lines() {
            let Ok(v) = json::parse(line) else {
                continue;
            };
            if adapter.is_none() {
                adapter = ADAPTERS.iter().copied().find(|a| a.detect(&v));
            }
            if let Some(a) = adapter {
                a.records(&v, &mut records);
                for r in records.drain(..) {
                    analysis.feed(a.name(), r);
                }
            }
        }
    }
    Ok(())
}

/// Returns the `.jsonl` files of the arguments, each with true if the user
/// named it. A directory gives its files at any depth, in sorted order. The
/// function returns a file once, even if two arguments give it.
pub fn collect_files(args: &[String]) -> Result<Vec<(std::path::PathBuf, bool)>, String> {
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) -> std::io::Result<()> {
        let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
        entries.sort_by_key(|e| e.file_name());
        for e in entries {
            let p = e.path();
            if e.file_type()?.is_dir() {
                walk(&p, out)?;
            } else if p.extension().is_some_and(|x| x == "jsonl") {
                out.push(p);
            }
        }
        Ok(())
    }
    let mut out: Vec<(std::path::PathBuf, bool)> = Vec::new();
    for a in args {
        let p = std::path::PathBuf::from(a);
        if p.is_dir() {
            let mut found = Vec::new();
            walk(&p, &mut found).map_err(|e| format!("{a}: {e}"))?;
            out.extend(found.into_iter().map(|f| (f, false)));
        } else if p.is_file() {
            out.push((p, true));
        } else {
            return Err(format!("{a}: no such file or directory"));
        }
    }
    let mut seen = HashSet::new();
    out.retain(|(p, _)| seen.insert(p.canonicalize().unwrap_or_else(|_| p.clone())));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsafe_names_are_redacted() {
        let ok = [
            (Field::Session, "0b7e2c1a-5f3d-4c8e-9a6b-1d2e3f4a5b6c"),
            (Field::Agent, "main"),
            (Field::Agent, "a4c8a27e0135498c0"),
            (Field::Origin, "task-notification"),
            (Field::Model, "claude-opus-5-5"),
            (Field::Model, "vendor.model-v1:0"),
            (Field::Tool, "Read"),
            (Field::Tool, "mcp__docs__search-pages"),
        ];
        for (f, v) in ok {
            assert_eq!(safe(f, v), v, "{f:?}");
        }
        let bad = [
            (Field::Session, "s-0001"),
            (Field::Session, "0B7E2C1A-5F3D-4C8E-9A6B-1D2E3F4A5B6C"),
            (Field::Agent, "agent-x"),
            (Field::Agent, ""),
            (Field::Origin, "Human"),
            (Field::Model, "SECRET-KEY"),
            (Field::Model, "model with spaces"),
            (Field::Tool, "rm -rf /"),
            (Field::Tool, "_hidden"),
            (Field::Tool, &"x".repeat(65)),
        ];
        for (f, v) in bad {
            assert_eq!(safe(f, v), REDACTED, "{f:?} {v}");
        }
    }

    #[test]
    fn times() {
        assert_eq!(parse_time("1970-01-01T00:00:00Z"), Some(0));
        assert_eq!(
            parse_time("2026-11-07T21:10:38.029Z"),
            Some(1_794_085_838_029)
        );
        assert_eq!(parse_time("2000-02-29T12:00:00.5Z"), Some(951_825_600_500));
        assert_eq!(parse_time("2026-11-07 21:10:38Z"), None);
        assert_eq!(format_time(1_794_085_838_029), "2026-11-07T21:10:38Z");
        assert_eq!(format_time(951_825_600_500), "2000-02-29T12:00:00Z");
    }

    #[test]
    fn shell_words() {
        let reads = |c: &str| shell_reads(&shell_commands(c));
        assert_eq!(
            reads("sed -n '1,40p' docs/a.md && cat \"docs/b c.md\" | head; grep -n x docs/c.md:12"),
            ["docs/a.md", "docs/b c.md", "docs/c.md"]
        );
        // Writes, staging and counts are not reads.
        assert!(
            reads("cat > docs/a.md; git add docs/b.md; wc -c docs/c.md; echo x >> docs/d.md")
                .is_empty()
        );
        assert_eq!(
            reads("LC_ALL=C /usr/bin/cat docs/e.md > out.md"),
            ["docs/e.md"]
        );
        assert_eq!(
            program(&shell_commands("X=1 ~/.cargo/bin/spec-dna context a")[0]),
            Some("spec-dna")
        );
        assert_eq!(
            spec_dna_runs(
                "X=1 spec-dna context a && spec-dna check --base main; spec-dna secret-word; spec-dna --help | head; spec-dna --json show x"
            ),
            ["context", "check", "other", "help", "show"]
        );
        assert!(is_plain_id("irrigation/valves#requirements") && is_plain_id("Ünterlagen/x"));
        for bad in ["KEY=value", "/home/u/a", "./a", "a.md", "a b", "a:b"] {
            assert!(!is_plain_id(bad), "{bad}");
        }
        assert_eq!(
            shell_searches(
                "grep -r x . | grep -v y; find . -name '*.md' && LC_ALL=C git grep z; cat a | grep q"
            ),
            3
        );
    }

    #[test]
    fn spec_dna_headers() {
        let text = "<!-- spec-dna context 0.1.0: a; 2 sections -->\n\n<!-- irrigation/valves#requirements h:c45e6a10f2970c6b spec/active entry -->\n## R\n- `irrigation/sensors` h:6860bfc09e21e7a5 10 chars (depends): Sensors\n<!-- irrigation/valves h:958433b88811cebb spec/active docs/irrigation/valves.md -->\n";
        assert_eq!(
            spec_dna_sections(text),
            ["irrigation/valves", "irrigation/valves#requirements"]
        );
    }
}

//! `spec-dna transcript` reads agent transcripts. For each session and each
//! episode, it reports token usage, tool calls, and the documents that the
//! agent read, as spec-dna ids and sections.
//!
//! A transcript can contain secrets. Thus this module prints counts, names of
//! tools and models, and spec-dna ids only. It never prints prompt, command,
//! path or file text. It reads that text in memory to find document reads,
//! and then drops it.
//!
//! An adapter converts one transcript format into `Record`s. The analysis does
//! not depend on the format. `ClaudeCode` reads Claude Code JSONL files.

use crate::json::{self, Obj, Value};
use crate::model::Corpus;
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
    Prompt { origin: String },
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
            if !request.is_empty() && u != &Value::Null {
                push(
                    out,
                    Event::Usage {
                        request: request.to_string(),
                        model: msg.get("model").as_str().unwrap_or("").to_string(),
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
            if o.get("isMeta").is_true() {
                return;
            }
            let origin = match o.get("origin").get("kind").as_str() {
                Some(k) => k.to_string(),
                None => "human".to_string(),
            };
            push(out, Event::Prompt { origin });
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
                agent: r.agent.clone(),
                origin: match &r.event {
                    Event::Prompt { origin } => origin.clone(),
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
                // A repeated request id replaces its earlier usage.
                if let Some((old_ei, old)) = s.requests.get(&request).copied() {
                    s.episodes[old_ei].tokens.sub(&old);
                    s.episodes[old_ei].tokens.add(&tokens);
                    s.requests.insert(request, (old_ei, tokens));
                } else {
                    let e = &mut s.episodes[ei];
                    e.requests += 1;
                    e.tokens.add(&tokens);
                    s.requests.insert(request, (ei, tokens));
                }
                if !model.is_empty() {
                    s.episodes[ei].models.insert(model);
                }
            }
            Event::Call { id, name, target } => {
                let e = &mut s.episodes[ei];
                *e.tools.entry(name.clone()).or_default() += 1;
                match &target {
                    Target::Search => e.searches += 1,
                    Target::Shell { command } => e.searches += shell_searches(command),
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

    /// Returns the report. `expect` lists the document ids that the task needs;
    /// with them, the report gives the time to the first relevant read and the
    /// share of expected documents that the agent read.
    pub fn report(&self, expect: &[String], as_json: bool) -> String {
        let mut out = String::new();
        let mut sessions_json = Vec::new();
        let (mut n_agents, mut n_episodes, mut n_requests) = (0, 0, 0);
        for (id, s) in &self.sessions {
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
        let what = if d.full || d.sections.is_empty() {
            "full".to_string()
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
            .bool("full", d.full || d.sections.is_empty())
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
    let read: Vec<&String> = expect.iter().filter(|x| e.docs.contains_key(*x)).collect();
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
/// - the path relative to the repository root, or to the working directory
///   of the agent;
/// - the longest tail of the path, with at least 2 segments, that is a
///   document path. Thus a run in an exported copy of the repository maps to
///   the same ids.
fn map_path(
    corpus: Option<&Corpus>,
    root: Option<&str>,
    cwd: Option<&str>,
    path: &str,
) -> Option<u32> {
    let c = corpus?;
    let path = path.replace('\\', "/");
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
            if d.first_ms.is_none() {
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
            // spec-dna output names its sections in headers. A served section
            // counts as a read; a listed stub does not.
            let served = spec_dna_sections(text);
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
            let docs: BTreeSet<String> = shell_paths(command)
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

/// Returns the words of a shell command that look like Markdown file paths.
/// Quotes keep a word together. A `:line` suffix (as `grep -n` prints) is
/// removed.
fn shell_paths(command: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word = String::new();
    let mut quote: Option<char> = None;
    for ch in command.chars() {
        match (quote, ch) {
            (Some(q), c) if c == q => quote = None,
            (Some(_), c) => word.push(c),
            (None, '"' | '\'') => quote = Some(ch),
            (None, c)
                if c.is_whitespace() || matches!(c, ';' | '|' | '&' | '<' | '>' | '(' | ')') =>
            {
                words.push(std::mem::take(&mut word));
            }
            (None, c) => word.push(c),
        }
    }
    words.push(word);
    words
        .into_iter()
        .map(|w| match w.rsplit_once(':') {
            Some((p, line)) if line.bytes().all(|b| b.is_ascii_digit()) => p.to_string(),
            _ => w,
        })
        .filter(|w| w.ends_with(".md") || w.ends_with(".markdown"))
        .collect()
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

/// Reads transcript files. `format` names an adapter; without it, the first
/// line that parses selects the adapter of each file.
pub fn read_files(
    files: &[std::path::PathBuf],
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
    for f in files {
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
        if adapter.is_none() {
            return Err(format!(
                "{}: no transcript adapter reads this file",
                f.display()
            ));
        }
    }
    Ok(())
}

/// Returns the `.jsonl` files of the arguments. A directory gives its files
/// at any depth, in sorted order.
pub fn collect_files(args: &[String]) -> Result<Vec<std::path::PathBuf>, String> {
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
    let mut out = Vec::new();
    for a in args {
        let p = std::path::PathBuf::from(a);
        if p.is_dir() {
            walk(&p, &mut out).map_err(|e| format!("{a}: {e}"))?;
        } else if p.is_file() {
            out.push(p);
        } else {
            return Err(format!("{a}: no such file or directory"));
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            shell_paths(
                "sed -n '1,40p' docs/a.md && cat \"docs/b c.md\" | head; grep -n x docs/c.md:12"
            ),
            ["docs/a.md", "docs/b c.md", "docs/c.md"]
        );
        assert_eq!(
            shell_searches(
                "grep -r x . | grep -v y; find . -name '*.md' && LC_ALL=C git grep z; cat a | grep q"
            ),
            3
        );
    }

    #[test]
    fn spec_dna_headers() {
        let text = "<!-- spec-dna context 0.1.0: a; 2 sections -->\n\n<!-- auth/session#requirements h:c45e6a10f2970c6b spec/active entry -->\n## R\n- `auth/tokens` h:6860bfc09e21e7a5 10 chars (depends): Tokens\n<!-- auth/session h:958433b88811cebb spec/active docs/auth/session.md -->\n";
        assert_eq!(
            spec_dna_sections(text),
            ["auth/session", "auth/session#requirements"]
        );
    }
}

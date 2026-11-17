//! The hooks of the agent harness: `spec-dna hook <name> [flags]`.
//!
//! A hook runs on each tool call of an agent. It reads one JSON object from
//! stdin and one file at most. It never reads `spec-dna.toml`, never checks the
//! pin and never scans the repository. Unknown flags and bad values are
//! ignored, the exit code is always 0, and an error gives no output.
//! `protect-tests` reads no file. It runs `git` at most 2 times, and only for
//! a path that matches a test glob. `protect-approvals` runs `git` only for a
//! command that holds the words `git` and `commit` (see its section).

use std::collections::{HashMap, HashSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, SystemTime};

use crate::json::{self, Obj, Value};

/// The most bytes that a hook reads from stdin or from a script file.
const MAX_INPUT: u64 = 1 << 20;
/// The meter reads a transcript backward in chunks of this size.
const CHUNK: u64 = 256 << 10;
/// The meter reads at most this many bytes from the end of a transcript.
const MAX_SCAN: u64 = 4 << 20;
/// A state file that is older than this is stale.
const STATE_TTL: Duration = Duration::from_secs(3 * 24 * 3600);
const DEFAULT_CAP: u64 = 10;

/// Runs one hook. It returns after it printed the output, if any.
pub fn run(args: &[String]) {
    // A panic must give no text and no exit code other than 0.
    std::panic::set_hook(Box::new(|_| {}));
    let _ = std::panic::catch_unwind(|| {
        let Some((name, flags)) = args.split_first() else {
            return;
        };
        let mut raw = Vec::new();
        if std::io::stdin()
            .take(MAX_INPUT)
            .read_to_end(&mut raw)
            .is_err()
        {
            return;
        }
        let Ok(input) = String::from_utf8(raw) else {
            return;
        };
        let out = match name.as_str() {
            "context-meter" => meter(
                &input,
                &MeterOpts::parse(flags),
                &std::env::temp_dir().join("spec-dna-meter"),
            ),
            "fanout-cap" => {
                let project = std::env::var_os("CLAUDE_PROJECT_DIR").map(PathBuf::from);
                let env_cap = std::env::var("SPEC_DNA_FANOUT_CAP").ok();
                fanout(
                    &input,
                    flag_cap(flags),
                    env_cap.as_deref(),
                    project.as_deref(),
                )
            }
            "protect-tests" => {
                let project = std::env::var_os("CLAUDE_PROJECT_DIR").map(PathBuf::from);
                protect(
                    &input,
                    &ProtectOpts::parse(flags),
                    project.as_deref(),
                    &mut run_git,
                )
            }
            "protect-approvals" => {
                let project = std::env::var_os("CLAUDE_PROJECT_DIR").map(PathBuf::from);
                approvals(
                    &input,
                    &ApprovalsOpts::parse(flags),
                    project.as_deref(),
                    &mut run_git,
                )
            }
            _ => None,
        };
        if let Some(text) = out {
            let _ = writeln!(std::io::stdout(), "{text}");
        }
    });
}

/// Returns the value of a flag, in the form `--name value` or `--name=value`.
fn flag_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    let mut found = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            found = args.get(i + 1).map(String::as_str);
            i += 1;
        } else if let Some(v) = args[i].strip_prefix(name).and_then(|r| r.strip_prefix('=')) {
            found = Some(v);
        }
        i += 1;
    }
    found
}

fn flag_cap(args: &[String]) -> Option<u64> {
    flag_value(args, "--cap").and_then(|v| v.parse().ok())
}

/// Returns each value of a flag that can repeat, in order, in the form
/// `--name value` or `--name=value`.
fn flag_values<'a>(args: &'a [String], name: &str) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            if let Some(v) = args.get(i + 1) {
                found.push(v.as_str());
            }
            i += 1;
        } else if let Some(v) = args[i].strip_prefix(name).and_then(|r| r.strip_prefix('=')) {
            found.push(v);
        }
        i += 1;
    }
    found
}

fn output(event: &str, key: &str, text: &str) -> String {
    let inner = Obj::new().str("hookEventName", event).str(key, text).end();
    Obj::new().raw("hookSpecificOutput", &inner).end()
}

// ---------------------------------------------------------------------------
// context-meter
// ---------------------------------------------------------------------------

/// The flags of `context-meter`. An empty `levels` list and a `subagent` of 0
/// turn the meter off.
#[derive(Default)]
pub struct MeterOpts {
    pub levels: Vec<u64>,
    pub subagent: u64,
}

impl MeterOpts {
    fn parse(args: &[String]) -> Self {
        let mut levels: Vec<u64> = flag_value(args, "--levels")
            .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
            .unwrap_or_default();
        levels.retain(|&l| l > 0);
        levels.sort_unstable();
        levels.dedup();
        let subagent = flag_value(args, "--subagent")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        MeterOpts { levels, subagent }
    }
}

const MSG_FIRST: &str = "spec-dna context meter: this session has about {k}K tokens of context, and each request sends all of them again. Delegate long searches and long reads to a subagent. Ask it for a short summary.";
const MSG_HIGHER: &str = "spec-dna context meter: this session has about {k}K tokens of context. Save the state of the task in a file: the goal, the work done, the next steps and the open questions. Then tell the user that a new session can continue from that file.";
const MSG_SUBAGENT: &str = "spec-dna context meter: this subagent has about {k}K tokens of context. Complete the current step. Then return your findings and the open work to the caller.";

/// An id of a session or of a subagent: 1 to 64 of `A-Za-z0-9_-`.
fn valid_id(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

/// Handles one PostToolUse event. It returns the JSON to print, if the meter
/// must speak. `state_dir` holds one empty file for each crossing.
pub fn meter(input: &str, opts: &MeterOpts, state_dir: &Path) -> Option<String> {
    let v = json::parse(input).ok()?;
    let transcript = v
        .get("transcript_path")
        .as_str()
        .filter(|s| !s.is_empty())?;
    let session = v.get("session_id").as_str().filter(|s| valid_id(s))?;
    let agent = match v.get("agent_id") {
        Value::Null => None,
        Value::Str(a) if a.is_empty() => None,
        Value::Str(a) if valid_id(a) => Some(a.as_str()),
        _ => return None,
    };
    let (path, levels, key) = match agent {
        Some(a) => {
            let dir = Path::new(transcript).parent()?;
            let file = format!("agent-{a}.jsonl");
            (
                dir.join(session).join("subagents").join(file),
                if opts.subagent > 0 {
                    vec![opts.subagent]
                } else {
                    Vec::new()
                },
                format!("{session}-{a}"),
            )
        }
        None => (
            PathBuf::from(transcript),
            opts.levels.clone(),
            session.to_string(),
        ),
    };
    let first = *levels.first()?;
    let context = last_context(&path)?;
    if context.saturating_mul(3) < first.saturating_mul(2) {
        // The context dropped: the agent compacted it. The next crossing speaks.
        for l in &levels {
            let _ = fs::remove_file(state_dir.join(format!("{key}-{l}")));
        }
        return None;
    }
    let level = *levels.iter().rev().find(|&&l| context >= l)?;
    fs::create_dir_all(state_dir).ok()?;
    remove_stale(state_dir);
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(state_dir.join(format!("{key}-{level}")))
        .ok()?;
    let template = match (agent.is_some(), level == first) {
        (true, _) => MSG_SUBAGENT,
        (false, true) => MSG_FIRST,
        (false, false) => MSG_HIGHER,
    };
    let text = template.replace("{k}", &(context / 1000).to_string());
    Some(output("PostToolUse", "additionalContext", &text))
}

/// Removes the state files that are older than `STATE_TTL`.
fn remove_stale(dir: &Path) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    let now = SystemTime::now();
    for e in entries.flatten() {
        let old = e
            .metadata()
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| now.duration_since(t).ok())
            .is_some_and(|age| age > STATE_TTL);
        if old {
            let _ = fs::remove_file(e.path());
        }
    }
}

/// Returns the context size of the last assistant message of a transcript. It
/// reads the file backward and stops after `MAX_SCAN` bytes.
fn last_context(path: &Path) -> Option<u64> {
    let mut file = File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let stop = len.saturating_sub(MAX_SCAN);
    let mut pos = len;
    // The start of a line that the next chunk completes.
    let mut carry: Vec<u8> = Vec::new();
    while pos > stop {
        let from = pos.saturating_sub(CHUNK).max(stop);
        let mut buf = vec![0u8; (pos - from) as usize];
        file.seek(SeekFrom::Start(from)).ok()?;
        file.read_exact(&mut buf).ok()?;
        buf.extend_from_slice(&carry);
        let mut end = buf.len();
        while let Some(k) = buf[..end].iter().rposition(|&c| c == b'\n') {
            if let Some(n) = line_context(&buf[k + 1..end]) {
                return Some(n);
            }
            end = k;
        }
        buf.truncate(end);
        carry = buf;
        pos = from;
    }
    // At the start of the file, the carry is a whole line.
    if pos == 0 { line_context(&carry) } else { None }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    let first = needle[0];
    let mut i = 0;
    while let Some(k) = hay[i..].iter().position(|&c| c == first) {
        i += k;
        if hay[i..].starts_with(needle) {
            return true;
        }
        i += 1;
    }
    false
}

/// Returns the context size of a transcript line, or `None` if the line is not
/// an assistant message with a usage object.
fn line_context(line: &[u8]) -> Option<u64> {
    if !contains(line, br#""type":"assistant""#) {
        return None;
    }
    let v = json::parse(std::str::from_utf8(line).ok()?).ok()?;
    if v.get("type").as_str() != Some("assistant") {
        return None;
    }
    let usage = v.get("message").get("usage");
    if !matches!(usage, Value::Obj(_)) {
        return None;
    }
    Some(
        usage
            .get("input_tokens")
            .as_u64()
            .saturating_add(usage.get("cache_creation_input_tokens").as_u64())
            .saturating_add(usage.get("cache_read_input_tokens").as_u64()),
    )
}

// ---------------------------------------------------------------------------
// fanout-cap
// ---------------------------------------------------------------------------

/// Handles one PreToolUse event of the `Workflow` tool. It returns the JSON to
/// print when the run needs the approval of the user. `env_cap` is the value of
/// `SPEC_DNA_FANOUT_CAP`, and `project` is `CLAUDE_PROJECT_DIR`.
pub fn fanout(
    input: &str,
    cap_flag: Option<u64>,
    env_cap: Option<&str>,
    project: Option<&Path>,
) -> Option<String> {
    let cap = env_cap
        .and_then(|v| v.trim().parse().ok())
        .or(cap_flag)
        .unwrap_or(DEFAULT_CAP);
    if cap == 0 {
        return None;
    }
    let v = json::parse(input).ok()?;
    let script = read_script(&v, project)?;
    let found = bound(&script)?;
    let reason = match found.total {
        None => format!(
            "spec-dna fan-out cap: line {} can start agents without a limit: a loop over a list that the run makes, a loop without a number limit, a recursive call or a nested workflow. Limit the loop, for example with .slice(0, {cap}). Or approve this run.",
            found.line
        ),
        Some(n) if n > cap => format!(
            "spec-dna fan-out cap: this workflow can start {n} agents, more than the cap of {cap}. Approve it only if you need the extra agents. To change the cap, start Claude Code with SPEC_DNA_FANOUT_CAP=<n>."
        ),
        Some(_) => return None,
    };
    let inner = Obj::new()
        .str("hookEventName", "PreToolUse")
        .str("permissionDecision", "ask")
        .str("permissionDecisionReason", &reason)
        .end();
    Some(Obj::new().raw("hookSpecificOutput", &inner).end())
}

/// Finds the script text of a Workflow call: the inline script, a script file
/// relative to `cwd`, or a named script of the project.
fn read_script(v: &Value, project: Option<&Path>) -> Option<String> {
    let tool = v.get("tool_input");
    let cwd = v.get("cwd").as_str().map(PathBuf::from);
    if let Some(s) = tool.get("script").as_str().filter(|s| !s.is_empty()) {
        return Some(s.to_string());
    }
    if let Some(p) = tool.get("scriptPath").as_str().filter(|s| !s.is_empty()) {
        return read_limited(&cwd.unwrap_or_default().join(p));
    }
    let name = tool.get("name").as_str()?;
    if name.is_empty() || name.contains(['/', '\\']) || name.contains("..") {
        return None;
    }
    let root = project.map(Path::to_path_buf).or(cwd)?;
    let dir = root.join(".claude").join("workflows");
    ["js", "mjs"]
        .iter()
        .find_map(|ext| read_limited(&dir.join(format!("{name}.{ext}"))))
}

fn read_limited(path: &Path) -> Option<String> {
    let mut raw = Vec::new();
    File::open(path)
        .ok()?
        .take(MAX_INPUT)
        .read_to_end(&mut raw)
        .ok()?;
    String::from_utf8(raw).ok()
}

/// A count of agents. `None` means that the count has no limit.
type Count = Option<u64>;

fn mul(a: Count, b: Count) -> Count {
    match (a, b) {
        (Some(0), _) | (_, Some(0)) => Some(0),
        (Some(x), Some(y)) => Some(x.saturating_mul(y)),
        _ => None,
    }
}

fn add(a: Count, b: Count) -> Count {
    Some(a?.saturating_add(b?))
}

/// The upper limit of the agents of a script.
#[derive(Debug, PartialEq)]
pub struct Bound {
    pub total: Count,
    /// The line of the first call site with no limit. It is 0 if there is none.
    pub line: usize,
}

/// Returns the upper limit of the agents that a script starts. It returns
/// `None` if the script cannot be read: then the caller allows the run.
pub fn bound(script: &str) -> Option<Bound> {
    let b = blank(script.as_bytes())?;
    let pair = pair_brackets(&b)?;
    let mut s = Scan {
        b: &b,
        pair,
        toks: Vec::new(),
        consts: HashMap::new(),
        funcs: Vec::new(),
        loops: Vec::new(),
        sites: Vec::new(),
        memo: Vec::new(),
    };
    s.toks = s.tokens();
    s.collect_declarations();
    s.collect_loops_and_sites();
    s.memo = vec![None; s.funcs.len()];
    let mut total = Some(0);
    let mut line = 0;
    for i in 0..s.sites.len() {
        let (pos, base) = s.sites[i];
        let n = s.count_at(pos, base, &mut Vec::new());
        if n.is_none() && line == 0 {
            line = 1 + b[..pos].iter().filter(|&&c| c == b'\n').count();
        }
        total = add(total, n);
    }
    Some(Bound { total, line })
}

/// Replaces comments, strings and the text of templates with spaces. It keeps
/// each newline and each offset. The code inside `${ }` stays. A regular
/// expression literal is not known: a quote in it can unbalance the brackets,
/// and then the script is allowed.
fn blank(src: &[u8]) -> Option<Vec<u8>> {
    let mut out = src.to_vec();
    let mut i = 0;
    blank_code(src, &mut out, &mut i, 0, false)?;
    Some(out)
}

fn put_blank(out: &mut [u8], i: usize) {
    if out[i] != b'\n' {
        out[i] = b' ';
    }
}

/// Blanks the code from `*i`. In a template (`in_tpl`), it stops after the `}`
/// that closes the `${`.
fn blank_code(src: &[u8], out: &mut [u8], i: &mut usize, depth: u32, in_tpl: bool) -> Option<()> {
    if depth > 32 {
        return None;
    }
    let n = src.len();
    let mut braces = 0usize;
    while *i < n {
        let c = src[*i];
        let next = src.get(*i + 1).copied();
        match c {
            b'/' if next == Some(b'/') => {
                while *i < n && src[*i] != b'\n' {
                    put_blank(out, *i);
                    *i += 1;
                }
            }
            b'/' if next == Some(b'*') => {
                put_blank(out, *i);
                put_blank(out, *i + 1);
                *i += 2;
                while *i < n && !(src[*i] == b'*' && src.get(*i + 1) == Some(&b'/')) {
                    put_blank(out, *i);
                    *i += 1;
                }
                for _ in 0..2 {
                    if *i < n {
                        put_blank(out, *i);
                        *i += 1;
                    }
                }
            }
            b'\'' | b'"' => {
                put_blank(out, *i);
                *i += 1;
                // An unterminated string ends at the end of the line.
                while *i < n && src[*i] != c && src[*i] != b'\n' {
                    if src[*i] == b'\\' && *i + 1 < n {
                        put_blank(out, *i);
                        *i += 1;
                    }
                    put_blank(out, *i);
                    *i += 1;
                }
                if *i < n && src[*i] == c {
                    put_blank(out, *i);
                    *i += 1;
                }
            }
            b'`' => {
                put_blank(out, *i);
                *i += 1;
                loop {
                    if *i >= n {
                        return None;
                    }
                    match src[*i] {
                        b'`' => {
                            put_blank(out, *i);
                            *i += 1;
                            break;
                        }
                        b'\\' if *i + 1 < n => {
                            put_blank(out, *i);
                            put_blank(out, *i + 1);
                            *i += 2;
                        }
                        b'$' if src.get(*i + 1) == Some(&b'{') => {
                            put_blank(out, *i);
                            put_blank(out, *i + 1);
                            *i += 2;
                            blank_code(src, out, i, depth + 1, true)?;
                        }
                        _ => {
                            put_blank(out, *i);
                            *i += 1;
                        }
                    }
                }
            }
            b'{' => {
                braces += 1;
                *i += 1;
            }
            b'}' => {
                if in_tpl && braces == 0 {
                    put_blank(out, *i);
                    *i += 1;
                    return Some(());
                }
                braces = braces.saturating_sub(1);
                *i += 1;
            }
            _ => *i += 1,
        }
    }
    if in_tpl { None } else { Some(()) }
}

const NONE: usize = usize::MAX;

/// Matches the brackets. For each bracket, the result has the index of its
/// partner. It returns `None` if the brackets are not balanced.
fn pair_brackets(b: &[u8]) -> Option<Vec<usize>> {
    let mut pair = vec![NONE; b.len()];
    let mut stack: Vec<usize> = Vec::new();
    for (i, &c) in b.iter().enumerate() {
        match c {
            b'(' | b'[' | b'{' => stack.push(i),
            b')' | b']' | b'}' => {
                let o = stack.pop()?;
                let want = match b[o] {
                    b'(' => b')',
                    b'[' => b']',
                    _ => b'}',
                };
                if c != want {
                    return None;
                }
                pair[o] = i;
                pair[i] = o;
            }
            _ => {}
        }
    }
    stack.is_empty().then_some(pair)
}

fn is_ws(c: u8) -> bool {
    matches!(c, b' ' | b'\t' | b'\n' | b'\r')
}

fn is_ident(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'$') || c >= 0x80
}

/// A loop: the span of the code that it repeats, and the repeat count.
struct Loop {
    start: usize,
    end: usize,
    size: Count,
}

/// A function with a name, and the span of its body.
struct Func<'a> {
    name: &'a [u8],
    /// The offset of the name in the declaration.
    decl: usize,
    start: usize,
    end: usize,
}

struct Scan<'a> {
    /// The blanked script.
    b: &'a [u8],
    pair: Vec<usize>,
    /// The identifiers of the script: start and end.
    toks: Vec<(usize, usize)>,
    /// The size of the array literal of each variable. `None`: no limit.
    consts: HashMap<&'a [u8], Count>,
    funcs: Vec<Func<'a>>,
    loops: Vec<Loop>,
    /// A call site of `agent(` (base 1) or of `workflow(` (no limit).
    sites: Vec<(usize, Count)>,
    /// The call count of each function, when known.
    memo: Vec<Option<Count>>,
}

const ITERATORS: [&[u8]; 8] = [
    b"reduce", b"filter", b"map", b"every", b"flatMap", b"some", b"forEach", b"find",
];

impl<'a> Scan<'a> {
    fn skip_ws(&self, mut i: usize) -> usize {
        while i < self.b.len() && is_ws(self.b[i]) {
            i += 1;
        }
        i
    }

    /// Returns the end of the text before `p`, without the spaces at its end.
    fn skip_ws_back(&self, mut p: usize) -> usize {
        while p > 0 && is_ws(self.b[p - 1]) {
            p -= 1;
        }
        p
    }

    /// Returns the end of the identifier that starts at `i`.
    fn ident_end(&self, mut i: usize) -> usize {
        while i < self.b.len() && is_ident(self.b[i]) {
            i += 1;
        }
        i
    }

    /// Returns the identifier at `i` (after spaces) and its end.
    fn ident_after(&self, i: usize) -> Option<(&'a [u8], usize)> {
        let s = self.skip_ws(i);
        let e = self.ident_end(s);
        (e > s).then(|| (&self.b[s..e], e))
    }

    /// Returns the offset of a `(` that follows `i`, after spaces.
    fn paren_after(&self, i: usize) -> Option<usize> {
        let p = self.skip_ws(i);
        (self.b.get(p) == Some(&b'(')).then_some(p)
    }

    /// Returns the identifiers of the script, each with its offset.
    fn tokens(&self) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < self.b.len() {
            if is_ident(self.b[i]) {
                let e = self.ident_end(i);
                // A number is not an identifier.
                if !self.b[i].is_ascii_digit() {
                    out.push((i, e));
                }
                i = e;
            } else {
                i += 1;
            }
        }
        out
    }

    /// Splits the content of a bracket pair at the commas of its top level.
    /// Each part is trimmed. A trailing comma gives no part.
    fn args(&self, open: usize) -> Vec<(usize, usize)> {
        let close = self.pair[open];
        let mut out = Vec::new();
        let mut start = open + 1;
        let mut i = open + 1;
        while i < close {
            match self.b[i] {
                b'(' | b'[' | b'{' => i = self.pair[i],
                b',' => {
                    out.push(self.trim(start, i));
                    start = i + 1;
                }
                _ => {}
            }
            i += 1;
        }
        let last = self.trim(start, close);
        if last.0 < last.1 {
            out.push(last);
        }
        out
    }

    fn trim(&self, s: usize, e: usize) -> (usize, usize) {
        let s = self.skip_ws(s).min(e);
        (s, self.skip_ws_back(e).max(s))
    }

    /// Returns the value and the end of a decimal number at `i`.
    fn number(&self, i: usize) -> Option<(u64, usize)> {
        let mut k = i;
        let mut n: u64 = 0;
        while k < self.b.len() && self.b[k].is_ascii_digit() {
            n = n
                .saturating_mul(10)
                .saturating_add((self.b[k] - b'0') as u64);
            k += 1;
        }
        let ok = k > i && !self.b.get(k).is_some_and(|&c| is_ident(c) || c == b'.');
        ok.then_some((n, k))
    }

    /// Returns the end of a statement that starts at `i`: a `;`, a closing
    /// bracket, a comma (if `comma`), or a line end that does not go on.
    fn stmt_end(&self, mut i: usize, comma: bool) -> usize {
        let n = self.b.len();
        while i < n {
            match self.b[i] {
                b'(' | b'[' | b'{' => i = self.pair[i],
                b')' | b']' | b'}' | b';' => return i,
                b',' if comma => return i,
                b'\n' => {
                    let j = self.skip_ws(i);
                    let before = self.skip_ws_back(i);
                    let goes_on = j < n
                        && (b".?:+-*/%&|^<>=,([".contains(&self.b[j])
                            || self.b[j..].starts_with(b"else"));
                    let open_end = before > 0 && b"=>+-*/%&|^?:,.!<~".contains(&self.b[before - 1]);
                    if j >= n || !(goes_on || open_end) {
                        return i;
                    }
                }
                _ => {}
            }
            i += 1;
        }
        n
    }

    /// Returns the span of the body of a loop or of a function that starts at
    /// `i`: a block, or one statement.
    fn body(&self, i: usize, comma: bool) -> (usize, usize) {
        let s = self.skip_ws(i);
        if self.b.get(s) == Some(&b'{') {
            (s, self.pair[s])
        } else {
            (s, self.stmt_end(s, comma))
        }
    }

    // ----- declarations (first pass) -----

    /// Reads the variables with an array literal, and the named functions.
    fn collect_declarations(&mut self) {
        let b = self.b;
        for k in 0..self.toks.len() {
            let (s, e) = self.toks[k];
            match &b[s..e] {
                b"function" => {
                    let mut p = self.skip_ws(e);
                    if b.get(p) == Some(&b'*') {
                        p = self.skip_ws(p + 1);
                    }
                    let pe = self.ident_end(p);
                    if pe > p {
                        self.add_function(&b[p..pe], p, pe);
                    }
                }
                b"const" | b"let" | b"var" => self.declaration(e),
                _ => {}
            }
        }
    }

    /// Reads `function name (...) { ... }`; `from` is the end of the name.
    fn add_function(&mut self, name: &'a [u8], decl: usize, from: usize) {
        if let Some(open) = self.paren_after(from) {
            let body = self.skip_ws(self.pair[open] + 1);
            if self.b.get(body) == Some(&b'{') {
                self.funcs.push(Func {
                    name,
                    decl,
                    start: body,
                    end: self.pair[body],
                });
            }
        }
    }

    /// Reads `const|let|var name = ...` after the keyword that ends at `e`.
    fn declaration(&mut self, e: usize) {
        let Some((name, after)) = self.ident_after(e) else {
            return;
        };
        let decl = after - name.len();
        let eq = self.skip_ws(after);
        if self.b.get(eq) != Some(&b'=') || matches!(self.b.get(eq + 1), Some(b'=' | b'>')) {
            return;
        }
        let v = self.skip_ws(eq + 1);
        let size = if self.b.get(v) == Some(&b'[') && self.literal_ends(self.pair[v] + 1) {
            self.literal_len(v)
        } else {
            None
        };
        // A name with two declarations takes the larger size.
        let known = self.consts.get(name).copied();
        self.consts.insert(
            name,
            match known {
                None => size,
                Some(old) => old.zip(size).map(|(x, y)| x.max(y)),
            },
        );
        // An arrow function, or a function expression, with a name.
        let mut p = v;
        if let Some((_, a)) = self.ident_after(p).filter(|(w, _)| *w == b"async") {
            p = a;
        }
        let p = self.skip_ws(p);
        if self.b[p..].starts_with(b"function") && !self.b.get(p + 8).is_some_and(|&c| is_ident(c))
        {
            let mut q = self.skip_ws(p + 8);
            if self.b.get(q) == Some(&b'*') {
                q = self.skip_ws(q + 1);
            }
            // The function can have its own name.
            q = self.ident_end(q);
            self.add_function(name, decl, q);
            return;
        }
        let params_end = if self.b.get(p) == Some(&b'(') {
            self.pair[p] + 1
        } else {
            self.ident_end(p)
        };
        if params_end == p {
            return;
        }
        let arrow = self.skip_ws(params_end);
        if self.b[arrow..].starts_with(b"=>") {
            let (start, end) = self.body(arrow + 2, true);
            self.funcs.push(Func {
                name,
                decl,
                start,
                end,
            });
        }
    }

    /// Tests whether a literal that ends before `i` is the whole value of a
    /// declaration: a `;`, a comma, or a new statement follows it.
    fn literal_ends(&self, i: usize) -> bool {
        let j = self.skip_ws(i);
        if j >= self.b.len() || matches!(self.b[j], b';' | b',' | b'}') {
            return true;
        }
        self.b[i..j].contains(&b'\n') && !b".([?+-*/%&|^<>=:".contains(&self.b[j])
    }

    // ----- sizes -----

    /// Returns the count of the elements of an array literal that starts at
    /// `open`.
    fn literal_len(&self, open: usize) -> Count {
        let args = self.args(open);
        if args.iter().any(|&(s, e)| self.b[s..e].starts_with(b"...")) {
            return None;
        }
        Some(args.len() as u64)
    }

    /// Returns the size of the expression in `s..e`.
    fn size(&self, s: usize, e: usize, depth: u32) -> Count {
        let (s, e) = self.trim(s, e);
        if s >= e || depth > 32 {
            return None;
        }
        let last = e - 1;
        if self.b[s] == b'[' && self.pair[s] == last {
            return self.literal_len(s);
        }
        if self.b[s..e].iter().all(|&c| is_ident(c)) {
            return self.consts.get(&self.b[s..e]).copied().flatten();
        }
        if self.b[last] != b')' || self.pair[last] == NONE || self.pair[last] < s {
            return None;
        }
        let open = self.pair[last];
        let name_end = self.skip_ws_back(open);
        let mut name_start = name_end;
        while name_start > s && is_ident(self.b[name_start - 1]) {
            name_start -= 1;
        }
        let name = &self.b[name_start..name_end];
        let dot = self.skip_ws_back(name_start);
        if dot > s && self.b[dot - 1] == b'.' {
            let mut recv_end = dot - 1;
            if recv_end > s && self.b[recv_end - 1] == b'?' {
                recv_end -= 1;
            }
            let recv = |this: &Self| this.size(s, recv_end, depth + 1);
            return match name {
                b"filter" | b"sort" | b"map" | b"reverse" => recv(self),
                b"slice" => {
                    let args = self.args(open);
                    let num = |k: usize| args.get(k).and_then(|&(a, b)| self.exact_number(a, b));
                    match args.len() {
                        0 => recv(self),
                        1 => Some(recv(self)?.saturating_sub(num(0)?)),
                        2 => Some(num(1)?.saturating_sub(num(0)?)),
                        _ => None,
                    }
                }
                b"from" if self.is_array(s, recv_end) => self.array_from_size(open),
                _ => None,
            };
        }
        None
    }

    fn is_array(&self, s: usize, e: usize) -> bool {
        &self.b[s..e] == b"Array"
    }

    fn exact_number(&self, s: usize, e: usize) -> Option<u64> {
        self.number(s).filter(|&(_, k)| k == e).map(|(n, _)| n)
    }

    /// Returns the size of the first argument of `Array.from(` at `open`:
    /// `{ length: N }` gives N.
    fn array_from_size(&self, open: usize) -> Count {
        let &(s, e) = self.args(open).first()?;
        if self.b[s] == b'{' && self.pair[s] == e - 1 {
            for (a, b) in self.args(s) {
                let Some((key, key_end)) = self.ident_after(a) else {
                    continue;
                };
                let colon = self.skip_ws(key_end);
                if key == b"length" && self.b.get(colon) == Some(&b':') {
                    let (v, ve) = self.trim(colon + 1, b);
                    return self.exact_number(v, ve);
                }
            }
            return None;
        }
        self.size(s, e, 1)
    }

    /// Returns the span of the receiver of a method whose `.` is at `dot`.
    fn receiver(&self, dot: usize) -> (usize, usize) {
        let mut end = self.skip_ws_back(dot);
        if end > 0 && self.b[end - 1] == b'?' {
            end -= 1;
        }
        let mut start = end;
        let mut p = end;
        loop {
            let mut q = self.skip_ws_back(p);
            let mut got = false;
            while q > 0 && matches!(self.b[q - 1], b')' | b']') && self.pair[q - 1] != NONE {
                q = self.pair[q - 1];
                got = true;
            }
            let ie = q;
            while q > 0 && is_ident(self.b[q - 1]) {
                q -= 1;
            }
            if q < ie {
                got = true;
            }
            if !got {
                return (start, end);
            }
            start = q;
            let mut d = self.skip_ws_back(q);
            if d > 0 && self.b[d - 1] == b'.' {
                d -= 1;
                if d > 0 && self.b[d - 1] == b'?' {
                    d -= 1;
                }
                p = d;
            } else {
                return (start, end);
            }
        }
    }

    // ----- loops and call sites (second pass) -----

    fn collect_loops_and_sites(&mut self) {
        let mut consumed: HashSet<usize> = HashSet::new();
        for k in 0..self.toks.len() {
            let (s, e) = self.toks[k];
            let word = &self.b[s..e];
            let after_dot = self.b[..self.skip_ws_back(s)].last() == Some(&b'.');
            if after_dot {
                if ITERATORS.contains(&word) {
                    self.iterator_method(s, e);
                }
                continue;
            }
            let plain = s == 0 || self.b[s - 1] != b'.';
            match word {
                b"agent" if plain && self.paren_after(e).is_some() => {
                    self.sites.push((s, Some(1)));
                }
                b"workflow" if plain && self.paren_after(e).is_some() => {
                    self.sites.push((s, None));
                }
                b"for" => self.for_loop(e),
                b"while" if !consumed.contains(&s) => {
                    if let Some(open) = self.paren_after(e) {
                        let size = self.cond_bound(open);
                        let (start, end) = self.body(self.pair[open] + 1, false);
                        self.loops.push(Loop { start, end, size });
                    }
                }
                b"do" => self.do_while(e, &mut consumed),
                b"pipeline" if plain => self.pipeline(e),
                b"Array" => self.array_from(e),
                _ => {}
            }
        }
    }

    /// `x.map(...)` and its relatives: the arguments repeat for each element.
    fn iterator_method(&mut self, s: usize, e: usize) {
        let Some(open) = self.paren_after(e) else {
            return;
        };
        let dot = self.skip_ws_back(s) - 1;
        let (start, end) = self.receiver(dot);
        let size = self.size(start, end, 0);
        self.loops.push(Loop {
            start: open,
            end: self.pair[open],
            size,
        });
    }

    /// `for (...) body`, `for await (...) body`.
    fn for_loop(&mut self, e: usize) {
        let mut p = e;
        if let Some((_, a)) = self.ident_after(p).filter(|(w, _)| *w == b"await") {
            p = a;
        }
        let Some(open) = self.paren_after(p) else {
            return;
        };
        let close = self.pair[open];
        let mut semis = Vec::new();
        let mut keyword = None;
        let mut i = open + 1;
        while i < close {
            match self.b[i] {
                b'(' | b'[' | b'{' => i = self.pair[i],
                b';' => semis.push(i),
                c if is_ident(c) && (i == 0 || !is_ident(self.b[i - 1])) => {
                    let w = &self.b[i..self.ident_end(i)];
                    if keyword.is_none() && (w == b"of" || w == b"in") {
                        keyword = Some(i + w.len());
                    }
                }
                _ => {}
            }
            i += 1;
        }
        let size = if semis.len() == 2 {
            self.simple_bound(semis[0] + 1, semis[1])
        } else if let Some(k) = keyword {
            self.size(k, close, 0)
        } else {
            None
        };
        let (start, end) = self.body(close + 1, false);
        self.loops.push(Loop { start, end, size });
    }

    /// `i < N` or `i <= N`, and nothing else.
    fn simple_bound(&self, s: usize, e: usize) -> Count {
        let (s, e) = self.trim(s, e);
        let ie = self.ident_end(s);
        let mut p = self.skip_ws(ie);
        if ie == s || self.b.get(p) != Some(&b'<') {
            return None;
        }
        p += 1;
        let eq = self.b.get(p) == Some(&b'=');
        let (n, k) = self.number(self.skip_ws(p + usize::from(eq)))?;
        (self.skip_ws(k) >= e).then_some(n.saturating_add(u64::from(eq)))
    }

    /// The count of a `while` condition: the first `< N` or `<= N` with a
    /// number.
    fn cond_bound(&self, open: usize) -> Count {
        let close = self.pair[open];
        for i in open + 1..close {
            if self.b[i] != b'<' || self.b[i - 1] == b'<' || self.b.get(i + 1) == Some(&b'<') {
                continue;
            }
            let eq = self.b.get(i + 1) == Some(&b'=');
            if let Some((n, _)) = self.number(self.skip_ws(i + 1 + usize::from(eq))) {
                return Some(n.saturating_add(u64::from(eq)));
            }
        }
        None
    }

    /// `do body while (cond)`; `e` is the end of `do`.
    fn do_while(&mut self, e: usize, consumed: &mut HashSet<usize>) {
        let (start, end) = self.body(e, false);
        // After a block, `end` is its `}`. After a statement, it is a `;`.
        let after = if self.b.get(end).is_some_and(|c| matches!(c, b'}' | b';')) {
            end + 1
        } else {
            end
        };
        let Some((w, a)) = self.ident_after(after) else {
            return;
        };
        let Some(open) = self.paren_after(a).filter(|_| w == b"while") else {
            return;
        };
        consumed.insert(a - 5);
        let size = self.cond_bound(open);
        self.loops.push(Loop { start, end, size });
    }

    /// `pipeline(X, s1, s2, ...)`: each stage repeats for each element of X.
    fn pipeline(&mut self, e: usize) {
        let Some(open) = self.paren_after(e) else {
            return;
        };
        let args = self.args(open);
        let Some(&(xs, xe)) = args.first() else {
            return;
        };
        let size = self.size(xs, xe, 0);
        for &(start, end) in &args[1..] {
            self.loops.push(Loop { start, end, size });
        }
    }

    /// `Array.from(X, fn)`: the function repeats for each element of X.
    fn array_from(&mut self, e: usize) {
        let dot = self.skip_ws(e);
        if self.b.get(dot) != Some(&b'.') {
            return;
        }
        let Some((_, a)) = self.ident_after(dot + 1).filter(|(w, _)| *w == b"from") else {
            return;
        };
        let Some(open) = self.paren_after(a) else {
            return;
        };
        let args = self.args(open);
        if args.len() >= 2 {
            self.loops.push(Loop {
                start: args[0].1,
                end: self.pair[open],
                size: self.array_from_size(open),
            });
        }
    }

    // ----- counts -----

    /// Returns the number of times that the code at `pos` runs, times `base`.
    fn count_at(&mut self, pos: usize, base: Count, visiting: &mut Vec<usize>) -> Count {
        let scope = self
            .funcs
            .iter()
            .enumerate()
            .filter(|(_, f)| f.start <= pos && pos < f.end)
            .max_by_key(|(_, f)| f.start)
            .map(|(i, _)| i);
        let lower = scope.map_or(0, |i| self.funcs[i].start);
        let mut n = base;
        for l in &self.loops {
            if l.start >= lower && l.start <= pos && pos < l.end {
                n = mul(n, l.size);
            }
        }
        match scope {
            Some(f) => mul(n, self.calls_of(f, visiting)),
            None => n,
        }
    }

    /// Returns how many times a function runs: one for each mention of its
    /// name outside its body, with the loops of the mention. A mention in its
    /// own body is a recursion, and it has no limit.
    fn calls_of(&mut self, f: usize, visiting: &mut Vec<usize>) -> Count {
        if let Some(done) = self.memo[f] {
            return done;
        }
        if visiting.contains(&f) {
            return None;
        }
        visiting.push(f);
        let (name, decl, start, end) = {
            let g = &self.funcs[f];
            (g.name, g.decl, g.start, g.end)
        };
        let mut total = Some(0);
        for k in 0..self.toks.len() {
            let (s, e) = self.toks[k];
            if &self.b[s..e] != name || s == decl {
                continue;
            }
            if self.b[..self.skip_ws_back(s)].last() == Some(&b'.') {
                continue;
            }
            if start <= s && s < end {
                total = None;
                break;
            }
            total = add(total, self.count_at(s, Some(1), visiting));
        }
        visiting.pop();
        self.memo[f] = Some(total);
        total
    }
}

// ---------------------------------------------------------------------------
// protect-tests
// ---------------------------------------------------------------------------

/// The flags of `protect-tests`. With no glob, the hook is off.
#[derive(Default)]
pub struct ProtectOpts {
    /// `--mode always`. Any other value, or no `--mode`, acts as
    /// `fix-branches`.
    pub always: bool,
    /// One `--test <glob>` for each glob of `[evidence] tests`.
    pub tests: Vec<String>,
}

impl ProtectOpts {
    fn parse(args: &[String]) -> Self {
        ProtectOpts {
            always: flag_value(args, "--mode") == Some("always"),
            tests: flag_values(args, "--test")
                .into_iter()
                .map(str::to_string)
                .collect(),
        }
    }
}

/// The answer of one `git` call: true for exit code 0, and its stdout.
pub struct GitOut {
    pub ok: bool,
    pub stdout: String,
}

/// Runs `git -C <root> <args>`. The call takes no lock, reads no input and
/// shows no error text. It returns `None` if `git` does not start.
fn run_git(root: &Path, args: &[&str]) -> Option<GitOut> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    Some(GitOut {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
    })
}

/// The part of `file` below `root`, if both use `/` and `file` is inside.
fn below<'a>(root: &str, file: &'a str) -> Option<&'a str> {
    let rest = file
        .strip_prefix(root.trim_end_matches('/'))?
        .strip_prefix('/')?;
    (!rest.is_empty()).then_some(rest)
}

/// Gives each `\` as `/`.
fn slashes(p: &str) -> String {
    p.replace('\\', "/")
}

/// True for a path that starts with `/` or with a drive letter and `:`.
fn is_absolute(p: &str) -> bool {
    p.starts_with('/')
        || (p.as_bytes().get(1) == Some(&b':') && p.as_bytes()[0].is_ascii_alphabetic())
}

/// Returns the root and the path of `file` relative to it. If `file` is not
/// below `root`, the function resolves links in `root` and in the folder of
/// `file`, and tries again (`/var` is a link to `/private/var` on macOS). Both
/// paths must be absolute. The relative path has no `..`, `.` or empty segment.
fn relative_to(root: &str, file: &str) -> Option<(String, String)> {
    let (root, file) = (slashes(root), slashes(file));
    if !is_absolute(&root) || !is_absolute(&file) {
        return None;
    }
    let (root, rel) = match below(&root, &file) {
        Some(rel) => (root.clone(), rel.to_string()),
        None => {
            let real_root = slashes(&std::fs::canonicalize(&root).ok()?.to_string_lossy());
            let path = Path::new(&file);
            let parent = std::fs::canonicalize(path.parent()?).ok()?;
            let real_file = slashes(&parent.join(path.file_name()?).to_string_lossy());
            let rel = below(&real_root, &real_file)?.to_string();
            (real_root, rel)
        }
    };
    rel.split('/')
        .all(|s| !s.is_empty() && s != "." && s != "..")
        .then_some((root, rel))
}

/// Handles one `PreToolUse` event of `Edit` or `Write`. It returns the JSON to
/// print when the file is a test file that git tracks, and (unless the mode is
/// `always`) the branch starts with `fix/`. `project` is `CLAUDE_PROJECT_DIR`.
/// `git` runs `git -C <root> <args>`. It runs 0 times for a path that matches
/// no glob, and at most 2 times.
pub fn protect(
    input: &str,
    opts: &ProtectOpts,
    project: Option<&Path>,
    git: &mut dyn FnMut(&Path, &[&str]) -> Option<GitOut>,
) -> Option<String> {
    if opts.tests.is_empty() {
        return None;
    }
    let v = json::parse(input).ok()?;
    if !matches!(v.get("tool_name").as_str(), Some("Edit" | "Write")) {
        return None;
    }
    let file = v.get("tool_input").get("file_path").as_str()?;
    let root = match project.and_then(Path::to_str).filter(|p| !p.is_empty()) {
        Some(p) => p,
        None => v.get("cwd").as_str().filter(|c| !c.is_empty())?,
    };
    let (root, rel) = relative_to(root, file)?;
    if !opts.tests.iter().any(|g| crate::glob::matches(g, &rel)) {
        return None;
    }
    let root = PathBuf::from(root);
    let mut branch = String::new();
    if !opts.always {
        let out = git(&root, &["symbolic-ref", "--short", "-q", "HEAD"])?;
        branch = out.stdout.trim_end_matches(['\n', '\r']).to_string();
        if !out.ok || !branch.starts_with("fix/") {
            return None;
        }
    }
    let spec = format!(":(literal){rel}");
    if !git(&root, &["ls-files", "--error-unmatch", "--", &spec])?.ok {
        return None;
    }
    let reason = if opts.always {
        format!(
            "spec-dna protect-tests: {rel} is a test file that git tracks. Approve the edit only if the test is wrong."
        )
    } else {
        format!(
            "spec-dna protect-tests: {rel} is a test file that git tracks, and {branch} is a fix branch. A fix changes the code, not the test. Approve the edit only if the test is wrong."
        )
    };
    let inner = Obj::new()
        .str("hookEventName", "PreToolUse")
        .str("permissionDecision", "ask")
        .str("permissionDecisionReason", &reason)
        .end();
    Some(Obj::new().raw("hookSpecificOutput", &inner).end())
}

// ---------------------------------------------------------------------------
// protect-approvals
// ---------------------------------------------------------------------------

/// The flags of `protect-approvals`: one `--intent <dir>` for each folder of
/// the `intent` layer, from the repository root. With no folder, the hook
/// still denies `spec-dna approve`, and it reads no commit.
#[derive(Default)]
pub struct ApprovalsOpts {
    pub intent: Vec<String>,
}

impl ApprovalsOpts {
    fn parse(args: &[String]) -> Self {
        ApprovalsOpts {
            intent: flag_values(args, "--intent")
                .into_iter()
                .map(|d| d.trim_matches('/').to_string())
                .filter(|d| !d.is_empty())
                .collect(),
        }
    }
}

/// Splits a command into segments and words. It removes the quote characters,
/// and it splits at `;`, `&`, `|`, `(`, `)`, a backtick and a line feed (so
/// `&&` and `||` split too), then at white space. It is a text scan, not a
/// shell parser: `sh -c "a; b"` gives the words of `a` and `b`.
fn command_segments(command: &str) -> Vec<Vec<String>> {
    let text: String = command
        .chars()
        .filter(|c| !matches!(c, '"' | '\''))
        .collect();
    text.split([';', '&', '|', '(', ')', '`', '\n'])
        .map(|seg| {
            seg.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .filter(|w| !w.is_empty())
        .collect()
}

/// The last part of a path word, without `.exe`: `/usr/bin/git` gives `git`.
fn program(word: &str) -> &str {
    let last = word.rsplit(['/', '\\']).next().unwrap_or(word);
    last.strip_suffix(".exe").unwrap_or(last)
}

/// The index of the first word `name` (as a program) in a segment that has the
/// word `after` later.
fn program_then(words: &[String], name: &str, after: &str) -> Option<usize> {
    let i = words.iter().position(|w| program(w) == name)?;
    words[i + 1..].iter().any(|w| w == after).then_some(i)
}

fn deny(reason: &str) -> String {
    let inner = Obj::new()
        .str("hookEventName", "PreToolUse")
        .str("permissionDecision", "deny")
        .str(
            "permissionDecisionReason",
            &format!("spec-dna protect-approvals: {reason}"),
        )
        .end();
    Obj::new().raw("hookSpecificOutput", &inner).end()
}

const ASK_PERSON: &str = "Only a person approves: ask the person to run `spec-dna approve <slug> <artifact>` in a terminal of their own.";

/// The approval value of an artifact file name, or `None` for another name.
fn approval_of_file(file: &str) -> Option<(&'static str, &'static str)> {
    match file {
        "intent.md" => Some(("intent", "accepted")),
        "spec.md" => Some(("spec", "approved")),
        "plan.md" => Some(("plan", "approved")),
        _ => None,
    }
}

/// The value of the `status` line of the front matter of a text.
fn status_of(text: &str) -> Option<String> {
    let mut lines = text.lines().map(|l| l.trim_end_matches('\r'));
    if lines.next()?.trim_end() != "---" {
        return None;
    }
    for line in lines {
        if line.trim_end() == "---" {
            return None;
        }
        let Some(rest) = line.strip_prefix("status:") else {
            continue;
        };
        let value = rest.split(" #").next().unwrap_or(rest).trim();
        return Some(value.trim_matches(['"', '\'']).to_string());
    }
    None
}

/// Handles one `PreToolUse` event of `Bash`. It returns the JSON of a deny
/// for a call that runs `spec-dna approve`, and for a `git commit` that moves
/// the status of an artifact of a change to its approval value. Another call
/// gives `None`, also when the input is bad or `git` fails. `project` is
/// `CLAUDE_PROJECT_DIR`. `git` runs `git -C <dir> <args>`. It runs 0 times for
/// a command with no word `commit` after a word `git`, and then 3 times, and 2
/// more times for each artifact file that the working tree changes.
pub fn approvals(
    input: &str,
    opts: &ApprovalsOpts,
    project: Option<&Path>,
    git: &mut dyn FnMut(&Path, &[&str]) -> Option<GitOut>,
) -> Option<String> {
    let v = json::parse(input).ok()?;
    if v.get("tool_name").as_str() != Some("Bash") {
        return None;
    }
    let command = v.get("tool_input").get("command").as_str()?;
    let segments = command_segments(command);
    // Rule A: `spec-dna approve` is for a person.
    if segments
        .iter()
        .any(|w| program_then(w, "spec-dna", "approve").is_some())
    {
        return Some(deny(
            "`spec-dna approve` is for a person. Ask the person to run it in a terminal of their own.",
        ));
    }
    if opts.intent.is_empty() {
        return None;
    }
    // Rule C: an edit and a commit in one call. The status is not in the tree
    // yet, so the state cannot show it.
    let mut before: Vec<&String> = Vec::new();
    for words in &segments {
        if let Some(i) = program_then(words, "git", "commit")
            && before.iter().copied().chain(&words[..i]).any(|w| {
                let w = w.to_ascii_lowercase();
                w.contains("accepted") || w.contains("approved")
            })
        {
            return Some(deny(&format!(
                "this call writes `accepted` or `approved` and runs `git commit`; run the edit and the commit as 2 calls, so the hook can read the edit. {ASK_PERSON}"
            )));
        }
        before.extend(words);
    }
    // Rule B: the state of the repository.
    let start = match v.get("cwd").as_str().filter(|c| !c.is_empty()) {
        Some(c) => c.to_string(),
        None => project
            .and_then(Path::to_str)
            .filter(|p| !p.is_empty())?
            .to_string(),
    };
    let start = PathBuf::from(start);
    // The folder that the commit runs in. `None`: a `cd` or a `-C` has a target
    // that the hook cannot read, so the hook fails closed (see `fallbacks`).
    let mut dir = Some(start.clone());
    let mut ask = |here: &Option<PathBuf>, amend: bool| -> Option<String> {
        let dirs = match here {
            Some(d) => vec![d.clone()],
            None => fallbacks(&start, project),
        };
        dirs.iter()
            .find_map(|d| moved_to_approval(d, amend, &opts.intent, git))
    };
    for words in &segments {
        if program(&words[0]) == "cd" {
            if let Some(target) = words.get(1) {
                dir = dir
                    .as_deref()
                    .and_then(|d| moved(d, &start, project, target));
            }
            continue;
        }
        let Some(g) = words.iter().position(|w| program(w) == "git") else {
            continue;
        };
        let Some(c) = words[g + 1..].iter().position(|w| w == "commit") else {
            continue;
        };
        // `git -C <dir>` moves the folder, in order.
        let mut here = dir.clone();
        let mut i = g + 1;
        while i < g + 1 + c {
            if words[i] == "-C" {
                here = match (here.as_deref(), words.get(i + 1)) {
                    (Some(d), Some(t)) => moved(d, &start, project, t),
                    _ => None,
                };
                i += 1;
            }
            i += 1;
        }
        let amend = words[g + 1 + c..].iter().any(|w| w == "--amend");
        if let Some(reason) = ask(&here, amend) {
            return Some(deny(&reason));
        }
    }
    None
}

/// The repositories to inspect when a folder cannot be read: the `cwd` of the
/// event and `CLAUDE_PROJECT_DIR`. The hook allows the call only if none of
/// them has an approval that this commit would record.
fn fallbacks(start: &Path, project: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = vec![start.to_path_buf()];
    if let Some(p) = project.filter(|p| !p.as_os_str().is_empty() && *p != start) {
        dirs.push(p.to_path_buf());
    }
    dirs
}

/// The folder that `cd <target>` or `git -C <target>` gives. A target that
/// starts with `~`, `$PWD`, `${PWD}`, `$CLAUDE_PROJECT_DIR` or
/// `${CLAUDE_PROJECT_DIR}` is expanded (`~` is `$HOME`; `$PWD` is the folder
/// before the move). Another `$` or `~` gives `None`: the hook cannot read it.
fn moved(from: &Path, start: &Path, project: Option<&Path>, target: &str) -> Option<PathBuf> {
    if !target.contains(['$', '~']) {
        return Some(from.join(target));
    }
    let (base, rest): (PathBuf, &str) = if let Some(r) = target.strip_prefix('~') {
        (std::env::var_os("HOME").map(PathBuf::from)?, r)
    } else {
        let var = target.strip_prefix('$')?;
        let (name, rest) = match var.strip_prefix('{') {
            Some(b) => b.split_once('}')?,
            None => {
                let end = var
                    .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                    .unwrap_or(var.len());
                var.split_at(end)
            }
        };
        let base = match name {
            "PWD" => from.to_path_buf(),
            "CLAUDE_PROJECT_DIR" => project
                .map(Path::to_path_buf)
                .unwrap_or_else(|| start.to_path_buf()),
            _ => return None,
        };
        (base, rest)
    };
    if rest.contains(['$', '~']) || !(rest.is_empty() || rest.starts_with('/')) {
        return None;
    }
    Some(base.join(rest.trim_start_matches('/')))
}

/// The reason to deny a commit in `dir`, if the commit sets the status of an
/// artifact file to its approval value. The old tree is `HEAD`, or `HEAD~1`
/// for an amend. A tree that is not there gives the empty tree.
fn moved_to_approval(
    dir: &Path,
    amend: bool,
    intent: &[String],
    git: &mut dyn FnMut(&Path, &[&str]) -> Option<GitOut>,
) -> Option<String> {
    let top = git(dir, &["rev-parse", "--show-toplevel"])?;
    let top = top.stdout.trim_end_matches(['\n', '\r']).to_string();
    if !top_ok(&top) {
        return None;
    }
    let root = PathBuf::from(&top);
    let specs: Vec<String> = intent
        .iter()
        .map(|d| format!(":(top,literal){d}"))
        .collect();
    let old = if amend { "HEAD~1" } else { "HEAD" };
    let mut args = vec!["diff", "--name-only", "-z", old, "--"];
    args.extend(specs.iter().map(String::as_str));
    let mut paths: Vec<String> = Vec::new();
    let mut old_tree = Some(old);
    match git(&root, &args) {
        Some(out) if out.ok => paths.extend(split_z(&out.stdout)),
        _ => {
            // No such commit: the files of the index have no old status.
            old_tree = None;
            let mut args = vec!["ls-files", "-z", "--cached", "--full-name", "--"];
            args.extend(specs.iter().map(String::as_str));
            paths.extend(split_z(&git(&root, &args)?.stdout));
        }
    }
    let mut args = vec![
        "ls-files",
        "-z",
        "--others",
        "--exclude-standard",
        "--full-name",
        "--",
    ];
    args.extend(specs.iter().map(String::as_str));
    paths.extend(split_z(&git(&root, &args)?.stdout));
    paths.sort();
    paths.dedup();
    for path in paths {
        let Some((slug, artifact, approved)) = artifact_of(&path, intent) else {
            continue;
        };
        let old_status = match old_tree {
            Some(rev) => git(&root, &["show", &format!("{rev}:{path}")])
                .filter(|o| o.ok)
                .and_then(|o| status_of(&o.stdout)),
            None => None,
        };
        if old_status.as_deref() == Some(approved) {
            continue;
        }
        let index = git(&root, &["show", &format!(":{path}")])
            .filter(|o| o.ok)
            .and_then(|o| status_of(&o.stdout));
        let tree = read_limited(&root.join(&path)).and_then(|t| status_of(&t));
        if index.as_deref() == Some(approved) || tree.as_deref() == Some(approved) {
            return Some(format!(
                "{path} has the status `{approved}` in this commit. Only a person approves. Set the status back, and ask the person to run `spec-dna approve {slug} {artifact}` in a terminal of their own."
            ));
        }
    }
    None
}

/// True if the output of `rev-parse --show-toplevel` is a path.
fn top_ok(top: &str) -> bool {
    !top.is_empty() && is_absolute(top)
}

fn split_z(out: &str) -> impl Iterator<Item = String> + '_ {
    out.split('\0')
        .filter(|p| !p.is_empty())
        .map(str::to_string)
}

/// The slug, the artifact name and the approval value of a path
/// `<dir>/<slug>/<artifact>.md`, if `dir` is one of the folders.
fn artifact_of(path: &str, intent: &[String]) -> Option<(String, &'static str, &'static str)> {
    intent.iter().find_map(|d| {
        let rest = path.strip_prefix(d.as_str())?.strip_prefix('/')?;
        let (slug, file) = rest.split_once('/')?;
        let (artifact, approved) = approval_of_file(file)?;
        (!slug.is_empty()).then(|| (slug.to_string(), artifact, approved))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static COUNTER: AtomicUsize = AtomicUsize::new(0);

    /// A unique temporary directory, removed when the value is dropped.
    struct Tmp(PathBuf);

    impl Tmp {
        fn new() -> Self {
            let n = COUNTER.fetch_add(1, Ordering::SeqCst);
            let dir =
                std::env::temp_dir().join(format!("spec-dna-hook-{}-{n}", std::process::id()));
            fs::create_dir_all(&dir).unwrap();
            Tmp(dir)
        }

        fn state(&self) -> PathBuf {
            self.0.join("state")
        }
    }

    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    // ----- fan-out bound -----

    fn total(script: &str) -> Count {
        bound(script).expect("balanced script").total
    }

    #[test]
    fn plain_calls_add_up() {
        assert_eq!(total("agent('a'); agent('b'); await agent ('c')"), Some(3));
        assert_eq!(total("const x = 1;"), Some(0));
    }

    #[test]
    fn call_site_needs_a_clean_start() {
        assert_eq!(
            total("myagent(1); agent_2(1); $agent(1); a.agent(1)"),
            Some(0)
        );
        assert_eq!(total("x = agent\n(1)"), Some(1));
    }

    #[test]
    fn comments_and_strings_are_ignored() {
        let script = "// agent(1)\n/* agent(2)\nagent(3) */\nconst s = 'agent(4)' + \"agent(5)\" + `agent(6)`;\nagent(7);";
        assert_eq!(total(script), Some(1));
        assert_eq!(total(r#"const s = 'it\'s agent(1)'; agent(2)"#), Some(1));
    }

    #[test]
    fn template_code_is_scanned() {
        assert_eq!(
            total("const s = `text agent(1) ${ await agent(2) } more`;"),
            Some(1)
        );
        assert_eq!(total("const s = `${ `${ agent(1) }` }`;"), Some(1));
    }

    #[test]
    fn iterator_methods_use_the_receiver_size() {
        for m in [
            "reduce", "filter", "map", "every", "flatMap", "some", "forEach", "find",
        ] {
            let script = format!("[1, 2, 3].{m}(x => agent(x))");
            assert_eq!(total(&script), Some(3), "{m}");
        }
    }

    #[test]
    fn receiver_forms() {
        let t = |r: &str| total(&format!("{r}.map(x => agent(x))"));
        assert_eq!(t("[]"), Some(0));
        assert_eq!(t("[1, 2,]"), Some(2));
        assert_eq!(t("[[1, 2], [3]]"), Some(2));
        assert_eq!(t("[...a, 1]"), None);
        assert_eq!(t("items"), None);
        assert_eq!(t("[1,2,3,4,5].slice(1, 3)"), Some(2));
        assert_eq!(t("[1,2,3,4,5].slice(2)"), Some(3));
        assert_eq!(t("items.slice(0, 4)"), Some(4));
        assert_eq!(t("items.slice(2)"), None);
        assert_eq!(t("items.slice(0, n)"), None);
        assert_eq!(t("[1,2,3].filter(f).sort().reverse().map(g)"), Some(3));
        assert_eq!(t("Array.from({ length: 6 })"), Some(6));
        assert_eq!(t("Array.from({ length: n })"), None);
        assert_eq!(t("Array.from({ length: 6 }).slice(0, 2)"), Some(2));
        assert_eq!(t("getItems().concat(b)"), None);
        assert_eq!(t("a?.slice(0, 2)"), Some(2));
    }

    #[test]
    fn receiver_over_several_lines() {
        let script = "const r = [1, 2, 3]\n  .filter(Boolean)\n  .map(x => agent(x));";
        assert_eq!(total(script), Some(3));
    }

    #[test]
    fn variable_with_a_literal() {
        assert_eq!(
            total("const xs = [1, 2, 3]; xs.map(x => agent(x));"),
            Some(3)
        );
        assert_eq!(total("let xs = [1, 2]\nxs.forEach(x => agent(x))"), Some(2));
        assert_eq!(total("var xs = [1]; xs.map(x => agent(x));"), Some(1));
        assert_eq!(total("const xs = load(); xs.map(x => agent(x));"), None);
        assert_eq!(
            total("const xs = [1, 2].concat(b); xs.map(x => agent(x));"),
            None
        );
        assert_eq!(
            total("const xs = [1]; { const xs = [1, 2, 3]; } xs.map(x => agent(x));"),
            Some(3)
        );
    }

    #[test]
    fn for_of_and_for_in() {
        assert_eq!(total("for (const x of [1, 2, 3]) { agent(x); }"), Some(3));
        assert_eq!(total("for (const x of [1, 2, 3]) agent(x);"), Some(3));
        assert_eq!(
            total("for (const [a, b] of [[1, 2], [3, 4]]) { agent(a); }"),
            Some(2)
        );
        assert_eq!(
            total("const o = [1, 2]; for (const k in o) { agent(k); }"),
            Some(2)
        );
        assert_eq!(total("for (const x of items) { agent(x); }"), None);
        assert_eq!(
            total("for await (const x of [1, 2]) { agent(x); }"),
            Some(2)
        );
    }

    #[test]
    fn classic_for() {
        assert_eq!(total("for (let i = 0; i < 4; i++) { agent(i); }"), Some(4));
        assert_eq!(total("for (let i = 0; i <= 4; i++) { agent(i); }"), Some(5));
        assert_eq!(total("for (let i = 0; i < n; i++) { agent(i); }"), None);
        assert_eq!(
            total("for (let i = 0; i < xs.length; i++) { agent(i); }"),
            None
        );
        assert_eq!(total("for (;;) { agent(1); }"), None);
        assert_eq!(
            total("for (let i = 0; i < 3; i++) agent(i);\nagent(9);"),
            Some(4)
        );
    }

    #[test]
    fn while_and_do_while() {
        assert_eq!(total("while (more()) { agent(1); }"), None);
        assert_eq!(total("while (i < 5) { agent(1); i++; }"), Some(5));
        assert_eq!(total("while (more() && i <= 5) { agent(1); }"), Some(6));
        assert_eq!(total("do { agent(1); } while (more());"), None);
        assert_eq!(total("do { agent(1); } while (i < 3);"), Some(3));
        assert_eq!(total("do { agent(1); } while (i < 3)\nagent(2);"), Some(4));
    }

    #[test]
    fn pipeline_stages() {
        assert_eq!(
            total("pipeline([1, 2, 3], x => agent(x), x => agent(x))"),
            Some(6)
        );
        assert_eq!(total("pipeline(items, x => agent(x))"), None);
        assert_eq!(total("pipeline([1, 2], x => x); agent(1)"), Some(1));
    }

    #[test]
    fn array_from_with_a_function() {
        assert_eq!(total("Array.from({ length: 3 }, () => agent(1))"), Some(3));
        assert_eq!(total("Array.from(items, x => agent(x))"), None);
    }

    #[test]
    fn loops_multiply() {
        let script = "for (const a of [1, 2]) { [1, 2, 3].map(b => agent(a, b)); }";
        assert_eq!(total(script), Some(6));
        assert_eq!(
            total("[1, 2].map(a => agent(a)); [1, 2, 3].map(a => agent(a))"),
            Some(5)
        );
        assert_eq!(total("[].map(x => agent(x))"), Some(0));
    }

    #[test]
    fn helper_function_counts_once_per_call() {
        let script = "async function run(x) { return agent(x); }\nawait run(1); await run(2);\n[1, 2, 3].map(x => run(x));";
        assert_eq!(total(script), Some(5));
        let arrow = "const run = async (x) => { await agent(x); };\nrun(1);\nrun(2);";
        assert_eq!(total(arrow), Some(2));
        let expr = "const run = x => agent(x);\nrun(1);\n[1, 2].forEach(run);";
        assert_eq!(total(expr), Some(3));
        let expr_decl = "const run = function (x) { return agent(x); };\nrun(1);";
        assert_eq!(total(expr_decl), Some(1));
    }

    #[test]
    fn loops_inside_a_helper_count() {
        let script = "function run() { [1, 2].map(x => agent(x)); }\nrun(); run();";
        assert_eq!(total(script), Some(4));
    }

    #[test]
    fn helper_without_a_call_counts_zero() {
        assert_eq!(total("function run() { agent(1); }\nagent(2);"), Some(1));
    }

    #[test]
    fn helper_called_by_a_helper() {
        let script =
            "function one() { agent(1); }\nfunction two() { one(); one(); }\ntwo(); two();";
        assert_eq!(total(script), Some(4));
    }

    #[test]
    fn recursion_has_no_limit() {
        let script = "function walk(n) {\n  agent(n);\n  walk(n - 1);\n}\nwalk(3);";
        assert_eq!(
            bound(script),
            Some(Bound {
                total: None,
                line: 2
            })
        );
        let mutual = "function a() { agent(1); b(); }\nfunction b() { a(); }\na();";
        assert_eq!(total(mutual), None);
    }

    #[test]
    fn nested_workflow_has_no_limit() {
        let b = bound("agent(1);\nawait workflow('other');").unwrap();
        assert_eq!(
            b,
            Bound {
                total: None,
                line: 2
            }
        );
        assert_eq!(total("// workflow(1)\nconst s = 'workflow(1)';"), Some(0));
    }

    #[test]
    fn unbalanced_brackets_are_allowed() {
        assert!(bound("agent(1").is_none());
        assert!(bound("agent(1)) }").is_none());
        assert!(bound("[1, 2).map(x => agent(x))").is_none());
        assert!(bound("const s = `${ agent(1) ").is_none());
        assert!(bound("agent(1); // (").is_some());
    }

    #[test]
    fn line_of_the_first_unbounded_site() {
        let script = "agent(1);\n\n// note\nfor (const x of items) {\n  agent(x);\n}";
        assert_eq!(
            bound(script),
            Some(Bound {
                total: None,
                line: 5
            })
        );
    }

    fn fan(script: &str, cap: Option<u64>, env: Option<&str>) -> Option<String> {
        let input = Obj::new()
            .raw("tool_input", &Obj::new().str("script", script).end())
            .end();
        fanout(&input, cap, env, None)
    }

    #[test]
    fn fanout_reasons() {
        let over = "[1, 2, 3].map(x => agent(x))";
        assert_eq!(fan(over, Some(3), None), None);
        let out = fan(over, Some(2), None).unwrap();
        let v = json::parse(&out).unwrap();
        let h = v.get("hookSpecificOutput");
        assert_eq!(h.get("hookEventName").as_str(), Some("PreToolUse"));
        assert_eq!(h.get("permissionDecision").as_str(), Some("ask"));
        assert_eq!(
            h.get("permissionDecisionReason").as_str(),
            Some(
                "spec-dna fan-out cap: this workflow can start 3 agents, more than the cap of 2. Approve it only if you need the extra agents. To change the cap, start Claude Code with SPEC_DNA_FANOUT_CAP=<n>."
            )
        );
        let out = fan("items.map(x => agent(x))", None, None).unwrap();
        let v = json::parse(&out).unwrap();
        assert_eq!(
            v.get("hookSpecificOutput")
                .get("permissionDecisionReason")
                .as_str(),
            Some(
                "spec-dna fan-out cap: line 1 can start agents without a limit: a loop over a list that the run makes, a loop without a number limit, a recursive call or a nested workflow. Limit the loop, for example with .slice(0, 10). Or approve this run."
            )
        );
    }

    #[test]
    fn fanout_cap_sources() {
        let script = "[1, 2, 3, 4, 5].map(x => agent(x))";
        assert_eq!(fan(script, None, None), None);
        assert!(fan(script, Some(4), None).is_some());
        // The environment wins over the flag.
        assert_eq!(fan(script, Some(4), Some("5")), None);
        assert!(fan(script, Some(9), Some("4")).is_some());
        // A bad environment value is ignored.
        assert!(fan(script, Some(4), Some("many")).is_some());
        // 0 turns the hook off, also for an unbounded script.
        assert_eq!(fan("items.map(x => agent(x))", Some(0), None), None);
        assert_eq!(fan("items.map(x => agent(x))", Some(4), Some("0")), None);
    }

    #[test]
    fn fanout_script_sources() {
        let tmp = Tmp::new();
        let wf = tmp.0.join(".claude").join("workflows");
        fs::create_dir_all(&wf).unwrap();
        fs::write(wf.join("wide.mjs"), "items.map(x => agent(x))").unwrap();
        fs::write(wf.join("narrow.js"), "agent(1)").unwrap();
        fs::write(tmp.0.join("own.js"), "items.map(x => agent(x))").unwrap();
        let mut cwd = String::new();
        json::string(&mut cwd, tmp.0.to_str().unwrap());
        let call = |tool_input: &str, project: Option<&Path>| {
            let input = format!(r#"{{"cwd":{cwd},"tool_input":{tool_input}}}"#);
            fanout(&input, None, None, project)
        };
        assert!(call(r#"{"name":"wide"}"#, Some(&tmp.0)).is_some());
        assert!(call(r#"{"name":"wide"}"#, None).is_some());
        assert_eq!(call(r#"{"name":"narrow"}"#, Some(&tmp.0)), None);
        assert_eq!(call(r#"{"name":"absent"}"#, Some(&tmp.0)), None);
        assert_eq!(call(r#"{"name":"../wide"}"#, Some(&tmp.0)), None);
        assert!(call(r#"{"scriptPath":"own.js"}"#, None).is_some());
        assert_eq!(call(r#"{"scriptPath":"absent.js"}"#, None), None);
        assert_eq!(call(r#"{}"#, None), None);
    }

    #[test]
    fn fanout_bad_input_is_silent() {
        assert_eq!(fanout("not json", None, None, None), None);
        assert_eq!(fanout("", None, None, None), None);
        assert_eq!(fanout("[]", None, None, None), None);
        assert_eq!(fan("items.map(x => agent(x)", None, None), None);
    }

    // ----- context meter -----

    fn assistant(input: u64, created: u64, read: u64) -> String {
        format!(
            r#"{{"type":"assistant","message":{{"role":"assistant","content":[{{"type":"text","text":"x"}}],"usage":{{"input_tokens":{input},"cache_creation_input_tokens":{created},"cache_read_input_tokens":{read},"output_tokens":5}}}}}}"#
        )
    }

    fn write_lines(path: &Path, lines: &[String]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, lines.join("\n") + "\n").unwrap();
    }

    fn event(transcript: &Path, session: &str, agent: Option<&str>) -> String {
        let mut o = Obj::new()
            .str("session_id", session)
            .str("transcript_path", transcript.to_str().unwrap());
        if let Some(a) = agent {
            o = o.str("agent_id", a);
        }
        o.end()
    }

    fn opts(levels: &[u64], subagent: u64) -> MeterOpts {
        MeterOpts {
            levels: levels.to_vec(),
            subagent,
        }
    }

    fn message(out: &str) -> String {
        let v = json::parse(out).unwrap();
        let h = v.get("hookSpecificOutput");
        assert_eq!(h.get("hookEventName").as_str(), Some("PostToolUse"));
        h.get("additionalContext").as_str().unwrap().to_string()
    }

    #[test]
    fn meter_main_levels() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        let o = opts(&[300_000, 600_000], 0);
        let ev = event(&t, "s1", None);
        write_lines(&t, &[assistant(10, 20_000, 100_000)]);
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
        write_lines(&t, &[assistant(10, 20_000, 300_000)]);
        let first = message(&meter(&ev, &o, &tmp.state()).unwrap());
        assert_eq!(
            first,
            "spec-dna context meter: this session has about 320K tokens of context, and each request sends all of them again. Delegate long searches and long reads to a subagent. Ask it for a short summary."
        );
        // The same crossing is silent.
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
        write_lines(&t, &[assistant(10, 20_000, 700_000)]);
        let higher = message(&meter(&ev, &o, &tmp.state()).unwrap());
        assert_eq!(
            higher,
            "spec-dna context meter: this session has about 720K tokens of context. Save the state of the task in a file: the goal, the work done, the next steps and the open questions. Then tell the user that a new session can continue from that file."
        );
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
    }

    #[test]
    fn meter_jump_over_a_level_speaks_the_highest() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        write_lines(&t, &[assistant(0, 0, 650_000)]);
        let out = meter(
            &event(&t, "s1", None),
            &opts(&[300_000, 600_000], 0),
            &tmp.state(),
        );
        assert!(message(&out.unwrap()).contains("Save the state of the task"));
    }

    #[test]
    fn meter_subagent_path_and_message() {
        let tmp = Tmp::new();
        let parent = tmp.0.join("proj").join("s1.jsonl");
        let sub = tmp
            .0
            .join("proj")
            .join("s1")
            .join("subagents")
            .join("agent-a_7.jsonl");
        // The parent transcript is huge, but it is not read.
        write_lines(&parent, &[assistant(0, 0, 900_000)]);
        write_lines(&sub, &[assistant(1, 0, 260_000)]);
        let ev = event(&parent, "s1", Some("a_7"));
        let o = opts(&[300_000], 250_000);
        assert_eq!(
            message(&meter(&ev, &o, &tmp.state()).unwrap()),
            "spec-dna context meter: this subagent has about 260K tokens of context. Complete the current step. Then return your findings and the open work to the caller."
        );
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
        // 0 turns the subagent meter off.
        assert_eq!(meter(&ev, &opts(&[300_000], 0), &tmp.state()), None);
        // The main session uses its own key.
        let main = event(&parent, "s1", None);
        assert!(meter(&main, &o, &tmp.state()).is_some());
    }

    #[test]
    fn meter_main_off_without_levels() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        write_lines(&t, &[assistant(0, 0, 900_000)]);
        assert_eq!(
            meter(&event(&t, "s1", None), &opts(&[], 250_000), &tmp.state()),
            None
        );
    }

    #[test]
    fn meter_two_calls_one_message() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        write_lines(&t, &[assistant(0, 0, 310_000)]);
        let ev = event(&t, "s1", None);
        let o = opts(&[300_000], 0);
        let a = meter(&ev, &o, &tmp.state());
        let b = meter(&ev, &o, &tmp.state());
        assert_eq!([a.is_some(), b.is_some()].iter().filter(|&&x| x).count(), 1);
    }

    #[test]
    fn meter_speaks_again_after_a_drop() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        let ev = event(&t, "s1", None);
        let o = opts(&[300_000], 0);
        write_lines(&t, &[assistant(0, 0, 310_000)]);
        assert!(meter(&ev, &o, &tmp.state()).is_some());
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
        // 190K is under 2/3 of 300K: the state resets.
        write_lines(&t, &[assistant(0, 0, 190_000)]);
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
        write_lines(&t, &[assistant(0, 0, 310_000)]);
        assert!(meter(&ev, &o, &tmp.state()).is_some());
        // A drop that stays above 2/3 does not reset.
        write_lines(&t, &[assistant(0, 0, 250_000)]);
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
        write_lines(&t, &[assistant(0, 0, 310_000)]);
        assert_eq!(meter(&ev, &o, &tmp.state()), None);
    }

    #[test]
    fn meter_reads_the_last_assistant_line() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        let lines = [
            assistant(0, 0, 500_000),
            assistant(0, 0, 320_000),
            r#"{"type":"user","message":{"role":"user","content":"hi"}}"#.to_string(),
            r#"{"type":"assistant","message":{"role":"assistant","content":[]}}"#.to_string(),
            r#"{"type":"system","subtype":"x"}"#.to_string(),
        ];
        write_lines(&t, &lines);
        let out = meter(&event(&t, "s1", None), &opts(&[300_000], 0), &tmp.state());
        assert!(message(&out.unwrap()).contains("about 320K"));
    }

    #[test]
    fn meter_skips_a_partial_last_line() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        let mut text = assistant(0, 0, 330_000);
        text.push('\n');
        text.push_str(&assistant(0, 0, 1)[..60]);
        fs::write(&t, text).unwrap();
        let out = meter(&event(&t, "s1", None), &opts(&[300_000], 0), &tmp.state());
        assert!(message(&out.unwrap()).contains("about 330K"));
    }

    #[test]
    fn meter_long_last_line() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        // The last line is longer than 1 chunk: its text is 700 KiB.
        let long = assistant(0, 0, 340_000).replace(
            r#""text":"x""#,
            &format!(r#""text":"{}""#, "y".repeat(700 << 10)),
        );
        let lines = [assistant(0, 0, 1_000_000), long];
        write_lines(&t, &lines);
        let out = meter(&event(&t, "s1", None), &opts(&[300_000], 0), &tmp.state());
        assert!(message(&out.unwrap()).contains("about 340K"));
    }

    #[test]
    fn meter_stops_after_4_mib() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        let filler = r#"{"type":"user","message":{"content":"padding padding padding padding"}}"#;
        let mut lines = vec![assistant(0, 0, 500_000)];
        lines.extend(std::iter::repeat_n(
            filler.to_string(),
            (5 << 20) / filler.len(),
        ));
        write_lines(&t, &lines);
        let out = meter(&event(&t, "s1", None), &opts(&[300_000], 0), &tmp.state());
        assert_eq!(out, None);
    }

    #[test]
    fn meter_bad_input_is_silent() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s1.jsonl");
        let o = opts(&[1], 1);
        assert_eq!(meter("not json", &o, &tmp.state()), None);
        assert_eq!(meter("", &o, &tmp.state()), None);
        assert_eq!(meter("{}", &o, &tmp.state()), None);
        // The file is missing.
        assert_eq!(meter(&event(&t, "s1", None), &o, &tmp.state()), None);
        write_lines(&t, &[assistant(0, 0, 99)]);
        // A bad id.
        for bad in ["", "a/b", "..", "a b", &"x".repeat(65)] {
            assert_eq!(
                meter(&event(&t, bad, None), &o, &tmp.state()),
                None,
                "{bad}"
            );
            // An empty agent id means no agent.
            if !bad.is_empty() {
                assert_eq!(
                    meter(&event(&t, "s1", Some(bad)), &o, &tmp.state()),
                    None,
                    "{bad}"
                );
            }
        }
        // A file with no usage, and an empty file.
        write_lines(&t, &[r#"{"type":"user"}"#.to_string()]);
        assert_eq!(meter(&event(&t, "s1", None), &o, &tmp.state()), None);
        fs::write(&t, "").unwrap();
        assert_eq!(meter(&event(&t, "s1", None), &o, &tmp.state()), None);
        assert!(!tmp.state().exists());
    }

    #[test]
    fn meter_removes_stale_state_files() {
        let tmp = Tmp::new();
        let t = tmp.0.join("s2.jsonl");
        fs::create_dir_all(tmp.state()).unwrap();
        let old = tmp.state().join("other-300000");
        let fresh = tmp.state().join("fresh-300000");
        fs::write(&old, "").unwrap();
        fs::write(&fresh, "").unwrap();
        let four_days = SystemTime::now() - Duration::from_secs(4 * 24 * 3600);
        File::options()
            .write(true)
            .open(&old)
            .unwrap()
            .set_modified(four_days)
            .unwrap();
        write_lines(&t, &[assistant(0, 0, 310_000)]);
        assert!(meter(&event(&t, "s2", None), &opts(&[300_000], 0), &tmp.state()).is_some());
        assert!(!old.exists());
        assert!(fresh.exists());
    }

    #[test]
    fn flags_ignore_bad_values() {
        let a = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let o = MeterOpts::parse(&a(&[
            "--levels",
            "600000,x,300000,0,300000",
            "--subagent",
            "250000",
            "--wat",
        ]));
        assert_eq!((o.levels, o.subagent), (vec![300_000, 600_000], 250_000));
        let o = MeterOpts::parse(&a(&["--levels=5", "--subagent", "many", "--levels"]));
        assert_eq!((o.levels.len(), o.subagent), (0, 0));
        assert_eq!(flag_cap(&a(&["--cap", "4"])), Some(4));
        assert_eq!(flag_cap(&a(&["--cap=4"])), Some(4));
        assert_eq!(flag_cap(&a(&["--cap", "x"])), None);
        assert_eq!(flag_cap(&a(&["--cap"])), None);
        assert_eq!(flag_cap(&a(&[])), None);
    }
    // ---- protect-approvals ----

    #[test]
    fn approvals_split_a_command_into_segments_and_words() {
        let words = |c: &str| command_segments(c);
        assert_eq!(
            words("cd /p && git add -A; git commit -m \"a b\" || echo 'x'"),
            vec![
                vec!["cd", "/p"],
                vec!["git", "add", "-A"],
                vec!["git", "commit", "-m", "a", "b"],
                vec!["echo", "x"],
            ]
        );
        // The inner text of `sh -c` and a substitution give their own words.
        assert_eq!(
            words("sh -c \"spec-dna approve a intent\""),
            vec![vec!["sh", "-c", "spec-dna", "approve", "a", "intent"],]
        );
        assert_eq!(
            words("echo $(spec-dna approve a intent)"),
            vec![
                vec!["echo", "$"],
                vec!["spec-dna", "approve", "a", "intent"]
            ]
        );
        assert_eq!(program("/usr/bin/git"), "git");
        assert_eq!(program("C:\\bin\\spec-dna.exe"), "spec-dna");
        assert_eq!(program("spec-dna-x"), "spec-dna-x");
    }

    #[test]
    fn approvals_read_the_status_and_the_artifact_of_a_path() {
        assert_eq!(
            status_of("---\r\nstatus: \"accepted\"  # ok\r\nclass: routine\r\n---\n"),
            Some("accepted".to_string())
        );
        assert_eq!(status_of("---\nclass: routine\n---\nstatus: draft\n"), None);
        assert_eq!(status_of("status: draft\n"), None);
        let dirs = vec!["docs/intent".to_string()];
        assert_eq!(
            artifact_of("docs/intent/a-b/plan.md", &dirs),
            Some(("a-b".to_string(), "plan", "approved"))
        );
        assert_eq!(
            artifact_of("docs/intent/a-b/intent.md", &dirs),
            Some(("a-b".to_string(), "intent", "accepted"))
        );
        for no in [
            "docs/intent/a-b/notes.md",
            "docs/intent/readme.md",
            "docs/intents/a-b/plan.md",
            "docs/other/a-b/plan.md",
        ] {
            assert_eq!(artifact_of(no, &dirs), None, "{no}");
        }
    }

    #[test]
    fn approvals_run_no_git_for_a_command_with_no_commit() {
        let ev = |c: &str| {
            format!(
                "{{\"cwd\":\"/p\",\"tool_name\":\"Bash\",\"tool_input\":{{\"command\":\"{c}\"}}}}"
            )
        };
        let opts = ApprovalsOpts {
            intent: vec!["docs/intent".into()],
        };
        let mut calls = 0;
        for c in ["git status", "git add -A && git diff", "ls; echo accepted"] {
            let out = approvals(&ev(c), &opts, None, &mut |_, _| {
                calls += 1;
                None
            });
            assert_eq!(out, None, "{c}");
        }
        assert_eq!(calls, 0);
        // The words `git` and `commit` run git, which fails here: no output.
        let out = approvals(&ev("git commit -m x"), &opts, None, &mut |_, _| {
            calls += 1;
            None
        });
        assert_eq!((out, calls), (None, 1));
        // The flag `--intent` repeats and trims a final `/`.
        let flags: Vec<String> = ["--intent", "a/b/", "--intent=c", "--other"]
            .map(String::from)
            .to_vec();
        assert_eq!(ApprovalsOpts::parse(&flags).intent, ["a/b", "c"]);
    }

    // ---- protect-tests ----

    fn ptest(always: bool, globs: &[&str]) -> ProtectOpts {
        ProtectOpts {
            always,
            tests: globs.iter().map(|g| g.to_string()).collect(),
        }
    }

    fn protect_event(tool: &str, file: &str, cwd: &str) -> String {
        let q = |s: &str| s.replace('\\', "\\\\");
        format!(
            "{{\"tool_name\":\"{tool}\",\"cwd\":\"{}\",\"tool_input\":{{\"file_path\":\"{}\"}}}}",
            q(cwd),
            q(file)
        )
    }

    /// Runs `protect` with a fake `git`. `branch` is the answer of
    /// `symbolic-ref` (`None`: a detached HEAD), and `tracked` is the answer
    /// of `ls-files`. Returns the output and the calls of `git`.
    fn pcall(
        input: &str,
        opts: &ProtectOpts,
        project: Option<&str>,
        branch: Option<&str>,
        tracked: bool,
    ) -> (Option<String>, Vec<(String, Vec<String>)>) {
        let mut calls = Vec::new();
        let out = protect(input, opts, project.map(Path::new), &mut |root, args| {
            calls.push((
                root.to_string_lossy().into_owned(),
                args.iter().map(|a| a.to_string()).collect::<Vec<_>>(),
            ));
            Some(if args[0] == "symbolic-ref" {
                GitOut {
                    ok: branch.is_some(),
                    stdout: branch.map(|b| format!("{b}\n")).unwrap_or_default(),
                }
            } else {
                GitOut {
                    ok: tracked,
                    stdout: String::new(),
                }
            })
        });
        (out, calls)
    }

    #[test]
    fn protect_runs_git_only_for_a_path_that_matches_a_glob() {
        let opts = ptest(false, &["**/tests/**", "**/*_test.*"]);
        let ev = |f: &str| protect_event("Edit", f, "/p");
        // No glob matches: no process.
        let (out, calls) = pcall(&ev("/p/src/lib.rs"), &opts, Some("/p"), Some("fix/x"), true);
        assert_eq!((out, calls.len()), (None, 0));
        // A match on another branch: 1 process.
        let (out, calls) = pcall(&ev("/p/tests/a.rs"), &opts, Some("/p"), Some("main"), true);
        assert_eq!((out, calls.len()), (None, 1));
        assert_eq!(calls[0].0, "/p");
        assert_eq!(calls[0].1, ["symbolic-ref", "--short", "-q", "HEAD"]);
        // A detached HEAD: 1 process.
        let (out, calls) = pcall(&ev("/p/tests/a.rs"), &opts, Some("/p"), None, true);
        assert_eq!((out, calls.len()), (None, 1));
        // A new file on a fix branch: 2 processes, no output.
        let (out, calls) = pcall(
            &ev("/p/tests/a.rs"),
            &opts,
            Some("/p"),
            Some("fix/x"),
            false,
        );
        assert_eq!((out, calls.len()), (None, 2));
        assert_eq!(
            calls[1].1,
            ["ls-files", "--error-unmatch", "--", ":(literal)tests/a.rs"]
        );
        // A tracked file on a fix branch: 2 processes, and the answer.
        let (out, calls) = pcall(&ev("/p/tests/a.rs"), &opts, Some("/p"), Some("fix/x"), true);
        assert_eq!(calls.len(), 2);
        let out = out.unwrap();
        assert!(out.contains("\"permissionDecision\":\"ask\""), "{out}");
        assert!(
            out.contains("tests/a.rs is a test file that git tracks, and fix/x is a fix branch")
        );
        // The mode `always` has no branch call.
        let always = ptest(true, &["**/tests/**"]);
        let (out, calls) = pcall(&ev("/p/tests/a.rs"), &always, Some("/p"), None, true);
        assert_eq!(calls.len(), 1);
        assert!(
            out.unwrap()
                .contains("tests/a.rs is a test file that git tracks. Approve")
        );
    }

    #[test]
    fn protect_reads_a_windows_path() {
        let opts = ptest(false, &["**/tests/**"]);
        let ev = protect_event("Write", "C:\\work\\p\\tests\\a.rs", "C:\\work\\p");
        let (out, calls) = pcall(&ev, &opts, Some("C:\\work\\p"), Some("fix/x"), true);
        assert!(
            out.unwrap()
                .contains("spec-dna protect-tests: tests/a.rs is")
        );
        // The root has no backslash when git runs.
        assert_eq!(calls[0].0, "C:/work/p");
        // The root has a final separator, and the cwd is the root.
        let ev = protect_event("Write", "C:\\work\\p\\tests\\a.rs", "C:\\work\\p\\");
        assert!(pcall(&ev, &opts, None, Some("fix/x"), true).0.is_some());
        let (out, _) = pcall(&ev, &opts, Some("C:\\work\\p\\"), Some("fix/x"), true);
        assert!(out.is_some());
        // A path of another drive, or of a longer folder name, is outside.
        for f in ["D:\\work\\p\\tests\\a.rs", "C:\\work\\pp\\tests\\a.rs"] {
            let ev = protect_event("Edit", f, "C:\\work\\p");
            let (out, calls) = pcall(&ev, &opts, Some("C:\\work\\p"), Some("fix/x"), true);
            assert_eq!((out, calls.len()), (None, 0), "{f}");
        }
    }

    #[test]
    fn protect_stops_on_each_bad_input_before_it_starts_git() {
        let opts = ptest(true, &["**"]);
        for input in [
            "",
            "not json",
            "[]",
            "{}",
            "{\"tool_name\":\"Edit\"}",
            "{\"tool_name\":\"Bash\",\"tool_input\":{\"file_path\":\"/p/a\"}}",
            "{\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":\"\"}}",
            "{\"tool_name\":\"Edit\",\"tool_input\":{\"file_path\":\"/p/a\"}}",
            "{\"tool_name\":\"Edit\",\"cwd\":\"\",\"tool_input\":{\"file_path\":\"/p/a\"}}",
        ] {
            let (out, calls) = pcall(input, &opts, None, Some("fix/x"), true);
            assert_eq!((out, calls.len()), (None, 0), "{input}");
        }
        // A segment `..`, `.` or an empty segment gives no output.
        for f in [
            "/p/../p/a",
            "/p/./a",
            "/p//a",
            "/p/a/..",
            "/p",
            "/p/",
            "a/b",
        ] {
            let (out, calls) = pcall(
                &protect_event("Edit", f, "/p"),
                &opts,
                Some("/p"),
                None,
                true,
            );
            assert_eq!((out, calls.len()), (None, 0), "{f}");
        }
        // No glob: no output, even with a test file.
        let none = ptest(true, &[]);
        let ev = protect_event("Edit", "/p/tests/a.rs", "/p");
        assert_eq!(pcall(&ev, &none, Some("/p"), None, true).1.len(), 0);
        // A `git` that does not start gives no output.
        let ev = protect_event("Edit", "/p/tests/a.rs", "/p");
        assert_eq!(
            protect(&ev, &ptest(true, &["**"]), None, &mut |_, _| None),
            None
        );
    }

    #[test]
    fn protect_flags_collect_each_test_glob() {
        let a = |s: &[&str]| s.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let o = ProtectOpts::parse(&a(&[
            "--mode",
            "always",
            "--test",
            "a/**",
            "--test=b/**",
            "--wat",
            "--test",
        ]));
        assert!(o.always);
        assert_eq!(o.tests, ["a/**", "b/**"]);
        let o = ProtectOpts::parse(&a(&["--mode=fix-branches"]));
        assert!(!o.always && o.tests.is_empty());
        assert!(!ProtectOpts::parse(&a(&["--mode", "x"])).always);
    }
}

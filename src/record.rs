//! The acceptance record (design 0.8.0, D3). One TOML file holds what an
//! acceptor tried, what it found and what it decided. This module has the
//! pure parts: the reader, the rules of a valid record, the hash of the code
//! that the record lists, the brief and the writer of the skeleton. It reads
//! no git data and no file. So the command `accept` and the checks of the
//! record (`finish` and `check`, part 2 of slice S4) use the same code.
//!
//! The `toml` crate of the build has no writer, so the writer makes the text
//! by hand. The reader keeps a value that the rules reject, such as `pending`,
//! so that the writer and the rules can use one type.
//!
//! The items that the checks of part 2 call are marked
//! `allow(dead_code)`: the command `accept` does not call them.
//!
//! Format 2 (design 0.8.0, D72) is the format that `accept` writes. Format 1 is
//! the format of the pre-release of 0.8.0. The reader still reads it, so an
//! archive file of format 1 stays readable ([`Record::parse`]), but a record of
//! format 1 in the folder of a change is not valid ([`validate`]).

use crate::change;
use crate::hash;
use crate::tree::Entry;

/// The value of `format` that this binary writes.
pub const FORMAT: i64 = 2;

/// The value of `format` of the pre-release of 0.8.0. The reader reads it. No
/// release has it, and only 2 archive files of this repository do.
pub const FORMAT_1: i64 = 1;

/// The message of SD928 for a record of format 1 in the folder of a change
/// (D72 item 3).
pub const FORMAT_1_MESSAGE: &str = "format 1 is the format of the pre-release of 0.8.0; delete the record, commit, and accept again";

/// The number after `spec-dna-brief` in the first line of a brief that this
/// binary writes. The fixed text of the brief belongs to this number: a change
/// of the text needs a new number (design 0.8.0, D3 item 5).
pub const BRIEF_FORMAT: u32 = 2;

/// The number of the brief of a record of format 1. The compare of SD928 still
/// computes it for an entry of such a record.
pub const BRIEF_FORMAT_1: u32 = 1;

/// The severities of a finding of format 2 (D72 item 1). A `blocker` breaks a
/// requirement, a delta line, an invariant, a rule of the project or a command
/// of `Proof`, loses data, or opens a security hole. A `note` is each other
/// finding.
pub const BLOCKER: &str = "blocker";
pub const NOTE: &str = "note";

/// The most bytes that a record can have.
pub const MAX_BYTES: usize = 16_384;

/// The name of the record in the folder of a change.
pub const FILE_NAME: &str = "acceptance.toml";

/// The role of an acceptor that has no role of `[owners]`.
pub const DEFAULT_ROLE: &str = "acceptor";

/// The value of a field that the acceptor still has to write.
pub const PENDING: &str = "pending";

/// The value of `session` and `agent` when the tool does not know them.
pub const UNKNOWN: &str = "unknown";

/// The most characters of a role name.
const MAX_ROLE: usize = 40;

/// One entry `[[acceptor]]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Acceptor {
    /// The role names of the entry, sorted (format 2). One entry of one pass
    /// answers each role of `[owners]` that matches. A record of format 1 has
    /// one `role`, and the reader gives it as the only item.
    pub roles: Vec<String>,
    /// The hash on the last line of the brief of this entry.
    pub brief: String,
    /// A claim: the session of the acceptor, or `unknown`.
    pub session: String,
    /// A claim: the agent id of the acceptor, or `unknown`.
    pub agent: String,
    /// A claim, optional.
    pub model: Option<String>,
    pub result: String,
}

/// One entry `[[proof]]`: the proof state of a requirement when the record was
/// written (D2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProofEntry {
    pub requirement: String,
    pub state: String,
    pub cases: Vec<String>,
}

/// One entry `[[invariant]]` (D6, slice S5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invariant {
    pub id: String,
    pub answer: String,
    pub reason: Option<String>,
}

/// One entry `[[deviation]]` (D7, slice S5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Deviation {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub verdict: String,
}

/// One entry `[[attempt]]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    /// The number of its `[[acceptor]]` entry, from 1.
    pub acceptor: i64,
    pub text: String,
    pub outcome: String,
}

/// One entry `[[finding]]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub acceptor: i64,
    /// `blocker` or `note`. The reader of a record of format 1 gives `important`
    /// as `blocker` and `nit` as `note`, so a reader shows an old archive file
    /// with the words of format 2.
    pub severity: String,
    pub path: String,
    pub line: Option<i64>,
    pub text: String,
}

/// One entry `[[follow-up]]` (D73). The reader keeps its fields, and
/// [`validate`] checks their shape only. The checks of the chain (the notes that
/// `closes` names, the trees of the links and the diff budget) are the rules of a
/// later slice.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FollowUp {
    /// The hash of the follow-up brief.
    pub brief: String,
    /// A claim: the session of the agent that checked the fix, or `unknown`.
    pub session: String,
    /// A claim: the agent id, or `unknown`.
    pub agent: String,
    /// The hash of `paths` after the fix.
    pub tree: String,
    /// The numbers of the `[[finding]]` entries that the follow-up closes, from 1.
    pub closes: Vec<i64>,
    /// `accepted` or `rejected`.
    pub result: String,
    pub text: String,
}

/// A record as the reader gives it. A string keeps the value of the file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Record {
    pub format: i64,
    pub change: String,
    pub scope: Vec<String>,
    pub paths: Vec<String>,
    pub tree: String,
    pub result: String,
    pub acknowledged_by: Vec<String>,
    pub acceptors: Vec<Acceptor>,
    pub proofs: Vec<ProofEntry>,
    pub invariants: Vec<Invariant>,
    pub deviations: Vec<Deviation>,
    pub attempts: Vec<Attempt>,
    pub findings: Vec<Finding>,
    pub follow_ups: Vec<FollowUp>,
}

/// What the rules of a valid record need to know about the change.
pub struct Expect<'a> {
    /// True for the text of an archive file, which no check validates after its
    /// pull request: a record of format 1 there is read and is not an error.
    pub archive: bool,
    /// The name of the folder of the change.
    pub slug: &'a str,
    /// True if the section `Files` of the plan lists a path.
    pub plan_has_files: bool,
    /// True if a task of the plan has `Files:` other than `none`.
    pub tasks_have_files: bool,
    /// True if a path is a code path of the code tree (D1).
    pub is_code_path: &'a dyn Fn(&str) -> bool,
}

// The reader.

type Table = toml::map::Map<String, toml::Value>;

/// The keys that the reader accepts in each table. The brief names each key
/// of the 5 tables of the acceptor (a unit test checks it).
const KEYS_TOP: [&str; 14] = [
    "format",
    "change",
    "scope",
    "paths",
    "tree",
    "result",
    "acknowledged-by",
    "acceptor",
    "proof",
    "invariant",
    "deviation",
    "attempt",
    "finding",
    "follow-up",
];
/// The tables of format 1: the same, with no `follow-up`.
const KEYS_TOP_1: [&str; 13] = [
    "format",
    "change",
    "scope",
    "paths",
    "tree",
    "result",
    "acknowledged-by",
    "acceptor",
    "proof",
    "invariant",
    "deviation",
    "attempt",
    "finding",
];
const KEYS_ACCEPTOR: [&str; 6] = ["roles", "brief", "session", "agent", "model", "result"];
/// The keys of an entry `[[acceptor]]` of format 1: `role` in place of `roles`.
const KEYS_ACCEPTOR_1: [&str; 6] = ["role", "brief", "session", "agent", "model", "result"];
const KEYS_FOLLOW_UP: [&str; 7] = [
    "brief", "session", "agent", "tree", "closes", "result", "text",
];
const KEYS_PROOF: [&str; 3] = ["requirement", "state", "cases"];
const KEYS_INVARIANT: [&str; 3] = ["id", "answer", "reason"];
const KEYS_DEVIATION: [&str; 4] = ["id", "kind", "text", "verdict"];
const KEYS_ATTEMPT: [&str; 3] = ["acceptor", "text", "outcome"];
const KEYS_FINDING: [&str; 5] = ["acceptor", "severity", "path", "line", "text"];

/// The key and the kind of a table that the reader knows.
fn check_keys(t: &Table, allowed: &[&str], at: &str) -> Result<(), String> {
    for k in t.keys() {
        if !allowed.contains(&k.as_str()) {
            return Err(format!("{at} has the unknown key `{k}`"));
        }
    }
    Ok(())
}

fn text_of(t: &Table, key: &str, at: &str) -> Result<String, String> {
    match t.get(key) {
        Some(toml::Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(format!("`{key}` of {at} must be a string")),
        None => Err(format!("{at} has no `{key}`")),
    }
}

fn opt_text(t: &Table, key: &str, at: &str) -> Result<Option<String>, String> {
    match t.get(key) {
        None => Ok(None),
        Some(toml::Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!("`{key}` of {at} must be a string")),
    }
}

fn int_of(t: &Table, key: &str, at: &str) -> Result<i64, String> {
    match t.get(key) {
        Some(toml::Value::Integer(n)) => Ok(*n),
        Some(_) => Err(format!("`{key}` of {at} must be an integer")),
        None => Err(format!("{at} has no `{key}`")),
    }
}

fn int_list_of(t: &Table, key: &str, at: &str) -> Result<Vec<i64>, String> {
    match t.get(key) {
        None => Ok(Vec::new()),
        Some(toml::Value::Array(a)) => a
            .iter()
            .map(|v| match v {
                toml::Value::Integer(n) => Ok(*n),
                _ => Err(format!("`{key}` of {at} must be a list of integers")),
            })
            .collect(),
        Some(_) => Err(format!("`{key}` of {at} must be a list of integers")),
    }
}

fn list_of(t: &Table, key: &str, at: &str) -> Result<Vec<String>, String> {
    match t.get(key) {
        None => Ok(Vec::new()),
        Some(toml::Value::Array(a)) => a
            .iter()
            .map(|v| match v {
                toml::Value::String(s) => Ok(s.clone()),
                _ => Err(format!("`{key}` of {at} must be a list of strings")),
            })
            .collect(),
        Some(_) => Err(format!("`{key}` of {at} must be a list of strings")),
    }
}

/// The tables of the array `key`, each with a name for a message.
fn tables<'a>(root: &'a Table, key: &str) -> Result<Vec<(String, &'a Table)>, String> {
    match root.get(key) {
        None => Ok(Vec::new()),
        Some(toml::Value::Array(a)) => a
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                toml::Value::Table(t) => Ok((format!("`[[{key}]]` entry {}", i + 1), t)),
                _ => Err(format!(
                    "`{key}` must be a list of tables, written `[[{key}]]`"
                )),
            })
            .collect(),
        Some(_) => Err(format!(
            "`{key}` must be a list of tables, written `[[{key}]]`"
        )),
    }
}

impl Record {
    /// The tree of the last link of the chain (D73 item 3): the `tree` of the last
    /// follow-up, else the `tree` of the record. The record is fresh when the hash
    /// of `paths` equals it (SD929).
    pub fn last_tree(&self) -> &str {
        self.follow_ups.last().map_or(&self.tree, |u| &u.tree)
    }

    /// The numbers (from 1) of the findings of the severity `note` that the first
    /// `upto` follow-ups do not close.
    pub fn open_notes(&self, upto: usize) -> Vec<usize> {
        let closed: Vec<i64> = self
            .follow_ups
            .iter()
            .take(upto)
            .flat_map(|u| u.closes.iter().copied())
            .collect();
        self.findings
            .iter()
            .enumerate()
            .filter(|(i, f)| f.severity == NOTE && !closed.contains(&(*i as i64 + 1)))
            .map(|(i, _)| i + 1)
            .collect()
    }

    /// Reads a record. It fails for a text over `MAX_BYTES`, a text that is no
    /// TOML, an unknown key or table, and a value of a wrong kind. It accepts
    /// the value `pending`, so the skeleton parses: `validate` rejects it.
    ///
    /// The value of `format` picks the keys. Format 1 has `role` in an entry
    /// `[[acceptor]]` and no table `[[follow-up]]`. The reader gives `role` as
    /// the only item of `roles`, and it gives the severities `important` and
    /// `nit` as `blocker` and `note`. Any other value of `format` reads with
    /// the keys of format 2, and [`validate`] reports it.
    pub fn parse(text: &str) -> Result<Record, String> {
        if text.len() > MAX_BYTES {
            return Err(format!(
                "the record has {} bytes, and a record has at most {MAX_BYTES}",
                text.len()
            ));
        }
        let root: Table = text
            .parse::<toml::Table>()
            .map_err(|e| format!("the record is not valid TOML: {}", e.message()))?;
        let here = "the record";
        let format = int_of(&root, "format", here)?;
        let old = format == FORMAT_1;
        check_keys(
            &root,
            if old { &KEYS_TOP_1 } else { &KEYS_TOP },
            "the record",
        )?;
        let mut r = Record {
            format,
            change: text_of(&root, "change", here)?,
            scope: list_of(&root, "scope", here)?,
            paths: list_of(&root, "paths", here)?,
            tree: text_of(&root, "tree", here)?,
            result: text_of(&root, "result", here)?,
            acknowledged_by: list_of(&root, "acknowledged-by", here)?,
            acceptors: Vec::new(),
            proofs: Vec::new(),
            invariants: Vec::new(),
            deviations: Vec::new(),
            attempts: Vec::new(),
            findings: Vec::new(),
            follow_ups: Vec::new(),
        };
        for (at, t) in tables(&root, "acceptor")? {
            check_keys(
                t,
                if old {
                    &KEYS_ACCEPTOR_1
                } else {
                    &KEYS_ACCEPTOR
                },
                &at,
            )?;
            let roles = if old {
                vec![text_of(t, "role", &at)?]
            } else if t.contains_key("roles") {
                list_of(t, "roles", &at)?
            } else {
                return Err(format!("{at} has no `roles`"));
            };
            r.acceptors.push(Acceptor {
                roles,
                brief: text_of(t, "brief", &at)?,
                session: text_of(t, "session", &at)?,
                agent: text_of(t, "agent", &at)?,
                model: opt_text(t, "model", &at)?,
                result: text_of(t, "result", &at)?,
            });
        }
        for (at, t) in tables(&root, "proof")? {
            check_keys(t, &KEYS_PROOF, &at)?;
            r.proofs.push(ProofEntry {
                requirement: text_of(t, "requirement", &at)?,
                state: text_of(t, "state", &at)?,
                cases: list_of(t, "cases", &at)?,
            });
        }
        for (at, t) in tables(&root, "invariant")? {
            check_keys(t, &KEYS_INVARIANT, &at)?;
            r.invariants.push(Invariant {
                id: text_of(t, "id", &at)?,
                answer: text_of(t, "answer", &at)?,
                reason: opt_text(t, "reason", &at)?,
            });
        }
        for (at, t) in tables(&root, "deviation")? {
            check_keys(t, &KEYS_DEVIATION, &at)?;
            r.deviations.push(Deviation {
                id: text_of(t, "id", &at)?,
                kind: text_of(t, "kind", &at)?,
                text: text_of(t, "text", &at)?,
                verdict: text_of(t, "verdict", &at)?,
            });
        }
        for (at, t) in tables(&root, "attempt")? {
            check_keys(t, &KEYS_ATTEMPT, &at)?;
            r.attempts.push(Attempt {
                acceptor: int_of(t, "acceptor", &at)?,
                text: text_of(t, "text", &at)?,
                outcome: text_of(t, "outcome", &at)?,
            });
        }
        for (at, t) in tables(&root, "finding")? {
            check_keys(t, &KEYS_FINDING, &at)?;
            let severity = text_of(t, "severity", &at)?;
            r.findings.push(Finding {
                acceptor: int_of(t, "acceptor", &at)?,
                severity: match (old, severity.as_str()) {
                    (true, "important") => BLOCKER.to_string(),
                    (true, "nit") => NOTE.to_string(),
                    _ => severity,
                },
                path: text_of(t, "path", &at)?,
                line: match t.get("line") {
                    None => None,
                    Some(_) => Some(int_of(t, "line", &at)?),
                },
                text: text_of(t, "text", &at)?,
            });
        }
        if !old {
            for (at, t) in tables(&root, "follow-up")? {
                check_keys(t, &KEYS_FOLLOW_UP, &at)?;
                r.follow_ups.push(FollowUp {
                    brief: text_of(t, "brief", &at)?,
                    session: text_of(t, "session", &at)?,
                    agent: text_of(t, "agent", &at)?,
                    tree: text_of(t, "tree", &at)?,
                    closes: int_list_of(t, "closes", &at)?,
                    result: text_of(t, "result", &at)?,
                    text: text_of(t, "text", &at)?,
                });
            }
        }
        Ok(r)
    }
}

// The rules of a valid record (SD928).

/// True if `s` is 16 hexadecimal digits: the form of a hash of `hash.rs`.
fn is_hash(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// True if `role` is a role name: `[a-z][a-z0-9-]*`, at most 40 characters
/// (design 0.8.0, D5 item 1).
pub fn valid_role(role: &str) -> bool {
    let mut chars = role.chars();
    role.chars().count() <= MAX_ROLE
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// True if `path` is a path from the root of the repository: no empty part, no
/// `.` or `..` part, no `\` and no first `/`.
fn is_root_path(path: &str) -> bool {
    !path.is_empty()
        && !path.contains('\\')
        && path
            .split('/')
            .all(|p| !p.is_empty() && p != "." && p != "..")
}

/// The proof states that a `[[proof]]` entry can have.
fn is_proof_state(s: &str) -> bool {
    crate::evidence::ProofState::ALL
        .iter()
        .any(|p| p.name() == s)
}

/// Checks the rules of a valid record. It returns one text for each broken
/// rule, in the order of the file; an empty list means a valid record. The
/// rule "the `brief` of each entry is the brief of its adding commit" needs
/// git, so the check of part 2 does it with `brief`.
pub fn validate(r: &Record, x: &Expect) -> Vec<String> {
    // A record of format 1 is the format of the pre-release (D72 item 3). In the
    // folder of a change it has one problem, whatever else it holds. An archive
    // file keeps its format, and no rule of format 2 applies to it.
    if r.format == FORMAT_1 {
        return if x.archive {
            Vec::new()
        } else {
            vec![FORMAT_1_MESSAGE.to_string()]
        };
    }
    let mut out: Vec<String> = Vec::new();
    if r.format != FORMAT {
        out.push(format!("`format` must be {FORMAT}, not {}", r.format));
    }
    if r.change != x.slug {
        out.push(format!(
            "`change` must be `{}`, the name of the folder, not `{}`",
            x.slug, r.change
        ));
    }
    if !is_hash(&r.tree) {
        out.push("`tree` must be 16 hexadecimal digits".to_string());
    }
    if r.scope.is_empty() && x.plan_has_files {
        out.push("`scope` is empty, and the plan has a path in `Files`".to_string());
    }
    for w in r.paths.windows(2) {
        if w[0] >= w[1] {
            out.push(format!(
                "`paths` must be sorted with no duplicate: `{}` comes before `{}`",
                w[0], w[1]
            ));
            break;
        }
    }
    for p in &r.paths {
        if !(x.is_code_path)(p) {
            out.push(format!("`paths` lists `{p}`, which is not a code path"));
        } else if !change::in_scope(&r.scope, p) {
            out.push(format!("`paths` lists `{p}`, which `scope` does not match"));
        }
    }
    if r.paths.is_empty() && x.tasks_have_files {
        out.push("`paths` is empty, and a task of the plan has `Files:` with a path".to_string());
    }
    if !matches!(r.result.as_str(), "accepted" | "rejected") {
        out.push(format!(
            "`result` must be `accepted` or `rejected`, not `{}`",
            r.result
        ));
    }
    let n = r.acceptors.len() as i64;
    for (i, a) in r.acceptors.iter().enumerate() {
        let at = format!("acceptor {}", i + 1);
        if a.roles.is_empty() {
            out.push(format!("{at}: `roles` must list at least one role name"));
        }
        for role in a.roles.iter().filter(|role| !valid_role(role)) {
            out.push(format!("{at}: `roles` must hold role names, not `{role}`"));
        }
        if let Some(w) = a.roles.windows(2).find(|w| w[0] >= w[1]) {
            out.push(format!(
                "{at}: `roles` must be sorted with no duplicate: `{}` comes before `{}`",
                w[0], w[1]
            ));
        }
        if !is_hash(&a.brief) {
            out.push(format!(
                "{at}: `brief` must be the hash of its brief: 16 hexadecimal digits"
            ));
        }
        if a.session.trim().is_empty() || a.agent.trim().is_empty() {
            out.push(format!("{at}: `session` and `agent` must not be empty"));
        }
        if !matches!(a.result.as_str(), "accepted" | "rejected") {
            out.push(format!(
                "{at}: `result` must be `accepted` or `rejected`, not `{}`",
                a.result
            ));
        }
        // An entry says `rejected` only for a reason (D72 item 2): a blocker of
        // its own, or one of the reasons that no entry owns.
        if a.result == "rejected"
            && !r
                .findings
                .iter()
                .any(|f| f.acceptor == i as i64 + 1 && f.severity == BLOCKER)
            && !other_reason(r)
        {
            out.push(format!(
                "{at}: `result` is `rejected`, and the entry has no `blocker` finding, and no `violated` invariant, no `rejected` deviation and no `rejected` follow-up gives a reason; a note never gives `rejected`"
            ));
        }
        if !r.attempts.iter().any(|t| t.acceptor == i as i64 + 1) {
            out.push(format!("{at} has no `[[attempt]]`"));
        }
    }
    for p in &r.proofs {
        if !is_proof_state(&p.state) {
            out.push(format!(
                "`[[proof]]` `{}`: `state` `{}` is not a proof state",
                p.requirement, p.state
            ));
        }
    }
    for (i, t) in r.attempts.iter().enumerate() {
        let at = format!("attempt {}", i + 1);
        if t.acceptor < 1 || t.acceptor > n {
            out.push(format!(
                "{at}: `acceptor` is {}, and the record has {n} acceptor entries",
                t.acceptor
            ));
        }
        if !matches!(t.outcome.as_str(), "held" | "broke") {
            out.push(format!(
                "{at}: `outcome` must be `held` or `broke`, not `{}`",
                t.outcome
            ));
        }
        if t.text.trim().is_empty() {
            out.push(format!("{at}: `text` is empty"));
        }
    }
    for (i, f) in r.findings.iter().enumerate() {
        let at = format!("finding {}", i + 1);
        if f.acceptor < 1 || f.acceptor > n {
            out.push(format!(
                "{at}: `acceptor` is {}, and the record has {n} acceptor entries",
                f.acceptor
            ));
        }
        if !matches!(f.severity.as_str(), BLOCKER | NOTE) {
            out.push(format!(
                "{at}: `severity` must be `{BLOCKER}` or `{NOTE}`, not `{}`",
                f.severity
            ));
        }
        if !is_root_path(&f.path) {
            out.push(format!(
                "{at}: `path` must be a path from the root, not `{}`",
                f.path
            ));
        }
        if f.line.is_some_and(|l| l < 1) {
            out.push(format!("{at}: `line` must be 1 or more"));
        }
        if f.text.trim().is_empty() {
            out.push(format!("{at}: `text` is empty"));
        }
    }
    for v in &r.invariants {
        if !matches!(v.answer.as_str(), "held" | "not-affected" | "violated") {
            out.push(format!(
                "invariant `{}`: `answer` must be `held`, `not-affected` or `violated`, not `{}`",
                v.id, v.answer
            ));
        }
        if matches!(v.answer.as_str(), "not-affected" | "violated")
            && v.reason.as_deref().is_none_or(|t| t.trim().is_empty())
        {
            out.push(format!(
                "invariant `{}`: the answer `{}` needs a `reason`",
                v.id, v.answer
            ));
        }
    }
    for d in &r.deviations {
        if !matches!(d.verdict.as_str(), "accepted" | "rejected") {
            out.push(format!(
                "deviation `{}`: `verdict` must be `accepted` or `rejected`, not `{}`",
                d.id, d.verdict
            ));
        }
    }
    // An id answers one item. SD938 reads the first entry of an id, so no check
    // reads a second entry with that id.
    let mut seen: Vec<String> = Vec::new();
    for v in &r.invariants {
        let key = change::parse_pin(&v.id).map_or_else(|_| v.id.clone(), |p| p.text());
        if seen.contains(&key) {
            out.push(format!(
                "invariant `{}` is the id of more than one `[[invariant]]` entry; keep one entry for each id",
                v.id
            ));
        } else {
            seen.push(key);
        }
    }
    let mut seen: Vec<&str> = Vec::new();
    for d in &r.deviations {
        if seen.contains(&d.id.as_str()) {
            out.push(format!(
                "deviation `{}` is the id of more than one `[[deviation]]` entry; keep one entry for each id",
                d.id
            ));
        } else {
            seen.push(&d.id);
        }
    }
    // The shape of each follow-up (D73). The rules of the chain are not here.
    for (i, u) in r.follow_ups.iter().enumerate() {
        let at = format!("follow-up {}", i + 1);
        if !is_hash(&u.brief) {
            out.push(format!(
                "{at}: `brief` must be the hash of its brief: 16 hexadecimal digits"
            ));
        }
        if !is_hash(&u.tree) {
            out.push(format!("{at}: `tree` must be 16 hexadecimal digits"));
        }
        if u.session.trim().is_empty() || u.agent.trim().is_empty() {
            out.push(format!("{at}: `session` and `agent` must not be empty"));
        }
        if !matches!(u.result.as_str(), "accepted" | "rejected") {
            out.push(format!(
                "{at}: `result` must be `accepted` or `rejected`, not `{}`",
                u.result
            ));
        }
        if u.closes.iter().any(|n| *n < 1) {
            out.push(format!("{at}: `closes` lists numbers from 1"));
        }
        if u.text.trim().is_empty() {
            out.push(format!("{at}: `text` is empty"));
        }
    }
    for what in pending_values(r) {
        out.push(format!(
            "{what} has the value `{PENDING}`; write the answer"
        ));
    }
    // The result logic (D72 item 2). `accepted` only when no other value
    // contradicts it, and `rejected` only when a reason exists. A note never
    // gives `rejected`.
    let blocker = r.findings.iter().any(|f| f.severity == BLOCKER);
    if r.result == "accepted" {
        let against: Vec<&str> = [
            (
                r.acceptors.iter().any(|a| a.result != "accepted"),
                "an acceptor did not accept",
            ),
            (blocker, "a finding is a `blocker`"),
            (
                r.invariants.iter().any(|v| v.answer == "violated"),
                "an invariant is `violated`",
            ),
            (
                r.deviations.iter().any(|d| d.verdict == "rejected"),
                "a deviation is `rejected`",
            ),
            (
                r.follow_ups.iter().any(|u| u.result == "rejected"),
                "a follow-up is `rejected`",
            ),
        ]
        .into_iter()
        .filter(|(bad, _)| *bad)
        .map(|(_, why)| why)
        .collect();
        if !against.is_empty() {
            out.push(format!(
                "`result` is `accepted`, and {}; set it to `rejected`",
                against.join(" and ")
            ));
        }
    }
    if r.result == "rejected" && !blocker && !other_reason(r) {
        out.push(
            "`result` is `rejected`, and no reason exists: no `blocker` finding, no `violated` invariant, no `rejected` deviation and no `rejected` follow-up; a note never gives `rejected`, so set `result` to `accepted`, or write the `blocker`"
                .to_string(),
        );
    }
    out
}

/// True if a reason for `rejected` exists that is no finding: an invariant that
/// is `violated`, a deviation that is `rejected`, or a follow-up that is
/// `rejected` (D72 item 2).
fn other_reason(r: &Record) -> bool {
    r.invariants.iter().any(|v| v.answer == "violated")
        || r.deviations.iter().any(|d| d.verdict == "rejected")
        || r.follow_ups.iter().any(|u| u.result == "rejected")
}

/// The places that have the value `pending`, each as a short name.
fn pending_values(r: &Record) -> Vec<String> {
    let mut out = Vec::new();
    let mut add = |ok: bool, what: String| {
        if ok {
            out.push(what);
        }
    };
    let is = |s: &str| s.trim() == PENDING;
    add(is(&r.result), "`result`".to_string());
    for (i, a) in r.acceptors.iter().enumerate() {
        for (k, v) in [
            ("brief", &a.brief),
            ("session", &a.session),
            ("agent", &a.agent),
            ("result", &a.result),
        ] {
            add(is(v), format!("acceptor {}: `{k}`", i + 1));
        }
        add(
            a.roles.iter().any(|r| is(r)),
            format!("acceptor {}: `roles`", i + 1),
        );
        add(
            a.model.as_deref().is_some_and(is),
            format!("acceptor {}: `model`", i + 1),
        );
    }
    for p in &r.proofs {
        add(
            is(&p.state),
            format!("`[[proof]]` `{}`: `state`", p.requirement),
        );
    }
    for v in &r.invariants {
        add(is(&v.answer), format!("invariant `{}`: `answer`", v.id));
        add(
            v.reason.as_deref().is_some_and(is),
            format!("invariant `{}`: `reason`", v.id),
        );
    }
    for d in &r.deviations {
        add(is(&d.verdict), format!("deviation `{}`: `verdict`", d.id));
        add(is(&d.text), format!("deviation `{}`: `text`", d.id));
    }
    for (i, t) in r.attempts.iter().enumerate() {
        add(is(&t.outcome), format!("attempt {}: `outcome`", i + 1));
        add(is(&t.text), format!("attempt {}: `text`", i + 1));
    }
    for (i, f) in r.findings.iter().enumerate() {
        add(is(&f.severity), format!("finding {}: `severity`", i + 1));
        add(is(&f.path), format!("finding {}: `path`", i + 1));
        add(is(&f.text), format!("finding {}: `text`", i + 1));
    }
    for (i, u) in r.follow_ups.iter().enumerate() {
        for (k, v) in [
            ("brief", &u.brief),
            ("session", &u.session),
            ("agent", &u.agent),
            ("tree", &u.tree),
            ("result", &u.result),
            ("text", &u.text),
        ] {
            add(is(v), format!("follow-up {}: `{k}`", i + 1));
        }
    }
    out
}

/// Reads and checks a text: the problems of `Record::parse`, or of `validate`.
/// An empty list is a valid record.
#[allow(dead_code)] // Used by the checks of part 2 of slice S4 (SD928).
pub fn problems(text: &str, x: &Expect) -> Vec<String> {
    match Record::parse(text) {
        Ok(r) => validate(&r, x),
        Err(e) => vec![e],
    }
}

// The code that the record lists.

/// The paths of `candidates` that `scope` matches and that are code paths,
/// sorted with no duplicate (design 0.8.0, D3 item 4).
pub fn select_paths(
    scope: &[String],
    candidates: &[String],
    is_code_path: &dyn Fn(&str) -> bool,
) -> Vec<String> {
    let mut out: Vec<String> = candidates
        .iter()
        .filter(|p| is_code_path(p) && change::in_scope(scope, p))
        .cloned()
        .collect();
    out.sort();
    out.dedup();
    out
}

/// The first line of the hashed text of a tree: the form of `tree.rs`.
const TREE_HEADER: &str = "spec-dna-tree 1\n";

/// The hash of the code tree over `paths` only (design 0.8.0, D3 item 4): one
/// line `<mode> <object id>\t<path>` for each path that `entries` has, and
/// `absent\t<path>` for a path that it does not have. The form is the form of
/// `tree::hash_entries`, so a list with no absent path gives the same hash as
/// that function gives for the same entries. `entries` is the code tree
/// (`tree::entries`). The paths are sorted by their bytes.
pub fn tree_hash(entries: &[Entry], paths: &[String]) -> String {
    let mut sorted: Vec<&String> = paths.iter().collect();
    sorted.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    sorted.dedup();
    let mut text: Vec<u8> = TREE_HEADER.as_bytes().to_vec();
    for p in sorted {
        match entries.iter().find(|e| e.2 == p.as_bytes()) {
            Some((mode, oid, _)) => {
                text.extend_from_slice(format!("{mode} {oid}\t{p}\n").as_bytes())
            }
            None => text.extend_from_slice(format!("absent\t{p}\n").as_bytes()),
        }
    }
    hash::hex(&hash::hash_bytes(&text))
}

// The brief.

/// The fixed text of the brief format 1: the task of the acceptor and the rules
/// of the record. A change of one byte changes the hash of each brief, so a
/// change needs a new `BRIEF_FORMAT`.
const BRIEF_TEXT: &str = "\
You are the acceptor of one change. You did not write its code. You try to
break the change, and you write one record about it. You edit no code.

Work from this text only. Do not ask the builder for more text.

Steps:

1. Read the intent, the spec and the plan below.
2. Run each command that the section `Proof` of the plan names. Run the tests of the paths under `paths`.
3. Run `spec-dna accept <slug> --session \"<your session id>\"`. Add `--role <role>` when your role is not `acceptor`. The command writes the skeleton `acceptance.toml` in the folder of the change, and it prints the number of your entry. Write that number as `acceptor` in each of your attempts and findings.
4. Copy the hash of the last line of this text into `brief` of your entry.
5. Try to break the change: bad input, limits, errors, each path of `Files`, and each requirement of the delta read as a test. Write each try as an `[[attempt]]` with the outcome `held` or `broke`.
6. Answer each invariant and each deviation that this text lists. Use `held`, `not-affected` or `violated` for an invariant, and `accepted` or `rejected` for a deviation. A `not-affected` or `violated` answer needs a `reason`.
7. Write each defect as a `[[finding]]` with the severity `important` or `nit`, a path from the root of the repository and a text. Do not fix a defect.
8. Set `result` of your entry and the top `result` to `accepted` or `rejected`. Set the top `result` to `accepted` only when each acceptor accepted, no finding is `important`, no invariant is `violated` and no deviation is `rejected`.
9. Commit `acceptance.toml` alone. This commit is part of your task. It is no approval commit, and it needs no go-ahead of the owner. Make it in a call of its own after the write. Edit no other file.

Tables of the record. Write only these keys:

- `[[acceptor]]`: `role`, `brief`, `session`, `agent`, `result` and the optional `model`.
- `[[attempt]]`: `acceptor`, `text` and `outcome` (`held` or `broke`).
- `[[finding]]`: `acceptor`, `severity` (`important` or `nit`), `path`, `line` (optional) and `text`.
- `[[invariant]]`: `id`, `answer` (`held`, `not-affected` or `violated`) and `reason`.
- `[[deviation]]`: `id`, `kind`, `text` and `verdict` (`accepted` or `rejected`).
- `[[proof]]`: `requirement`, `state` and `cases`. Do not change it.

Rules of the record:

- The file is TOML. It has `format = 1`, no unknown key and at most 16,384 bytes.
- No value is `pending`. Replace each `pending` value with your answer.
- Each `[[acceptor]]` has `role`, `brief`, `session`, `agent` and `result`. Do not change `scope`, `paths`, `tree` or `acknowledged-by`.
- Each `[[acceptor]]` has at least one `[[attempt]]`.
- Each `[[attempt]]` and `[[finding]]` names the number of its `[[acceptor]]` entry, counted from 1.
- If the code changes after the record, the record is stale. Do not change the code.

The text of the change follows. A line that starts with `=== ` names a part.
";

/// The fixed text of the brief format 2 (design 0.8.0, D72 item 4): the text of
/// format 1, with the 3 passes of the review, the meaning of `blocker` and
/// `note`, the rule of the shell (D75 item 1) and the rule of the commit (D75
/// item 4). The acceptor is also the reviewer, so one run gives one record. A
/// change of one byte changes the hash of each brief, so a change needs a new
/// `BRIEF_FORMAT`.
const BRIEF_TEXT_2: &str = "\
You are the acceptor and the reviewer of one change. You did not write its code. You review the change, you try to break it, and you write one record about it. You edit no code.

Work from this text only. Do not ask the builder for more text.

The shell:

- Start each shell call with the `cd '<root>' &&` of your prompt. If the `cd` fails, stop and report it.
- Put each test repository that you make under the scratch folder of your session. Never write in another checkout.

Steps:

1. Read the intent, the spec and the plan below.
2. Run each command that the section `Proof` of the plan names. Run the tests of the paths under `paths`.
3. Review the change in 3 passes, and give the file and the line of each finding. Bugs: a logic error, a missing check, a wrong edge case, and a test that does not prove the change. Security: a leaked secret, an unsafe input, and an access that the change does not need. Compliance: compare the code with the delta, the invariants and the deviations below, and with the rules of the project. Report a broken requirement, a delta line that the code does not meet, and a file that `Files` does not list.
4. Run `spec-dna accept <slug> --session \"<your session id>\"`. Add `--role <role>` for each role of the first line of this text, except `acceptor`. The command writes the skeleton `acceptance.toml` in the folder of the change, and it prints the number of your entry. Write that number as `acceptor` in each of your attempts and findings.
5. Copy the hash of the last line of this text into `brief` of your entry.
6. Try to break the change: bad input, limits, errors, each path of `Files`, and each requirement of the delta read as a test. Write each try as an `[[attempt]]` with the outcome `held` or `broke`.
7. Answer each invariant and each deviation that this text lists. Use `held`, `not-affected` or `violated` for an invariant, and `accepted` or `rejected` for a deviation. A `not-affected` or `violated` answer needs a `reason`.
8. Write each defect as a `[[finding]]` with a severity, a path from the root of the repository and a text. Use `blocker` for a defect that breaks a requirement, a delta line, an invariant, a rule of the project or a command of `Proof`, loses data, or opens a security hole. Use `note` for each other defect. Do not fix a defect.
9. Set `result` of your entry and the top `result` to `accepted` or `rejected`. Use `rejected` only for a reason: a `blocker`, a `violated` invariant, or a `rejected` deviation. A `note` never gives `rejected`.
10. Commit `acceptance.toml` alone. This commit is part of your task. It is no approval commit: it changes no `status` line, and it needs no go-ahead of the owner. Make it in a call of its own after the write. Edit no other file.

Tables of the record. Write only these keys:

- `[[acceptor]]`: `roles`, `brief`, `session`, `agent`, `result` and the optional `model`.
- `[[attempt]]`: `acceptor`, `text` and `outcome` (`held` or `broke`).
- `[[finding]]`: `acceptor`, `severity` (`blocker` or `note`), `path`, `line` (optional) and `text`.
- `[[invariant]]`: `id`, `answer` (`held`, `not-affected` or `violated`) and `reason`.
- `[[deviation]]`: `id`, `kind`, `text` and `verdict` (`accepted` or `rejected`).
- `[[proof]]`: `requirement`, `state` and `cases`. Do not change it.
- Write no `[[follow-up]]` entry.

Rules of the record:

- The file is TOML. It has `format = 2`, no unknown key and at most 16,384 bytes.
- No value is `pending`. Replace each `pending` value with your answer.
- Each `[[acceptor]]` has `roles`, `brief`, `session`, `agent` and `result`. Do not change `scope`, `paths`, `tree` or `acknowledged-by`.
- Each `[[acceptor]]` has at least one `[[attempt]]`.
- Each `[[attempt]]` and `[[finding]]` names the number of its `[[acceptor]]` entry, counted from 1.
- The top `result` is `accepted` only when each acceptor accepted, no finding is a `blocker`, no invariant is `violated` and no deviation is `rejected`.
- If the code changes after the record, the record is stale. Do not change the code.

The text of the change follows. A line that starts with `=== ` names a part.
";

/// The input of `brief`: everything that the brief is a function of. The
/// command reads these values from the commit; the check of part 2 reads them
/// from the tree of the commit that added the entry.
pub struct Brief<'a> {
    pub slug: &'a str,
    /// The number of the format of the brief: [`BRIEF_FORMAT`] or
    /// [`BRIEF_FORMAT_1`]. It picks the fixed text and the first line.
    pub format: u32,
    /// The role names of the entry, sorted: `acceptor` when `--role` is absent
    /// and no role of `[owners]` matches. The first line holds them, with `,`
    /// between them. A brief of format 1 has one.
    pub entry_roles: &'a [String],
    pub intent: &'a str,
    pub spec: Option<&'a str>,
    pub plan: &'a str,
    pub scope: &'a [String],
    pub paths: &'a [String],
    pub tree: &'a str,
    /// The items of `Invariants` (D6), as text. Empty: no section.
    pub invariants: &'a [String],
    /// The items of `Deviations` (D7), as text. Empty: no section.
    pub deviations: &'a [String],
    /// The roles of `[owners]` that match a path of `paths` (D5), sorted.
    pub roles: &'a [String],
}

/// Appends a text and one final line feed if the text lacks it.
fn push_whole(out: &mut String, text: &str) {
    out.push_str(text);
    if !text.ends_with('\n') {
        out.push('\n');
    }
}

fn push_list(out: &mut String, name: &str, items: &[String], none: bool) {
    if items.is_empty() && !none {
        return;
    }
    out.push_str(&format!("=== {name} ===\n"));
    if items.is_empty() {
        out.push_str("none\n");
    }
    for i in items {
        out.push_str(&format!("- {i}\n"));
    }
}

/// The text before the last line of the brief (design 0.8.0, D3 item 5).
fn brief_body(b: &Brief) -> String {
    let mut out = format!(
        "spec-dna-brief {} {} {}\n",
        b.format,
        b.slug,
        b.entry_roles.join(",")
    );
    out.push_str(if b.format == BRIEF_FORMAT_1 {
        BRIEF_TEXT
    } else {
        BRIEF_TEXT_2
    });
    out.push_str("=== intent.md ===\n");
    push_whole(&mut out, b.intent);
    if let Some(spec) = b.spec {
        out.push_str("=== spec.md ===\n");
        push_whole(&mut out, spec);
    }
    out.push_str("=== plan.md ===\n");
    push_whole(&mut out, b.plan);
    push_list(&mut out, "scope", b.scope, true);
    push_list(&mut out, "paths", b.paths, true);
    out.push_str(&format!("=== tree ===\n{}\n", b.tree));
    push_list(&mut out, "invariants", b.invariants, false);
    push_list(&mut out, "deviations", b.deviations, false);
    push_list(&mut out, "roles", b.roles, false);
    out
}

/// The hash of a brief body: the first 8 bytes of SHA-256 of the bytes, as 16
/// hexadecimal digits (the form of `hash.rs`). It hashes the bytes with no
/// change of the white space, so an edit of the layout of a file changes it.
fn body_hash(body: &str) -> String {
    hash::hex(&hash::hash_bytes(body.as_bytes()))
}

/// The brief: one text that is a function of `b` only. The last line is
/// `brief <hash>`, where the hash is the hash of the text before it.
pub fn brief(b: &Brief) -> String {
    let mut out = brief_body(b);
    let h = body_hash(&out);
    out.push_str(&format!("brief {h}\n"));
    out
}

/// The hash on the last line of a brief, or `None` if the text has no such
/// line or the hash does not match the text before it.
pub fn brief_hash(text: &str) -> Option<String> {
    let body = text.strip_suffix('\n')?;
    let at = body.rfind('\n')? + 1;
    let hash = body[at..].strip_prefix("brief ")?;
    (is_hash(hash) && body_hash(&text[..at]) == hash).then(|| hash.to_string())
}

// The brief of a follow-up (design 0.8.0, D73 item 8).

/// The most bytes that the diff of the whole chain can have (D73 item 2). The text
/// of the diff over `paths`, from the commit of the record to the head, is under
/// this budget. More notes do not buy more diff.
pub const MAX_CHAIN_DIFF: usize = 16_384;

/// The number after `spec-dna-follow-up` in the first line of a follow-up brief.
pub const FOLLOW_UP_FORMAT: u32 = 2;

/// The fixed text of the follow-up brief. A change of one byte changes the hash of
/// each follow-up brief, so a change needs a new `FOLLOW_UP_FORMAT`.
const FOLLOW_UP_TEXT: &str = "\
You check the fix of one or more notes of an acceptance record. You did not write the fix. You edit no code.

Work from this text only. Read the whole diff below against the requirements below.

Steps:

1. Run each command of `Proof`. Start each shell call with the `cd '<root>' &&` of your prompt, and stop when the `cd` fails.
2. Run `spec-dna accept <slug> --follow-up --session \"<your session id>\"`. It adds a `[[follow-up]]` entry to `acceptance.toml`. Copy the hash of the last line of this text into `brief` of the entry.
3. Keep in `closes` each note that the diff fixes, and remove the others.
4. Set `result` to `rejected` when the diff changes other behavior or a command of `Proof` fails. Else set it to `accepted`.
5. Write in `text` the commands that you ran and their results.
6. Commit `acceptance.toml` alone. This commit is part of your task. It is no approval commit: it changes no `status` line, and it needs no go-ahead of the owner. Edit no other file.

The text of the change follows. A line that starts with `=== ` names a part.
";

/// One open note of the follow-up brief.
pub struct BriefNote<'a> {
    /// The number of the `[[finding]]` entry, from 1.
    pub number: usize,
    pub path: &'a str,
    pub line: Option<i64>,
    pub text: &'a str,
}

/// The input of [`follow_up_brief`]: everything that the follow-up brief is a
/// function of.
pub struct FollowUpBrief<'a> {
    pub slug: &'a str,
    /// The text of the section `Delta` of the spec, or empty.
    pub delta: &'a str,
    /// The items of `Invariants` and `Deviations`, as text (D6 and D7).
    pub invariants: &'a [String],
    pub deviations: &'a [String],
    /// The text of the section `Proof` of the plan.
    pub proof: &'a str,
    pub notes: &'a [BriefNote<'a>],
    /// The tree of the last link, and the tree of the code that the follow-up binds.
    pub from_tree: &'a str,
    pub to_tree: &'a str,
    /// One line `<path> <old mode> <old id> <new mode> <new id>` for each path of
    /// the diff of the fix: the input of the diff. A side that the tree lacks is
    /// `absent absent`.
    pub fix: &'a [String],
    /// The output of `git diff`: it is not part of the hash.
    pub diff: &'a str,
}

/// The text of a section of a Markdown file: the lines after `## <heading>` up to
/// the next heading of level 1 or 2, with the blank lines at both ends removed.
/// Empty when the file has no such section.
pub fn section_text(markdown: &str, heading: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let mut inside = false;
    for line in markdown.lines() {
        let t = line.trim_end();
        if let Some(h) = t.strip_prefix("## ") {
            if inside {
                break;
            }
            inside = h.trim() == heading;
            continue;
        }
        if inside && t.starts_with("# ") {
            break;
        }
        if inside {
            out.push(line);
        }
    }
    while out.first().is_some_and(|l| l.trim().is_empty()) {
        out.remove(0);
    }
    while out.last().is_some_and(|l| l.trim().is_empty()) {
        out.pop();
    }
    out.join("\n")
}

/// The follow-up brief: one text that is a function of `b` only. The last line is
/// `brief <hash>`. The hash covers each byte before the line `=== diff ===`, so
/// the part `=== fix ===` binds the content and another version of git, which can
/// print another diff, gives no false SD928.
pub fn follow_up_brief(b: &FollowUpBrief) -> String {
    let mut out = format!("spec-dna-follow-up {FOLLOW_UP_FORMAT} {}\n", b.slug);
    out.push_str(FOLLOW_UP_TEXT);
    out.push_str("=== delta ===\n");
    if b.delta.is_empty() {
        out.push_str("none\n");
    } else {
        push_whole(&mut out, b.delta);
    }
    push_list(&mut out, "invariants", b.invariants, false);
    push_list(&mut out, "deviations", b.deviations, false);
    out.push_str("=== proof ===\n");
    if b.proof.is_empty() {
        out.push_str("none\n");
    } else {
        push_whole(&mut out, b.proof);
    }
    out.push_str("=== notes ===\n");
    for n in b.notes {
        match n.line {
            Some(l) => out.push_str(&format!("{} {}:{l} {}\n", n.number, n.path, n.text)),
            None => out.push_str(&format!("{} {} {}\n", n.number, n.path, n.text)),
        }
    }
    out.push_str(&format!(
        "=== trees ===\nfrom {}\nto {}\n",
        b.from_tree, b.to_tree
    ));
    out.push_str("=== fix ===\n");
    for l in b.fix {
        out.push_str(l);
        out.push('\n');
    }
    let h = body_hash(&out);
    out.push_str("=== diff ===\n");
    push_whole_or_none(&mut out, b.diff);
    out.push_str(&format!("brief {h}\n"));
    out
}

fn push_whole_or_none(out: &mut String, text: &str) {
    if !text.is_empty() {
        push_whole(out, text);
    }
}

/// The hash on the last line of a follow-up brief, or `None` if the text has no
/// such line or the hash does not match the text before `=== diff ===`.
pub fn follow_up_brief_hash(text: &str) -> Option<String> {
    let body = text.strip_suffix('\n')?;
    let at = body.rfind('\n')? + 1;
    let hash = body[at..].strip_prefix("brief ")?;
    let cut = text.rfind("\n=== diff ===\n")? + 1;
    (is_hash(hash) && body_hash(&text[..cut]) == hash).then(|| hash.to_string())
}

/// Adds one entry `[[follow-up]]` at the end of the text of a record. The fields
/// that the acceptor writes are `pending`. Each other byte stays, and the new entry
/// has the line end of the file.
pub fn add_follow_up(text: &str, session: &str, tree: &str, closes: &[usize]) -> String {
    let list: Vec<String> = closes.iter().map(|n| n.to_string()).collect();
    let block = format!(
        "[[follow-up]]\nbrief = {}\nsession = {}\nagent = {}\ntree = {}\ncloses = [{}]\nresult = {}\ntext = {}\n",
        toml_string(PENDING),
        toml_string(session),
        toml_string(UNKNOWN),
        toml_string(tree),
        list.join(", "),
        toml_string(PENDING),
        toml_string(PENDING)
    );
    let eol = if text.find('\n').is_some_and(|i| text[..i].ends_with('\r')) {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = text.trim_end_matches(['\r', '\n']).to_string();
    out.push_str(eol);
    out.push_str(eol);
    out.push_str(&block.replace('\n', eol));
    out
}

// The writer.

/// A basic string of TOML: the quotes, the escapes of `"` and `\`, and the
/// escapes of each control character.
pub fn toml_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                out.push_str(&format!("\\u{:04X}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn toml_list(items: &[String]) -> String {
    let parts: Vec<String> = items.iter().map(|i| toml_string(i)).collect();
    format!("[{}]", parts.join(", "))
}

/// The value of `session` for a value of `--session`: an empty value is
/// `unknown`.
pub fn session_value(given: Option<&str>) -> String {
    match given.map(str::trim) {
        Some(s) if !s.is_empty() => s.to_string(),
        _ => UNKNOWN.to_string(),
    }
}

/// The text of one entry `[[acceptor]]` of a skeleton: the fields that the
/// acceptor still writes are `pending`.
pub fn acceptor_block(roles: &[String], session: &str) -> String {
    format!(
        "[[acceptor]]\nroles = {}\nbrief = {}\nsession = {}\nagent = {}\nresult = {}\n",
        toml_list(roles),
        toml_string(PENDING),
        toml_string(session),
        toml_string(UNKNOWN),
        toml_string(PENDING)
    )
}

/// The computed fields and the tables of a skeleton.
pub struct Skeleton<'a> {
    pub slug: &'a str,
    pub scope: &'a [String],
    pub paths: &'a [String],
    pub tree: &'a str,
    /// The sorted role names of the first entry.
    pub roles: &'a [String],
    pub session: &'a str,
    pub proofs: &'a [ProofEntry],
    /// The ids of the invariants of the spec (D6, slice S5).
    pub invariants: &'a [String],
    /// The deviations of the plan (D7, slice S5): id, kind and text.
    pub deviations: &'a [(String, String, String)],
}

/// The skeleton of a record: the computed fields, one entry `[[acceptor]]`, one
/// `[[proof]]` for each requirement, and one `[[invariant]]` and `[[deviation]]`
/// for each item. Each answer starts as `pending`.
pub fn skeleton(s: &Skeleton) -> String {
    let mut out = format!(
        "format = {FORMAT}\nchange = {}\nscope = {}\npaths = {}\ntree = {}\nresult = {}\nacknowledged-by = []\n",
        toml_string(s.slug),
        toml_list(s.scope),
        toml_list(s.paths),
        toml_string(s.tree),
        toml_string(PENDING)
    );
    out.push('\n');
    out.push_str(&acceptor_block(s.roles, s.session));
    for p in s.proofs {
        out.push_str(&format!(
            "\n[[proof]]\nrequirement = {}\nstate = {}\ncases = {}\n",
            toml_string(&p.requirement),
            toml_string(&p.state),
            toml_list(&p.cases)
        ));
    }
    for id in s.invariants {
        out.push_str(&invariant_block(id));
    }
    for (id, kind, text) in s.deviations {
        out.push_str(&deviation_block(id, kind, text));
    }
    out
}

/// The text of one `[[invariant]]` entry of a skeleton.
fn invariant_block(id: &str) -> String {
    format!(
        "\n[[invariant]]\nid = {}\nanswer = {}\n",
        toml_string(id),
        toml_string(PENDING)
    )
}

/// The text of one `[[deviation]]` entry of a skeleton.
fn deviation_block(id: &str, kind: &str, text: &str) -> String {
    format!(
        "\n[[deviation]]\nid = {}\nkind = {}\ntext = {}\nverdict = {}\n",
        toml_string(id),
        toml_string(kind),
        toml_string(text),
        toml_string(PENDING)
    )
}

/// The bytes of the parts of the text of a record that grow with the change:
/// the line `paths`, the `[[invariant]]` entries and the `[[deviation]]` entries,
/// and the bytes of all other lines. The text can be over [`MAX_BYTES`], so the
/// function reads the lines and not the record. A record over the limit names the
/// largest part. The sizes are the sizes of the text that `accept` would write, so
/// an answer that an acceptor wrote in an entry counts too.
pub fn text_parts(text: &str) -> TextParts {
    let mut p = TextParts::default();
    // The name of the table of the last header, or `None` before the first.
    let mut table: Option<String> = None;
    for line in text.split_inclusive('\n') {
        if let Some((_, name)) = table_header(line) {
            table = Some(name.to_string());
        }
        let n = line.len();
        match table.as_deref() {
            Some("invariant") => p.invariants += n,
            Some("deviation") => p.deviations += n,
            None if line.starts_with("paths = ") => p.paths += n,
            _ => p.other += n,
        }
    }
    p
}

/// The result of [`text_parts`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextParts {
    pub paths: usize,
    pub invariants: usize,
    pub deviations: usize,
    /// All other bytes: the acceptors, the attempts, the findings and the proofs.
    pub other: usize,
}

/// The bytes of a record with each `\r\n` read as `\n` (ADR 0019). With
/// `core.autocrlf`, the working tree has CRLF and git has LF, with a clean
/// `git status`: it is the same record. A lone `\r` stays.
pub fn to_lf(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len());
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'\r' && bytes.get(i + 1) == Some(&b'\n') {
            continue;
        }
        out.push(b);
    }
    out
}

/// The header of a table on one line: `Some((true, name))` for `[[name]]` and
/// `Some((false, name))` for `[name]`. White space and a comment after `#` do
/// not count, and the white space inside the brackets does not count. `None` if
/// the line is no header.
fn table_header(line: &str) -> Option<(bool, &str)> {
    let t = line.split('#').next().unwrap_or("").trim();
    if let Some(inner) = t.strip_prefix("[[").and_then(|r| r.strip_suffix("]]")) {
        return Some((true, inner.trim()));
    }
    let inner = t.strip_prefix('[')?.strip_suffix(']')?;
    Some((false, inner.trim()))
}

/// Adds one entry `[[acceptor]]` to the text of a record, after its last
/// `[[acceptor]]` table. Each other byte stays, so the comments and the tables
/// of the first acceptor stay. It returns the new text.
pub fn add_acceptor(text: &str, roles: &[String], session: &str) -> String {
    let block = acceptor_block(roles, session);
    // The end of the last `[[acceptor]]` table: the first line that starts a
    // table after it, or the end of the text. Another table (an attempt) can sit
    // between 2 acceptor tables, so the scan reads the whole text.
    let mut offset = 0;
    let mut last: Option<usize> = None;
    let mut insert_at = text.len();
    let mut open = false;
    for line in text.split_inclusive('\n') {
        match table_header(line) {
            Some((true, "acceptor")) => {
                last = Some(offset);
                open = true;
            }
            Some(_) if open => {
                insert_at = offset;
                open = false;
            }
            _ => {}
        }
        offset += line.len();
    }
    if open {
        insert_at = text.len();
    }
    let (head, tail) = text.split_at(if last.is_some() {
        insert_at
    } else {
        text.len()
    });
    // The new text has the line end of the file: CRLF if its first line ends so.
    let eol = if text.find('\n').is_some_and(|i| text[..i].ends_with('\r')) {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = head.trim_end_matches(['\r', '\n']).to_string();
    out.push_str(eol);
    out.push_str(eol);
    out.push_str(&block.replace('\n', eol));
    if !tail.is_empty() {
        out.push_str(eol);
        out.push_str(tail);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_parts_count_the_lines_of_each_table_also_over_the_limit() {
        let text = "format = 1\npaths = [\"a\"]\n\n[[acceptor]]\nrole = \"x\"\n\n[[invariant]]\nid = \"i\"\nreason = \"rr\"\n\n[[deviation]]\nid = \"d\"\n\n[[attempt]]\ntext = \"t\"\n";
        let p = text_parts(text);
        assert_eq!(p.paths, "paths = [\"a\"]\n".len());
        assert_eq!(
            p.invariants,
            "[[invariant]]\nid = \"i\"\nreason = \"rr\"\n\n".len()
        );
        assert_eq!(p.deviations, "[[deviation]]\nid = \"d\"\n\n".len());
        assert_eq!(p.paths + p.invariants + p.deviations + p.other, text.len());
    }

    const HASH: &str = "9c1b7e0d4a113f2a";

    fn code(p: &str) -> bool {
        !p.ends_with(".md")
    }

    fn valid_text() -> String {
        format!(
            "format = 2\nchange = \"demo\"\nscope = [\"src/**\"]\npaths = [\"src/a.rs\", \"src/b.rs\"]\ntree = \"3f2a9c1b7e0d4a11\"\nresult = \"accepted\"\nacknowledged-by = []\n\n[[acceptor]]\nroles = [\"acceptor\"]\nbrief = \"{HASH}\"\nsession = \"s1\"\nagent = \"unknown\"\nresult = \"accepted\"\n\n[[proof]]\nrequirement = \"requirements/a#k\"\nstate = \"passed-once\"\ncases = [\"t::one\"]\n\n[[attempt]]\nacceptor = 1\ntext = \"Sent a NaN value.\"\noutcome = \"held\"\n\n[[finding]]\nacceptor = 1\nseverity = \"note\"\npath = \"src/a.rs\"\nline = 8\ntext = \"A name has no unit.\"\n"
        )
    }

    fn expect(plan_files: bool, task_files: bool) -> Expect<'static> {
        Expect {
            archive: false,
            slug: "demo",
            plan_has_files: plan_files,
            tasks_have_files: task_files,
            is_code_path: &code,
        }
    }

    fn problems_of(text: &str) -> Vec<String> {
        problems(text, &expect(true, true))
    }

    /// The text after one replace. The pattern must be in the text.
    fn edit(from: &str, to: &str) -> String {
        let t = valid_text();
        assert!(t.contains(from), "{from}");
        t.replacen(from, to, 1)
    }

    fn has(list: &[String], word: &str) -> bool {
        list.iter().any(|p| p.contains(word))
    }

    #[test]
    fn a_valid_record_has_no_problem_and_each_field_reads() {
        assert_eq!(problems_of(&valid_text()), Vec::<String>::new());
        let r = Record::parse(&valid_text()).unwrap();
        assert_eq!(r.format, 2);
        assert_eq!(r.scope, ["src/**"]);
        assert_eq!(r.paths, ["src/a.rs", "src/b.rs"]);
        assert_eq!(r.acceptors[0].brief, HASH);
        assert_eq!(r.proofs[0].cases, ["t::one"]);
        assert_eq!(r.findings[0].line, Some(8));
    }

    #[test]
    fn the_reader_stops_on_the_structure() {
        for (text, word) in [
            ("format = 2\nchange = ".to_string(), "not valid TOML"),
            (
                edit("acknowledged-by = []\n", "acknowledged-by = []\nnote = 1\n"),
                "unknown key `note`",
            ),
            (
                edit(
                    "roles = [\"acceptor\"]\n",
                    "roles = [\"acceptor\"]\nmood = \"x\"\n",
                ),
                "unknown key `mood`",
            ),
            (
                format!("{}\n[[review]]\nx = 1\n", valid_text()),
                "unknown key `review`",
            ),
            (edit("format = 2", "format = \"2\""), "must be an integer"),
            (
                edit("scope = [\"src/**\"]", "scope = \"src/**\""),
                "list of strings",
            ),
            (
                edit("acceptor = 1\ntext", "acceptor = \"1\"\ntext"),
                "must be an integer",
            ),
            (edit("[[proof]]\n", "[proof]\n"), "list of tables"),
            (edit("brief = \"9c1b7e0d4a113f2a\"\n", ""), "has no `brief`"),
        ] {
            let e = Record::parse(&text).unwrap_err();
            assert!(e.contains(word), "{word}: {e}");
        }
    }

    #[test]
    fn a_record_over_16384_bytes_gives_a_problem() {
        let mut text = valid_text();
        text.push_str(&format!("# {}\n", "x".repeat(MAX_BYTES - text.len())));
        assert!(text.len() > MAX_BYTES);
        let p = problems_of(&text);
        assert!(has(&p, "16384"), "{p:?}");
        // The limit is on the bytes: a text of exactly 16,384 bytes is valid.
        let mut text = valid_text();
        let pad = MAX_BYTES - text.len() - 3;
        text.push_str(&format!("# {}\n", "x".repeat(pad)));
        assert_eq!(text.len(), MAX_BYTES);
        assert_eq!(problems_of(&text), Vec::<String>::new());
    }

    #[test]
    fn each_rule_of_a_valid_record_is_checked() {
        let cases: Vec<(String, &str)> = vec![
            (edit("format = 2", "format = 3"), "`format` must be 2"),
            (
                edit("change = \"demo\"", "change = \"other\""),
                "`change` must be `demo`",
            ),
            (
                edit("tree = \"3f2a9c1b7e0d4a11\"", "tree = \"3f2a\""),
                "`tree` must be 16 hexadecimal",
            ),
            (
                edit("tree = \"3f2a9c1b7e0d4a11\"", "tree = \"3f2a9c1b7e0d4a1g\""),
                "`tree` must be 16 hexadecimal",
            ),
            (
                edit(
                    "scope = [\"src/**\"]\npaths = [\"src/a.rs\", \"src/b.rs\"]",
                    "scope = []\npaths = []",
                ),
                "`scope` is empty",
            ),
            (
                edit(
                    "[\"src/a.rs\", \"src/b.rs\"]",
                    "[\"src/b.rs\", \"src/a.rs\"]",
                ),
                "must be sorted",
            ),
            (
                edit(
                    "[\"src/a.rs\", \"src/b.rs\"]",
                    "[\"src/a.rs\", \"src/a.rs\"]",
                ),
                "no duplicate",
            ),
            (
                edit("[\"src/a.rs\", \"src/b.rs\"]", "[\"src/a.rs\", \"x/y.rs\"]"),
                "`scope` does not match",
            ),
            (
                edit("[\"src/a.rs\", \"src/b.rs\"]", "[\"src/a.md\"]"),
                "not a code path",
            ),
            (
                edit("paths = [\"src/a.rs\", \"src/b.rs\"]", "paths = []"),
                "`paths` is empty",
            ),
            (
                edit(
                    "result = \"accepted\"\nacknowledged",
                    "result = \"maybe\"\nacknowledged",
                ),
                "`result` must be `accepted` or `rejected`",
            ),
            (
                edit("roles = [\"acceptor\"]", "roles = [\"Security\"]"),
                "`roles` must hold role names",
            ),
            (
                edit("brief = \"9c1b7e0d4a113f2a\"", "brief = \"abc\""),
                "`brief` must be the hash",
            ),
            (
                edit("session = \"s1\"", "session = \"\""),
                "`session` and `agent` must not be empty",
            ),
            (
                edit("agent = \"unknown\"", "agent = \" \""),
                "`session` and `agent` must not be empty",
            ),
            (
                edit("state = \"passed-once\"", "state = \"great\""),
                "not a proof state",
            ),
            (
                edit("outcome = \"held\"", "outcome = \"fine\""),
                "`outcome` must be `held` or `broke`",
            ),
            (
                edit("acceptor = 1\ntext = \"Sent", "acceptor = 2\ntext = \"Sent"),
                "the record has 1 acceptor entries",
            ),
            (
                edit("acceptor = 1\nseverity", "acceptor = 3\nseverity"),
                "finding 1: `acceptor` is 3",
            ),
            (
                edit("severity = \"note\"", "severity = \"major\""),
                "`severity` must be `blocker` or `note`",
            ),
            (
                edit("path = \"src/a.rs\"", "path = \"/src/a.rs\""),
                "path from the root",
            ),
            (
                edit("path = \"src/a.rs\"", "path = \"../a.rs\""),
                "path from the root",
            ),
            (
                edit("path = \"src/a.rs\"", "path = \"\""),
                "path from the root",
            ),
            (edit("line = 8", "line = 0"), "`line` must be 1 or more"),
            (
                edit("text = \"A name has no unit.\"", "text = \"\""),
                "finding 1: `text` is empty",
            ),
        ];
        for (text, word) in cases {
            let p = problems_of(&text);
            assert!(has(&p, word), "{word}: {p:?}");
        }
        // Each acceptor needs an attempt.
        let two = format!(
            "{}\n{}",
            valid_text(),
            acceptor_block(&["security".to_string()], "s2")
        )
        .replace(
            "roles = [\"security\"]\nbrief = \"pending\"",
            "roles = [\"security\"]\nbrief = \"1111111111111111\"",
        );
        let two = two.replace(
            "session = \"s2\"\nagent = \"unknown\"\nresult = \"pending\"",
            "session = \"s2\"\nagent = \"unknown\"\nresult = \"accepted\"",
        );
        assert!(
            has(&problems_of(&two), "acceptor 2 has no `[[attempt]]`"),
            "{:?}",
            problems_of(&two)
        );
        let with_attempt =
            format!("{two}\n[[attempt]]\nacceptor = 2\ntext = \"Tried.\"\noutcome = \"held\"\n");
        assert_eq!(problems_of(&with_attempt), Vec::<String>::new());
        // A path that the plan lists needs no scope when the plan lists none.
        let none = edit(
            "scope = [\"src/**\"]\npaths = [\"src/a.rs\", \"src/b.rs\"]",
            "scope = []\npaths = []",
        );
        assert!(problems(&none, &expect(false, false)).is_empty());
        assert!(has(
            &problems(&none, &expect(false, true)),
            "`paths` is empty"
        ));
    }

    #[test]
    fn no_value_is_pending() {
        for (from, to, word) in [
            (
                "result = \"accepted\"\nacknowledged",
                "result = \"pending\"\nacknowledged",
                "`result`",
            ),
            (
                "brief = \"9c1b7e0d4a113f2a\"",
                "brief = \"pending\"",
                "acceptor 1: `brief`",
            ),
            (
                "session = \"s1\"",
                "session = \"pending\"",
                "acceptor 1: `session`",
            ),
            (
                "agent = \"unknown\"\nresult = \"accepted\"",
                "agent = \"pending\"\nresult = \"accepted\"",
                "acceptor 1: `agent`",
            ),
            (
                "outcome = \"held\"",
                "outcome = \"pending\"",
                "attempt 1: `outcome`",
            ),
        ] {
            let p = problems_of(&edit(from, to));
            assert!(
                p.iter().any(|m| m.contains(word) && m.contains("pending")),
                "{word}: {p:?}"
            );
        }
        // The skeleton parses and has pending values.
        let sk = skeleton(&Skeleton {
            slug: "demo",
            scope: &["src/**".to_string()],
            paths: &["src/a.rs".to_string()],
            tree: "3f2a9c1b7e0d4a11",
            roles: &["acceptor".to_string()],
            session: "s1",
            proofs: &[],
            invariants: &["rules/x#a@0123456789abcdef".to_string()],
            deviations: &[("d1".to_string(), "plan".to_string(), "Moved.".to_string())],
        });
        let r = Record::parse(&sk).unwrap();
        let p = problems(&sk, &expect(true, true));
        assert!(
            has(&p, "invariant `rules/x#a@0123456789abcdef`: `answer`"),
            "{p:?}"
        );
        assert!(has(&p, "deviation `d1`: `verdict`"), "{p:?}");
        assert!(has(&p, "acceptor 1: `brief`"), "{p:?}");
        assert_eq!(r.result, "pending");
    }

    #[test]
    fn the_answers_of_an_invariant_and_a_deviation_follow_the_rules() {
        let base = valid_text();
        let inv = |answer: &str, reason: &str| {
            format!(
                "{base}\n[[invariant]]\nid = \"rules/x#a@0123456789abcdef\"\nanswer = \"{answer}\"\n{reason}"
            )
        };
        assert!(problems_of(&inv("held", "")).is_empty());
        assert!(has(
            &problems_of(&inv("not-affected", "")),
            "needs a `reason`"
        ));
        assert!(has(
            &problems_of(&inv("violated", "reason = \"\"\n")),
            "needs a `reason`"
        ));
        assert!(
            problems_of(&inv("not-affected", "reason = \"The path is not used.\"\n")).is_empty()
        );
        assert!(has(&problems_of(&inv("sure", "")), "`answer` must be"));
        let dev = |verdict: &str| {
            format!(
                "{base}\n[[deviation]]\nid = \"d1\"\nkind = \"plan\"\ntext = \"Moved.\"\nverdict = \"{verdict}\"\n"
            )
        };
        assert!(problems_of(&dev("accepted")).is_empty());
        assert!(has(&problems_of(&dev("later")), "`verdict` must be"));
    }

    #[test]
    fn a_repeated_id_of_an_invariant_or_a_deviation_is_a_problem() {
        let base = valid_text();
        let dev = "\n[[deviation]]\nid = \"d1\"\nkind = \"plan\"\ntext = \"Moved.\"\nverdict = \"accepted\"\n";
        let one = format!("{base}{dev}");
        assert!(problems_of(&one).is_empty());
        let two = format!("{one}{dev}");
        let found = problems_of(&two);
        assert!(
            has(&found, "deviation `d1`") && has(&found, "more than one `[[deviation]]`"),
            "{found:?}"
        );
        let inv = "\n[[invariant]]\nid = \"rules/x#a@0123456789abcdef\"\nanswer = \"held\"\n";
        assert!(problems_of(&format!("{base}{inv}")).is_empty());
        let found = problems_of(&format!("{base}{inv}{inv}"));
        assert!(
            has(&found, "invariant `rules/x#a@0123456789abcdef`")
                && has(&found, "more than one `[[invariant]]`"),
            "{found:?}"
        );
        // Two other ids are no repeat.
        let other = dev.replace("d1", "d2");
        assert!(problems_of(&format!("{base}{dev}{other}")).is_empty());
    }

    #[test]
    fn accepted_needs_that_nothing_speaks_against_it() {
        let base = valid_text();
        let important = edit("severity = \"note\"", "severity = \"blocker\"");
        assert!(has(&problems_of(&important), "a finding is a `blocker`"));
        assert!(has(
            &problems_of(&format!(
                "{base}\n[[invariant]]\nid = \"i\"\nanswer = \"violated\"\nreason = \"Broken.\"\n"
            )),
            "an invariant is `violated`"
        ));
        assert!(has(
            &problems_of(&format!(
                "{base}\n[[deviation]]\nid = \"d1\"\nkind = \"plan\"\ntext = \"x\"\nverdict = \"rejected\"\n"
            )),
            "a deviation is `rejected`"
        ));
        let rejected = edit(
            "session = \"s1\"\nagent = \"unknown\"\nresult = \"accepted\"",
            "session = \"s1\"\nagent = \"unknown\"\nresult = \"rejected\"",
        );
        assert!(has(&problems_of(&rejected), "an acceptor did not accept"));
        // The top result `rejected` with an important finding is valid.
        let ok = important.replace(
            "result = \"accepted\"\nacknowledged",
            "result = \"rejected\"\nacknowledged",
        );
        assert!(problems_of(&ok).is_empty(), "{:?}", problems_of(&ok));
    }

    #[test]
    fn paths_are_selected_sorted_with_no_duplicate() {
        let scope = vec!["src/**".to_string(), "Cargo.lock".to_string()];
        let cand: Vec<String> = [
            "src/z.rs",
            "README.md",
            "Cargo.lock",
            "src/a.rs",
            "src/z.rs",
            "x/y.rs",
            "src/n.md",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        assert_eq!(
            select_paths(&scope, &cand, &code),
            ["Cargo.lock", "src/a.rs", "src/z.rs"]
        );
    }

    fn e(mode: &str, oid: &str, path: &str) -> Entry {
        (mode.to_string(), oid.to_string(), path.as_bytes().to_vec())
    }

    #[test]
    fn the_tree_hash_covers_the_listed_paths_only() {
        let all = vec![
            e("100644", "aa", "src/a.rs"),
            e("100755", "bb", "b.sh"),
            e("100644", "cc", "other.rs"),
        ];
        let listed = vec!["src/a.rs".to_string(), "b.sh".to_string()];
        // With no absent path it equals the hash of the same entries.
        let same = crate::tree::hash_entries(&all[..2]);
        assert_eq!(tree_hash(&all, &listed), same);
        // An entry that is not listed changes nothing; a listed one does.
        let mut other = all.clone();
        other[2] = e("100644", "dd", "other.rs");
        assert_eq!(tree_hash(&other, &listed), same);
        other[0] = e("100644", "ab", "src/a.rs");
        assert_ne!(tree_hash(&other, &listed), same);
        // An absent path has its own line, and it differs from a missing list item.
        let gone = vec!["gone.rs".to_string()];
        let text = "spec-dna-tree 1\nabsent\tgone.rs\n";
        assert_eq!(
            tree_hash(&all, &gone),
            hash::hex(&hash::hash_bytes(text.as_bytes()))
        );
        assert_ne!(tree_hash(&all, &gone), tree_hash(&all, &[]));
        // The order of the list does not matter.
        let rev: Vec<String> = listed.iter().rev().cloned().collect();
        assert_eq!(tree_hash(&all, &rev), tree_hash(&all, &listed));
    }

    /// A list of role names that lives for the whole test run.
    fn entry(names: &[&str]) -> &'static [String] {
        Box::leak(
            names
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<String>>()
                .into_boxed_slice(),
        )
    }

    fn sample<'a>(roles: &'a [String], scope: &'a [String], paths: &'a [String]) -> Brief<'a> {
        Brief {
            slug: "demo",
            format: BRIEF_FORMAT,
            entry_roles: entry(&["acceptor"]),
            intent: "# Intent\n",
            spec: Some("# Spec\n"),
            plan: "# Plan",
            scope,
            paths,
            tree: "3f2a9c1b7e0d4a11",
            invariants: &[],
            deviations: &[],
            roles,
        }
    }

    #[test]
    fn the_brief_is_a_function_of_its_input_and_names_its_hash() {
        let scope = vec!["src/**".to_string()];
        let paths = vec!["src/a.rs".to_string()];
        let b = brief(&sample(&[], &scope, &paths));
        assert_eq!(b, brief(&sample(&[], &scope, &paths)));
        assert!(b.starts_with("spec-dna-brief 2 demo acceptor\n"));
        assert!(b.contains(
            "=== intent.md ===\n# Intent\n=== spec.md ===\n# Spec\n=== plan.md ===\n# Plan\n"
        ));
        assert!(b.contains(
            "=== scope ===\n- src/**\n=== paths ===\n- src/a.rs\n=== tree ===\n3f2a9c1b7e0d4a11\n"
        ));
        // The 3 optional sections are absent when they are empty.
        for name in ["invariants", "deviations", "roles"] {
            assert!(!b.contains(&format!("=== {name} ===")), "{name}");
        }
        let last = b.lines().last().unwrap();
        let h = last.strip_prefix("brief ").unwrap();
        assert_eq!(h.len(), 16);
        assert_eq!(brief_hash(&b).as_deref(), Some(h));
        // The hash is over the text before the last line.
        let body = &b[..b.len() - last.len() - 1];
        assert_eq!(h, hash::hex(&hash::hash_bytes(body.as_bytes())));
        // A change of any input changes the last line.
        let mut changed = sample(&[], &scope, &paths);
        changed.plan = "# Plan 2";
        assert_ne!(brief(&changed), b);
        let mut role = sample(&[], &scope, &paths);
        role.entry_roles = entry(&["db-reviewer", "security-reviewer"]);
        let r = brief(&role);
        assert!(r.starts_with("spec-dna-brief 2 demo db-reviewer,security-reviewer\n"));
        assert_ne!(r.lines().last(), b.lines().last());
        let roles = vec!["security-reviewer".to_string()];
        let with_roles = brief(&sample(&roles, &scope, &paths));
        assert!(with_roles.contains("=== roles ===\n- security-reviewer\n"));
        let mut no_spec = sample(&[], &scope, &paths);
        no_spec.spec = None;
        assert!(!brief(&no_spec).contains("=== spec.md ==="));
        // A line that is not a hash of its text gives no hash.
        assert_eq!(brief_hash(&b.replace("# Intent", "# Intent!")), None);
        assert_eq!(brief_hash("no hash\n"), None);
    }

    #[test]
    fn the_brief_names_each_key_that_the_reader_accepts_in_a_table_of_the_acceptor() {
        let tables = [
            "acceptor",
            "attempt",
            "finding",
            "invariant",
            "deviation",
            "proof",
        ];
        // Format 2: `roles` and the severities `blocker` and `note`.
        let all = KEYS_ACCEPTOR
            .iter()
            .chain(&KEYS_PROOF)
            .chain(&KEYS_INVARIANT)
            .chain(&KEYS_DEVIATION)
            .chain(&KEYS_ATTEMPT)
            .chain(&KEYS_FINDING);
        for key in all {
            assert!(BRIEF_TEXT_2.contains(&format!("`{key}`")), "{key}");
        }
        for table in tables {
            assert!(BRIEF_TEXT_2.contains(&format!("`[[{table}]]`")), "{table}");
            assert!(KEYS_TOP.contains(&table));
        }
        assert!(BRIEF_TEXT_2.contains("`severity` (`blocker` or `note`)"));
        assert!(BRIEF_TEXT_2.contains("Write no `[[follow-up]]` entry."));
        assert!(BRIEF_TEXT_2.contains("no finding is a `blocker`"));
        // Format 1 is the text of the pre-release, byte for byte.
        let all_1 = KEYS_ACCEPTOR_1.iter().chain(&KEYS_PROOF);
        for key in all_1 {
            assert!(BRIEF_TEXT.contains(&format!("`{key}`")), "{key}");
        }
        for table in tables {
            assert!(BRIEF_TEXT.contains(&format!("`[[{table}]]`")), "{table}");
            assert!(KEYS_TOP_1.contains(&table));
        }
        assert!(BRIEF_TEXT.contains("each acceptor accepted"));
        assert!(BRIEF_TEXT.contains("`severity` (`important` or `nit`)"));
    }

    #[test]
    fn the_brief_says_that_the_commit_of_the_record_is_part_of_the_task() {
        for part in [
            "Commit `acceptance.toml` alone.",
            "This commit is part of your task.",
            "Make it in a call of its own after the write.",
            "Edit no other file.",
        ] {
            assert!(BRIEF_TEXT.contains(part), "{part}");
            assert!(BRIEF_TEXT_2.contains(part), "{part}");
        }
        assert!(
            BRIEF_TEXT.contains("It is no approval commit, and it needs no go-ahead of the owner.")
        );
        // The rule of the commit of D75 item 4, in the words of the design.
        assert!(BRIEF_TEXT_2.contains(
            "It is no approval commit: it changes no `status` line, and it needs no go-ahead of the owner."
        ));
    }

    #[test]
    fn the_fixed_text_of_format_1_is_about_3_kilobytes_and_has_no_pending_rule_gap() {
        let n = BRIEF_TEXT.len();
        assert!((2000..=4000).contains(&n), "{n}");
        assert!(BRIEF_TEXT.is_ascii());
        assert!(BRIEF_TEXT.ends_with('\n'));
    }

    /// D72 item 4: the fixed text of format 2 holds the 3 passes, the meaning of
    /// the 2 severities, the rule of the shell and the rule of the commit, in about
    /// 4 KB.
    #[test]
    fn the_fixed_text_of_format_2_is_about_4_kilobytes_and_names_each_duty() {
        let n = BRIEF_TEXT_2.len();
        assert!((3500..=4500).contains(&n), "{n}");
        assert!(BRIEF_TEXT_2.is_ascii());
        assert!(BRIEF_TEXT_2.ends_with('\n'));
        for part in [
            "Bugs:",
            "Security:",
            "Compliance:",
            "Use `blocker` for a defect that breaks a requirement, a delta line, an invariant, a rule of the project or a command of `Proof`, loses data, or opens a security hole.",
            "Use `note` for each other defect.",
            "A `note` never gives `rejected`.",
            "Start each shell call with the `cd '<root>' &&` of your prompt. If the `cd` fails, stop and report it.",
            "scratch folder",
        ] {
            assert!(BRIEF_TEXT_2.contains(part), "{part}");
        }
        assert!(!BRIEF_TEXT_2.contains("important"));
    }

    /// The layout of the brief and its fixed text are pinned by one hash. A
    /// change of the text or of the layout needs a new brief format (D3 item 5).
    #[test]
    fn the_brief_of_format_1_has_a_pinned_hash() {
        let scope = vec!["src/**".to_string()];
        let paths = vec!["src/a.rs".to_string()];
        let mut old = sample(&[], &scope, &paths);
        old.format = BRIEF_FORMAT_1;
        let b = brief(&old);
        assert!(b.starts_with("spec-dna-brief 1 demo acceptor\n"));
        assert_eq!(brief_hash(&b).as_deref(), Some(PINNED_BRIEF_HASH));
    }

    const PINNED_BRIEF_HASH: &str = "c6493dfa353660cc";

    #[test]
    fn the_brief_of_format_2_has_a_pinned_hash() {
        let scope = vec!["src/**".to_string()];
        let paths = vec!["src/a.rs".to_string()];
        let b = brief(&sample(&[], &scope, &paths));
        assert_eq!(brief_hash(&b).as_deref(), Some(PINNED_BRIEF_HASH_2));
    }

    const PINNED_BRIEF_HASH_2: &str = "9ffc1376d24a6374";

    #[test]
    fn toml_strings_round_trip_through_the_reader() {
        for s in [
            "plain",
            "a\"b",
            "a\\b",
            "line\nbreak",
            "tab\there",
            "\u{1}\u{7f}",
            "ü 日本",
            "C:\\x\\\"y\"",
        ] {
            let text = format!("v = {}\n", toml_string(s));
            let t: toml::Table = text.parse().unwrap();
            assert_eq!(t["v"].as_str(), Some(s), "{s:?}");
        }
        let items = vec!["a\"b".to_string(), "c\\d".to_string()];
        let text = format!("v = {}\n", toml_list(&items));
        let t: toml::Table = text.parse().unwrap();
        let got: Vec<&str> = t["v"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|v| v.as_str())
            .collect();
        assert_eq!(got, ["a\"b", "c\\d"]);
    }

    #[test]
    fn the_skeleton_round_trips_and_lists_each_entry() {
        let proofs = vec![ProofEntry {
            requirement: "requirements/a#k".to_string(),
            state: "tagged".to_string(),
            cases: vec![],
        }];
        let scope = vec!["src/\"q\".rs".to_string()];
        let sk = skeleton(&Skeleton {
            slug: "demo",
            scope: &scope,
            paths: &[],
            tree: "3f2a9c1b7e0d4a11",
            roles: &["acceptor".to_string()],
            session: "s\"1",
            proofs: &proofs,
            invariants: &[],
            deviations: &[],
        });
        let r = Record::parse(&sk).unwrap();
        assert_eq!(r.scope, scope);
        assert_eq!(r.acceptors[0].session, "s\"1");
        assert_eq!(r.acceptors[0].agent, "unknown");
        assert_eq!(r.acceptors[0].brief, "pending");
        assert_eq!(r.proofs, proofs);
        assert!(r.acknowledged_by.is_empty());
    }

    #[test]
    fn a_second_acceptor_is_added_and_the_other_bytes_stay() {
        let text = format!("# a comment\n{}", valid_text());
        let out = add_acceptor(&text, &["security".to_string()], "s2");
        let r = Record::parse(&out).unwrap();
        assert_eq!(r.acceptors.len(), 2);
        assert_eq!(r.acceptors[1].roles, ["security"]);
        assert_eq!(r.acceptors[1].session, "s2");
        assert_eq!(r.acceptors[1].brief, "pending");
        // The first acceptor, the comment and each table stay.
        assert!(out.starts_with("# a comment\n"));
        assert_eq!(r.acceptors[0], Record::parse(&text).unwrap().acceptors[0]);
        assert_eq!(r.attempts.len(), 1);
        assert_eq!(r.findings.len(), 1);
        assert_eq!(r.proofs.len(), 1);
        // A third entry goes after the second.
        let third = Record::parse(&add_acceptor(&out, &["acceptor".to_string()], "s3")).unwrap();
        assert_eq!(third.acceptors.len(), 3);
        assert_eq!(third.acceptors[2].session, "s3");
        // A record that ends with its last acceptor.
        let only = format!(
            "{}\n{}",
            valid_text()[..valid_text().find("[[proof]]").unwrap()].trim_end(),
            ""
        );
        let out = add_acceptor(&only, &["acceptor".to_string()], "s9");
        assert_eq!(Record::parse(&out).unwrap().acceptors.len(), 2);
    }

    #[test]
    fn the_new_acceptor_goes_after_the_last_acceptor_table_also_between_other_tables() {
        // A hand-written layout: an attempt sits between the 2 acceptor tables.
        let acceptor = |n: u32| {
            format!(
                "[[acceptor]]\nroles = [\"acceptor\"]\nbrief = \"{HASH}\"\nsession = \"s{n}\"\nagent = \"unknown\"\nresult = \"accepted\"\n"
            )
        };
        let attempt = |n: u32| {
            format!("[[attempt]]\nacceptor = {n}\ntext = \"Tried {n}.\"\noutcome = \"held\"\n")
        };
        let head = "format = 2\nchange = \"demo\"\nscope = [\"src/**\"]\npaths = [\"src/a.rs\"]\ntree = \"3f2a9c1b7e0d4a11\"\nresult = \"accepted\"\nacknowledged-by = []\n";
        let text = format!(
            "{head}\n{}\n{}\n{}\n{}",
            acceptor(1),
            attempt(1),
            acceptor(2),
            attempt(2)
        );
        let out = add_acceptor(&text, &["security".to_string()], "s3");
        let r = Record::parse(&out).unwrap();
        assert_eq!(r.acceptors.len(), 3);
        assert_eq!(r.acceptors[2].session, "s3");
        assert_eq!(r.acceptors[2].roles, ["security"]);
        // The new table follows the second acceptor, and the second attempt
        // keeps its place after the new entry.
        let at = |needle: &str| out.find(needle).unwrap();
        assert!(at("session = \"s2\"") < at("session = \"s3\""));
        assert!(at("session = \"s3\"") < at("Tried 2."));
        assert!(at("Tried 1.") < at("session = \"s2\""));
        assert_eq!(r.attempts.len(), 2);
    }

    #[test]
    fn the_acceptor_header_is_read_with_spaces_a_comment_and_an_indent() {
        let head = "format = 2\nchange = \"demo\"\nscope = [\"src/**\"]\npaths = [\"src/a.rs\"]\ntree = \"3f2a9c1b7e0d4a11\"\nresult = \"accepted\"\nacknowledged-by = []\n";
        let body = |n: u32| {
            format!(
                "roles = [\"acceptor\"]\nbrief = \"{HASH}\"\nsession = \"s{n}\"\nagent = \"unknown\"\nresult = \"accepted\"\n"
            )
        };
        let attempt = |n: u32| {
            format!("[[attempt]]\nacceptor = {n}\ntext = \"Tried {n}.\"\noutcome = \"held\"\n")
        };
        for (first, second) in [
            ("[[acceptor]]  # x", "[[ acceptor ]]"),
            ("  [[acceptor]]", "[[acceptor]] # y"),
            ("[[ acceptor ]]", "\t[[  acceptor  ]]\t# z"),
        ] {
            let text = format!(
                "{head}\n{first}\n{}\n{}\n{second}\n{}\n{}",
                body(1),
                attempt(1),
                body(2),
                attempt(2)
            );
            assert_eq!(Record::parse(&text).unwrap().acceptors.len(), 2);
            let out = add_acceptor(&text, &["security".to_string()], "s3");
            let r = Record::parse(&out).unwrap();
            assert_eq!(r.acceptors.len(), 3, "{out}");
            assert_eq!(r.acceptors[2].session, "s3", "{out}");
            assert_eq!(r.acceptors[2].roles, ["security"], "{out}");
            let at = |needle: &str| out.find(needle).unwrap();
            assert!(at("session = \"s2\"") < at("session = \"s3\""), "{out}");
            assert!(at("session = \"s3\"") < at("Tried 2."), "{out}");
        }
    }

    #[test]
    fn a_second_acceptor_is_written_with_the_line_end_of_the_file() {
        // A file with CRLF gets CRLF for the whole new entry, and no lone LF.
        let crlf = valid_text().replace('\n', "\r\n");
        let out = add_acceptor(&crlf, &["security".to_string()], "s2");
        assert_eq!(Record::parse(&out).unwrap().acceptors.len(), 2);
        assert_eq!(out.matches('\n').count(), out.matches("\r\n").count());
        assert!(out.starts_with(&crlf[..crlf.find("[[proof]]").unwrap()]));
        // A file with LF stays with LF.
        let lf = add_acceptor(&valid_text(), &["security".to_string()], "s2");
        assert!(!lf.contains('\r'));
        // A file that ends with its last acceptor.
        let only = crlf[..crlf.find("[[proof]]").unwrap()]
            .trim_end()
            .to_string();
        let out = add_acceptor(&only, &["acceptor".to_string()], "s9");
        assert_eq!(out.matches('\n').count(), out.matches("\r\n").count());
        assert_eq!(Record::parse(&out).unwrap().acceptors.len(), 2);
    }

    #[test]
    fn a_crlf_is_read_as_a_lf_and_a_lone_cr_stays() {
        assert_eq!(to_lf(b"a\r\nb\n\r\n"), b"a\nb\n\n");
        assert_eq!(to_lf(b"a\rb\r"), b"a\rb\r");
        assert_eq!(to_lf(b""), b"");
    }

    #[test]
    fn a_role_is_a_lowercase_name() {
        for ok in ["acceptor", "security-reviewer", "a1", &"a".repeat(40)] {
            assert!(valid_role(ok), "{ok}");
        }
        for bad in ["", "A", "1a", "-a", "a b", "a_b", "é", &"a".repeat(41)] {
            assert!(!valid_role(bad), "{bad}");
        }
    }

    /// A record of format 1, as the pre-release wrote it.
    fn old_text() -> String {
        valid_text()
            .replace("format = 2", "format = 1")
            .replace("roles = [\"acceptor\"]", "role = \"acceptor\"")
            .replace("severity = \"note\"", "severity = \"nit\"")
    }

    /// D72 item 3: a record of format 1 in a folder has one problem, with the
    /// words of the design. An archive file of format 1 is read and no rule of
    /// format 2 applies to it. The reader shows the old severities as the new ones.
    #[test]
    fn a_record_of_format_1_is_read_but_not_valid_in_a_folder() {
        let old = old_text();
        let r = Record::parse(&old).unwrap();
        assert_eq!(r.format, 1);
        assert_eq!(r.acceptors[0].roles, ["acceptor"]);
        assert_eq!(r.findings[0].severity, "note");
        let imp = old.replace("severity = \"nit\"", "severity = \"important\"");
        assert_eq!(Record::parse(&imp).unwrap().findings[0].severity, "blocker");
        assert_eq!(
            problems_of(&old),
            [
                "format 1 is the format of the pre-release of 0.8.0; delete the record, commit, and accept again"
            ]
        );
        // The message is the only one, also for a record that breaks other rules.
        let broken = old.replace("brief = \"9c1b7e0d4a113f2a\"", "brief = \"pending\"");
        assert_eq!(problems_of(&broken).len(), 1);
        let archive = Expect {
            archive: true,
            ..expect(true, true)
        };
        assert!(problems(&old, &archive).is_empty());
        assert!(problems(&broken, &archive).is_empty());
        // The table `follow-up` and the key `roles` are not keys of format 1.
        for text in [
            format!("{old}\n[[follow-up]]\nx = 1\n"),
            old.replace("role = \"acceptor\"", "roles = [\"acceptor\"]"),
        ] {
            assert!(Record::parse(&text).is_err());
        }
        // Format 2 has no `role`.
        let two = valid_text().replace("roles = [\"acceptor\"]", "role = \"acceptor\"");
        assert!(
            Record::parse(&two)
                .unwrap_err()
                .contains("unknown key `role`")
        );
    }

    #[test]
    fn the_roles_of_an_entry_are_names_sorted_with_no_duplicate() {
        let roles = |v: &str| edit("roles = [\"acceptor\"]", &format!("roles = {v}"));
        assert!(problems_of(&roles("[\"db-reviewer\", \"security-reviewer\"]")).is_empty());
        for (v, word) in [
            ("[]", "`roles` must list at least one role name"),
            (
                "[\"security-reviewer\", \"db-reviewer\"]",
                "`roles` must be sorted",
            ),
            ("[\"a\", \"a\"]", "no duplicate"),
            ("[\"Db\"]", "`roles` must hold role names"),
        ] {
            let p = problems_of(&roles(v));
            assert!(has(&p, word), "{v}: {p:?}");
        }
    }

    #[test]
    fn a_rejected_result_needs_a_reason_and_a_note_never_gives_it() {
        let rejected = |t: String| {
            t.replace(
                "result = \"accepted\"\nacknowledged",
                "result = \"rejected\"\nacknowledged",
            )
        };
        // Notes only: no reason.
        let p = problems_of(&rejected(valid_text()));
        assert!(has(&p, "no reason exists"), "{p:?}");
        assert!(has(&p, "a note never gives `rejected`"), "{p:?}");
        // A blocker is a reason.
        let blocker = edit("severity = \"note\"", "severity = \"blocker\"");
        assert!(problems_of(&rejected(blocker.clone())).is_empty());
        // The 3 other reasons.
        let base = valid_text();
        for tail in [
            "\n[[invariant]]\nid = \"i\"\nanswer = \"violated\"\nreason = \"Broken.\"\n",
            "\n[[deviation]]\nid = \"d1\"\nkind = \"plan\"\ntext = \"x\"\nverdict = \"rejected\"\n",
            "\n[[follow-up]]\nbrief = \"9c1b7e0d4a113f2a\"\nsession = \"s9\"\nagent = \"unknown\"\ntree = \"3f2a9c1b7e0d4a11\"\ncloses = [1]\nresult = \"rejected\"\ntext = \"The fix breaks a case.\"\n",
        ] {
            let t = rejected(format!("{base}{tail}"));
            assert!(problems_of(&t).is_empty(), "{tail}: {:?}", problems_of(&t));
        }
        // An entry that says `rejected` needs a blocker of its own, or such a reason.
        let entry_rejected = |t: String| {
            t.replace(
                "session = \"s1\"\nagent = \"unknown\"\nresult = \"accepted\"",
                "session = \"s1\"\nagent = \"unknown\"\nresult = \"rejected\"",
            )
        };
        let p = problems_of(&entry_rejected(rejected(valid_text())));
        assert!(has(&p, "acceptor 1: `result` is `rejected`"), "{p:?}");
        assert!(problems_of(&entry_rejected(rejected(blocker))).is_empty());
        // A rejected follow-up makes `accepted` invalid.
        let fu = format!(
            "{base}\n[[follow-up]]\nbrief = \"9c1b7e0d4a113f2a\"\nsession = \"s9\"\nagent = \"unknown\"\ntree = \"3f2a9c1b7e0d4a11\"\ncloses = [1]\nresult = \"rejected\"\ntext = \"x\"\n"
        );
        let p = problems_of(&fu);
        assert!(has(&p, "a follow-up is `rejected`"), "{p:?}");
        // A blocker with `accepted` is invalid.
        let p = problems_of(&edit("severity = \"note\"", "severity = \"blocker\""));
        assert!(has(&p, "a finding is a `blocker`"), "{p:?}");
    }

    /// The table `[[follow-up]]` is read and its shape is checked. The chain rules
    /// (the notes that `closes` names, the trees, the diff budget) are a later slice.
    #[test]
    fn a_follow_up_is_read_and_checked_for_its_shape_only() {
        let ok = format!(
            "{}\n[[follow-up]]\nbrief = \"0b7e2d91c4a5f310\"\nsession = \"s9\"\nagent = \"unknown\"\ntree = \"5d0e7c2a91b3f4e8\"\ncloses = [1]\nresult = \"accepted\"\ntext = \"TIMEOUT is TIMEOUT_MS now.\"\n",
            valid_text()
        );
        assert!(problems_of(&ok).is_empty(), "{:?}", problems_of(&ok));
        let r = Record::parse(&ok).unwrap();
        assert_eq!(
            r.follow_ups,
            [FollowUp {
                brief: "0b7e2d91c4a5f310".to_string(),
                session: "s9".to_string(),
                agent: "unknown".to_string(),
                tree: "5d0e7c2a91b3f4e8".to_string(),
                closes: vec![1],
                result: "accepted".to_string(),
                text: "TIMEOUT is TIMEOUT_MS now.".to_string(),
            }]
        );
        let bad = |from: &str, to: &str| ok.replace(from, to);
        for (text, word) in [
            (
                bad("brief = \"0b7e2d91c4a5f310\"", "brief = \"x\""),
                "follow-up 1: `brief` must be",
            ),
            (
                bad("tree = \"5d0e7c2a91b3f4e8\"", "tree = \"x\""),
                "follow-up 1: `tree` must be",
            ),
            (
                bad("session = \"s9\"", "session = \"\""),
                "follow-up 1: `session` and `agent`",
            ),
            (
                bad("result = \"accepted\"\ntext", "result = \"fine\"\ntext"),
                "follow-up 1: `result` must be",
            ),
            (
                bad("closes = [1]", "closes = [0]"),
                "follow-up 1: `closes` lists numbers from 1",
            ),
            (
                bad("text = \"TIMEOUT is TIMEOUT_MS now.\"", "text = \" \""),
                "follow-up 1: `text` is empty",
            ),
            (
                bad("tree = \"5d0e7c2a91b3f4e8\"", "tree = \"pending\""),
                "follow-up 1: `tree`",
            ),
        ] {
            let p = problems_of(&text);
            assert!(has(&p, word), "{word}: {p:?}");
        }
        // A wrong kind of value, and an unknown key, stop the reader.
        for (text, word) in [
            (bad("closes = [1]", "closes = [\"1\"]"), "list of integers"),
            (
                bad("closes = [1]", "closes = [1]\nmood = 1"),
                "unknown key `mood`",
            ),
            (bad("tree = \"5d0e7c2a91b3f4e8\"\n", ""), "has no `tree`"),
        ] {
            let e = Record::parse(&text).unwrap_err();
            assert!(e.contains(word), "{word}: {e}");
        }
    }

    #[test]
    fn the_skeleton_of_format_2_has_roles_and_the_new_format() {
        let sk = skeleton(&Skeleton {
            slug: "demo",
            scope: &[],
            paths: &[],
            tree: "3f2a9c1b7e0d4a11",
            roles: &["db-reviewer".to_string(), "security-reviewer".to_string()],
            session: "s1",
            proofs: &[],
            invariants: &[],
            deviations: &[],
        });
        assert!(sk.starts_with("format = 2\n"));
        assert!(sk.contains("roles = [\"db-reviewer\", \"security-reviewer\"]\n"));
        assert!(!sk.contains("\nrole = "));
        let r = Record::parse(&sk).unwrap();
        assert_eq!(r.acceptors[0].roles, ["db-reviewer", "security-reviewer"]);
    }

    #[test]
    fn a_section_text_stops_at_the_next_heading_and_drops_the_blank_ends() {
        let md = "# T\n\n## Delta\n\n- ADD a#b: x.\n- ADD a#c: y.\n\n## Proof\n\nRun it.\n";
        assert_eq!(section_text(md, "Delta"), "- ADD a#b: x.\n- ADD a#c: y.");
        assert_eq!(section_text(md, "Proof"), "Run it.");
        assert_eq!(section_text(md, "Risks"), "");
    }

    #[test]
    fn the_follow_up_brief_hash_covers_the_text_before_the_diff_only() {
        let notes = [BriefNote {
            number: 1,
            path: "src/a.rs",
            line: Some(3),
            text: "A name.",
        }];
        let make = |diff: &str| {
            follow_up_brief(&FollowUpBrief {
                slug: "demo",
                delta: "- ADD a#b: x.",
                invariants: &[],
                deviations: &["d1 (plan): Moved.".to_string()],
                proof: "Run it.",
                notes: &notes,
                from_tree: "3f2a9c1b7e0d4a11",
                to_tree: "5d0e7c2a91b3f4e8",
                fix: &["src/a.rs absent absent 100644 abc".to_string()],
                diff,
            })
        };
        let a = make("+one\n");
        let b = make("+another text of the diff\n");
        assert!(a.starts_with("spec-dna-follow-up 2 demo\n"));
        assert_ne!(a, b);
        // The diff is not hashed, so the hash is the same.
        assert_eq!(a.lines().last(), b.lines().last());
        assert_eq!(follow_up_brief_hash(&a), follow_up_brief_hash(&b));
        assert!(follow_up_brief_hash(&a).is_some());
        // The text before the diff is hashed.
        let c = a.replace("A name.", "Another name.");
        assert_eq!(follow_up_brief_hash(&c), None);
        assert!(a.contains("=== notes ===\n1 src/a.rs:3 A name.\n"));
        assert!(a.contains("=== deviations ===\n- d1 (plan): Moved.\n"));
        assert!(!a.contains("=== invariants ==="));
        let t = FOLLOW_UP_TEXT.len();
        assert!(
            (500..=1200).contains(&t) && FOLLOW_UP_TEXT.is_ascii(),
            "{t}"
        );
    }

    #[test]
    fn a_follow_up_is_added_at_the_end_and_the_chain_counts_open_notes() {
        let text = format!(
            "{}\n[[finding]]\nacceptor = 1\nseverity = \"note\"\npath = \"src/b.rs\"\ntext = \"Another.\"\n",
            valid_text()
        );
        let out = add_follow_up(&text, "s2", "5d0e7c2a91b3f4e8", &[1, 2]);
        let r = Record::parse(&out).unwrap();
        assert_eq!(r.follow_ups.len(), 1);
        assert_eq!(r.follow_ups[0].closes, [1, 2]);
        assert_eq!(r.follow_ups[0].brief, "pending");
        assert_eq!(r.last_tree(), "5d0e7c2a91b3f4e8");
        assert!(out.starts_with(text.trim_end()));
        assert_eq!(r.open_notes(0), [1, 2]);
        assert_eq!(r.open_notes(1), Vec::<usize>::new());
        let crlf = add_follow_up(&text.replace('\n', "\r\n"), "s2", "5d0e7c2a91b3f4e8", &[1]);
        assert_eq!(crlf.matches('\n').count(), crlf.matches("\r\n").count());
        assert_eq!(
            Record::parse(&valid_text()).unwrap().last_tree(),
            "3f2a9c1b7e0d4a11"
        );
    }

    #[test]
    fn session_unknown_when_absent_or_empty() {
        assert_eq!(session_value(None), "unknown");
        assert_eq!(session_value(Some("")), "unknown");
        assert_eq!(session_value(Some("  ")), "unknown");
        assert_eq!(session_value(Some("abc")), "abc");
    }
}

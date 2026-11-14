//! The agent harness (ADR 0004). A role table, the `[harness]` configuration,
//! the generated files for Claude Code (agent files, a team page, a line in
//! `CLAUDE.md`, and a merge into `.claude/settings.json`), and the `check` of
//! these files. docs/harness.md describes the result.
//!
//! The module keeps no prompt text outside the agent texts below. Each
//! generated file has a marker as its last line: `<!-- spec-dna harness v<ver>
//! h:<hash> -->`. The hash covers the text above the marker. Thus `check` can
//! tell a manual edit (SD501) from an old text (SD502).

use crate::adapters::VERSION;
use crate::config::Fields;
use crate::diag::{Code, Diag};
use crate::hash::{hash_normalized, hex};
use crate::json::{self, Value};
use toml::Value as Toml;

pub const SETTINGS: &str = ".claude/settings.json";
pub const CLAUDE: &str = "CLAUDE.md";
pub const AGENTS_DIR: &str = ".claude/agents/";
pub const TEAM_PAGE: &str = ".claude/agents/model-team.md";

const OPEN: &str = "<!-- spec-dna harness v";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    None,
    Lean,
    Full,
}

impl Level {
    pub fn name(self) -> &'static str {
        match self {
            Level::None => "none",
            Level::Lean => "lean",
            Level::Full => "full",
        }
    }

    pub fn from_name(s: &str) -> Option<Level> {
        [Level::None, Level::Lean, Level::Full]
            .into_iter()
            .find(|l| l.name() == s)
    }

    /// Returns true if the level installs the role.
    pub fn includes(self, role: Role) -> bool {
        match self {
            Level::None => false,
            Level::Lean => !role.full_only(),
            Level::Full => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Orchestrator,
    Advisor,
    Reviewer,
    Researcher,
    Implementer,
    SpecWriter,
    Helper,
}

impl Role {
    pub const ALL: [Role; 7] = [
        Role::Orchestrator,
        Role::Advisor,
        Role::Reviewer,
        Role::Researcher,
        Role::Implementer,
        Role::SpecWriter,
        Role::Helper,
    ];

    /// The key in `[harness.roles]`.
    pub fn key(self) -> &'static str {
        match self {
            Role::Orchestrator => "orchestrator",
            Role::Advisor => "advisor",
            Role::Reviewer => "reviewer",
            Role::Researcher => "researcher",
            Role::Implementer => "implementer",
            Role::SpecWriter => "spec-writer",
            Role::Helper => "helper",
        }
    }

    pub fn from_key(s: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|r| r.key() == s)
    }

    /// Returns true for a role that only level `full` installs.
    pub fn full_only(self) -> bool {
        matches!(self, Role::Implementer | Role::SpecWriter)
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Role::Orchestrator | Role::Reviewer => "opus",
            Role::Advisor => "fable",
            Role::Researcher | Role::Helper => "haiku",
            Role::Implementer | Role::SpecWriter => "sonnet",
        }
    }

    pub fn default_effort(self) -> Effort {
        match self {
            Role::Orchestrator | Role::Advisor => Effort::XHigh,
            Role::Helper => Effort::Inherit,
            _ => Effort::High,
        }
    }

    /// The generated agent file of the role. The orchestrator and the helper
    /// have none: they are settings.
    pub fn agent(self) -> Option<&'static AgentDef> {
        AGENT_DEFS.iter().find(|a| a.role == self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Effort {
    /// No effort is written: the session keeps its own.
    Inherit,
    Low,
    Medium,
    High,
    XHigh,
    Max,
}

impl Effort {
    pub fn name(self) -> &'static str {
        match self {
            Effort::Inherit => "inherit",
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::XHigh => "xhigh",
            Effort::Max => "max",
        }
    }

    pub fn from_name(s: &str) -> Option<Effort> {
        [
            Effort::Inherit,
            Effort::Low,
            Effort::Medium,
            Effort::High,
            Effort::XHigh,
            Effort::Max,
        ]
        .into_iter()
        .find(|e| e.name() == s)
    }
}

/// The model of a role for one adapter (`claude-code`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Model {
    /// The role is off.
    None,
    /// The agent file gets `model: inherit`. For the orchestrator and the
    /// helper, spec-dna writes no key.
    Inherit,
    /// An alias (`opus`) or a model id.
    Id(String),
}

impl Model {
    /// Reads a model value: a keyword (`none`, `inherit`) or a model.
    pub fn parse(s: &str) -> Model {
        match s {
            "none" => Model::None,
            "inherit" => Model::Inherit,
            _ => Model::Id(s.to_string()),
        }
    }

    pub fn name(&self) -> &str {
        match self {
            Model::None => "none",
            Model::Inherit => "inherit",
            Model::Id(s) => s,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleCfg {
    pub role: Role,
    pub model: Model,
    pub effort: Effort,
    /// The advisor only: the model of the `fallback-advisor` agent.
    pub agent: Model,
    /// The orchestrator only: the models of `fallbackModel`.
    pub fallback: Vec<String>,
}

impl RoleCfg {
    pub fn new(role: Role) -> RoleCfg {
        RoleCfg {
            role,
            model: Model::Id(role.default_model().to_string()),
            effort: role.default_effort(),
            agent: if role == Role::Advisor {
                Model::Id("opus".into())
            } else {
                Model::None
            },
            fallback: if role == Role::Orchestrator {
                vec!["opus".into(), "sonnet".into()]
            } else {
                Vec::new()
            },
        }
    }
}

/// The `[harness]` table.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Harness {
    pub level: Level,
    pub deny: Vec<String>,
    pub context_meter: Vec<u32>,
    pub subagent_meter: u32,
    pub fanout_cap: u32,
    /// One entry for each role, in the order of `Role::ALL`.
    pub roles: Vec<RoleCfg>,
    /// The effort for each model alias, for a switch with `/model`.
    pub efforts: Vec<(String, Effort)>,
}

pub const DEFAULT_CONTEXT_METER: [u32; 2] = [300_000, 600_000];
pub const DEFAULT_SUBAGENT_METER: u32 = 250_000;
pub const DEFAULT_FANOUT_CAP: u32 = 10;

/// The aliases of Claude Code and their model ids, for `modelSettings`.
const MODEL_IDS: [(&str, &str); 4] = [
    ("fable", "claude-fable-5-2"),
    ("opus", "claude-opus-5-5"),
    ("sonnet", "claude-sonnet-5-5"),
    ("haiku", "claude-haiku-5-5"),
];

/// Returns the model id for `modelSettings`. An alias gives its id, and any
/// other plain value is the id. A value with a context suffix (`opus[1m]`) or
/// a keyword of Claude Code has no id.
pub fn model_id(model: &str) -> Option<String> {
    if model.contains('[') || matches!(model, "opusplan" | "default" | "best") {
        return None;
    }
    Some(
        MODEL_IDS
            .iter()
            .find(|(a, _)| *a == model)
            .map_or(model, |(_, id)| id)
            .to_string(),
    )
}

/// The rank of a known alias: a higher number is a stronger model.
fn rank(model: &str) -> Option<u8> {
    let alias = model.split('[').next().unwrap_or(model);
    match alias {
        "fable" => Some(4),
        "opus" => Some(3),
        "sonnet" => Some(2),
        "haiku" => Some(1),
        _ => None,
    }
}

fn token(s: &str, key: &str, what: &str) -> Result<(), String> {
    if s.is_empty() || s.contains(char::is_whitespace) {
        return Err(format!(
            "`{key}` must be {what}: a string that is not empty and has no spaces"
        ));
    }
    Ok(())
}

impl Harness {
    /// The defaults for a level (ADR 0004, "Roles").
    pub fn new(level: Level) -> Harness {
        Harness {
            level,
            deny: Vec::new(),
            context_meter: DEFAULT_CONTEXT_METER.to_vec(),
            subagent_meter: DEFAULT_SUBAGENT_METER,
            fanout_cap: DEFAULT_FANOUT_CAP,
            roles: Role::ALL.iter().map(|r| RoleCfg::new(*r)).collect(),
            efforts: vec![("sonnet".into(), Effort::High)],
        }
    }

    pub fn cfg(&self, role: Role) -> &RoleCfg {
        &self.roles[Role::ALL.iter().position(|r| *r == role).unwrap_or(0)]
    }

    pub fn cfg_mut(&mut self, role: Role) -> &mut RoleCfg {
        let i = Role::ALL.iter().position(|r| *r == role).unwrap_or(0);
        &mut self.roles[i]
    }

    /// Returns true if the level includes the role and the role is not off.
    pub fn on(&self, role: Role) -> bool {
        self.level.includes(role) && self.cfg(role).model != Model::None
    }

    pub fn context_hook_on(&self) -> bool {
        self.level != Level::None && (!self.context_meter.is_empty() || self.subagent_meter > 0)
    }

    pub fn fanout_hook_on(&self) -> bool {
        self.level != Level::None && self.fanout_cap > 0
    }

    /// Reads the `[harness]` table. With level `none`, it still reads the other
    /// keys, but it does not check the roles against the level.
    pub fn parse(v: &Toml) -> Result<Harness, String> {
        let tbl = v.as_table().ok_or("`harness` must be a table")?;
        let mut f = Fields::new(tbl, "harness.");
        let name = f.req_str("level")?;
        let level = Level::from_name(&name).ok_or_else(|| {
            format!("`harness.level` must be \"none\", \"lean\" or \"full\", not `{name}`")
        })?;
        let mut h = Harness::new(level);
        for d in f.opt_strs("deny")? {
            token(&d, "harness.deny", "a tool name")?;
            if !h.deny.contains(&d) {
                h.deny.push(d);
            }
        }
        if let Some(v) = f.take("context-meter") {
            let bad = || {
                "`harness.context-meter` must be an array of positive integers in ascending order"
                    .to_string()
            };
            let arr = v.as_array().ok_or_else(bad)?;
            let mut out: Vec<u32> = Vec::new();
            for x in arr {
                match x {
                    Toml::Integer(n) if *n > 0 && *n <= i64::from(u32::MAX) => out.push(*n as u32),
                    _ => return Err(bad()),
                }
            }
            if out.windows(2).any(|w| w[0] >= w[1]) {
                return Err(bad());
            }
            h.context_meter = out;
        }
        if let Some(n) = f.opt_u32("subagent-meter")? {
            h.subagent_meter = n;
        }
        if let Some(n) = f.opt_u32("fanout-cap")? {
            h.fanout_cap = n;
        }
        if let Some(v) = f.take("roles") {
            let roles = v.as_table().ok_or("`harness.roles` must be a table")?;
            for (key, rv) in roles {
                let role = Role::from_key(key).ok_or_else(|| {
                    let keys: Vec<&str> = Role::ALL.iter().map(|r| r.key()).collect();
                    format!(
                        "`harness.roles.{key}`: unknown role; use {}",
                        keys.join(", ")
                    )
                })?;
                if level != Level::None && !level.includes(role) {
                    return Err(format!("`harness.roles.{key}`: the role is in level full"));
                }
                let rt = rv
                    .as_table()
                    .ok_or_else(|| format!("`harness.roles.{key}` must be a table"))?;
                *h.cfg_mut(role) = parse_role(role, rt)?;
            }
        }
        if let Some(v) = f.take("efforts") {
            let tbl = v.as_table().ok_or("`harness.efforts` must be a table")?;
            h.efforts.clear();
            for (model, ev) in tbl {
                let key = format!("harness.efforts.{model}");
                token(model, "harness.efforts", "a model name")?;
                let e = ev
                    .as_str()
                    .and_then(Effort::from_name)
                    .filter(|e| *e != Effort::Max)
                    .ok_or_else(|| {
                        format!("`{key}` must be \"low\", \"medium\", \"high\", \"xhigh\" or \"inherit\"")
                    })?;
                h.efforts.push((model.clone(), e));
            }
        }
        f.done()?;
        // `deny` must leave each generated agent at least one tool. An empty
        // `tools:` line can mean all tools.
        for g in generated_files(&h) {
            if let Some(def) = AGENT_DEFS.iter().find(|d| Some(d.name) == g.agent)
                && def.tools.iter().all(|t| h.deny.iter().any(|d| d == t))
            {
                return Err(format!(
                    "`harness.deny` removes each tool of the agent `{}`; remove a tool from `deny`, or turn the agent off",
                    def.name
                ));
            }
        }
        Ok(h)
    }

    /// Returns the warnings of a configuration. They are not errors.
    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.level == Level::None {
            return out;
        }
        let (orch, adv) = (self.cfg(Role::Orchestrator), self.cfg(Role::Advisor));
        if let (Model::Id(o), Model::Id(a)) = (&orch.model, &adv.model)
            && let (Some(ro), Some(ra)) = (rank(o), rank(a))
            && ra < ro
        {
            out.push(format!(
                "the advisor ({a}) ranks below the main model ({o}); advice from a weaker model has little value"
            ));
        }
        if let Model::Id(o) = &orch.model
            && orch.effort != Effort::Inherit
            && model_id(o).is_none()
        {
            out.push(format!(
                "the effort of the main model is not written: `{o}` has no model id for `modelSettings`; set the effort with `/effort`"
            ));
        }
        out
    }

    /// Writes the `[harness]` section for `spec-dna.toml`. `parse` reads it
    /// back to an equal value.
    pub fn toml_section(&self) -> String {
        let mut s = format!("[harness]\nlevel = {}\n", quote(self.level.name()));
        if self.level == Level::None {
            return s;
        }
        let nums = |v: &[u32]| v.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
        let strs = |v: &[String]| v.iter().map(|x| quote(x)).collect::<Vec<_>>().join(", ");
        s += &format!("deny = [{}]\n", strs(&self.deny));
        s += &format!("context-meter = [{}]\n", nums(&self.context_meter));
        s += &format!("subagent-meter = {}\n", self.subagent_meter);
        s += &format!("fanout-cap = {}\n\n[harness.roles]\n", self.fanout_cap);
        for r in Role::ALL.into_iter().filter(|r| self.level.includes(*r)) {
            let c = self.cfg(r);
            let mut parts = vec![format!("claude-code = {}", quote(c.model.name()))];
            if r == Role::Advisor {
                parts.push(format!("agent = {}", quote(c.agent.name())));
            }
            if r != Role::Helper {
                parts.push(format!("effort = {}", quote(c.effort.name())));
            }
            if r == Role::Orchestrator {
                parts.push(format!("fallback = [{}]", strs(&c.fallback)));
            }
            s += &format!("{} = {{ {} }}\n", r.key(), parts.join(", "));
        }
        s += "\n[harness.efforts]\n";
        for (m, e) in &self.efforts {
            s += &format!("{} = {}\n", toml_key(m), quote(e.name()));
        }
        s
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

fn toml_key(s: &str) -> String {
    if !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        s.to_string()
    } else {
        quote(s)
    }
}

fn parse_role(role: Role, t: &toml::Table) -> Result<RoleCfg, String> {
    let prefix = format!("harness.roles.{}.", role.key());
    let mut f = Fields::new(t, &prefix);
    let mut cfg = RoleCfg::new(role);
    let model = |s: &str, key: &str| -> Result<Model, String> {
        token(s, &format!("{prefix}{key}"), "a model")?;
        Ok(Model::parse(s))
    };
    if let Some(s) = f.opt_str("claude-code")? {
        cfg.model = model(&s, "claude-code")?;
    }
    if role != Role::Helper
        && let Some(s) = f.opt_str("effort")?
    {
        cfg.effort = Effort::from_name(&s).ok_or_else(|| {
            format!(
                "`{prefix}effort` must be \"low\", \"medium\", \"high\", \"xhigh\", \"max\" or \"inherit\""
            )
        })?;
    }
    if role == Role::Advisor
        && let Some(s) = f.opt_str("agent")?
    {
        cfg.agent = model(&s, "agent")?;
    }
    if role == Role::Orchestrator
        && let Some(v) = f.take("fallback")
    {
        let arr = v
            .as_array()
            .ok_or_else(|| format!("`{prefix}fallback` must be an array of models"))?;
        cfg.fallback.clear();
        for x in arr {
            let s = x
                .as_str()
                .ok_or_else(|| format!("`{prefix}fallback` must be an array of models"))?;
            token(s, &format!("{prefix}fallback"), "a model")?;
            if matches!(s, "none" | "inherit") {
                return Err(format!("`{prefix}fallback` cannot hold `{s}`"));
            }
            cfg.fallback.push(s.to_string());
        }
    }
    f.done()?;
    let key = |k: &str| format!("`{prefix}{k}`");
    match (role, &cfg.model) {
        (Role::Orchestrator, Model::None) => {
            return Err(format!("{} cannot be \"none\"", key("claude-code")));
        }
        (Role::Advisor, Model::Inherit) => {
            return Err(format!("{} cannot be \"inherit\"", key("claude-code")));
        }
        (Role::Advisor, Model::Id(m)) if m.contains("haiku") => {
            return Err(format!(
                "{} cannot be \"{m}\": the advisor model cannot be haiku",
                key("claude-code")
            ));
        }
        _ => {}
    }
    if role == Role::Orchestrator && cfg.effort == Effort::Max {
        return Err(format!("{} cannot be \"max\"", key("effort")));
    }
    Ok(cfg)
}

// ---- The generated agent files ----

/// The text of one generated agent file, without the frontmatter.
pub struct AgentDef {
    pub role: Role,
    pub name: &'static str,
    pub tools: &'static [&'static str],
    pub omit_claude_md: bool,
    pub description: &'static str,
    pub body: &'static str,
}

impl AgentDef {
    pub fn path(&self) -> String {
        format!("{AGENTS_DIR}{}.md", self.name)
    }
}

const AGENT_DEFS: [AgentDef; 5] = [
    AgentDef {
        role: Role::Advisor,
        name: "fallback-advisor",
        tools: &["Read", "Bash"],
        omit_claude_md: false,
        description: "Gives advice at a decision point - the approach to take, the cause of a repeated error, or whether a task is complete. It reads files and runs read-only commands. It never edits. Use it when the advisor tool is not available.",
        body: r"You give advice to the main session. You do not see its conversation. Use only the brief and the repository. Do not change files or git state.

1. Read the goal, the current state, the options and the question in the brief.
2. Verify each fact that your advice needs. Read the file or run a read-only command. If a fact is missing, ask for it and stop.
3. Prefer the simplest option that meets the goal. Name the risk that can make it fail.

Answer in 200 words or fewer:
- Advice: the next 1 to 3 steps.
- Reason: the evidence for them.
- Risk: what can fail, and the check that shows it.",
    },
    AgentDef {
        role: Role::Reviewer,
        name: "checker",
        tools: &["Read", "Bash", "Monitor", "TaskStop"],
        omit_claude_md: false,
        description: "Reviews finished work independently. It runs the tests, examines the diff, probes edge cases and lists each defect. It never edits. Use it before you report that a task is complete.",
        body: r"You review work that another agent reports as complete. Do not change files or git state. Put temporary files in a temporary directory, and remove them at the end.

1. Find the change: `git status`, `git diff`, and the files in the brief.
2. Run the tests and checks that cover the change. Use the commands of the project instruction files.
3. Probe the change with bad input, limits, missing files and errors. Run each probe.
4. Compare the change with the brief. Report missing work and extra work.

Report:
- Each defect: the file and line or the command, the expected result, and the actual result.
- Each check that you could not run, with the reason.
- If you found no defect, list the commands that you ran.",
    },
    AgentDef {
        role: Role::Researcher,
        name: "Explore",
        tools: &["Read", "Grep", "Glob", "Bash", "WebFetch", "WebSearch"],
        omit_claude_md: true,
        description: "Searches this repository and the web, and only reads. It returns paths, short quotes and URLs. Use it for a search that needs more than one command. Give the depth - quick, medium or thorough.",
        body: r"You find information and report it. Do not change files.

- Search with `git grep`, `git ls-files` or the search tools. Use Bash only for commands that read.
- Read a large file in parts.
- Do not print secrets. For a file that holds keys or tokens, give the path only.
- Stop when you can answer the question of the brief.

Answer:
- Result: 1 or 2 lines.
- Evidence: `path:line` with a short quote, or a URL.
- Not found: what you did not find or did not search.",
    },
    AgentDef {
        role: Role::Implementer,
        name: "implementer",
        tools: &["Read", "Grep", "Glob", "Bash", "Edit", "Write"],
        omit_claude_md: false,
        description: "Writes the code and the tests of one delegated task. Then it runs the project checks. Use it for a code change that the main session delegates.",
        body: r"You make the one change that the brief describes. Stay inside the files and the scope of the brief.

1. Start with the files in the brief. Open other files only when the change needs them.
2. For a defect, write the test first. Run it and check that it fails.
3. Make the change. Keep it small. Follow the conventions of the files that you edit.
4. Run the tests and checks that the project instruction files name. Fix each failure.
5. If the task is not clear, or a check fails for a reason outside the brief, stop and report.

Report: the files that you changed, the commands that you ran with their result, and the open issues. Do not commit.",
    },
    AgentDef {
        role: Role::SpecWriter,
        name: "spec-writer",
        tools: &["Read", "Grep", "Glob", "Bash", "Edit", "Write"],
        omit_claude_md: false,
        description: "Writes and updates the specifications of the project with spec-dna. Use it for a new spec, decision or runbook, or to keep the specs true after a code change.",
        body: r"You keep the specifications true. Use spec-dna for each step.

1. Run `spec-dna context <files>` for the files of the change. Read a listed section with `spec-dna show <id>#<anchor>` only when you need it.
2. Run `spec-dna show --conventions <type>` before you edit a document of that type.
3. For a new document, run `spec-dna new <type> <id>`. Then replace each placeholder.
4. Change an accepted decision only in its amendment section, or write a new decision that supersedes it.
5. Run `spec-dna check`. Fix each error.

Report: the documents that you changed, and the output of `spec-dna check`. Do not commit.",
    },
];

fn marker_text(before: &str) -> String {
    format!("{OPEN}{VERSION} h:{} -->", hex(&hash_normalized(before)))
}

/// Returns the position of a valid marker at the end of a line, with its
/// version and hash. Prose that quotes the form of the marker does not match.
fn marker_at_end(line: &str) -> Option<(usize, &str, &str)> {
    let m = line.rfind(OPEN)?;
    let inner = line[m + OPEN.len()..].strip_suffix(" -->")?;
    let (version, hash) = inner.split_once(" h:")?;
    let ok = !version.is_empty()
        && version
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-+".contains(&b))
        && hash.len() == 16
        && hash.bytes().all(|b| b.is_ascii_hexdigit());
    ok.then_some((m, version, hash))
}

struct FileMarker<'a> {
    /// The offset of the marker line.
    start: usize,
    version: &'a str,
    hash: &'a str,
    /// The text above the marker line.
    before: &'a str,
    /// The text after the marker line.
    after: &'a str,
}

impl FileMarker<'_> {
    /// Returns true if a person edited the file: the text above the marker no
    /// longer has the recorded hash, or text follows the marker.
    fn edited(&self) -> bool {
        hex(&hash_normalized(self.before)) != self.hash || !self.after.trim().is_empty()
    }
}

/// Finds the marker of a generated file: the last line that has a marker and
/// nothing else. Text after it is a hand edit.
fn file_marker(text: &str) -> Option<FileMarker<'_>> {
    let mut found = None;
    let mut start = 0;
    for l in text.split_inclusive('\n') {
        let end = start + l.len();
        if let Some((0, version, hash)) = marker_at_end(l.trim_end()) {
            found = Some(FileMarker {
                start,
                version,
                hash,
                before: &text[..start],
                after: &text[end..],
            });
        }
        start = end;
    }
    found
}

/// Returns true if the text of a file is a generated harness file.
pub fn has_marker(text: &str) -> bool {
    file_marker(text).is_some()
}

fn line_of(text: &str, pos: usize) -> u32 {
    text[..pos].matches('\n').count() as u32 + 1
}

fn render_agent(def: &AgentDef, model: &Model, effort: Effort, deny: &[String]) -> String {
    let mut s = format!(
        "---\nname: {}\ndescription: {}\nmodel: {}\n",
        def.name,
        def.description,
        model.name()
    );
    if effort != Effort::Inherit {
        s += &format!("effort: {}\n", effort.name());
    }
    let tools: Vec<&str> = def
        .tools
        .iter()
        .copied()
        .filter(|t| !deny.iter().any(|d| d == t))
        .collect();
    s += &format!("tools: {}\n", tools.join(", "));
    if def.omit_claude_md {
        s += "omitClaudeMd: true\n";
    }
    s += &format!("---\n\n{}\n", def.body);
    format!("{s}\n{}\n", marker_text(&s))
}

fn shown(m: &Model) -> &str {
    match m {
        Model::Inherit => "inherit",
        other => other.name(),
    }
}

fn team_page(h: &Harness) -> String {
    let mut rows = Vec::new();
    let eff = |e: Effort| e.name().to_string();
    let c = |r: Role| h.cfg(r);
    rows.push(format!(
        "| orchestrator | `model` in `{SETTINGS}` | {} | {} |",
        shown(&c(Role::Orchestrator).model),
        eff(c(Role::Orchestrator).effort)
    ));
    if h.on(Role::Advisor) {
        rows.push(format!(
            "| advisor | `advisorModel` in `{SETTINGS}` | {} | - |",
            shown(&c(Role::Advisor).model)
        ));
        if c(Role::Advisor).agent != Model::None {
            rows.push(format!(
                "| advisor fallback | `{AGENTS_DIR}fallback-advisor.md` | {} | {} |",
                shown(&c(Role::Advisor).agent),
                eff(c(Role::Advisor).effort)
            ));
        }
    }
    for r in [
        Role::Reviewer,
        Role::Researcher,
        Role::Implementer,
        Role::SpecWriter,
    ] {
        if let (true, Some(def)) = (h.on(r), r.agent()) {
            rows.push(format!(
                "| {} | `{}` | {} | {} |",
                r.key(),
                def.path(),
                shown(&c(r).model),
                eff(c(r).effort)
            ));
        }
    }
    if h.on(Role::Helper) && c(Role::Helper).model != Model::Inherit {
        rows.push(format!(
            "| helper | `env.CLAUDE_CODE_SUBAGENT_MODEL` | {} | - |",
            shown(&c(Role::Helper).model)
        ));
    }
    let mut s = format!(
        "# Model team\n\nThis page describes the agent harness of this project for people. Agents do not need to read it.\n\n| Role | File or setting | Model | Effort |\n| --- | --- | --- | --- |\n{}\n",
        rows.join("\n")
    );
    if h.on(Role::Advisor) {
        let adv = h.cfg(Role::Advisor);
        s += "\n## Advisor\n\nAt each consult, the advisor reads the whole context. The price of the advisor model applies.";
        if adv.agent != Model::None {
            s += " If the advisor tool is missing, fails or declines, the main model briefs `fallback-advisor`.";
        }
        s += "\n";
    }
    s += "\n## Subagent model\n\nA subagent that has no model of its own uses the helper model. An Agent tool call with a `model` parameter overrides it.\n";
    if !h.cfg(Role::Orchestrator).fallback.is_empty() {
        s += "\n## Outage\n\n`fallbackModel` replaces the main model only when it is overloaded or not available.\n";
    }
    if let (true, Model::Id(m)) = (h.on(Role::Advisor), &h.cfg(Role::Advisor).model) {
        s += &format!(
            "\n## No access to the advisor model\n\nRun `/model {m}` once and approve the usage billing. Then select the main model again. Until then, `fallback-advisor` gives the advice.\n"
        );
    }
    s += "\n## Precedence\n\nThis project file has priority over the user settings. The options `--model`, `--effort` and `--advisor`, and a resumed session, have priority over this project file.\n";
    s += "\n## Reload\n\nClaude Code reloads `advisorModel`, `fallbackModel`, `env` and the `.claude/agents` directory while it runs. `model` and `modelSettings` apply at the next start.\n";
    if h.context_hook_on() || h.fanout_hook_on() {
        s += "\n## Hooks\n\n";
        if h.context_hook_on() {
            s += "`context-meter` tells an agent the size of its context once at each level. ";
        }
        if h.fanout_hook_on() {
            s += "`fanout-cap` asks the person before a workflow whose agent count can exceed the cap. ";
        }
        s += "Each teammate needs spec-dna 0.5.0 or later on `PATH`. Without it, the hooks do nothing.\n";
    }
    s += "\n## Check after a restart\n\n1. Read the header. It shows the main model.\n2. Run `/advisor`. It shows the advisor.\n3. Type `@`. The list shows the agents.\n4. Run `/tasks`. It lists the running agents.\n5. When Claude consults the advisor, an `Advising` line shows. Press `Ctrl+O` to read the advice.\n6. Run `/usage`. It shows the cost of each model.\n";
    s += "\nGenerated by spec-dna. Change `[harness]` in `spec-dna.toml` and run `spec-dna init --reconfigure`.\n";
    format!("{s}\n{}\n", marker_text(&s))
}

/// A file that the harness generates as a whole.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    pub path: String,
    pub text: String,
    /// The agent name, or `None` for the team page.
    pub agent: Option<&'static str>,
    /// A short description for a preview.
    pub note: String,
}

/// The files that a configuration generates as a whole: the agent files in the
/// order of the roles, and the team page. Level `none` gives no file.
pub fn generated_files(h: &Harness) -> Vec<Generated> {
    let mut out = Vec::new();
    if h.level == Level::None {
        return out;
    }
    for def in &AGENT_DEFS {
        if !h.on(def.role) {
            continue;
        }
        let c = h.cfg(def.role);
        let model = if def.role == Role::Advisor {
            &c.agent
        } else {
            &c.model
        };
        if *model == Model::None {
            continue;
        }
        let note = if c.effort == Effort::Inherit {
            format!("agent {}: {}", def.name, model.name())
        } else {
            format!("agent {}: {}, {}", def.name, model.name(), c.effort.name())
        };
        out.push(Generated {
            path: def.path(),
            text: render_agent(def, model, c.effort, &h.deny),
            agent: Some(def.name),
            note,
        });
    }
    out.push(Generated {
        path: TEAM_PAGE.to_string(),
        text: team_page(h),
        agent: None,
        note: "team page".to_string(),
    });
    out
}

/// Returns true for a path that the harness can write: an agent file in
/// `.claude/agents`, `.claude/settings.json` and `CLAUDE.md`. `init` and
/// `check` read these paths.
pub fn wanted(path: &str) -> bool {
    path == CLAUDE
        || path == SETTINGS
        || path
            .strip_prefix(AGENTS_DIR)
            .is_some_and(|r| r.ends_with(".md") && !r.contains('/'))
}

/// Returns true for a path that `init` writes for the harness, whatever the
/// configuration is. A change of `spec-dna.toml` with these files is still a
/// change of its own (SD503).
pub fn is_output_path(path: &str) -> bool {
    path == CLAUDE
        || path == SETTINGS
        || path == TEAM_PAGE
        || AGENT_DEFS.iter().any(|d| d.path() == path)
}

// ---- The line of CLAUDE.md ----

fn shown_model(m: &Model) -> String {
    match m {
        Model::Id(s) => s.clone(),
        _ => "the main model".to_string(),
    }
}

/// The text of the model-team line, without the marker.
fn claude_body(h: &Harness) -> String {
    let adv = h.on(Role::Advisor);
    let mut s = format!(
        "Model team: {} builds. ",
        shown_model(&h.cfg(Role::Orchestrator).model)
    );
    if adv {
        s += &format!(
            "The advisor ({}) gives advice and writes no code. ",
            shown_model(&h.cfg(Role::Advisor).model)
        );
    }
    if h.on(Role::Reviewer) {
        s += &format!(
            "`checker` ({}) tests finished work. ",
            shown_model(&h.cfg(Role::Reviewer).model)
        );
    }
    if h.on(Role::Researcher) {
        s += &format!(
            "`Explore` ({}) searches. ",
            shown_model(&h.cfg(Role::Researcher).model)
        );
    }
    if adv {
        s += "Consult the advisor before you choose an approach, before you say that a task is done, and when the same error occurs twice. Each consult sends the whole context at the advisor price, so consult before the context grows large. ";
        if h.cfg(Role::Advisor).agent != Model::None {
            s += "If the advisor tool is missing, fails or declines, brief `fallback-advisor`. ";
        }
    }
    if h.on(Role::Reviewer) {
        s += "Run `checker` before you say that a task is done. A trivial task needs only the one command that proves it. ";
    }
    if h.on(Role::Researcher) {
        s += "Send a search that needs more than one command, and a docs search, to `Explore`. Run a one-command search yourself. ";
    }
    match (h.on(Role::Implementer), h.on(Role::SpecWriter)) {
        (true, true) => {
            s += "Give a code change to `implementer` and a spec change to `spec-writer`, one writer at a time. "
        }
        (true, false) => s += "Give a code change to `implementer`. ",
        (false, true) => s += "Give a spec change to `spec-writer`. ",
        _ => {}
    }
    s += "Give a subagent that writes code or reviews work the model `sonnet` or `opus`, not `haiku`. For a small task that fits in this session, use no subagent. ";
    if adv {
        s += "If a file contradicts the advice, say so in one more consult. ";
    }
    s
}

/// The model-team line of `CLAUDE.md`. Level `none` gives `None`.
pub fn claude_line(h: &Harness) -> Option<String> {
    if h.level == Level::None {
        return None;
    }
    let body = claude_body(h);
    Some(format!("{body}{}", marker_text(&body)))
}

/// Finds the line of `CLAUDE.md` that ends with a harness marker. Returns
/// (start, end with the line break, version, recorded hash, text before the
/// marker). The marker of the spec-dna line (`<!-- spec-dna v`) does not match.
fn find_line(text: &str) -> Option<(usize, usize, &str, &str, &str)> {
    let mut start = 0;
    for l in text.split_inclusive('\n') {
        let end = start + l.len();
        let body = l.trim_end_matches(['\n', '\r']);
        if let Some((m, version, hash)) = marker_at_end(body) {
            return Some((start, end, version, hash, body[..m].trim_end()));
        }
        start = end;
    }
    None
}

/// Puts the model-team line into the text of `CLAUDE.md`, or removes it for
/// level `none`. A new line goes at the end, after a blank line.
pub fn upsert_claude(text: &str, h: &Harness) -> String {
    let Some(new) = claude_line(h) else {
        return remove_claude_line(text);
    };
    if let Some((start, end, ..)) = find_line(text) {
        return format!("{}{new}\n{}", &text[..start], &text[end..]);
    }
    if text.trim().is_empty() {
        return format!("{new}\n");
    }
    let sep = if text.ends_with("\n\n") {
        ""
    } else if text.ends_with('\n') {
        "\n"
    } else {
        "\n\n"
    };
    format!("{text}{sep}{new}\n")
}

/// Removes the model-team line and the blank line before it.
pub fn remove_claude_line(text: &str) -> String {
    match find_line(text) {
        Some((start, end, ..)) => {
            let start = if text[..start].ends_with("\n\n") {
                start - 1
            } else {
                start
            };
            format!("{}{}", &text[..start], &text[end..])
        }
        None => text.to_string(),
    }
}

// ---- The merge into .claude/settings.json ----

const CONTEXT_HOOK: &str = "context-meter";
const FANOUT_HOOK: &str = "fanout-cap";

/// One entry that the harness expects in `.claude/settings.json`.
#[derive(Clone, Debug, PartialEq)]
enum Want {
    Set {
        path: Vec<String>,
        value: Value,
    },
    Deny(String),
    /// A hook handler. No command: the hook is off.
    Hook {
        event: &'static str,
        matcher: &'static str,
        name: &'static str,
        command: Option<String>,
    },
}

impl Want {
    /// The JSON path, for a message.
    fn label(&self) -> String {
        match self {
            Want::Set { path, .. } => path.join("."),
            Want::Deny(t) => format!("permissions.deny[{t}]"),
            Want::Hook { event, name, .. } => format!("hooks.{event} (spec-dna hook {name})"),
        }
    }
}

fn s(x: &str) -> Value {
    Value::Str(x.to_string())
}

fn put(wants: &mut Vec<Want>, path: &[&str], value: Value) {
    let path: Vec<String> = path.iter().map(|p| p.to_string()).collect();
    if let Some(Want::Set { value: v, .. }) = wants
        .iter_mut()
        .find(|w| matches!(w, Want::Set { path: p, .. } if *p == path))
    {
        *v = value;
    } else {
        wants.push(Want::Set { path, value });
    }
}

fn context_command(h: &Harness) -> String {
    let mut c = String::from("spec-dna hook context-meter");
    if !h.context_meter.is_empty() {
        let l: Vec<String> = h.context_meter.iter().map(u32::to_string).collect();
        c += &format!(" --levels {}", l.join(","));
    }
    if h.subagent_meter > 0 {
        c += &format!(" --subagent {}", h.subagent_meter);
    }
    c + " 2>/dev/null || true"
}

fn wants(h: &Harness) -> Vec<Want> {
    let mut w = Vec::new();
    if h.level != Level::None {
        for (m, e) in &h.efforts {
            if let (Some(id), true) = (model_id(m), *e != Effort::Inherit) {
                put(&mut w, &["modelSettings", &id, "effortLevel"], s(e.name()));
            }
        }
        let orch = h.cfg(Role::Orchestrator);
        if let Model::Id(m) = &orch.model {
            put(&mut w, &["model"], s(m));
            if let (Some(id), true) = (model_id(m), orch.effort != Effort::Inherit) {
                put(
                    &mut w,
                    &["modelSettings", &id, "effortLevel"],
                    s(orch.effort.name()),
                );
            }
        }
        if !orch.fallback.is_empty() {
            put(
                &mut w,
                &["fallbackModel"],
                Value::Arr(orch.fallback.iter().map(|m| s(m)).collect()),
            );
        }
        if let (true, Model::Id(m)) = (h.on(Role::Advisor), &h.cfg(Role::Advisor).model) {
            put(&mut w, &["advisorModel"], s(m));
        }
        if let (true, Model::Id(m)) = (h.on(Role::Helper), &h.cfg(Role::Helper).model) {
            put(&mut w, &["env", "CLAUDE_CODE_SUBAGENT_MODEL"], s(m));
        }
        for d in &h.deny {
            w.push(Want::Deny(d.clone()));
        }
    }
    w.push(Want::Hook {
        event: "PostToolUse",
        matcher: "*",
        name: CONTEXT_HOOK,
        command: h.context_hook_on().then(|| context_command(h)),
    });
    w.push(Want::Hook {
        event: "PreToolUse",
        matcher: "Workflow",
        name: FANOUT_HOOK,
        command: h.fanout_hook_on().then(|| {
            format!(
                "spec-dna hook fanout-cap --cap {} 2>/dev/null || true",
                h.fanout_cap
            )
        }),
    });
    w
}

fn get_mut<'a>(v: &'a mut Value, key: &str) -> Option<&'a mut Value> {
    match v {
        Value::Obj(f) => f.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v),
        _ => None,
    }
}

/// Returns the value of a key of an object. A missing key, or a value that is
/// not an object, gives a new empty object or replaces the value.
fn entry<'a>(v: &'a mut Value, key: &str) -> &'a mut Value {
    if !matches!(v, Value::Obj(_)) {
        *v = Value::Obj(Vec::new());
    }
    match v {
        Value::Obj(f) => {
            let i = match f.iter().position(|(k, _)| k == key) {
                Some(i) => i,
                None => {
                    f.push((key.to_string(), Value::Null));
                    f.len() - 1
                }
            };
            &mut f[i].1
        }
        _ => unreachable!("the value is an object"),
    }
}

fn get_path<'a>(doc: &'a Value, path: &[String]) -> &'a Value {
    path.iter().fold(doc, |v, k| v.get(k))
}

/// Returns true if a handler runs `spec-dna hook <name>`.
fn is_handler_of(handler: &Value, name: &str) -> bool {
    handler
        .get("command")
        .as_str()
        .and_then(|c| c.strip_prefix("spec-dna hook "))
        .and_then(|c| c.strip_prefix(name))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
}

fn apply_want(doc: &mut Value, want: &Want, remove_off: bool) {
    match want {
        Want::Set { path, value } => {
            let mut cur = &mut *doc;
            for k in path {
                cur = entry(cur, k);
            }
            *cur = value.clone();
        }
        Want::Deny(t) => {
            let deny = entry(entry(doc, "permissions"), "deny");
            if !matches!(deny, Value::Arr(_)) {
                *deny = Value::Arr(Vec::new());
            }
            if let Value::Arr(a) = deny
                && !a.iter().any(|x| x.as_str() == Some(t))
            {
                a.push(s(t));
            }
        }
        Want::Hook {
            event,
            matcher,
            name,
            command: Some(cmd),
        } => {
            let groups = entry(entry(doc, "hooks"), event);
            if !matches!(groups, Value::Arr(_)) {
                *groups = Value::Arr(Vec::new());
            }
            let Value::Arr(groups) = groups else { return };
            let mut found = false;
            for g in groups.iter_mut() {
                if let Some(Value::Arr(hs)) = get_mut(g, "hooks") {
                    for h in hs.iter_mut().filter(|h| is_handler_of(h, name)) {
                        found = true;
                        *entry(h, "command") = s(cmd);
                    }
                }
            }
            if !found {
                let handler = Value::Obj(vec![
                    ("type".into(), s("command")),
                    ("command".into(), s(cmd)),
                ]);
                groups.push(Value::Obj(vec![
                    ("matcher".into(), s(matcher)),
                    ("hooks".into(), Value::Arr(vec![handler])),
                ]));
            }
        }
        Want::Hook {
            event,
            name,
            command: None,
            ..
        } => {
            if !remove_off {
                return;
            }
            let Some(hooks) = get_mut(doc, "hooks") else {
                return;
            };
            let Some(Value::Arr(groups)) = get_mut(hooks, event) else {
                return;
            };
            groups.retain_mut(|g| {
                let Some(Value::Arr(hs)) = get_mut(g, "hooks") else {
                    return true;
                };
                let before = hs.len();
                hs.retain(|h| !is_handler_of(h, name));
                before == hs.len() || !hs.is_empty()
            });
        }
    }
}

fn satisfied(doc: &Value, want: &Want) -> bool {
    match want {
        Want::Set { path, value } => get_path(doc, path) == value,
        Want::Deny(t) => doc
            .get("permissions")
            .get("deny")
            .as_arr()
            .iter()
            .any(|x| x.as_str() == Some(t)),
        Want::Hook {
            event,
            name,
            command: Some(cmd),
            ..
        } => {
            // Each handler of the name must have the command, as `init` sets
            // it in each of them. One handler at least must exist.
            let handlers: Vec<&Value> = doc
                .get("hooks")
                .get(event)
                .as_arr()
                .iter()
                .flat_map(|g| g.get("hooks").as_arr())
                .filter(|h| is_handler_of(h, name))
                .collect();
            !handlers.is_empty()
                && handlers
                    .iter()
                    .all(|h| h.get("command").as_str() == Some(cmd))
        }
        Want::Hook { command: None, .. } => true,
    }
}

/// Lists the keys that an old configuration set and a new one does not, and
/// that the file still has. spec-dna never deletes them.
fn kept_keys(doc: &Value, old: &Harness, new: &Harness) -> Vec<String> {
    let now = wants(new);
    let mut out = Vec::new();
    for w in wants(old) {
        let still = now.iter().any(|n| match (&w, n) {
            (Want::Set { path: a, .. }, Want::Set { path: b, .. }) => a == b,
            (Want::Deny(a), Want::Deny(b)) => a == b,
            _ => false,
        });
        let present = match &w {
            Want::Set { path, .. } => *get_path(doc, path) != Value::Null,
            Want::Deny(_) => satisfied(doc, &w),
            Want::Hook { .. } => false,
        };
        if !still && present {
            out.push(format!(
                "kept {}: spec-dna no longer sets it; remove it by hand if you do not want it",
                w.label()
            ));
        }
    }
    out
}

/// Returns an error for a container that spec-dna writes into and that has the
/// wrong type. `init` would replace it and lose its data. `null` is no data.
fn shape_error(doc: &Value, ws: &[Want]) -> Option<String> {
    let check = |path: Vec<String>, array: bool| -> Option<String> {
        for i in 0..path.len() {
            let last = i + 1 == path.len();
            let ok = match get_path(doc, &path[..=i]) {
                Value::Null => true,
                Value::Arr(_) => last && array,
                Value::Obj(_) => !(last && array),
                _ => false,
            };
            if !ok {
                return Some(format!(
                    "{SETTINGS} is not valid for the harness: `{}` must be {}",
                    path[..=i].join("."),
                    if last && array {
                        "an array"
                    } else {
                        "an object"
                    }
                ));
            }
        }
        None
    };
    let path = |keys: &[&str]| keys.iter().map(|k| k.to_string()).collect::<Vec<_>>();
    ws.iter().find_map(|w| match w {
        Want::Set { path: p, .. } => check(p[..p.len() - 1].to_vec(), false),
        Want::Deny(_) => check(path(&["permissions", "deny"]), true),
        Want::Hook {
            event,
            command: Some(_),
            ..
        } => check(path(&["hooks", event]), true),
        Want::Hook { .. } => None,
    })
}

fn parse_settings(text: &str) -> Result<Value, String> {
    let v = json::parse(text).map_err(|e| format!("{SETTINGS} is not valid JSON: {e}"))?;
    if !matches!(v, Value::Obj(_)) {
        return Err(format!(
            "{SETTINGS} is not valid for the harness: its root is not an object"
        ));
    }
    Ok(v)
}

// ---- The plan of `init` ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Create,
    Change,
    Remove,
}

impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Change => "change",
            Action::Remove => "remove",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileChange {
    pub path: String,
    pub action: Action,
    /// A short description for the preview.
    pub note: String,
    /// The new text of the file. Empty for `Remove`.
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// `init` with an existing configuration. It writes and never removes.
    Upgrade,
    /// A new configuration, or `init --reconfigure`. It also removes the
    /// generated files that the configuration no longer produces.
    Reconfigure,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Plan {
    pub changes: Vec<FileChange>,
    /// Lines that start with `kept`: a key or a file that spec-dna leaves.
    pub kept: Vec<String>,
    pub warnings: Vec<String>,
    /// The bytes that the harness adds to each request of the main session:
    /// the line of `CLAUDE.md` and the agent descriptions.
    pub request_bytes: usize,
}

/// Returns the name of an agent file: the `name` of its front matter, else the
/// file stem.
fn agent_name(path: &str, text: &str) -> String {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) == Some("---") {
        for l in lines {
            if l.trim_end() == "---" {
                break;
            }
            if let Some(v) = l.strip_prefix("name:") {
                return v.trim().trim_matches(['"', '\'']).to_string();
            }
        }
    }
    let file = path.rsplit('/').next().unwrap_or(path);
    file.strip_suffix(".md").unwrap_or(file).to_string()
}

/// Finds the roles whose agent name belongs to a file that spec-dna did not
/// generate. `existing` holds (path, text) of the files that exist. spec-dna
/// never replaces such a file. Returns (role, path of the file).
pub fn name_conflicts(existing: &[(String, String)]) -> Vec<(Role, String)> {
    let mut out = Vec::new();
    for def in &AGENT_DEFS {
        for (p, t) in existing {
            if p.starts_with(AGENTS_DIR)
                && wanted(p)
                && !has_marker(t)
                && (*p == def.path() || agent_name(p, t) == def.name)
            {
                out.push((def.role, p.clone()));
            }
        }
    }
    out
}

/// Returns true if an agent file or the line of `CLAUDE.md` exists.
pub fn is_installed(existing: &[(String, String)]) -> bool {
    existing.iter().any(|(p, t)| {
        (p.starts_with(AGENTS_DIR) && has_marker(t)) || (p == CLAUDE && find_line(t).is_some())
    })
}

enum State {
    NoMarker,
    Edited,
    Outdated,
    Current,
}

fn file_state(actual: &str, expected: &str) -> State {
    match file_marker(actual) {
        None => State::NoMarker,
        Some(m) if m.edited() => State::Edited,
        Some(_) if actual == expected => State::Current,
        Some(_) => State::Outdated,
    }
}

/// Plans the harness files of a configuration. `existing` holds (path, text)
/// of each existing file that `wanted` accepts. `old` is the configuration that
/// wrote the files, if any. For `CLAUDE.md`, give the text after `init` has
/// put its own line into it.
///
/// A file with a manual edit stays, and the plan names it in `kept`. A file at
/// the path of an agent, or with the name of an agent, that spec-dna did not
/// generate gives an error, as does a `.claude/settings.json` that is not valid
/// JSON.
pub fn plan(
    existing: &[(String, String)],
    old: Option<&Harness>,
    new: &Harness,
    mode: Mode,
) -> Result<Plan, String> {
    let get = |p: &str| existing.iter().find(|e| e.0 == p).map(|e| e.1.as_str());
    let expected = generated_files(new);
    let mut plan = Plan::default();
    for (role, path) in name_conflicts(existing) {
        // The advisor with no `fallback-advisor` agent writes no file there.
        if let Some(def) = role.agent()
            && expected.iter().any(|g| g.path == def.path())
        {
            return Err(format!(
                "{path} holds the agent `{}`, and spec-dna did not generate it; rename the agent, or set the role `{}` to \"none\" in [harness.roles]",
                def.name,
                role.key()
            ));
        }
    }
    for g in &expected {
        match get(&g.path) {
            None => plan.changes.push(FileChange {
                path: g.path.clone(),
                action: Action::Create,
                note: g.note.clone(),
                text: g.text.clone(),
            }),
            Some(t) => match file_state(t, &g.text) {
                State::Current => {}
                State::Edited => plan.kept.push(format!(
                    "kept {}: edited by hand (SD501); delete it and run `spec-dna init` to generate it again",
                    g.path
                )),
                State::Outdated => plan.changes.push(FileChange {
                    path: g.path.clone(),
                    action: Action::Change,
                    note: g.note.clone(),
                    text: g.text.clone(),
                }),
                State::NoMarker => {
                    return Err(format!(
                        "{} exists, and spec-dna did not generate it; move it, or turn the harness off",
                        g.path
                    ));
                }
            },
        }
    }
    if mode == Mode::Reconfigure {
        for (p, t) in existing {
            if !p.starts_with(AGENTS_DIR) || !wanted(p) || expected.iter().any(|g| g.path == *p) {
                continue;
            }
            let Some(m) = file_marker(t) else { continue };
            if !m.edited() {
                plan.changes.push(FileChange {
                    path: p.clone(),
                    action: Action::Remove,
                    note: "the configuration does not generate it".into(),
                    text: String::new(),
                });
            } else {
                plan.kept.push(format!("kept {p}: edited by hand (SD501)"));
            }
        }
    }
    claude_change(&mut plan, get(CLAUDE), new, mode);
    settings_change(&mut plan, get(SETTINGS), old, new, mode)?;
    plan.warnings = new.warnings();
    plan.request_bytes = claude_line(new).map_or(0, |l| l.len() + 1)
        + generated_files(new)
            .iter()
            .filter_map(|g| {
                let def = AGENT_DEFS.iter().find(|d| Some(d.name) == g.agent)?;
                Some(format!("- {}: {}\n", def.name, def.description).len())
            })
            .sum::<usize>();
    Ok(plan)
}

fn claude_change(plan: &mut Plan, current: Option<&str>, new: &Harness, mode: Mode) {
    if new.level == Level::None && mode == Mode::Upgrade {
        return;
    }
    if let Some((start, _, _, hash, before)) = current.and_then(find_line) {
        if hex(&hash_normalized(before)) != hash {
            let line = line_of(current.unwrap_or(""), start);
            plan.kept.push(format!(
                "kept {CLAUDE}:{line}: edited by hand (SD501); delete the line and run `spec-dna init` to generate it again"
            ));
            return;
        }
    } else if new.level == Level::None {
        return;
    }
    let text = upsert_claude(current.unwrap_or(""), new);
    match current {
        None => plan.changes.push(FileChange {
            path: CLAUDE.into(),
            action: Action::Create,
            note: "CLAUDE.md: model-team line".into(),
            text,
        }),
        Some(_) if text.trim().is_empty() => {
            plan.changes.push(FileChange {
                path: CLAUDE.into(),
                action: Action::Remove,
                note: "CLAUDE.md: model-team line was its only text".into(),
                text: String::new(),
            });
        }
        Some(c) if text != c => plan.changes.push(FileChange {
            path: CLAUDE.into(),
            action: Action::Change,
            note: if new.level == Level::None {
                "CLAUDE.md: remove the model-team line".into()
            } else {
                "CLAUDE.md: model-team line".into()
            },
            text,
        }),
        Some(_) => {}
    }
}

fn settings_change(
    plan: &mut Plan,
    current: Option<&str>,
    old: Option<&Harness>,
    new: &Harness,
    mode: Mode,
) -> Result<(), String> {
    let mut doc = match current {
        Some(t) => parse_settings(t)?,
        None => Value::Obj(Vec::new()),
    };
    let before = doc.clone();
    let ws = wants(new);
    if let Some(e) = shape_error(&doc, &ws) {
        return Err(e);
    }
    for w in &ws {
        apply_want(&mut doc, w, mode == Mode::Reconfigure);
    }
    if let Some(old) = old {
        plan.kept.extend(kept_keys(&before, old, new));
    }
    if doc == before {
        return Ok(());
    }
    let mut parts: Vec<String> = Vec::new();
    for w in &ws {
        let key = match w {
            Want::Set { path, .. } => path[0].as_str(),
            Want::Deny(_) => "permissions",
            Want::Hook { .. } => continue,
        };
        if doc.get(key) != before.get(key) && !parts.iter().any(|p| p == key) {
            parts.push(key.to_string());
        }
    }
    if doc.get("hooks") != before.get("hooks") {
        let n = ws
            .iter()
            .filter(|w| {
                matches!(
                    w,
                    Want::Hook {
                        command: Some(_),
                        ..
                    }
                )
            })
            .count();
        parts.push(if n == 0 {
            "hooks removed".to_string()
        } else {
            format!("{n} hooks")
        });
    }
    plan.changes.push(FileChange {
        path: SETTINGS.into(),
        action: if current.is_some() {
            Action::Change
        } else {
            Action::Create
        },
        note: format!("settings: {}", parts.join(", ")),
        text: json::pretty(&doc),
    });
    Ok(())
}

// ---- check ----

/// Checks the harness files of one tree. `files` holds (path, text) of each
/// existing file that `wanted` accepts. The configuration must have level
/// other than `none`; otherwise the result is empty.
pub fn check(h: &Harness, files: &[(String, String)]) -> Vec<Diag> {
    let mut out = Vec::new();
    if h.level == Level::None {
        return out;
    }
    let find = |p: &str| files.iter().find(|f| f.0 == p).map(|f| f.1.as_str());
    let d = |code: Code, path: &str, line: u32, target: &str, message: String| Diag {
        path: path.to_string(),
        line,
        code,
        doc: path.to_string(),
        target: target.to_string(),
        message,
        severity: code.severity(),
    };
    let expected = generated_files(h);
    for g in &expected {
        let Some(text) = find(&g.path) else {
            out.push(d(
                Code::HarnessMissing,
                &g.path,
                1,
                "file",
                format!("the generated file is absent ({})", g.note),
            ));
            continue;
        };
        match file_marker(text) {
            None => out.push(d(
                Code::HarnessMissing,
                &g.path,
                1,
                "file",
                "the file has no harness marker, so spec-dna did not generate it".into(),
            )),
            Some(m) if m.edited() => out.push(d(
                Code::AdapterModified,
                &g.path,
                line_of(text, m.start),
                "file",
                "the generated harness file was edited by hand; delete the file and run `spec-dna init`".into(),
            )),
            Some(m) if text != g.text => out.push(d(
                Code::AdapterOutdated,
                &g.path,
                line_of(text, m.start),
                "file",
                format!(
                    "the harness file (v{}) differs from what spec-dna {VERSION} generates for the configuration",
                    m.version
                ),
            )),
            Some(_) => {}
        }
    }
    for (p, t) in files {
        if p.starts_with(AGENTS_DIR)
            && wanted(p)
            && !expected.iter().any(|g| g.path == *p)
            && has_marker(t)
        {
            out.push(d(
                Code::AdapterOutdated,
                p,
                1,
                "file",
                "the configuration does not generate this file; run `spec-dna init --reconfigure` to remove it".into(),
            ));
        }
    }
    let expected_line = claude_line(h).unwrap_or_default();
    match find(CLAUDE).and_then(find_line) {
        None => out.push(d(
            Code::HarnessMissing,
            CLAUDE,
            1,
            "line",
            "the model-team line is absent".into(),
        )),
        Some((start, end, version, hash, before)) => {
            let text = find(CLAUDE).unwrap_or("");
            let line = line_of(text, start);
            if hex(&hash_normalized(before)) != hash {
                out.push(d(
                    Code::AdapterModified,
                    CLAUDE,
                    line,
                    "line",
                    "the generated model-team line was edited by hand; delete the line and run `spec-dna init`".into(),
                ));
            } else if text[start..end].trim_end() != expected_line {
                out.push(d(
                    Code::AdapterOutdated,
                    CLAUDE,
                    line,
                    "line",
                    format!("the model-team line (v{version}) differs from what spec-dna {VERSION} generates for the configuration"),
                ));
            }
        }
    }
    let ws: Vec<Want> = wants(h)
        .into_iter()
        .filter(|w| !matches!(w, Want::Hook { command: None, .. }))
        .collect();
    match find(SETTINGS) {
        None => {
            for w in &ws {
                out.push(d(
                    Code::AdapterOutdated,
                    SETTINGS,
                    1,
                    &w.label(),
                    format!("{SETTINGS} is absent, so `{}` is not set", w.label()),
                ));
            }
        }
        Some(text) => match parse_settings(text) {
            Err(e) => out.push(d(Code::HarnessSettingsInvalid, SETTINGS, 1, "", e)),
            Ok(doc) if shape_error(&doc, &ws).is_some() => out.push(d(
                Code::HarnessSettingsInvalid,
                SETTINGS,
                1,
                "",
                shape_error(&doc, &ws).unwrap_or_default(),
            )),
            Ok(doc) => {
                for w in ws.iter().filter(|w| !satisfied(&doc, w)) {
                    out.push(d(
                        Code::AdapterOutdated,
                        SETTINGS,
                        1,
                        &w.label(),
                        format!(
                            "`{}` is absent or differs from the configuration",
                            w.label()
                        ),
                    ));
                }
            }
        },
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(extra: &str) -> Result<Harness, String> {
        let t: toml::Table = format!("[harness]\n{extra}").parse().unwrap();
        Harness::parse(&t["harness"])
    }

    fn lean() -> Harness {
        Harness::new(Level::Lean)
    }

    fn full() -> Harness {
        Harness::new(Level::Full)
    }

    fn files(h: &Harness) -> Vec<(String, String)> {
        generated_files(h)
            .into_iter()
            .map(|g| (g.path, g.text))
            .collect()
    }

    /// All files of a clean install of the configuration.
    fn installed(h: &Harness) -> Vec<(String, String)> {
        let mut v = files(h);
        v.push((CLAUDE.into(), upsert_claude("", h)));
        let p = plan(&v, None, h, Mode::Reconfigure).unwrap();
        let s = p.changes.iter().find(|c| c.path == SETTINGS).unwrap();
        v.push((SETTINGS.into(), s.text.clone()));
        v
    }

    fn codes(d: &[Diag]) -> Vec<&'static str> {
        d.iter().map(|d| d.code.code()).collect()
    }

    #[test]
    fn parses_the_defaults_and_every_key() {
        let h = parse("level = \"lean\"").unwrap();
        assert_eq!(h, lean());
        assert_eq!(h.context_meter, vec![300_000, 600_000]);
        assert_eq!(h.efforts, vec![("sonnet".to_string(), Effort::High)]);
        let h = parse(
            r#"level = "full"
deny = ["NotebookEdit", "NotebookEdit"]
context-meter = []
subagent-meter = 0
fanout-cap = 4
[harness.roles]
orchestrator = { claude-code = "opus[1m]", effort = "high", fallback = ["sonnet"] }
advisor = { claude-code = "none" }
reviewer = { claude-code = "inherit", effort = "inherit" }
helper = { claude-code = "inherit" }
[harness.efforts]
opus = "xhigh"
"#,
        )
        .unwrap();
        assert_eq!(h.deny, vec!["NotebookEdit".to_string()]);
        assert!(h.context_meter.is_empty());
        assert_eq!(h.fanout_cap, 4);
        assert_eq!(
            h.cfg(Role::Orchestrator).model,
            Model::Id("opus[1m]".into())
        );
        assert_eq!(
            h.cfg(Role::Orchestrator).fallback,
            vec!["sonnet".to_string()]
        );
        assert!(!h.on(Role::Advisor));
        assert_eq!(h.cfg(Role::Reviewer).model, Model::Inherit);
        assert_eq!(h.cfg(Role::Researcher).model, Model::Id("haiku".into()));
        assert_eq!(h.efforts, vec![("opus".to_string(), Effort::XHigh)]);
        assert!(!h.context_hook_on() && h.fanout_hook_on());
    }

    #[test]
    fn rejects_each_bad_value() {
        let cases: [(&str, &str); 25] = [
            ("", "`harness.level` is required"),
            ("level = \"max\"", "must be \"none\""),
            ("level = 1", "must be a string"),
            (
                "level = \"lean\"\nunknown = 1",
                "unknown key `harness.unknown`",
            ),
            ("level = \"lean\"\ndeny = [\"a b\"]", "`harness.deny`"),
            ("level = \"lean\"\ncontext-meter = [0]", "positive integers"),
            ("level = \"lean\"\ncontext-meter = [5, 5]", "ascending"),
            ("level = \"lean\"\ncontext-meter = [9, 5]", "ascending"),
            ("level = \"lean\"\nfanout-cap = -1", "non-negative"),
            (
                "level = \"lean\"\n[harness.roles]\nimplementer = {}",
                "`harness.roles.implementer`: the role is in level full",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nboss = {}",
                "unknown role",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nreviewer = 1",
                "must be a table",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nreviewer = { codex = \"x\" }",
                "unknown key `harness.roles.reviewer.codex`",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nhelper = { effort = \"high\" }",
                "unknown key `harness.roles.helper.effort`",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nreviewer = { claude-code = \"a b\" }",
                "no spaces",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nreviewer = { effort = \"huge\" }",
                "`harness.roles.reviewer.effort` must be",
            ),
            (
                "level = \"lean\"\n[harness.roles]\norchestrator = { claude-code = \"none\" }",
                "cannot be \"none\"",
            ),
            (
                "level = \"lean\"\n[harness.roles]\norchestrator = { effort = \"max\" }",
                "cannot be \"max\"",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nadvisor = { claude-code = \"haiku\" }",
                "cannot be \"haiku\"",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nadvisor = { claude-code = \"inherit\" }",
                "cannot be \"inherit\"",
            ),
            (
                "level = \"lean\"\n[harness.roles]\norchestrator = { fallback = [\"none\"] }",
                "cannot hold",
            ),
            (
                "level = \"lean\"\n[harness.efforts]\nsonnet = \"max\"",
                "`harness.efforts.sonnet` must be",
            ),
            (
                "level = \"lean\"\n[harness.roles]\nadvisor = { claude-code = \"claude-haiku-4-5\" }",
                "cannot be \"claude-haiku-4-5\"",
            ),
            (
                "level = \"lean\"\ndeny = [\"Read\", \"Bash\"]",
                "removes each tool of the agent `fallback-advisor`",
            ),
            (
                "level = \"full\"\ndeny = [\"Read\", \"Grep\", \"Glob\", \"Bash\", \"WebFetch\", \"WebSearch\"]\n[harness.roles]\nadvisor = { agent = \"none\" }",
                "removes each tool of the agent `Explore`",
            ),
        ];
        for (src, want) in cases {
            let e = parse(src).unwrap_err();
            assert!(e.contains(want), "{src:?}: {e}");
        }
        // An agent that is off needs no tool. The advisor can be haiku in its agent.
        parse("level = \"lean\"\ndeny = [\"Read\", \"Bash\"]\n[harness.roles]\nadvisor = { agent = \"none\" }").unwrap();
        parse("level = \"none\"\ndeny = [\"Read\", \"Bash\"]").unwrap();
        parse("level = \"lean\"\n[harness.roles]\nadvisor = { agent = \"haiku\" }").unwrap();
        // With level none the roles are not checked against the level.
        let h = parse("level = \"none\"\n[harness.roles]\nimplementer = {}").unwrap();
        assert_eq!(h.level, Level::None);
        assert!(!h.on(Role::Orchestrator));
    }

    #[test]
    fn config_parse_reads_the_harness_table() {
        let base = "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n";
        assert!(
            crate::config::Config::parse(base)
                .unwrap()
                .harness
                .is_none()
        );
        let c =
            crate::config::Config::parse(&format!("{base}[harness]\nlevel = \"full\"\n")).unwrap();
        assert_eq!(c.harness, Some(full()));
        let e =
            crate::config::Config::parse(&format!("{base}[harness]\nlevel = \"x\"\n")).unwrap_err();
        assert!(e.contains("harness.level"), "{e}");
    }

    #[test]
    fn toml_section_round_trips() {
        for h in [lean(), full(), Harness::new(Level::None), {
            let mut h = full();
            h.deny = vec!["NotebookEdit".into()];
            h.context_meter.clear();
            h.cfg_mut(Role::Reviewer).model = Model::None;
            h.cfg_mut(Role::Advisor).agent = Model::None;
            h.cfg_mut(Role::Orchestrator).fallback.clear();
            h.efforts.clear();
            h
        }] {
            let text = h.toml_section();
            let t: toml::Table = text.parse().unwrap_or_else(|e| panic!("{e}\n{text}"));
            assert_eq!(Harness::parse(&t["harness"]).unwrap(), h, "{text}");
        }
    }

    #[test]
    fn warns_when_the_advisor_ranks_below_the_main_model() {
        assert!(lean().warnings().is_empty());
        let mut h = lean();
        h.cfg_mut(Role::Advisor).model = Model::Id("sonnet".into());
        let w = h.warnings();
        assert_eq!(w.len(), 1);
        assert!(w[0].contains("ranks below"), "{w:?}");
        h.cfg_mut(Role::Advisor).model = Model::Id("my-model".into());
        assert!(h.warnings().is_empty(), "an unknown model has no rank");
        let mut h = lean();
        h.cfg_mut(Role::Orchestrator).model = Model::Id("opus[1m]".into());
        let w = h.warnings();
        assert_eq!(w.len(), 1);
        assert!(
            w[0].contains("effort of the main model is not written"),
            "{w:?}"
        );
        assert!(Harness::new(Level::None).warnings().is_empty());
    }

    #[test]
    fn model_ids() {
        assert_eq!(model_id("opus").as_deref(), Some("claude-opus-5-5"));
        assert_eq!(model_id("fable").as_deref(), Some("claude-fable-5-2"));
        assert_eq!(model_id("claude-x-1").as_deref(), Some("claude-x-1"));
        for m in ["opus[1m]", "opusplan", "default", "best"] {
            assert_eq!(model_id(m), None, "{m}");
        }
    }

    #[test]
    fn agent_file_has_the_frontmatter_the_body_and_the_marker() {
        let f = files(&lean());
        let (path, text) = f.iter().find(|f| f.0.ends_with("checker.md")).unwrap();
        assert_eq!(path, ".claude/agents/checker.md");
        assert!(
            text.starts_with(
                "---\nname: checker\ndescription: Reviews finished work independently"
            )
        );
        assert!(text.contains("\nmodel: opus\neffort: high\ntools: Read, Bash, Monitor, TaskStop\n---\n\nYou review work"));
        let last = text.lines().last().unwrap();
        assert!(
            last.starts_with(&format!("{OPEN}{VERSION} h:")) && last.ends_with(" -->"),
            "{last}"
        );
        assert!(text.ends_with(" -->\n"));
        let m = file_marker(text).unwrap();
        assert_eq!(m.hash, hex(&hash_normalized(m.before)));
        // The advisor file takes the agent model. Explore omits CLAUDE.md.
        let adv = &f
            .iter()
            .find(|f| f.0.ends_with("fallback-advisor.md"))
            .unwrap()
            .1;
        assert!(
            adv.contains("\nmodel: opus\neffort: xhigh\ntools: Read, Bash\n---"),
            "{adv}"
        );
        let ex = &f.iter().find(|f| f.0.ends_with("Explore.md")).unwrap().1;
        assert!(
            ex.contains(
                "\ntools: Read, Grep, Glob, Bash, WebFetch, WebSearch\nomitClaudeMd: true\n---"
            ),
            "{ex}"
        );
        // Lean has no writer. Full has both.
        assert_eq!(f.len(), 4, "3 agent files and the team page");
        assert_eq!(files(&full()).len(), 6);
        // Texts have one-line descriptions without ": ".
        for def in &AGENT_DEFS {
            assert!(
                !def.description.contains('\n') && !def.description.contains(": "),
                "{}",
                def.name
            );
        }
    }

    #[test]
    fn agent_file_follows_the_configuration() {
        let mut h = full();
        h.deny = vec!["Bash".into(), "WebSearch".into()];
        h.cfg_mut(Role::Reviewer).model = Model::Inherit;
        h.cfg_mut(Role::Reviewer).effort = Effort::Inherit;
        h.cfg_mut(Role::Implementer).model = Model::None;
        h.cfg_mut(Role::Advisor).agent = Model::None;
        let f = files(&h);
        let names: Vec<&str> = f.iter().map(|f| f.0.as_str()).collect();
        assert_eq!(
            names,
            [
                ".claude/agents/checker.md",
                ".claude/agents/Explore.md",
                ".claude/agents/spec-writer.md",
                ".claude/agents/model-team.md"
            ]
        );
        let c = &f[0].1;
        assert!(
            c.contains("\nmodel: inherit\ntools: Read, Monitor, TaskStop\n---"),
            "{c}"
        );
        assert!(f[1].1.contains("tools: Read, Grep, Glob, WebFetch\n"));
        assert!(generated_files(&Harness::new(Level::None)).is_empty());
    }

    #[test]
    fn team_page_lists_the_roles_and_stays_small() {
        let page = &files(&full()).last().unwrap().1.clone();
        assert!(page.starts_with("# Model team\n"));
        for want in [
            "| orchestrator | `model` in `.claude/settings.json` | opus | xhigh |",
            "| advisor | `advisorModel` in `.claude/settings.json` | fable | - |",
            "| advisor fallback | `.claude/agents/fallback-advisor.md` | opus | xhigh |",
            "| reviewer | `.claude/agents/checker.md` | opus | high |",
            "| spec-writer | `.claude/agents/spec-writer.md` | sonnet | high |",
            "| helper | `env.CLAUDE_CODE_SUBAGENT_MODEL` | haiku | - |",
            "## Outage",
            "## No access to the advisor model",
            "Run `/model fable` once",
            "## Precedence",
            "## Reload",
            "## Hooks",
            "spec-dna 0.5.0 or later",
            "## Check after a restart",
            "spec-dna init --reconfigure",
        ] {
            assert!(page.contains(want), "{want}\n{page}");
        }
        assert!(has_marker(page));
        assert!(page.len() < 3 * 1024, "{} bytes", page.len());
        let mut h = lean();
        h.cfg_mut(Role::Advisor).model = Model::None;
        h.context_meter.clear();
        h.subagent_meter = 0;
        h.fanout_cap = 0;
        let page = files(&h).pop().unwrap().1;
        assert!(
            !page.contains("## Advisor")
                && !page.contains("## Hooks")
                && !page.contains("fallback-advisor")
        );
    }

    #[test]
    fn claude_line_follows_the_roles() {
        let line = claude_line(&lean()).unwrap();
        assert!(!line.contains('\n'));
        assert!(line.starts_with("Model team: opus builds. The advisor (fable) gives advice and writes no code. `checker` (opus) tests finished work. `Explore` (haiku) searches. Consult the advisor"), "{line}");
        assert!(line.contains(
            "brief `fallback-advisor`. Run `checker` before you say that a task is done."
        ));
        assert!(line.contains("`Explore`. Run a one-command search yourself. Give a subagent that writes code or reviews work the model `sonnet` or `opus`, not `haiku`."));
        assert!(line.contains("say so in one more consult. <!-- spec-dna harness v"));
        assert!(!line.contains("implementer"));
        assert!(claude_line(&full()).unwrap().contains("Give a code change to `implementer` and a spec change to `spec-writer`, one writer at a time. "));
        let mut h = lean();
        h.cfg_mut(Role::Advisor).model = Model::None;
        h.cfg_mut(Role::Orchestrator).model = Model::Inherit;
        let line = claude_line(&h).unwrap();
        assert!(
            line.starts_with("Model team: the main model builds. `checker`"),
            "{line}"
        );
        for gone in ["advisor", "Advisor", "consult", "fallback-advisor"] {
            assert!(!line.contains(gone), "{gone}: {line}");
        }
        assert_eq!(claude_line(&Harness::new(Level::None)), None);
    }

    #[test]
    fn claude_line_upsert_removal_and_the_spec_dna_line() {
        let h = lean();
        let line = claude_line(&h).unwrap();
        assert_eq!(upsert_claude("", &h), format!("{line}\n"));
        let t = upsert_claude("# Rules\n", &h);
        assert_eq!(t, format!("# Rules\n\n{line}\n"));
        assert_eq!(upsert_claude(&t, &h), t, "idempotent");
        let mut h2 = h.clone();
        h2.cfg_mut(Role::Reviewer).model = Model::Id("sonnet".into());
        let t2 = upsert_claude(&t, &h2);
        assert!(
            t2.starts_with("# Rules\n\n")
                && t2.contains("`checker` (sonnet)")
                && t2.matches("Model team:").count() == 1
        );
        assert_eq!(remove_claude_line(&t), "# Rules\n");
        assert_eq!(upsert_claude(&t, &Harness::new(Level::None)), "# Rules\n");
        // The two generated lines stay apart, in any order.
        let both = crate::adapters::upsert_line(&t);
        assert_eq!(upsert_claude(&both, &h), both);
        assert!(crate::adapters::check(&[(CLAUDE, both.clone())]).is_empty());
        assert_eq!(crate::adapters::remove_line(&both), t);
        assert_eq!(
            remove_claude_line(&both),
            format!("# Rules\n\n{}\n", crate::adapters::line())
        );
        let only_ours = upsert_claude("", &h);
        assert!(crate::adapters::check(&[(CLAUDE, only_ours.clone())]).is_empty());
        // Prose that quotes the form of the marker is not the line.
        let prose = "The line ends with `<!-- spec-dna harness v<version> h:<hash> -->`.\n";
        assert!(find_line(prose).is_none());
    }

    fn settings_of(h: &Harness, current: Option<&str>) -> (Value, Plan) {
        let p = plan(
            &current
                .map(|c| vec![(SETTINGS.to_string(), c.to_string())])
                .unwrap_or_default(),
            None,
            h,
            Mode::Reconfigure,
        )
        .unwrap();
        let s = p
            .changes
            .iter()
            .find(|c| c.path == SETTINGS)
            .map(|c| c.text.clone());
        (
            json::parse(&s.unwrap_or_default()).unwrap_or(Value::Null),
            p,
        )
    }

    #[test]
    fn settings_are_created_from_the_defaults() {
        let (doc, p) = settings_of(&lean(), None);
        let c = p.changes.iter().find(|c| c.path == SETTINGS).unwrap();
        assert_eq!(c.action, Action::Create);
        assert_eq!(
            c.note,
            "settings: modelSettings, model, fallbackModel, advisorModel, env, 2 hooks"
        );
        assert_eq!(doc.get("model").as_str(), Some("opus"));
        assert_eq!(doc.get("advisorModel").as_str(), Some("fable"));
        assert_eq!(doc.get("fallbackModel").as_arr().len(), 2);
        assert_eq!(
            doc.get("env").get("CLAUDE_CODE_SUBAGENT_MODEL").as_str(),
            Some("haiku")
        );
        assert_eq!(
            doc.get("modelSettings")
                .get("claude-opus-5-5")
                .get("effortLevel")
                .as_str(),
            Some("xhigh")
        );
        assert_eq!(
            doc.get("modelSettings")
                .get("claude-sonnet-5-5")
                .get("effortLevel")
                .as_str(),
            Some("high")
        );
        let post = &doc.get("hooks").get("PostToolUse").as_arr()[0];
        assert_eq!(post.get("matcher").as_str(), Some("*"));
        assert_eq!(
            post.get("hooks").as_arr()[0].get("command").as_str(),
            Some(
                "spec-dna hook context-meter --levels 300000,600000 --subagent 250000 2>/dev/null || true"
            )
        );
        let pre = &doc.get("hooks").get("PreToolUse").as_arr()[0];
        assert_eq!(pre.get("matcher").as_str(), Some("Workflow"));
        assert_eq!(
            pre.get("hooks").as_arr()[0].get("command").as_str(),
            Some("spec-dna hook fanout-cap --cap 10 2>/dev/null || true")
        );
        assert!(
            c.text.starts_with("{\n  \"modelSettings\": {\n"),
            "{}",
            c.text
        );
        assert!(c.text.ends_with("}\n"));
    }

    #[test]
    fn the_orchestrator_effort_wins_over_the_efforts_table() {
        let mut h = lean();
        h.efforts = vec![("opus".into(), Effort::Low)];
        let (doc, _) = settings_of(&h, None);
        assert_eq!(
            doc.get("modelSettings")
                .get("claude-opus-5-5")
                .get("effortLevel")
                .as_str(),
            Some("xhigh")
        );
        h.cfg_mut(Role::Orchestrator).effort = Effort::Inherit;
        let (doc, _) = settings_of(&h, None);
        assert_eq!(
            doc.get("modelSettings")
                .get("claude-opus-5-5")
                .get("effortLevel")
                .as_str(),
            Some("low")
        );
        h.cfg_mut(Role::Orchestrator).model = Model::Inherit;
        h.cfg_mut(Role::Helper).model = Model::Inherit;
        let (doc, _) = settings_of(&h, None);
        assert_eq!(doc.get("model"), &Value::Null);
        assert_eq!(doc.get("env"), &Value::Null);
    }

    #[test]
    fn settings_keep_foreign_keys_in_order_and_merge_the_rest() {
        let cur = r#"{"theme": "dark",
    "modelSettings": {"claude-opus-5-5": {"effortLevel": "low", "other": 1}, "x": {}},
    "permissions": {"allow": ["Read"], "deny": ["Bash(rm:*)"]},
    "env": {"A": "1"},
    "hooks": {"PostToolUse": [{"matcher": "Edit", "hooks": [{"type": "command", "command": "echo hi"}]}]},
    "tail": [1, 2]}"#;
        let mut h = lean();
        h.deny = vec!["NotebookEdit".into(), "Bash(rm:*)".into()];
        let (doc, _) = settings_of(&h, Some(cur));
        let Value::Obj(fields) = &doc else { panic!() };
        let keys: Vec<&str> = fields.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(
            &keys[..5],
            ["theme", "modelSettings", "permissions", "env", "hooks"]
        );
        assert_eq!(keys[5], "tail");
        let ms = doc.get("modelSettings").get("claude-opus-5-5");
        assert_eq!(ms.get("effortLevel").as_str(), Some("xhigh"));
        assert_eq!(ms.get("other").as_u64(), 1);
        assert!(matches!(doc.get("modelSettings").get("x"), Value::Obj(_)));
        let deny: Vec<&str> = doc
            .get("permissions")
            .get("deny")
            .as_arr()
            .iter()
            .filter_map(Value::as_str)
            .collect();
        assert_eq!(deny, ["Bash(rm:*)", "NotebookEdit"]);
        assert_eq!(doc.get("permissions").get("allow").as_arr().len(), 1);
        assert_eq!(doc.get("env").get("A").as_str(), Some("1"));
        assert_eq!(
            doc.get("env").get("CLAUDE_CODE_SUBAGENT_MODEL").as_str(),
            Some("haiku")
        );
        let post = doc.get("hooks").get("PostToolUse").as_arr();
        assert_eq!(post.len(), 2, "the foreign group stays, ours is new");
        assert_eq!(post[0].get("matcher").as_str(), Some("Edit"));
        assert_eq!(doc.get("tail").as_arr().len(), 2);
    }

    #[test]
    fn a_handler_in_a_foreign_group_is_updated_in_place() {
        let cur = r#"{"hooks": {"PostToolUse": [{"matcher": "Bash", "hooks": [
    {"type": "command", "command": "echo hi"},
    {"type": "command", "command": "spec-dna hook context-meter --levels 1 2>/dev/null || true", "timeout": 5}
    ]}]}}"#;
        let (doc, _) = settings_of(&lean(), Some(cur));
        let post = doc.get("hooks").get("PostToolUse").as_arr();
        assert_eq!(post.len(), 1);
        assert_eq!(post[0].get("matcher").as_str(), Some("Bash"));
        let hs = post[0].get("hooks").as_arr();
        assert_eq!(hs.len(), 2);
        assert!(
            hs[1]
                .get("command")
                .as_str()
                .unwrap()
                .contains("--levels 300000,600000")
        );
        assert_eq!(hs[1].get("timeout").as_u64(), 5);
    }

    #[test]
    fn a_hook_that_is_off_is_removed_with_its_empty_group() {
        let h = lean();
        let t = installed(&h);
        let settings = t.iter().find(|f| f.0 == SETTINGS).unwrap().1.clone();
        let with_foreign = settings.replace(
            "\"hooks\": {",
            "\"hooks\": {\n    \"Stop\": [{\"hooks\": [{\"type\": \"command\", \"command\": \"echo done\"}]}],",
        );
        let mut off = lean();
        off.fanout_cap = 0;
        let existing = vec![(SETTINGS.to_string(), with_foreign.clone())];
        let p = plan(&existing, Some(&h), &off, Mode::Reconfigure).unwrap();
        let c = p.changes.iter().find(|c| c.path == SETTINGS).unwrap();
        let doc = json::parse(&c.text).unwrap();
        assert_eq!(doc.get("hooks").get("PreToolUse").as_arr().len(), 0);
        assert_eq!(doc.get("hooks").get("PostToolUse").as_arr().len(), 1);
        assert_eq!(doc.get("hooks").get("Stop").as_arr().len(), 1);
        // The upgrade path never removes.
        let p = plan(&existing, Some(&h), &off, Mode::Upgrade).unwrap();
        assert!(
            p.changes.iter().all(|c| c.path != SETTINGS),
            "{:?}",
            p.changes
        );
    }

    #[test]
    fn settings_are_not_written_when_equal() {
        let h = full();
        let t = installed(&h);
        let p = plan(&t, Some(&h), &h, Mode::Reconfigure).unwrap();
        assert!(p.changes.is_empty(), "{:?}", p.changes);
        assert!(p.kept.is_empty());
        // The same entries in a file with other formatting: no write.
        let compact = json::parse(&t.iter().find(|f| f.0 == SETTINGS).unwrap().1).unwrap();
        let Value::Obj(f) = &compact else { panic!() };
        let mut text = String::from("{");
        for (i, (k, v)) in f.iter().enumerate() {
            if i > 0 {
                text.push(',');
            }
            text += &format!("\"{k}\":{}", json::pretty(v).trim().replace("\n  ", " "));
        }
        text.push('}');
        let mut t2 = t.clone();
        t2.iter_mut().find(|f| f.0 == SETTINGS).unwrap().1 = text;
        let p = plan(&t2, Some(&h), &h, Mode::Reconfigure).unwrap();
        assert!(p.changes.is_empty(), "{:?}", p.changes);
    }

    #[test]
    fn settings_that_are_not_json_refuse_the_plan() {
        let e = plan(
            &[(SETTINGS.into(), "{".into())],
            None,
            &lean(),
            Mode::Reconfigure,
        )
        .unwrap_err();
        assert!(e.contains("not valid JSON"), "{e}");
        let e = plan(
            &[(SETTINGS.into(), "[]".into())],
            None,
            &lean(),
            Mode::Reconfigure,
        )
        .unwrap_err();
        assert!(e.contains("root is not an object"), "{e}");
    }

    #[test]
    fn keys_that_the_new_configuration_drops_are_kept_with_a_note() {
        let old = lean();
        let t = installed(&old);
        let mut new = lean();
        new.cfg_mut(Role::Advisor).model = Model::None;
        new.cfg_mut(Role::Orchestrator).model = Model::Id("sonnet".into());
        new.deny = vec![];
        let mut old2 = old.clone();
        old2.deny = vec!["NotebookEdit".into()];
        let t = {
            let p = plan(&t, None, &old2, Mode::Reconfigure).unwrap();
            let mut t = t;
            for c in p.changes {
                t.iter_mut().find(|f| f.0 == c.path).unwrap().1 = c.text;
            }
            t
        };
        let p = plan(&t, Some(&old2), &new, Mode::Reconfigure).unwrap();
        for want in [
            "kept advisorModel: spec-dna no longer sets it; remove it by hand if you do not want it",
            "kept permissions.deny[NotebookEdit]: spec-dna no longer sets it",
            "kept modelSettings.claude-opus-5-5.effortLevel: spec-dna no longer sets it",
        ] {
            assert!(
                p.kept.iter().any(|k| k.starts_with(want)),
                "{want}: {:?}",
                p.kept
            );
        }
        assert!(
            !p.kept.iter().any(|k| k.contains("kept model:")),
            "{:?}",
            p.kept
        );
        // The scalars stay in the new text.
        let c = p.changes.iter().find(|c| c.path == SETTINGS).unwrap();
        let doc = json::parse(&c.text).unwrap();
        assert_eq!(doc.get("advisorModel").as_str(), Some("fable"));
        assert_eq!(doc.get("model").as_str(), Some("sonnet"));
    }

    #[test]
    fn reconfigure_to_a_smaller_level_removes_generated_files() {
        let h = full();
        let mut t = installed(&h);
        let p = plan(&t, Some(&h), &lean(), Mode::Reconfigure).unwrap();
        let removed: Vec<&str> = p
            .changes
            .iter()
            .filter(|c| c.action == Action::Remove)
            .map(|c| c.path.as_str())
            .collect();
        assert_eq!(
            removed,
            [
                ".claude/agents/implementer.md",
                ".claude/agents/spec-writer.md"
            ]
        );
        // An edited file stays, and the plan says so.
        let i = t
            .iter()
            .position(|f| f.0.ends_with("spec-writer.md"))
            .unwrap();
        t[i].1 = t[i]
            .1
            .replace("You keep the specifications true.", "You do my work.");
        let p = plan(&t, Some(&h), &lean(), Mode::Reconfigure).unwrap();
        assert!(
            p.kept.iter().any(
                |k| k.starts_with("kept .claude/agents/spec-writer.md: edited by hand (SD501)")
            ),
            "{:?}",
            p.kept
        );
        assert_eq!(
            p.changes
                .iter()
                .filter(|c| c.action == Action::Remove)
                .count(),
            1
        );
        // The upgrade path removes nothing.
        let p = plan(&t, Some(&h), &lean(), Mode::Upgrade).unwrap();
        assert!(p.changes.iter().all(|c| c.action != Action::Remove));
        // Level none removes all files and the line, and keeps the scalars.
        let t = installed(&h);
        let p = plan(&t, Some(&h), &Harness::new(Level::None), Mode::Reconfigure).unwrap();
        assert_eq!(
            p.changes
                .iter()
                .filter(|c| c.action == Action::Remove)
                .count(),
            6 + 1,
            "the CLAUDE.md held only our line"
        );
        let c = p.changes.iter().find(|c| c.path == SETTINGS).unwrap();
        let doc = json::parse(&c.text).unwrap();
        assert_eq!(doc.get("model").as_str(), Some("opus"));
        assert_eq!(doc.get("hooks").get("PostToolUse").as_arr().len(), 0);
        assert!(p.kept.iter().any(|k| k.starts_with("kept model: ")));
    }

    #[test]
    fn claude_md_with_own_text_keeps_it_when_the_line_goes() {
        let h = lean();
        let t = vec![(CLAUDE.to_string(), upsert_claude("# Rules\n", &h))];
        let p = plan(&t, Some(&h), &Harness::new(Level::None), Mode::Reconfigure).unwrap();
        let c = p.changes.iter().find(|c| c.path == CLAUDE).unwrap();
        assert_eq!((c.action, c.text.as_str()), (Action::Change, "# Rules\n"));
        let edited = vec![(
            CLAUDE.to_string(),
            t[0].1.replace("Model team:", "Our team:"),
        )];
        let p = plan(&edited, Some(&h), &h, Mode::Reconfigure).unwrap();
        assert!(p.changes.iter().all(|c| c.path != CLAUDE));
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept CLAUDE.md:3: edited by hand (SD501)")),
            "{:?}",
            p.kept
        );
    }

    #[test]
    fn a_file_that_spec_dna_did_not_generate_is_never_replaced() {
        let own = vec![(
            ".claude/agents/reviewer-agent.md".to_string(),
            "---\nname: checker\ndescription: mine\n---\nText\n".to_string(),
        )];
        assert_eq!(
            name_conflicts(&own),
            vec![(
                Role::Reviewer,
                ".claude/agents/reviewer-agent.md".to_string()
            )]
        );
        let e = plan(&own, None, &lean(), Mode::Reconfigure).unwrap_err();
        assert!(
            e.contains("holds the agent `checker`") && e.contains("\"none\""),
            "{e}"
        );
        let mut h = lean();
        h.cfg_mut(Role::Reviewer).model = Model::None;
        assert!(plan(&own, None, &h, Mode::Reconfigure).is_ok());
        // The advisor with no fallback agent writes no file at that path.
        let adv = vec![(
            ".claude/agents/fallback-advisor.md".to_string(),
            "mine\n".to_string(),
        )];
        assert!(plan(&adv, None, &lean(), Mode::Reconfigure).is_err());
        let mut h = lean();
        h.cfg_mut(Role::Advisor).agent = Model::None;
        let p = plan(&adv, None, &h, Mode::Reconfigure).unwrap();
        assert!(
            !p.changes
                .iter()
                .any(|c| c.path.ends_with("fallback-advisor.md"))
        );
        // A file without a front matter name uses its stem.
        let stem = vec![(
            ".claude/agents/Explore.md".to_string(),
            "own text\n".to_string(),
        )];
        assert_eq!(
            name_conflicts(&stem),
            vec![(Role::Researcher, ".claude/agents/Explore.md".to_string())]
        );
        // A generated file is not a conflict.
        assert!(name_conflicts(&files(&lean())).is_empty());
        // Another name is not a conflict.
        let other = vec![(".claude/agents/other.md".to_string(), "x\n".to_string())];
        assert!(name_conflicts(&other).is_empty());
        let e = plan(
            &[(TEAM_PAGE.to_string(), "mine\n".into())],
            None,
            &lean(),
            Mode::Upgrade,
        )
        .unwrap_err();
        assert!(e.contains("model-team.md exists"), "{e}");
    }

    #[test]
    fn the_upgrade_path_regenerates_old_files_and_keeps_edited_ones() {
        let h = lean();
        let mut t = installed(&h);
        let i = t.iter().position(|f| f.0.ends_with("checker.md")).unwrap();
        let old_version = t[i].1.replace(&format!("v{VERSION} "), "v0.0.1 ");
        // An old marker with the right hash for its text: the text is not
        // stale, so only the version part differs. Make the text differ too.
        let text = old_version.replace("You review work", "You inspect work");
        let m = file_marker(&text).unwrap();
        let rehashed = format!(
            "{}{OPEN}0.0.1 h:{} -->\n",
            m.before,
            hex(&hash_normalized(m.before))
        );
        t[i].1 = rehashed;
        let j = t.iter().position(|f| f.0.ends_with("Explore.md")).unwrap();
        t[j].1 = t[j].1.replace("You find information", "You guess");
        let p = plan(&t, Some(&h), &h, Mode::Upgrade).unwrap();
        assert_eq!(p.changes.len(), 1, "{:?}", p.changes);
        assert_eq!(
            (p.changes[0].path.as_str(), p.changes[0].action),
            (".claude/agents/checker.md", Action::Change)
        );
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/Explore.md: edited by hand (SD501)"))
        );
    }

    #[test]
    fn the_plan_reports_the_request_bytes_and_the_warnings() {
        let mut h = lean();
        h.cfg_mut(Role::Advisor).model = Model::Id("sonnet".into());
        let p = plan(&[], None, &h, Mode::Reconfigure).unwrap();
        assert!(p.request_bytes > claude_line(&h).unwrap().len());
        assert!(p.request_bytes < 2_500, "{}", p.request_bytes);
        assert_eq!(p.warnings.len(), 1);
        let names: Vec<&str> = p.changes.iter().map(|c| c.path.as_str()).collect();
        assert_eq!(
            names,
            [
                ".claude/agents/fallback-advisor.md",
                ".claude/agents/checker.md",
                ".claude/agents/Explore.md",
                ".claude/agents/model-team.md",
                "CLAUDE.md",
                ".claude/settings.json"
            ]
        );
        assert_eq!(p.changes[0].note, "agent fallback-advisor: opus, xhigh");
        assert_eq!(p.changes[1].note, "agent checker: opus, high");
        assert_eq!(p.changes[4].note, "CLAUDE.md: model-team line");
    }

    #[test]
    fn check_is_clean_after_an_install() {
        for h in [lean(), full()] {
            let t = installed(&h);
            assert!(check(&h, &t).is_empty(), "{:?}", check(&h, &t));
        }
        assert!(check(&Harness::new(Level::None), &[]).is_empty());
    }

    #[test]
    fn check_reports_each_finding_with_its_code() {
        let h = lean();
        let clean = installed(&h);
        let edit = |path: &str, f: &dyn Fn(&str) -> String| {
            let mut t = clean.clone();
            let e = t.iter_mut().find(|x| x.0 == path).unwrap();
            e.1 = f(&e.1);
            t
        };
        let drop = |path: &str| {
            let mut t = clean.clone();
            t.retain(|x| x.0 != path);
            t
        };
        // SD504: an expected file is absent, or has no marker.
        let d = check(&h, &drop(".claude/agents/checker.md"));
        assert_eq!(codes(&d), ["SD504"]);
        assert_eq!(
            (d[0].path.as_str(), d[0].target.as_str()),
            (".claude/agents/checker.md", "file")
        );
        let d = check(&h, &drop(TEAM_PAGE));
        assert_eq!(codes(&d), ["SD504"]);
        let d = check(
            &h,
            &edit(".claude/agents/checker.md", &|_| "own text\n".into()),
        );
        assert_eq!(codes(&d), ["SD504"]);
        assert!(d[0].message.contains("no harness marker"));
        // SD501: the hash differs from the text.
        let d = check(
            &h,
            &edit(".claude/agents/checker.md", &|t| {
                t.replace("You review", "You skim")
            }),
        );
        assert_eq!(codes(&d), ["SD501"]);
        assert!(d[0].line > 5);
        // SD502: the text is not what this version generates.
        let d = check(
            &h,
            &edit(".claude/agents/checker.md", &|t| {
                let m = file_marker(t).unwrap();
                let before = m.before.replace("model: opus", "model: sonnet");
                format!("{before}{}\n", marker_text(&before))
            }),
        );
        assert_eq!(codes(&d), ["SD502"]);
        let d = check(
            &h,
            &edit(".claude/agents/Explore.md", &|t| {
                t.replace(&format!("v{VERSION} "), "v0.0.1 ")
            }),
        );
        assert_eq!(codes(&d), ["SD502"], "the marker text is part of the file");
        // SD502: a marked file that the configuration does not produce.
        let mut t = clean.clone();
        t.push((
            ".claude/agents/implementer.md".into(),
            files(&full())
                .iter()
                .find(|f| f.0.ends_with("implementer.md"))
                .unwrap()
                .1
                .clone(),
        ));
        let d = check(&h, &t);
        assert_eq!(codes(&d), ["SD502"]);
        assert!(d[0].message.contains("init --reconfigure"));
        t.push((".claude/agents/own.md".into(), "own\n".into()));
        assert_eq!(check(&h, &t).len(), 1, "an unmarked file is not ours");
        // The CLAUDE.md line.
        let d = check(&h, &drop(CLAUDE));
        assert_eq!((codes(&d), d[0].target.as_str()), (vec!["SD504"], "line"));
        let d = check(&h, &edit(CLAUDE, &|_| crate::adapters::upsert_line("")));
        assert_eq!(
            codes(&d),
            ["SD504"],
            "the spec-dna line is not the model-team line"
        );
        let d = check(&h, &edit(CLAUDE, &|t| t.replace("Model team", "Our team")));
        assert_eq!(codes(&d), ["SD501"]);
        let d = check(
            &h,
            &edit(CLAUDE, &|t| {
                let before = t
                    .split(" <!--")
                    .next()
                    .unwrap()
                    .replace("`checker` (opus)", "`checker` (haiku)");
                format!("{before} {}\n", marker_text(&before))
            }),
        );
        assert_eq!(codes(&d), ["SD502"]);
        // settings.json.
        let d = check(&h, &drop(SETTINGS));
        assert_eq!(
            d.len(),
            wants(&h)
                .iter()
                .filter(|w| !matches!(w, Want::Hook { command: None, .. }))
                .count()
        );
        assert!(d.iter().all(|d| d.code == Code::AdapterOutdated));
        let d = check(
            &h,
            &edit(SETTINGS, &|t| {
                t.replace("\"advisorModel\": \"fable\"", "\"advisorModel\": \"opus\"")
            }),
        );
        assert_eq!(
            (codes(&d), d[0].target.as_str()),
            (vec!["SD502"], "advisorModel")
        );
        let d = check(&h, &edit(SETTINGS, &|t| t.replace("--cap 10", "--cap 11")));
        assert_eq!(codes(&d), ["SD502"]);
        assert!(d[0].target.starts_with("hooks.PreToolUse"));
        let d = check(
            &h,
            &edit(SETTINGS, &|t| t.replace("\"model\": \"opus\",", "")),
        );
        assert_eq!(d[0].target, "model");
        let d = check(&h, &edit(SETTINGS, &|_| "{".into()));
        assert_eq!(codes(&d), ["SD505"]);
        assert_eq!(d[0].severity, crate::diag::Severity::Error);
        let d = check(&h, &edit(SETTINGS, &|_| "[]".into()));
        assert_eq!(codes(&d), ["SD505"]);
        // A foreign key does no harm.
        let d = check(
            &h,
            &edit(SETTINGS, &|t| t.replacen('{', "{\"theme\": \"dark\",", 1)),
        );
        assert!(d.is_empty(), "{d:?}");
    }

    #[test]
    fn text_after_the_marker_is_a_hand_edit() {
        let h = lean();
        let clean = installed(&h);
        for path in [".claude/agents/checker.md", TEAM_PAGE] {
            let mut t = clean.clone();
            let e = t.iter_mut().find(|x| x.0 == path).unwrap();
            e.1.push_str("extra\n");
            let d = check(&h, &t);
            assert_eq!(codes(&d), ["SD501"], "{path}: {d:?}");
            assert!(
                d[0].message
                    .contains("delete the file and run `spec-dna init`"),
                "{}",
                d[0].message
            );
            // `init` keeps the file, and still repairs the other drift.
            let mut old = t.clone();
            let c = old
                .iter_mut()
                .find(|x| x.0 == ".claude/agents/Explore.md")
                .unwrap();
            c.1 = c.1.replace("You find information", "You guess");
            let c = old
                .iter_mut()
                .find(|x| x.0 == ".claude/agents/Explore.md")
                .unwrap();
            let m = file_marker(&c.1).unwrap();
            let before = m.before.to_string();
            c.1 = format!("{before}{}\n", marker_text(&before));
            for mode in [Mode::Upgrade, Mode::Reconfigure] {
                let p = plan(&old, Some(&h), &h, mode).unwrap();
                assert!(
                    p.kept
                        .iter()
                        .any(|k| k.starts_with(&format!("kept {path}: edited by hand"))),
                    "{path}: {:?}",
                    p.kept
                );
                assert!(!p.changes.iter().any(|c| c.path == path));
                assert!(
                    p.changes
                        .iter()
                        .any(|c| c.path == ".claude/agents/Explore.md"),
                    "{:?}",
                    p.changes
                );
            }
        }
        // Reconfigure to a smaller level keeps an edited file that it would remove.
        let full = Harness::new(Level::Full);
        let mut t = installed(&full);
        let e = t
            .iter_mut()
            .find(|x| x.0 == ".claude/agents/implementer.md")
            .unwrap();
        e.1.push_str("extra\n");
        let p = plan(&t, Some(&full), &h, Mode::Reconfigure).unwrap();
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/implementer.md: edited by hand")),
            "{:?}",
            p.kept
        );
        assert!(
            !p.changes
                .iter()
                .any(|c| c.action == Action::Remove && c.path == ".claude/agents/implementer.md")
        );
        // A file with text after the marker is not a conflict of names.
        let mut t = files(&h);
        t[1].1.push_str("extra\n");
        assert!(name_conflicts(&t).is_empty());
        assert!(is_installed(&t));
    }

    #[test]
    fn the_claude_line_message_says_to_delete_the_line() {
        let h = lean();
        let mut t = installed(&h);
        let e = t.iter_mut().find(|x| x.0 == CLAUDE).unwrap();
        e.1 = e.1.replace("Model team", "Our team");
        let d = check(&h, &t);
        assert_eq!(codes(&d), ["SD501"]);
        assert!(
            d[0].message
                .contains("delete the line and run `spec-dna init`"),
            "{}",
            d[0].message
        );
    }

    #[test]
    fn check_reports_each_handler_that_differs() {
        let h = lean();
        let mut t = installed(&h);
        let e = t.iter_mut().find(|x| x.0 == SETTINGS).unwrap();
        let mut doc = json::parse(&e.1).unwrap();
        let extra = json::parse(
            r#"{"matcher":"Bash","hooks":[{"type":"command","command":"spec-dna hook context-meter --levels 1"}]}"#,
        )
        .unwrap();
        let Some(Value::Arr(groups)) = get_mut(entry(&mut doc, "hooks"), "PostToolUse") else {
            panic!("no groups");
        };
        groups.push(extra);
        e.1 = json::pretty(&doc);
        let d = check(&h, &t);
        assert_eq!(codes(&d), ["SD502"], "{d:?}");
        assert!(d[0].target.starts_with("hooks.PostToolUse"), "{d:?}");
        // `init` rewrites the same handler, so the two agree.
        let p = plan(&t, Some(&h), &h, Mode::Upgrade).unwrap();
        let s = p.changes.iter().find(|c| c.path == SETTINGS).unwrap();
        t.retain(|x| x.0 != SETTINGS);
        t.push((SETTINGS.into(), s.text.clone()));
        assert!(check(&h, &t).is_empty());
    }

    #[test]
    fn a_value_of_the_wrong_type_refuses_the_plan() {
        let mut h = lean();
        h.deny = vec!["NotebookEdit".into()];
        for (text, path, ty) in [
            (
                r#"{"permissions":{"deny":"Bash"}}"#,
                "permissions.deny",
                "an array",
            ),
            (r#"{"permissions":[]}"#, "permissions", "an object"),
            (r#"{"hooks":[]}"#, "hooks", "an object"),
            (
                r#"{"hooks":{"PostToolUse":{}}}"#,
                "hooks.PostToolUse",
                "an array",
            ),
            (r#"{"env":"x"}"#, "env", "an object"),
            (r#"{"modelSettings":[]}"#, "modelSettings", "an object"),
            (
                r#"{"modelSettings":{"claude-opus-5-5":1}}"#,
                "modelSettings.claude-opus-5-5",
                "an object",
            ),
        ] {
            let e = plan(
                &[(SETTINGS.to_string(), text.to_string())],
                None,
                &h,
                Mode::Reconfigure,
            )
            .unwrap_err();
            assert!(e.contains(&format!("`{path}` must be {ty}")), "{text}: {e}");
            let d = check(&h, &[(SETTINGS.to_string(), text.to_string())]);
            let d: Vec<&Diag> = d.iter().filter(|d| d.path == SETTINGS).collect();
            assert_eq!(d.len(), 1, "{text}: {d:?}");
            assert_eq!(d[0].code, Code::HarnessSettingsInvalid);
            assert!(d[0].message.contains(&format!("`{path}` must be {ty}")));
        }
        // Null is no data; level none writes no such key.
        let p = plan(
            &[(SETTINGS.to_string(), r#"{"env":null}"#.to_string())],
            None,
            &h,
            Mode::Reconfigure,
        );
        assert!(p.is_ok());
        let none = Harness::new(Level::None);
        let p = plan(
            &[(SETTINGS.to_string(), r#"{"env":"x"}"#.to_string())],
            Some(&h),
            &none,
            Mode::Reconfigure,
        );
        assert!(p.is_ok());
    }

    #[test]
    fn marker_finder_does_not_mix_with_the_adapter_marker() {
        let adapter = crate::adapters::line();
        assert!(find_line(&adapter).is_none());
        let ours = claude_line(&lean()).unwrap();
        assert!(crate::adapters::check(&[(CLAUDE, format!("{ours}\n"))]).is_empty());
        assert!(
            crate::adapters::has_own_text(&format!("{ours}\n")),
            "adapters see a harness line as own text"
        );
        assert!(has_marker(&files(&lean())[0].1));
        assert!(!has_marker(
            "# Notes\n\n<!-- spec-dna harness v<version> h:<hash> -->\n"
        ));
        // Text after the marker is a hand edit. The file stays a generated file.
        let mut text = files(&lean())[0].1.clone();
        text.push_str("tail\n");
        assert!(has_marker(&text));
    }

    #[test]
    fn output_paths_and_wanted_paths() {
        for p in [
            CLAUDE,
            SETTINGS,
            TEAM_PAGE,
            ".claude/agents/checker.md",
            ".claude/agents/Explore.md",
            ".claude/agents/fallback-advisor.md",
        ] {
            assert!(is_output_path(p), "{p}");
            assert!(wanted(p), "{p}");
        }
        assert!(!is_output_path(".claude/agents/own.md"));
        assert!(wanted(".claude/agents/own.md"));
        assert!(
            !wanted(".claude/agents/sub/own.md")
                && !wanted("docs/a.md")
                && !wanted(".claude/agents/x.txt")
        );
        assert!(is_installed(&installed(&lean())));
        assert!(!is_installed(&[(
            CLAUDE.into(),
            crate::adapters::upsert_line("")
        )]));
    }
}

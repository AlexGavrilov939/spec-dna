//! `spec-dna init` (docs/init.md). It detects the facts of the repository
//! (`detect.rs`). It asks at most 14 questions with the detected defaults (see
//! `QUESTIONS`). It
//! shows a preview (the full `spec-dna.toml` and the files to create or
//! change), and it writes only after a confirmation. It never edits a
//! document. It changes an instruction file (`AGENTS.md`, `CLAUDE.md`) only by
//! its generated lines: the spec-dna line, and the model-team line of the
//! agent harness (ADR 0004).
//!
//! The modes are:
//!
//! - interactive (a terminal)
//! - `--non-interactive` (the detected defaults, also without a terminal)
//! - `--answers <file>`
//! - `--reconfigure` (the current values as defaults)
//! - `--dry-run` (the preview only)
//!
//! If `spec-dna.toml` exists and `--reconfigure` is not set, `init` updates
//! the adapters, and the harness files if the configuration has a `[harness]`
//! table. It never removes a file then.

use crate::adapters::{self, VERSION};
use crate::config::{self, Approval, ChangePolicy, Config, DEFAULT_BUDGETS, Gate, Profile, Rules};
use crate::detect::{self, Facts, GitInfo, Row};
use crate::diag::Severity;
use crate::harness::{self, Effort, GoneRole, Harness, Level, Model, Protect, Role, Skill};
use crate::json::{self, Obj};
use crate::layer::{self, Graph};
use crate::model::DocType;
use crate::repo::{Fatal, Repo, Snapshot};
use crate::{evals, harness_text, skeleton};
use std::io::{BufRead, IsTerminal, Write};

/// The options of `init`.
pub struct Opts {
    pub root: Option<String>,
    pub non_interactive: bool,
    pub answers: Option<String>,
    pub reconfigure: bool,
    pub dry_run: bool,
    pub json: bool,
    /// Each `--approver <e-mail>` (owner review of 0.7.0, item 1).
    pub approvers: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    Covers,
    VerifiedBy,
    Both,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Budgets {
    Default,
    Relaxed,
    Tight,
    Off,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Adapter {
    AgentsMd,
    ClaudeMd,
    Skill,
}

const ADAPTERS: [(Adapter, &str); 3] = [
    (Adapter::AgentsMd, "agents-md"),
    (Adapter::ClaudeMd, "claude-md"),
    (Adapter::Skill, "skill"),
];

/// The answer to question 2: the layer graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphChoice {
    /// The graph for a project of fewer than 30 documents.
    Small,
    /// The graph for a project of 30 documents or more.
    Default,
    /// The `[layers]` tables of the current `spec-dna.toml`.
    Custom,
}

impl GraphChoice {
    fn name(self) -> &'static str {
        match self {
            GraphChoice::Small => "small",
            GraphChoice::Default => "default",
            GraphChoice::Custom => "custom",
        }
    }

    fn from_name(s: &str) -> Option<GraphChoice> {
        [
            GraphChoice::Small,
            GraphChoice::Default,
            GraphChoice::Custom,
        ]
        .into_iter()
        .find(|g| g.name() == s)
    }

    /// The choice that the size of a project gives.
    fn of_size(documents: usize) -> GraphChoice {
        if documents < SMALL_LIMIT {
            GraphChoice::Small
        } else {
            GraphChoice::Default
        }
    }
}

/// A project of fewer documents than this gets the small graph.
const SMALL_LIMIT: usize = 30;

/// The questions of `init` with their numbers in the design
/// (docs/design/0.6.0.md, "Init questions"), and the name that the preview
/// shows. Question 12 has a second prompt for the parts of the harness, with
/// the same number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Question {
    Profile,
    Graph,
    Areas,
    Policy,
    Gate,
    Evidence,
    Language,
    Mapping,
    Budgets,
    Adapters,
    Ci,
    Harness,
    Models,
    Evals,
}

/// The questions in the order that `init` asks them: the number of the design
/// and the name in the preview.
const QUESTIONS: [(Question, usize, &str); 14] = [
    (Question::Profile, 1, "profile"),
    (Question::Graph, 2, "layer graph"),
    (Question::Areas, 3, "areas of requirements"),
    (Question::Policy, 4, "change policy"),
    (Question::Gate, 5, "change gate"),
    (Question::Evidence, 6, "evidence policy and link style"),
    (
        Question::Language,
        7,
        "normative language and heading aliases",
    ),
    (Question::Mapping, 8, "mapping"),
    (Question::Budgets, 9, "budget level"),
    (Question::Adapters, 10, "agent adapters"),
    (Question::Ci, 11, "CI step, hook and base branch"),
    (Question::Harness, 12, "agent harness"),
    (Question::Models, 13, "harness models"),
    (Question::Evals, 14, "evals"),
];

impl Question {
    fn entry(self) -> (Question, usize, &'static str) {
        *QUESTIONS
            .iter()
            .find(|q| q.0 == self)
            .expect("each question is in the table")
    }

    fn number(self) -> usize {
        self.entry().1
    }

    /// The start of a prompt: the number, a point and the text.
    fn prompt(self, text: &str) -> String {
        format!("{}. {text}", self.number())
    }
}

/// The answer to question 14.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvalsAnswer {
    /// No `[evals]` table. A current table goes, and its files stay.
    None,
    /// The table `[evals]` and the starter files.
    Skeleton,
}

impl EvalsAnswer {
    fn name(self) -> &'static str {
        match self {
            EvalsAnswer::None => "none",
            EvalsAnswer::Skeleton => "skeleton",
        }
    }

    fn from_name(s: &str) -> Option<EvalsAnswer> {
        [EvalsAnswer::None, EvalsAnswer::Skeleton]
            .into_iter()
            .find(|a| a.name() == s)
    }
}

/// The 3 parts of question 12: `skills`, `review-md` and `protect-tests`.
/// A part that the person did not set follows its default. The default of
/// `skills` follows the final change policy, and the default of
/// `protect-tests` follows the test globs, so `settle_parts` computes both
/// after the answers.
#[derive(Clone, Debug, Default)]
pub struct Parts {
    /// True: `skills` has no value in the file and no answer.
    derive_skills: bool,
    derive_review: bool,
    derive_protect: bool,
    /// The stage skills whose file exists with no marker. The default leaves
    /// them out.
    unmarked_skills: Vec<Skill>,
    /// True if `REVIEW.md` exists with no marker.
    unmarked_review: bool,
}

impl Parts {
    /// All 3 parts follow their defaults.
    fn derived() -> Parts {
        Parts {
            derive_skills: true,
            derive_review: true,
            derive_protect: true,
            ..Parts::default()
        }
    }
}

/// The answers of the questions, with the detected defaults.
#[derive(Clone, Debug)]
pub struct Plan {
    pub kind: &'static str,
    pub profile: Profile,
    pub graph: GraphChoice,
    /// For `custom`: the graph of the current file, and true if the file has
    /// the tables `[layers]`. `None` if there is no current file.
    pub custom: Option<(Graph, bool)>,
    /// The areas of `requirements/`. Empty: one folder.
    pub areas: Vec<String>,
    /// The answer to question 4: `[change] policy`.
    pub change_policy: ChangePolicy,
    /// The answer to question 5: `[change] gate`. The question is not asked
    /// for the policy `none`, and the key keeps its value.
    pub change_gate: Gate,
    /// True if `init` writes the table `[change]` also for the policy
    /// `none`: a new project, or a current file that has the table.
    pub change_table: bool,
    /// `[change] high-risk-layers` of the current file. `None`: the default
    /// layers. `init` writes only the layers that the graph has.
    pub high_risk_layers: Option<Vec<String>>,
    /// `[change] high-risk-paths` of the current file.
    pub high_risk_paths: Vec<String>,
    /// `[change] max-fanout`.
    pub max_fanout: u32,
    /// `[change] max-task-files`. No question asks for it. `--reconfigure`
    /// keeps the value if the current file sets the key. `None`: no key.
    pub max_task_files: Option<u32>,
    /// `[change] base`. No question asks for it. `--reconfigure` keeps the
    /// value that the file sets. The config loader has checked the name.
    pub change_base: Option<String>,
    /// `[change] approval` and `[change] approvers` (design 0.7.0, D15). No
    /// question asks for them. `settle_approval` sets them from the profile
    /// and from `git_email`, unless the current file sets `approval`.
    pub approval: Approval,
    pub approvers: Vec<String>,
    /// The list that a person gave: `--approver`, the answers key
    /// `approvers` or the approvers prompt of question 5. `Some(vec![])`
    /// means "no approvers". `None`: no source gave a list.
    pub approvers_given: Option<Vec<String>>,
    /// True if the table `[change]` of the current file has the key
    /// `approval`. Then `init` keeps the 2 keys.
    pub approval_set: bool,
    /// The value of `git config user.email`, or `None` if it is not set.
    pub git_email: Option<String>,
    /// The documents that detection found in the roots.
    pub documents: usize,
    /// The root of a new project (`docs`, or the directory of `--root`).
    pub root: String,
    /// The reasons why `init` did not write a root or a default type.
    pub notes: Vec<String>,
    /// For `--reconfigure`: the key `types` of the current file. The file
    /// keeps it, and it narrows the types of the skill.
    pub kept_types: Vec<DocType>,
    /// `None`: the profile decides. `Some(s)`: the rule `evidence`.
    pub evidence: Option<Option<Severity>>,
    pub style: Style,
    pub tests: Vec<String>,
    pub language: String,
    pub outcome: Vec<String>,
    pub amendments: Vec<String>,
    pub rows: Vec<Row>,
    /// True if the roots are the layout of a new project.
    pub layout: bool,
    pub budgets: Budgets,
    pub adapters: Vec<Adapter>,
    pub ci: String,
    pub hook: String,
    pub base: String,
    pub exclude: Vec<String>,
    /// For `--reconfigure`: the lines of keys that no question asks for, such
    /// as `install-source` or `[context]`. They stay as they are.
    pub kept: Vec<String>,
    /// For `--reconfigure`: the other rules of `[rules]`.
    pub rules: Rules,
    /// For `--reconfigure`: the `[budgets.<type>]` tables. A new budget level
    /// replaces them.
    pub kept_budgets: Vec<String>,
    /// The agent harness (ADR 0004).
    pub harness: Harness,
    /// The harness of the current configuration, for `--reconfigure`.
    pub harness_old: Option<Harness>,
    /// Why a role is off by default.
    pub harness_notes: Vec<String>,
    /// The parts of question 12 that follow a default.
    pub parts: Parts,
    /// The answer to question 14.
    pub evals: EvalsAnswer,
    /// The table `[evals]` that `init` writes for the answer `skeleton`: the
    /// current table, or the defaults.
    pub evals_table: config::Evals,
    /// True if the current file has the table `[evals]`.
    pub evals_current: bool,
}

impl Plan {
    /// The layer graph that the answer to question 2 gives.
    fn layers(&self) -> Graph {
        match self.graph {
            // The caller checked that there is a current file.
            GraphChoice::Custom => self
                .custom
                .as_ref()
                .map_or_else(|| Graph::implied(&[]), |c| c.0.clone()),
            size => Graph::of_size(size.name()).expect("a size has a graph"),
        }
    }

    /// True if the new file has the tables `[layers.*]`. `custom` keeps what
    /// the current file has: a file with no such tables stays without them.
    fn writes_layers(&self) -> bool {
        match self.graph {
            GraphChoice::Custom => self.custom.as_ref().is_some_and(|c| c.1),
            _ => true,
        }
    }
}

impl Plan {
    /// True if the new file has the table `[change]`. A file with no table
    /// and the policy `none` stays without it: no table means `none`.
    fn writes_change(&self) -> bool {
        self.change_policy != ChangePolicy::None || self.change_table
    }

    /// The layers of `[change] high-risk-layers`: the current or the default
    /// names, without the names that the graph does not have.
    fn high_risk_layers_written(&self) -> Vec<String> {
        let graph = self.layers();
        let names: Vec<String> = match &self.high_risk_layers {
            Some(v) => v.clone(),
            None => config::DEFAULT_HIGH_RISK_LAYERS
                .map(str::to_string)
                .to_vec(),
        };
        names
            .into_iter()
            .filter(|n| graph.index(n).is_some())
            .collect()
    }

    /// The `[change]` table, or an empty string.
    fn change_section(&self) -> String {
        if !self.writes_change() {
            return String::new();
        }
        let mut out = format!(
            "\n[change]  # the change process (docs/config.md)\npolicy = {}  # \"three-class\", \"two-class\" or \"none\"\ngate = {}  # \"block\", \"warn\" or \"off\"\nhigh-risk-layers = {}  # a delta in these layers makes a change higher-risk\nhigh-risk-paths = {}  # globs from the repository root\nmax-fanout = {}  # documents that one change can touch before SD905\napproval = {}  # \"base\" or \"branch\": where an approval can be\n",
            toml_str(self.change_policy.name()),
            toml_str(self.change_gate.name()),
            toml_list(&self.high_risk_layers_written()),
            toml_list(&self.high_risk_paths),
            self.max_fanout,
            toml_str(self.approval.name())
        );
        if !self.approvers.is_empty() {
            out.push_str(&format!(
                "approvers = {}  # the author e-mails of the approval commits\n",
                toml_list(&self.approvers)
            ));
        }
        if let Some(n) = self.max_task_files {
            out.push_str(&format!(
                "max-task-files = {n}  # tasks of an approved plan: paths in `Files:` before SD924\n"
            ));
        }
        if let Some(b) = &self.change_base {
            out.push_str(&format!(
                "base = {}  # the local branch that the change checks use as the base\n",
                toml_str(b)
            ));
        }
        out
    }

    /// The table `[evals]` and the empty line after it, or an empty string.
    /// It comes directly before `[harness]`.
    fn evals_section(&self) -> String {
        if self.evals != EvalsAnswer::Skeleton {
            return String::new();
        }
        let e = &self.evals_table;
        format!(
            "[evals]  # agent evals (docs/config.md)\ndir = {}\nmin-pass-rate = {}  # the share of evals that must pass\nruns = {}  # the runs of an eval that sets no `runs`\nmax-total-usd = {}  # eval run refuses or stops above this spend\n\n",
            toml_str(&e.dir),
            e.min_pass_rate,
            e.runs,
            e.max_total_usd
        )
    }

    /// The `CODEOWNERS` line for the specs of the changes, or `None` for the
    /// policy `none`. Only a higher-risk change has a spec. The path is the
    /// folder of the `intent` layer in the first mixed root.
    fn codeowners(&self) -> Option<String> {
        if self.change_policy == ChangePolicy::None {
            return None;
        }
        let graph = self.layers();
        let dir = graph
            .layer_of_type(DocType::Intent)
            .and_then(|i| graph.layers()[i].dir())
            .unwrap_or("intent");
        let root = self
            .rows
            .iter()
            .find(|r| r.ty.is_none())
            .map_or(self.root.as_str(), |r| r.path.as_str());
        let root = root.trim_matches('/');
        let prefix = if root.is_empty() || root == "." {
            String::new()
        } else {
            format!("{root}/")
        };
        Some(format!("{prefix}{dir}/*/spec.md @OWNER"))
    }
}

/// The warning of a policy that needs types: the current `types` list lacks a
/// type of the intent layer (D16). `init` does not edit the key, because no
/// question asks for it. A file with no list allows each type.
fn types_warning(plan: &Plan) -> Option<String> {
    if plan.change_policy == ChangePolicy::None || plan.kept_types.is_empty() {
        return None;
    }
    let absent: Vec<&str> = [DocType::Intent, DocType::Spec, DocType::Plan]
        .into_iter()
        .filter(|t| !plan.kept_types.contains(t))
        .map(DocType::name)
        .collect();
    let list = match absent.split_last() {
        None => return None,
        Some((last, [])) => last.to_string(),
        Some((last, [first])) => format!("{first} and {last}"),
        Some((last, head)) => format!("{} and {last}", head.join(", ")),
    };
    Some(format!(
        "the policy {} needs the types intent, spec and plan; add {list} to types in spec-dna.toml",
        plan.change_policy.name()
    ))
}

/// The note that gives the `CODEOWNERS` line. `init` never writes the file.
fn codeowners_note(line: &str) -> String {
    format!(
        "CODEOWNERS: add the line `{line}` to your CODEOWNERS file, and replace `@OWNER` with the owner. `init` does not write that file. Only a higher-risk change has a spec, so the line protects its approval. A higher-risk plan has the same path as a routine plan, so a path rule cannot select it. The process of your repository (for example branch protection) must require the owner for it"
    )
}

/// The policy is `none` if the graph has no `intent` layer: a change needs
/// the folder `intent/<slug>/`. A note says why. The call is idempotent.
fn settle_change(plan: &mut Plan) {
    if plan.change_policy == ChangePolicy::None {
        return;
    }
    if plan.layers().layer_of_type(DocType::Intent).is_some() {
        return;
    }
    let note = format!(
        "the policy `{}` needs an `intent` layer, and the {} graph has none: `init` writes the policy `none`. Add a layer with `spec-dna layer add intent --types intent,spec,plan`, then run `init --reconfigure` to choose a policy",
        plan.change_policy.name(),
        plan.graph.name()
    );
    plan.change_policy = ChangePolicy::None;
    // The answer asked for a policy, so the file says `none` and does not
    // leave the key out.
    plan.change_table = true;
    if !plan.notes.contains(&note) {
        plan.notes.push(note);
    }
}

/// Sets `[change] approval` and `[change] approvers` (design 0.7.0, D15, and
/// the owner review, item 1). The sources, in the order of priority: the list
/// that a person gave (`approvers_given`), the current file that sets
/// `approval` (it keeps its 2 keys), and the e-mail of git, if it is not the
/// address of an agent or a bot. `strict` gets `base` and no new approvers,
/// because it reads the approvals of the base only. With no person to list,
/// the result is `base`, no approvers and a note. The call is idempotent.
fn settle_approval(plan: &mut Plan) {
    if !plan.writes_change() {
        return;
    }
    let note = |plan: &mut Plan, n: String| {
        if !plan.notes.contains(&n) {
            plan.notes.push(n);
        }
    };
    if plan.profile == Profile::Strict {
        plan.approval = Approval::Base;
        if plan.approvers_given.as_ref().is_some_and(|l| !l.is_empty()) {
            note(
                plan,
                "`strict` reads the approvals of the base only, so `init` writes no approvers"
                    .to_string(),
            );
        }
        return;
    }
    if let Some(list) = &plan.approvers_given {
        plan.approval = if list.is_empty() {
            Approval::Base
        } else {
            Approval::Branch
        };
        plan.approvers = list.clone();
        return;
    }
    if plan.approval_set {
        for e in plan
            .approvers
            .clone()
            .iter()
            .filter(|e| config::is_agent_address(e))
        {
            note(
                plan,
                format!(
                    "`[change] approvers` lists `{e}`, the address of an agent or a bot, and `init` keeps it. Remove it in a pull request of its own (SD926)"
                ),
            );
        }
        return;
    }
    if plan.approvers.is_empty() {
        let email = plan.git_email.clone().filter(|e| !e.trim().is_empty());
        match email {
            Some(e) if !config::is_agent_address(&e) => plan.approvers = vec![e],
            Some(e) => note(
                plan,
                format!(
                    "git `user.email` is `{e}`, the address of an agent or a bot, so `init` writes `[change] approval = \"base\"` and no approvers. To use the approval in the branch, run `spec-dna init --reconfigure --approver <your e-mail>` (docs/config.md)"
                ),
            ),
            None => note(
                plan,
                "git has no `user.email`, so `init` writes `[change] approval = \"base\"` and no approvers. To use the approval in the branch, set `[change] approval = \"branch\"` and `[change] approvers` in spec-dna.toml (docs/config.md)".to_string(),
            ),
        }
    }
    plan.approval = if plan.approvers.is_empty() {
        Approval::Base
    } else {
        Approval::Branch
    };
}

/// The e-mails of a list that a person gave, checked one by one. An item
/// follows the rules of the config reader (not empty, an `@`, no `*`), and it
/// is never the address of an agent or a bot. `source` names where the list
/// comes from, for the message. The result has no item twice (the compare
/// ignores case).
fn parse_approvers(source: &str, items: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for raw in items {
        let e = raw.trim();
        if e.is_empty() || e.contains('*') || !e.contains('@') {
            return Err(format!(
                "{source}: `{e}` is not an e-mail; give the e-mail in full, such as `owner@example.test` (docs/config.md)"
            ));
        }
        if config::is_agent_address(e) {
            return Err(format!(
                "{source}: `{e}` is the address of an agent or a bot; give the e-mail of a person (docs/config.md)"
            ));
        }
        if !out.iter().any(|x| x.eq_ignore_ascii_case(e)) {
            out.push(e.to_string());
        }
    }
    Ok(out)
}

/// The default of the approvers prompt: the list of the first source that has
/// one. The sources are the list that a flag or an answers file gave, the
/// current list of `--reconfigure`, and the e-mail of git if it is not the
/// address of an agent or a bot. The result can be empty.
fn approvers_default(plan: &Plan) -> Vec<String> {
    if let Some(list) = &plan.approvers_given {
        return list.clone();
    }
    if plan.approval_set {
        return plan.approvers.clone();
    }
    plan.git_email
        .iter()
        .filter(|e| !config::is_agent_address(e))
        .cloned()
        .collect()
}

/// The second prompt of question 5: the e-mails whose approval commits count.
/// The prompt runs for `lite` and `standard`, and only when `init` writes the
/// table `[change]` with a policy other than `none`. The answer `none` or an
/// empty list gives `approval = "base"`.
fn ask_approvers(plan: &mut Plan, io: &mut Ask) -> Result<(), String> {
    if plan.profile == Profile::Strict
        || plan.change_policy == ChangePolicy::None
        || !plan.writes_change()
    {
        return Ok(());
    }
    if let Some(e) = plan.git_email.iter().find(|e| config::is_agent_address(e)) {
        io.say(&format!(
            "    git `user.email` is `{e}`, the address of an agent or a bot, so it is not offered."
        ));
    }
    io.say("    An approval commit of one of these e-mails counts in the branch (`approval = \"branch\"`). List persons only. none writes `approval = \"base\"`: the merge is the approval.");
    let default = approvers_default(plan);
    let shown = if default.is_empty() {
        "none".to_string()
    } else {
        default.join(", ")
    };
    let a = io.line(
        &Question::Gate.prompt(
            "Approvers: the e-mails whose approval commits count, comma-separated, or none",
        ),
        &shown,
    );
    let list = if a.eq_ignore_ascii_case("none") {
        Vec::new()
    } else {
        parse_approvers("question 5", &words(&a))?
    };
    // The current list of `--reconfigure` as the answer changes nothing.
    let same = plan.approval_set
        && plan.approvers_given.is_none()
        && list.len() == plan.approvers.len()
        && list
            .iter()
            .all(|e| plan.approvers.iter().any(|x| x.eq_ignore_ascii_case(e)));
    if !same {
        plan.approvers_given = Some(list);
    }
    Ok(())
}

/// `strict` gets `[harness] protect-approvals = true` in a file that has no
/// such key, so the profile has no hidden effect (ADR 0017). A file that has
/// the key keeps its value, and the level `none` writes no key.
fn settle_approvals_hook(plan: &mut Plan) {
    if plan.profile == Profile::Strict
        && plan.harness.level != Level::None
        && plan.harness.protect_approvals.is_none()
    {
        plan.harness.protect_approvals = Some(true);
    }
}

/// The types that a graph has, in the order of `DocType::ALL`.
fn graph_types(g: &Graph) -> Vec<DocType> {
    DocType::ALL
        .into_iter()
        .filter(|t| g.layer_of_type(*t).is_some())
        .collect()
}

/// The roots of a new project: one mixed root with no `default-type`. The
/// folder of a layer gives the type of a document (`requirements/` gives
/// `requirements`), so a new document needs no `type:` line.
fn layout_rows(root: &str) -> Vec<Row> {
    let mut main = blank_row(root, None, DocType::Research);
    main.no_default_type = true;
    main.why = "new documents: the folder of a layer gives the type; `spec-dna new <type> <id>` writes a document there".into();
    vec![main]
}

fn blank_row(path: &str, ty: Option<DocType>, default_type: DocType) -> Row {
    Row {
        path: path.to_string(),
        ty,
        default_type,
        no_default_type: false,
        status: None,
        legacy_ids: Vec::new(),
        legacy_keys: Vec::new(),
        status_map: Vec::new(),
        required_off: Vec::new(),
        exclude: Vec::new(),
        docs: 0,
        decisions: 0,
        extra: Vec::new(),
        why: String::new(),
    }
}

/// Lists the files of the working tree and reads the facts. Returns the file
/// list too: tracked files, and untracked files that git does not ignore.
fn detect_repo(repo: &Repo) -> Result<(Facts, Vec<String>), Fatal> {
    let snap = Snapshot::worktree(repo, &[], true)?;
    let files: Vec<String> = snap.files.iter().map(|e| e.path.clone()).collect();
    let root = repo.root.clone();
    let facts = detect::detect(
        &files,
        |p| std::fs::read_to_string(root.join(p)).ok(),
        GitInfo {
            base: repo.git.default_branch(),
        },
    );
    Ok((facts, files))
}

/// The plan of the detected defaults.
fn defaults(facts: &Facts, root: Option<&str>) -> Plan {
    let rows: Vec<Row> = facts.rows.clone();
    let layout = rows.is_empty();
    let docs_root = root.unwrap_or("docs").trim_matches('/');
    let docs_root = if docs_root.is_empty() { "." } else { docs_root };
    // The size of the project gives the graph: the documents of the detected
    // roots. A new project has none, so it gets the small graph.
    let documents: usize = rows.iter().map(|r| r.docs).sum();
    let mut adapters_default = vec![Adapter::Skill];
    let agents = facts.agent_files.iter().any(|f| f == "AGENTS.md");
    let claude = facts.agent_files.iter().any(|f| f == "CLAUDE.md");
    if agents || !claude {
        adapters_default.insert(0, Adapter::AgentsMd);
    }
    adapters_default.insert(adapters_default.len() - 1, Adapter::ClaudeMd);
    let (language, language_note) = detected_language(facts);
    Plan {
        kind: if facts.brownfield {
            "brownfield"
        } else {
            "greenfield"
        },
        profile: Profile::Standard,
        graph: GraphChoice::of_size(documents),
        custom: None,
        areas: facts.areas.clone(),
        change_policy: ChangePolicy::ThreeClass,
        change_gate: Gate::Block,
        change_table: true,
        high_risk_layers: None,
        high_risk_paths: Vec::new(),
        max_fanout: config::DEFAULT_MAX_FANOUT,
        max_task_files: None,
        change_base: None,
        approval: Approval::Base,
        approvers: Vec::new(),
        approvers_given: None,
        approval_set: false,
        git_email: None,
        documents,
        root: docs_root.to_string(),
        notes: language_note
            .into_iter()
            .chain(facts.areas_dropped.iter().map(|(kept, other)| {
                format!(
                    "the folders `{other}` and `{kept}` of `requirements/` differ only in case, and the reader of `spec-dna.toml` stops such a pair. `areas` lists `{kept}`, which has more documents or the first name. Rename `{other}`, or move its documents with `spec-dna mv`, and add it to `areas` by hand if you want it"
                )
            }))
            .collect(),
        kept_types: Vec::new(),
        evidence: None,
        style: Style::Both,
        tests: facts.tests.iter().map(|t| t.0.clone()).collect(),
        language,
        outcome: facts.outcome_aliases.clone(),
        amendments: facts.amendment_aliases.clone(),
        rows: if layout { layout_rows(docs_root) } else { rows },
        layout,
        budgets: Budgets::Default,
        adapters: adapters_default,
        ci: if facts.ci.iter().any(|c| c == "GitHub Actions") {
            "github".into()
        } else if facts.ci.iter().any(|c| c == "GitLab CI") {
            "gitlab".into()
        } else {
            "none".into()
        },
        hook: facts
            .hooks
            .iter()
            .find(|h| ["pre-commit", "husky", "lefthook"].contains(&h.as_str()))
            .cloned()
            .unwrap_or_else(|| "none".into()),
        base: if facts.base.is_empty() {
            "main".into()
        } else {
            facts.base.clone()
        },
        exclude: facts.exclude.iter().map(|e| e.0.clone()).collect(),
        kept: Vec::new(),
        rules: Vec::new(),
        kept_budgets: Vec::new(),
        harness: Harness::new(Level::Lean),
        harness_old: None,
        harness_notes: Vec::new(),
        parts: Parts::derived(),
        evals: EvalsAnswer::Skeleton,
        evals_table: default_evals(config::DEFAULT_EVALS_DIR),
        evals_current: false,
    }
}

/// The table `[evals]` with the default values and the folder `dir`.
fn default_evals(dir: &str) -> config::Evals {
    config::Evals {
        dir: dir.to_string(),
        min_pass_rate: config::DEFAULT_EVALS_MIN_PASS_RATE,
        runs: config::DEFAULT_EVALS_RUNS,
        max_total_usd: config::DEFAULT_EVALS_MAX_TOTAL_USD,
    }
}

/// The plan of the current configuration, for `--reconfigure`.
fn current(cfg: &Config, text: &str, facts: &Facts) -> Plan {
    let mut p = defaults(facts, None);
    p.kind = "reconfigure";
    // The language of the current file is the default of question 7. The only
    // note of `defaults` is about the detected language, so it ends here.
    p.language = cfg.language.clone();
    p.notes.clear();
    p.profile = cfg.profile;
    p.layout = false;
    // The current layers are the default of question 2.
    p.graph = GraphChoice::Custom;
    p.custom = Some((cfg.layers.clone(), cfg.layers_declared));
    p.areas = cfg.areas.clone();
    // The current `[change]` is the default of questions 4 and 5. A file with
    // no table has the policy `none`, and `init` writes no table for it unless
    // the answer changes the policy.
    p.change_policy = cfg.change_policy;
    p.change_table = text
        .parse::<toml::Table>()
        .is_ok_and(|t| t.contains_key("change"));
    // A table with no `gate` leaves SD910 to the profile. The gate that gives
    // the same severity is the default, so a default answer changes nothing.
    p.change_gate = match cfg.change_gate {
        Some(g) => g,
        None if p.change_table && cfg.profile == Profile::Lite => Gate::Warn,
        None => Gate::Block,
    };
    // The current value stays only if it is not the default of the current
    // graph. The default follows the new graph (D17).
    p.high_risk_layers = (p.change_table
        && cfg.high_risk_layers != config::default_high_risk_layers(&cfg.layers))
    .then(|| cfg.high_risk_layers.clone());
    p.high_risk_paths = cfg.high_risk_paths.clone();
    p.max_fanout = cfg.max_fanout;
    p.max_task_files = text
        .parse::<toml::Table>()
        .ok()
        .and_then(|t| {
            t.get("change")
                .and_then(toml::Value::as_table)
                .map(|c| c.contains_key("max-task-files"))
        })
        .unwrap_or(false)
        .then_some(cfg.max_task_files);
    p.change_base = cfg.change_base.clone();
    p.approval = cfg.change_approval;
    p.approvers = cfg.change_approvers.clone();
    p.approval_set = text
        .parse::<toml::Table>()
        .ok()
        .and_then(|t| {
            t.get("change")
                .and_then(toml::Value::as_table)
                .map(|c| c.contains_key("approval"))
        })
        .unwrap_or(false);
    p.kept_types = cfg.types.clone();
    p.exclude = cfg.exclude.clone();
    p.evidence = cfg.rules.iter().find(|r| r.0 == "evidence").map(|r| r.1);
    // Every current rule stays. The answers for evidence, budgets and the
    // language replace their rule when `render` writes the file.
    p.rules = cfg.rules.clone();
    if cfg.rules.iter().any(|r| r.0 == "budgets" && r.1.is_none()) {
        p.budgets = Budgets::Off;
    }
    if cfg.evidence_tests.is_empty() {
        p.style = Style::VerifiedBy;
    }
    // The current globs stay. The default list renders as no `[evidence]`.
    p.tests = cfg.evidence_tests.clone();
    p.rows = cfg.roots.iter().map(row_of_root).collect();
    if let Some(h) = &cfg.harness {
        p.harness = h.clone();
        p.harness_old = Some(h.clone());
    }
    // A part that the file sets stays. A part that the file has no key for
    // follows its default: `Harness::parse` cannot tell an absent `skills`
    // from `skills = []`, so the raw table decides.
    let has_key = |k: &str| {
        text.parse::<toml::Table>()
            .ok()
            .and_then(|t| {
                t.get("harness")
                    .and_then(|h| h.as_table().map(|h| h.contains_key(k)))
            })
            .unwrap_or(false)
    };
    p.parts = Parts {
        derive_skills: !has_key("skills"),
        derive_review: !has_key("review-md"),
        derive_protect: !has_key("protect-tests"),
        ..Parts::default()
    };
    // The current table is the default of question 14. A file with no table
    // has the answer `none`.
    p.evals_current = cfg.evals.is_some();
    p.evals = if cfg.evals.is_some() {
        EvalsAnswer::Skeleton
    } else {
        EvalsAnswer::None
    };
    if let Some(e) = &cfg.evals {
        p.evals_table = e.clone();
    }
    // The outcome and amendment aliases are in the rows already.
    p.outcome = Vec::new();
    p.amendments = Vec::new();
    // Keep the keys that no question asks for, as written.
    let table: toml::Table = text.parse().unwrap_or_default();
    for key in ["install-source", "self-urls", "types"] {
        if let Some(v) = table.get(key) {
            p.kept.push(format!("{key} = {}", toml_value(v)));
        }
    }
    for key in [
        "lint",
        "context",
        "repos",
        "conventions",
        "groups",
        "templates",
        "spec-dna-sha256",
    ] {
        if let Some(toml::Value::Table(t)) = table.get(key) {
            p.kept.push(format!("\n[{key}]"));
            for (k, v) in t {
                p.kept.push(format!("{} = {}", toml_key(k), toml_value(v)));
            }
        }
    }
    if let Some(toml::Value::Table(t)) = table.get("budgets") {
        for (ty, v) in t {
            if let toml::Value::Table(b) = v {
                p.kept_budgets.push(format!("\n[budgets.{ty}]"));
                for k in ["soft", "hard"] {
                    if let Some(v) = b.get(k) {
                        p.kept_budgets.push(format!("{k} = {}", toml_value(v)));
                    }
                }
            }
        }
    }
    p
}

/// A row of a configured root, for `--reconfigure` and for the roots of an
/// answers file.
fn row_of_root(r: &config::Root) -> Row {
    let mut row = blank_row(
        if r.path.is_empty() { "." } else { &r.path },
        r.single_type,
        r.default_type.unwrap_or(DocType::Research),
    );
    // A mixed root with no `default-type` stays so.
    row.no_default_type = r.single_type.is_none() && r.default_type.is_none();
    row.status = r.status.clone();
    row.exclude = r.exclude.clone();
    row.legacy_ids = r
        .legacy_ids
        .iter()
        .map(|l| {
            if l.file.source == format!("{}*", l.alias) {
                l.alias.clone()
            } else {
                format!("{}|{}", l.alias, l.file.source)
            }
        })
        .collect();
    row.legacy_keys = r.legacy_keys.iter().map(|k| k.source.clone()).collect();
    row.status_map = r.status_map.clone();
    // The section keys that differ from the defaults stay as written.
    let defaults = config::default_required_sections();
    let mut required = Vec::new();
    let mut amendments = Vec::new();
    for t in row.types() {
        let groups = &r.required_sections[t as usize];
        if *groups != defaults[t as usize] {
            let g: Vec<String> = groups.iter().map(|g| toml_list(g)).collect();
            required.push(format!("{} = [{}]", t.name(), g.join(", ")));
        }
        let names = &r.amendment_sections[t as usize];
        let default: Vec<String> = if t == DocType::Decision {
            config::DEFAULT_AMENDMENT_SECTIONS
                .map(str::to_string)
                .to_vec()
        } else {
            Vec::new()
        };
        if *names != default {
            amendments.push(format!("{} = {}", t.name(), toml_list(names)));
        }
    }
    if !required.is_empty() {
        row.extra
            .push(format!("required-sections = {{ {} }}", required.join(", ")));
    }
    if !amendments.is_empty() {
        row.extra.push(format!(
            "amendment-sections = {{ {} }}",
            amendments.join(", ")
        ));
    }
    // The section keys of 0.3 (no default value): history, companion and
    // live sections.
    for (key, lists) in [
        ("history-sections", &r.history_sections),
        ("companion-sections", &r.companion_sections),
        ("live-sections", &r.live_sections),
    ] {
        // A list that differs from the default stays. The default of a
        // decision is the live section `Status`, so `decision = []` is a
        // choice of the file, and `decision = ["Status"]` is not.
        let default = |t: DocType| -> Vec<String> {
            if key == "live-sections" && t == DocType::Decision {
                config::DEFAULT_LIVE_SECTIONS.map(str::to_string).to_vec()
            } else {
                Vec::new()
            }
        };
        let v: Vec<String> = row
            .types()
            .into_iter()
            .filter(|&t| lists[t as usize] != default(t))
            .map(|t| format!("{} = {}", t.name(), toml_list(&lists[t as usize])))
            .collect();
        if !v.is_empty() {
            row.extra.push(format!("{key} = {{ {} }}", v.join(", ")));
        }
    }
    if !r.enforce_budgets {
        row.extra.push("enforce-budgets = false".into());
    }
    if let Some(p) = r.profile {
        row.extra.push(format!("profile = {}", toml_str(p.name())));
    }
    if !r.rules.is_empty() {
        let v: Vec<String> = r
            .rules
            .iter()
            .map(|(k, s)| format!("{} = {}", toml_key(k), toml_str(severity_name(*s))))
            .collect();
        row.extra.push(format!("rules = {{ {} }}", v.join(", ")));
    }
    row.why = String::new();
    row
}

fn toml_key(k: &str) -> String {
    if !k.is_empty()
        && k.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        k.to_string()
    } else {
        toml_str(k)
    }
}

/// A TOML basic string.
fn toml_str(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04X}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn toml_list(items: &[String]) -> String {
    let v: Vec<String> = items.iter().map(|s| toml_str(s)).collect();
    format!("[{}]", v.join(", "))
}

fn toml_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => toml_str(s),
        toml::Value::Integer(n) => n.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Datetime(d) => d.to_string(),
        toml::Value::Array(a) => {
            let v: Vec<String> = a.iter().map(toml_value).collect();
            format!("[{}]", v.join(", "))
        }
        toml::Value::Table(t) => {
            let v: Vec<String> = t
                .iter()
                .map(|(k, v)| format!("{} = {}", toml_key(k), toml_value(v)))
                .collect();
            format!("{{ {} }}", v.join(", "))
        }
    }
}

fn severity_name(s: Option<Severity>) -> &'static str {
    match s {
        None => "off",
        Some(s) => s.name(),
    }
}

/// Renders the plan as `spec-dna.toml`.
pub fn render(plan: &Plan) -> String {
    let mut out = String::from(
        "# spec-dna configuration (docs/config.md). `spec-dna init` wrote it; edit it freely.\n",
    );
    out.push_str(&format!("spec-dna = {}\n", toml_str(VERSION)));
    out.push_str(&format!(
        "profile = {}  # lite, standard or strict (docs/profiles.md)\n",
        toml_str(plan.profile.name())
    ));
    if !plan.exclude.is_empty() {
        out.push_str(&format!("exclude = {}\n", toml_list(&plan.exclude)));
    }
    if !plan.areas.is_empty() {
        out.push_str(&format!(
            "areas = {}  # the folders of requirements/\n",
            toml_list(&plan.areas)
        ));
    }
    if !plan.language.eq_ignore_ascii_case(config::DEFAULT_LANGUAGE) {
        out.push_str(&format!(
            "language = {}  # the language of normative text (a BCP 47 tag)\n",
            toml_str(&plan.language)
        ));
    }
    if plan.kept.iter().all(|k| !k.starts_with("install-source")) {
        out.push_str(&format!(
            "# install-source = {}  # a local or private clone\n",
            toml_str(config::DEFAULT_INSTALL_SOURCE)
        ));
    }
    let mut kept_top: Vec<&String> = Vec::new();
    let mut kept_tables: Vec<&String> = Vec::new();
    let mut in_table = false;
    for k in &plan.kept {
        in_table |= k.starts_with('\n');
        if in_table {
            kept_tables.push(k);
        } else {
            kept_top.push(k);
        }
    }
    for k in kept_top {
        out.push_str(&format!("{k}\n"));
    }
    // Rules: the answers that differ from the profile. The answers replace the
    // rule of the same group. The other rules stay.
    let mut rules: Vec<(String, &str)> = plan
        .rules
        .iter()
        .map(|(k, s)| (k.clone(), severity_name(*s)))
        .collect();
    let mut set = |key: &str, value: Option<&'static str>| {
        let at = rules.iter().position(|r| r.0 == key);
        match (at, value) {
            (Some(i), Some(v)) => rules[i].1 = v,
            (None, Some(v)) => rules.push((key.to_string(), v)),
            (Some(i), None) => {
                rules.remove(i);
            }
            (None, None) => {}
        }
    };
    set("evidence", plan.evidence.map(severity_name));
    match plan.budgets {
        Budgets::Off => set("budgets", Some("off")),
        // A budget level replaces a rule that turned the budgets off.
        _ if plan.rules.iter().any(|r| r.0 == "budgets" && r.1.is_none()) => set("budgets", None),
        _ => {}
    }
    if !rules.is_empty() {
        out.push_str("\n[rules]  # overrides of the profile, by rule group or by code\n");
        for (k, v) in rules {
            out.push_str(&format!("{} = {}\n", toml_key(&k), toml_str(v)));
        }
    }
    match plan.style {
        Style::VerifiedBy => {
            out.push_str("\n[evidence]\ntests = []  # evidence by `verified-by` only\n")
        }
        _ if !plan.tests.is_empty()
            && plan.tests != config::DEFAULT_TEST_GLOBS.map(str::to_string) =>
        {
            out.push_str(&format!(
                "\n[evidence]\ntests = {}  # test files with `covers: <doc-id>#<key>` tags\n",
                toml_list(&plan.tests)
            ))
        }
        _ => {}
    }
    let factor = match plan.budgets {
        Budgets::Relaxed => Some(2.0),
        Budgets::Tight => Some(0.5),
        _ => None,
    };
    if let Some(f) = factor {
        for t in &graph_types(&plan.layers()) {
            let (soft, hard) = DEFAULT_BUDGETS[*t as usize];
            if hard > 0 {
                out.push_str(&format!(
                    "\n[budgets.{}]\nsoft = {}\nhard = {}\n",
                    t.name(),
                    (f64::from(soft) * f) as u32,
                    (f64::from(hard) * f) as u32
                ));
            }
        }
    }
    if plan.budgets == Budgets::Default {
        for k in &plan.kept_budgets {
            out.push_str(&format!("{k}\n"));
        }
    }
    for k in kept_tables {
        out.push_str(&format!("{k}\n"));
    }
    out.push_str(&plan.change_section());
    out.push('\n');
    out.push_str(&plan.evals_section());
    out.push_str(&plan.harness.toml_section());
    if plan.writes_layers() {
        out.push_str("\n# The layer graph (docs/config.md). `spec-dna layer add` adds a layer.\n");
        out.push_str(&layer::render_graph(&plan.layers()));
    }
    for r in &plan.rows {
        out.push('\n');
        if !r.why.is_empty() {
            out.push_str(&format!("# {}\n", r.why));
        }
        out.push_str(&render_row(plan, r));
    }
    out
}

fn render_row(plan: &Plan, r: &Row) -> String {
    let mut out = format!("[[roots]]\npath = {}\n", toml_str(&r.path));
    match r.ty {
        Some(t) => out.push_str(&format!("type = {}\n", toml_str(t.name()))),
        None => {
            out.push_str("type = \"mixed\"\n");
            if !r.no_default_type {
                out.push_str(&format!(
                    "default-type = {}\n",
                    toml_str(r.default_type.name())
                ));
            }
        }
    }
    if let Some(s) = &r.status {
        out.push_str(&format!("status = {}\n", toml_str(s)));
    }
    if !r.exclude.is_empty() {
        out.push_str(&format!("exclude = {}\n", toml_list(&r.exclude)));
    }
    if !r.legacy_ids.is_empty() {
        let v: Vec<String> = r
            .legacy_ids
            .iter()
            .map(|l| match l.split_once('|') {
                Some((a, f)) => format!("{{ alias = {}, file = {} }}", toml_str(a), toml_str(f)),
                None => toml_str(l),
            })
            .collect();
        out.push_str(&format!("legacy-ids = [{}]\n", v.join(", ")));
    }
    if !r.legacy_keys.is_empty() {
        out.push_str(&format!("legacy-keys = {}\n", toml_list(&r.legacy_keys)));
    }
    if !r.status_map.is_empty() {
        let v: Vec<String> = r
            .status_map
            .iter()
            .map(|(a, b)| format!("{} = {}", toml_key(a), toml_str(b)))
            .collect();
        out.push_str(&format!("status-map = {{ {} }}\n", v.join(", ")));
    }
    // The headings of other languages go to the roots that hold decisions, or
    // that can hold new ones (a new project).
    let decisions = r.ty == Some(DocType::Decision)
        || (r.default_type == DocType::Decision && !r.no_default_type)
        || (r.ty.is_none() && (r.decisions > 0 || plan.layout));
    let mut required: Vec<String> = Vec::new();
    if decisions && !plan.outcome.is_empty() && !r.required_off.contains(&DocType::Decision) {
        let mut h = plan.outcome.clone();
        h.extend(config::DEFAULT_DECISION_SECTIONS.map(str::to_string));
        required.push(format!("decision = {}", toml_list(&h)));
    }
    for t in &r.required_off {
        required.push(format!("{} = []", t.name()));
    }
    let has = |k: &str| r.extra.iter().any(|e| e.starts_with(k));
    if !required.is_empty() && !has("required-sections") {
        out.push_str(&format!(
            "required-sections = {{ {} }}\n",
            required.join(", ")
        ));
    }
    if decisions && !plan.amendments.is_empty() && !has("amendment-sections") {
        let mut h = plan.amendments.clone();
        h.extend(config::DEFAULT_AMENDMENT_SECTIONS.map(str::to_string));
        out.push_str(&format!(
            "amendment-sections = {{ decision = {} }}\n",
            toml_list(&h)
        ));
    }
    for e in &r.extra {
        out.push_str(&format!("{e}\n"));
    }
    out
}

/// One file that `init` creates or changes.
pub struct Change {
    pub path: String,
    pub created: bool,
    /// True if `init` deletes the file. Only the harness removes files.
    pub remove: bool,
    pub text: String,
    pub note: String,
}

impl Change {
    fn verb(&self) -> &'static str {
        if self.remove {
            "remove"
        } else if self.created {
            "create"
        } else {
            "change"
        }
    }
}

/// A reader of the text files of a tree: the text of a path, or `None` if the
/// tree has no such file. `init` reads the disk. `upgrade` reads a tree in
/// memory (D148).
pub(crate) type ReadText<'a> = dyn Fn(&str) -> Option<String> + 'a;

/// The reader of the work directory. A file that exists but cannot be read
/// gives an empty text, as before.
fn disk(repo: &Repo) -> impl Fn(&str) -> Option<String> + '_ {
    move |p| {
        let path = repo.root.join(p);
        path.exists()
            .then(|| std::fs::read_to_string(path).unwrap_or_default())
    }
}

/// The adapter files for a set of adapters.
pub(crate) fn adapter_changes(
    src: &ReadText,
    chosen: &[Adapter],
    types: &[DocType],
) -> Result<Vec<Change>, Fatal> {
    let read = |p: &str| src(p).unwrap_or_default();
    let exists = |p: &str| src(p).is_some();
    let mut out = Vec::new();
    // An older version wrote a `claude` block that imported AGENTS.md. If
    // AGENTS.md has text of its own, the import stays as a plain line.
    let agents_target = chosen.contains(&Adapter::AgentsMd);
    let claude = adapters::migrate_claude(
        &read(adapters::CLAUDE),
        adapters::has_own_text(&read(adapters::AGENTS)),
    );
    let imports = adapters::imports_agents(&claude);
    // A file that the agent reads through an import keeps no second copy.
    let claude_target = chosen.contains(&Adapter::ClaudeMd) && !(agents_target && imports);
    for (path, target) in [
        (adapters::AGENTS, agents_target),
        (adapters::CLAUDE, claude_target),
    ] {
        let old = read(path);
        let current = if path == adapters::CLAUDE {
            claude.clone()
        } else {
            old.clone()
        };
        let new = if target {
            adapters::upsert_line(&current)
        } else {
            adapters::remove_line(&current)
        };
        if new != old && !(new.is_empty() && !exists(path)) {
            out.push(Change {
                path: path.to_string(),
                created: !exists(path),
                remove: false,
                text: new,
                note: if target {
                    "one generated line".into()
                } else {
                    "remove the generated line".into()
                },
            });
        }
    }
    let skill = read(adapters::SKILL);
    if chosen.contains(&Adapter::Skill) {
        if !skill.is_empty() && !skill.contains("<!-- spec-dna:begin skill ") {
            return Err(Fatal::Usage(format!(
                "{} exists and was not generated by spec-dna; move it first",
                adapters::SKILL
            )));
        }
        if skill != adapters::skill_file(types) {
            out.push(Change {
                path: adapters::SKILL.into(),
                created: skill.is_empty(),
                remove: false,
                text: adapters::skill_file(types),
                note: "the skill".into(),
            });
        }
    }
    Ok(out)
}

/// Joins lines that keep their YAML indentation (a source file of this
/// repository indents by 4 spaces, so the YAML lines are strings here).
fn lines(items: &[&str]) -> String {
    items.iter().map(|l| format!("{l}\n")).collect()
}

/// The step that checks out the repository, with the full commit SHA that
/// Dependabot keeps current in this repository. `spec-dna.yml` and
/// `agent-evals.yml` use it, so `init` needs no network. The version tag is in
/// the trailing comment (`AGENTS.md`, "Conventions").
const CHECKOUT_USES: &str = "actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1";

/// The step "Install the pinned spec-dna" of the CI jobs, with its indent for
/// a step of a job. With the sum of the release asset in `spec-dna.toml` and
/// no `install-source`, the step downloads the binary and verifies its sum.
/// Otherwise it builds the pin from source.
fn install_step() -> String {
    let download = format!(
        "            curl --fail --silent --show-error --location --output \"$RUNNER_TEMP/$asset\" \"{}/releases/download/v$pin/$asset\"",
        config::DEFAULT_INSTALL_SOURCE
    );
    let build = format!(
        "            cargo install --locked --git \"${{src:-{}}}\" --tag \"v$pin\" spec-dna",
        config::DEFAULT_INSTALL_SOURCE
    );
    lines(&[
        "      - name: Install the pinned spec-dna",
        "        run: |",
        "          pin=$(sed -n 's/^spec-dna = \"\\(.*\\)\"/\\1/p' spec-dna.toml)",
        "          src=$(sed -n 's/^install-source = \"\\(.*\\)\"/\\1/p' spec-dna.toml)",
        "          asset=\"spec-dna-$pin-x86_64-unknown-linux-gnu\"",
        "          sum=$(sed -n \"s/^\\\"$asset\\\" = \\\"\\(.*\\)\\\"/\\1/p\" spec-dna.toml)",
        "          if [ -z \"$src\" ] && [ -n \"$sum\" ]; then",
        &download,
        "            echo \"$sum  $RUNNER_TEMP/$asset\" | sha256sum --check -",
        "            install -D -m 755 \"$RUNNER_TEMP/$asset\" \"$HOME/.local/bin/spec-dna\"",
        "            echo \"$HOME/.local/bin\" >> \"$GITHUB_PATH\"",
        "          else",
        &build,
        "          fi",
    ])
}

/// The GitHub Actions job, as in docs/quickstart.md.
fn github_workflow(base: &str) -> String {
    let check =
        format!("      - run: spec-dna check --base \"origin/${{GITHUB_BASE_REF:-{base}}}\"");
    let head = lines(&[
        "name: Specs",
        "on: pull_request",
        "permissions: {}",
        "jobs:",
        "  spec-dna:",
        "    runs-on: ubuntu-24.04",
        "    permissions:",
        "      contents: read",
        "    steps:",
        &format!("      - uses: {CHECKOUT_USES}"),
        "        with:",
        "          fetch-depth: 0",
        "          persist-credentials: false",
    ]);
    format!("{head}{}{}", install_step(), lines(&[&check]))
}

/// The workflow of the evals: `.github/workflows/agent-evals.yml`.
fn evals_workflow(dir: &str, base: &str) -> String {
    let checkout = lines(&[
        &format!("      - uses: {CHECKOUT_USES}"),
        "        with:",
        "          persist-credentials: false",
    ]);
    skeleton::workflow(dir, base, &checkout, &install_step())
}

/// The local hook of pre-commit.
fn pre_commit_hook() -> String {
    lines(&[
        "- repo: local",
        "  hooks:",
        "    - id: spec-dna",
        "      name: spec-dna check",
        "      entry: spec-dna check --staged",
        "      language: system",
        "      pass_filenames: false",
        "      always_run: true",
    ])
}

/// The hook files that `init` writes or upgrades. A change of `spec-dna.toml`
/// with one of them is still a change of its own (SD503). `init` writes no
/// file for lefthook.
pub const HOOK_FILES: [&str; 2] = [".pre-commit-config.yaml", ".husky/pre-commit"];

/// The hook that 0.4.1 wrote runs `spec-dna check`, which reads the working
/// tree. A commit can pass while the committed tree fails. The hook of this
/// version runs `spec-dna check --staged`, which reads the index. This
/// function changes the old line of a hook file in place. It changes only the
/// exact line that 0.4.1 wrote. Any other text stays the same.
pub(crate) fn hook_upgrade(src: &ReadText, path: &str) -> Option<Change> {
    let old = src(path)?;
    let (is_old, new_line): (fn(&str) -> bool, &str) = if path == ".husky/pre-commit" {
        (|l| l == "spec-dna check", "spec-dna check --staged")
    } else {
        (
            |l| l == "entry: spec-dna check",
            "entry: spec-dna check --staged",
        )
    };
    let mut changed = false;
    let text: String = old
        .split_inclusive('\n')
        .map(|l| {
            let body = l.trim_end_matches(['\n', '\r']);
            let t = body.trim_start();
            if is_old(t.trim_end()) {
                changed = true;
                let pad = &body[..body.len() - t.len()];
                format!("{pad}{new_line}{}", &l[body.len()..])
            } else {
                l.to_string()
            }
        })
        .collect();
    changed.then(|| Change {
        path: path.to_string(),
        created: false,
        remove: false,
        text,
        note: "the hook of 0.4.1 reads the working tree; it now runs `spec-dna check --staged`"
            .into(),
    })
}

/// Appends the local hook to `.pre-commit-config.yaml`, if `repos:` is its
/// last top-level key. Otherwise returns `None`, and the preview gives the
/// text to add manually.
fn pre_commit_append(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.lines().collect();
    let repos = lines.iter().rposition(|l| l.trim_end() == "repos:")?;
    let after_top = lines[repos + 1..]
        .iter()
        .any(|l| !l.is_empty() && !l.starts_with([' ', '\t', '-', '#']));
    if after_top {
        return None;
    }
    let indent = lines[repos + 1..]
        .iter()
        .find_map(|l| {
            let t = l.trim_start();
            t.starts_with("- repo:").then(|| l.len() - t.len())
        })
        .unwrap_or(2);
    let pad = " ".repeat(indent);
    let block: String = pre_commit_hook()
        .lines()
        .map(|l| format!("{pad}{l}\n"))
        .collect();
    let mut t = text.to_string();
    if !t.ends_with('\n') {
        t.push('\n');
    }
    Some(format!("{t}{block}"))
}

/// The CI and hook files, and the text to add manually.
fn ci_changes(repo: &Repo, plan: &Plan) -> (Vec<Change>, Vec<String>) {
    let mut out = Vec::new();
    let mut manual = Vec::new();
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).ok();
    match plan.ci.as_str() {
        "github" => {
            let path = ".github/workflows/spec-dna.yml";
            let text = github_workflow(&plan.base);
            match read(path) {
                None => out.push(Change {
                    path: path.into(),
                    created: true,
                    remove: false,
                    text,
                    note: "the CI job: `spec-dna check --base`".into(),
                }),
                // A workflow of the project stays as it is.
                Some(old) if old != text => manual.push(format!(
                    "{path} exists and differs from the job of this version; init does not change it"
                )),
                Some(_) => {}
            }
        }
        "gitlab" => manual.push(format!(
            "Add this job to .gitlab-ci.yml:\nspec-dna:\n  stage: test\n  script:\n    - cargo install --locked --git {} --tag v{VERSION}\n    - git fetch origin \"${{CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-{}}}\"\n    - spec-dna check --base \"origin/${{CI_MERGE_REQUEST_TARGET_BRANCH_NAME:-{}}}\"\n  rules:\n    - if: $CI_PIPELINE_SOURCE == \"merge_request_event\"",
            config::DEFAULT_INSTALL_SOURCE,
            plan.base,
            plan.base
        )),
        _ => {}
    }
    match plan.hook.as_str() {
        "pre-commit" => {
            let path = ".pre-commit-config.yaml";
            let old = read(path).unwrap_or_default();
            if old.contains("entry: spec-dna check") {
                out.extend(hook_upgrade(&disk(repo), path));
            } else if old.is_empty() {
                out.push(Change {
                    path: path.into(),
                    created: true,
                    remove: false,
                    text: format!("repos:\n{}", indent2(&pre_commit_hook())),
                    note: "a local hook: `spec-dna check --staged`".into(),
                });
            } else if let Some(text) = pre_commit_append(&old) {
                out.push(Change {
                    path: path.into(),
                    created: false,
                    remove: false,
                    text,
                    note: "a local hook at the end: `spec-dna check --staged`".into(),
                });
            } else {
                manual.push(format!(
                    "Add this hook under `repos:` in .pre-commit-config.yaml:\n{}",
                    indent2(&pre_commit_hook()).trim_end()
                ));
            }
        }
        "husky" => {
            let path = ".husky/pre-commit";
            let old = read(path).unwrap_or_default();
            if old.lines().any(|l| l.trim() == "spec-dna check") {
                out.extend(hook_upgrade(&disk(repo), path));
            } else if !old.lines().any(|l| l.trim() == "spec-dna check --staged") {
                let mut t = old.clone();
                if !t.is_empty() && !t.ends_with('\n') {
                    t.push('\n');
                }
                t.push_str("spec-dna check --staged\n");
                out.push(Change {
                    path: path.into(),
                    created: old.is_empty(),
                    remove: false,
                    text: t,
                    note: "one line: `spec-dna check --staged`".into(),
                });
            }
        }
        "lefthook" => manual.push(
            "Add this command to lefthook.yml:\npre-commit:\n  commands:\n    spec-dna:\n      run: spec-dna check --staged".into(),
        ),
        _ => {}
    }
    (out, manual)
}

/// The path of the workflow of the evals.
const EVALS_WORKFLOW: &str = ".github/workflows/agent-evals.yml";

/// The 2 commands that enable the CI runs of the evals.
const EVALS_COMMANDS: [&str; 2] = [
    "gh secret set ANTHROPIC_API_KEY",
    "gh variable set SPEC_DNA_EVALS --body true",
];

/// The starter files of the answer `skeleton`, and the notes. `init` writes
/// a file once: a path that exists stays, and a note names it. The workflow
/// is only for the CI answer `github` (D43). With a current table, `init`
/// writes no file. `tests` holds the globs of `[evidence] tests` that the new
/// configuration has.
fn skeleton_changes(repo: &Repo, plan: &Plan, tests: &[String]) -> (Vec<Change>, Vec<String>) {
    let dir = plan.evals_table.dir.as_str();
    let mut notes = Vec::new();
    if plan.evals == EvalsAnswer::None {
        if plan.evals_current {
            notes.push(format!(
                "`init` removes the table `[evals]`. The files in `{dir}/` stay. Delete them by hand if you do not need them"
            ));
        }
        return (Vec::new(), notes);
    }
    if plan.evals_current {
        return (Vec::new(), notes);
    }
    if dir != config::DEFAULT_EVALS_DIR {
        notes.push(format!(
            "`{}/` holds files of the project, so `[evals] dir` is `{dir}`",
            config::DEFAULT_EVALS_DIR
        ));
    }
    let pathspecs: Vec<String> = crate::git::glob_pathspecs(tests)
        .iter()
        .map(|p| harness::shell_quote(p))
        .collect();
    let paths = evals::skeleton_paths(dir);
    let mut files = vec![
        (
            paths[0].clone(),
            skeleton::check_sh(&pathspecs),
            "the checks of the evals",
        ),
        (
            paths[1].clone(),
            skeleton::example_json(dir),
            "an example eval",
        ),
        (
            paths[2].clone(),
            skeleton::NOTES.to_string(),
            "the fixture of the example eval",
        ),
    ];
    if plan.ci == "github" {
        files.push((
            paths[3].clone(),
            evals_workflow(dir, &plan.base),
            "the CI job of the evals: it runs only if the variable SPEC_DNA_EVALS is true",
        ));
    }
    let mut out = Vec::new();
    for (path, text, note) in files {
        if repo.root.join(&path).exists() {
            notes.push(format!("{path} exists, so `init` did not write it"));
        } else {
            out.push(Change {
                path,
                created: true,
                remove: false,
                text,
                note: note.into(),
            });
        }
    }
    (out, notes)
}

fn indent2(text: &str) -> String {
    text.lines().map(|l| format!("  {l}\n")).collect()
}

/// Reads one answer from the terminal. An empty line or the end of the input
/// keeps the default.
struct Ask<'a> {
    input: &'a mut dyn BufRead,
    out: &'a mut dyn Write,
}

impl Ask<'_> {
    fn line(&mut self, prompt: &str, default: &str) -> String {
        let _ = write!(self.out, "{prompt} [{default}]: ");
        let _ = self.out.flush();
        let mut s = String::new();
        match self.input.read_line(&mut s) {
            Ok(n) if n > 0 && !s.trim().is_empty() => s.trim().to_string(),
            _ => default.to_string(),
        }
    }

    fn say(&mut self, text: &str) {
        let _ = writeln!(self.out, "{text}");
    }
}

fn list(items: &[String]) -> String {
    items.join(", ")
}

fn words(s: &str) -> Vec<String> {
    s.split([',', ' '])
        .map(str::trim)
        .filter(|w| !w.is_empty())
        .map(str::to_string)
        .collect()
}

/// Reads the answer to question 2. `custom` needs the layers of a current
/// `spec-dna.toml`.
fn parse_graph(w: &str, plan: &Plan) -> Result<GraphChoice, String> {
    let g = GraphChoice::from_name(w)
        .ok_or_else(|| format!("graph `{w}`: use small, default or custom"))?;
    if g == GraphChoice::Custom && plan.custom.is_none() {
        return Err(
            "graph `custom` keeps the layers of the current spec-dna.toml, and there is none: use small or default (or `init --reconfigure` on an existing file)".into(),
        );
    }
    Ok(g)
}

/// Reads the answer to question 3: the names of the folders of
/// `requirements/`, or `none`.
fn parse_areas(v: &[String]) -> Result<Vec<String>, String> {
    if v.len() == 1 && v[0] == "none" {
        return Ok(Vec::new());
    }
    let mut out: Vec<String> = Vec::new();
    for a in v {
        if a.is_empty()
            || a == "none"
            || a == "."
            || a == ".."
            || a.contains(['/', '\\'])
            || a.trim() != a
        {
            return Err(format!(
                "area `{a}`: write the name of one folder of requirements/, or `none`"
            ));
        }
        if out.contains(a) {
            return Err(format!("area `{a}` is listed twice"));
        }
        out.push(a.clone());
    }
    Ok(out)
}

fn areas_value(areas: &[String]) -> String {
    if areas.is_empty() {
        "none".into()
    } else {
        list(areas)
    }
}

/// Reads the answer to question 4.
fn parse_policy(w: &str) -> Result<ChangePolicy, String> {
    Ok(match w {
        "three-class" => ChangePolicy::ThreeClass,
        "two-class" => ChangePolicy::TwoClass,
        "none" => ChangePolicy::None,
        _ => {
            return Err(format!(
                "change policy `{w}`: use three-class, two-class or none"
            ));
        }
    })
}

/// Reads the answer to question 5.
fn parse_gate(w: &str) -> Result<Gate, String> {
    Ok(match w {
        "block" => Gate::Block,
        "warn" => Gate::Warn,
        "off" => Gate::Off,
        _ => return Err(format!("gate `{w}`: use block, warn or off")),
    })
}

fn parse_evidence(w: &str) -> Result<Option<Option<Severity>>, String> {
    Ok(match w {
        "profile" => None,
        "off" => Some(None),
        "warning" => Some(Some(Severity::Warning)),
        "error" => Some(Some(Severity::Error)),
        _ => {
            return Err(format!(
                "evidence `{w}`: use profile, off, warning or error"
            ));
        }
    })
}

fn parse_style(w: &str) -> Result<Style, String> {
    Ok(match w {
        "covers" => Style::Covers,
        "verified-by" => Style::VerifiedBy,
        "both" => Style::Both,
        _ => return Err(format!("link style `{w}`: use covers, verified-by or both")),
    })
}

fn parse_budgets(w: &str) -> Result<Budgets, String> {
    Ok(match w {
        "default" => Budgets::Default,
        "relaxed" => Budgets::Relaxed,
        "tight" => Budgets::Tight,
        "off" => Budgets::Off,
        _ => {
            return Err(format!(
                "budget level `{w}`: use default, relaxed, tight or off"
            ));
        }
    })
}

fn parse_adapters(v: &[String]) -> Result<Vec<Adapter>, String> {
    let mut out = Vec::new();
    for w in v {
        if w == "none" {
            continue;
        }
        let a = ADAPTERS
            .iter()
            .find(|x| x.1 == w)
            .ok_or_else(|| format!("adapter `{w}`: use agents-md, claude-md, skill or none"))?;
        out.push(a.0);
    }
    Ok(out)
}

fn adapter_names(v: &[Adapter]) -> Vec<String> {
    if v.is_empty() {
        return vec!["none".into()];
    }
    v.iter()
        .map(|a| ADAPTERS.iter().find(|x| x.0 == *a).unwrap().1.to_string())
        .collect()
}

fn check_choice(what: &str, w: &str, allowed: &[&str]) -> Result<String, String> {
    if allowed.contains(&w) {
        Ok(w.to_string())
    } else {
        Err(format!("{what} `{w}`: use {}", allowed.join(", ")))
    }
}

fn parse_level(w: &str) -> Result<Level, String> {
    Level::from_name(w).ok_or_else(|| format!("harness level `{w}`: use none, lean or full"))
}

/// The line of question 13: one `role=model[:effort]` word for each role of
/// the level. The advisor word has the model of `fallback-advisor` after a
/// slash, if it is not the default.
fn models_line(h: &Harness) -> String {
    let lean = Role::ALL.into_iter().filter(|r| !r.full_only());
    let full = Role::ALL.into_iter().filter(|r| r.full_only());
    let mut out = Vec::new();
    for r in lean.chain(full).filter(|r| h.level.includes(*r)) {
        let c = h.cfg(r);
        let mut w = format!("{}={}", r.key(), c.model.name());
        if r == Role::Advisor && c.agent != Model::Id("opus".into()) {
            w += &format!("/{}", c.agent.name());
        }
        let shown = match r {
            Role::Helper => false,
            Role::Advisor => c.effort != Effort::XHigh,
            _ => true,
        };
        if shown && c.model != Model::None {
            w += &format!(":{}", c.effort.name());
        }
        out.push(w);
    }
    out.join(" ")
}

/// Applies `role=model[:effort]` words to the harness. The advisor word can
/// be `advisor=<model>/<agent model>[:effort]`. The effort of the advisor
/// belongs to its agent.
fn apply_models(h: &mut Harness, words: &[String]) -> Result<(), String> {
    for w in words {
        let (key, value) = w
            .split_once('=')
            .ok_or_else(|| format!("`{w}`: write role=model[:effort]"))?;
        if let Some(gone) = GoneRole::from_key(key) {
            return Err(gone.word_message());
        }
        let role = Role::from_key(key).ok_or_else(|| {
            let keys: Vec<&str> = Role::ALL.iter().map(|r| r.key()).collect();
            format!("unknown role `{key}`; use {}", keys.join(", "))
        })?;
        if h.level != Level::None && !h.level.includes(role) {
            return Err(format!("role `{key}` is in level full"));
        }
        // A suffix with a digit belongs to the model id, for example `-v1:0`.
        // A suffix with no digit, or an empty one, must be an effort.
        let (spec, effort) = match value.rsplit_once(':') {
            Some((m, e)) => match Effort::from_name(e) {
                Some(x) => (m, Some(x)),
                None if !e.bytes().any(|b| b.is_ascii_digit()) => {
                    return Err(format!(
                        "`{w}`: unknown effort `{e}`; use low, medium, high, xhigh, max or inherit"
                    ));
                }
                None => (value, None),
            },
            None => (value, None),
        };
        let (model, agent) = match spec.split_once('/') {
            Some((m, a)) if role == Role::Advisor => (m, Some(a)),
            _ => (spec, None),
        };
        if model.is_empty() || agent.is_some_and(str::is_empty) {
            return Err(format!("`{w}`: the model is empty"));
        }
        if role == Role::Helper && effort.is_some() {
            return Err(format!("`{w}`: the role `helper` has no effort"));
        }
        let c = h.cfg_mut(role);
        c.model = Model::parse(model);
        if let Some(a) = agent {
            c.agent = Model::parse(a);
        }
        if let Some(e) = effort {
            c.effort = e;
        }
    }
    Ok(())
}

/// Reads the `[harness]` table that the harness renders, with the rules of
/// `spec-dna.toml`. It reports a model or an effort that the file does not
/// accept as a usage error.
fn validate_harness(h: &Harness) -> Result<(), String> {
    let t: toml::Table = h
        .toml_section()
        .parse()
        .map_err(|e: toml::de::Error| e.to_string())?;
    match t.get("harness") {
        Some(v) => Harness::parse(v).map(|_| ()),
        None => Ok(()),
    }
}

/// The harness files that exist: (path, text) of `CLAUDE.md`,
/// `.claude/settings.json`, `REVIEW.md`, `.claude/agents/*.md` and
/// `.claude/skills/*/SKILL.md`. The adapter skill is not one of them.
fn harness_existing(repo: &Repo) -> Vec<(String, String)> {
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).ok();
    let mut out = Vec::new();
    for p in [harness::CLAUDE, harness::SETTINGS, harness::REVIEW] {
        if let Some(t) = read(p) {
            out.push((p.to_string(), t));
        }
    }
    let names = |dir: &str| -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(repo.root.join(dir))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();
        names.sort();
        names
    };
    for n in names(harness::AGENTS_DIR) {
        let p = format!("{}{n}", harness::AGENTS_DIR);
        if harness::wanted(&p)
            && let Some(t) = read(&p)
        {
            out.push((p, t));
        }
    }
    for n in names(harness::SKILLS_DIR) {
        let p = format!("{}{n}/SKILL.md", harness::SKILLS_DIR);
        if harness::wanted(&p)
            && let Some(t) = read(&p)
        {
            out.push((p, t));
        }
    }
    out
}

/// Turns off each role whose agent name belongs to a file that spec-dna did
/// not generate, and tells why. With `outside_level`, only a role that the
/// level does not include changes: the configuration set the other roles, and
/// a conflict with them is an error of `plan`.
fn apply_conflicts(
    h: &mut Harness,
    existing: &[(String, String)],
    outside_level: bool,
) -> Vec<String> {
    let mut notes = Vec::new();
    for (role, path) in harness::name_conflicts(existing) {
        if outside_level && h.level.includes(role) {
            continue;
        }
        let Some(def) = role.agent() else { continue };
        if role == Role::Advisor {
            // The advisor tool needs no file. Only the fallback agent goes.
            h.cfg_mut(role).agent = Model::None;
            notes.push(format!(
                "{path} holds the agent `{}`, and spec-dna did not generate it: the advisor gets no `fallback-advisor` file",
                def.name
            ));
        } else {
            h.cfg_mut(role).model = Model::None;
            notes.push(format!(
                "{path} holds the agent `{}`, and spec-dna did not generate it: the role `{}` is none",
                def.name,
                role.key()
            ));
        }
    }
    notes
}

/// The defaults of the harness for a new configuration: level `lean`, the
/// models of the table, the deny rule for notebooks, the main model and the
/// advisor of an existing `.claude/settings.json`, and no role over a file of
/// the project. Returns the harness and the reasons of the roles that are off.
fn harness_defaults(repo: &Repo, existing: &[(String, String)]) -> (Harness, Vec<String>) {
    let mut h = Harness::new(Level::Lean);
    let notebooks = repo
        .git
        .ls_files()
        .is_ok_and(|f| f.iter().any(|e| e.0.ends_with(".ipynb")));
    if !notebooks {
        h.deny.push("NotebookEdit".into());
    }
    let settings = existing
        .iter()
        .find(|e| e.0 == harness::SETTINGS)
        .and_then(|e| json::parse(&e.1).ok());
    if let Some(v) = settings {
        let pick = |key: &str| {
            v.get(key)
                .as_str()
                .filter(|m| !m.is_empty() && !m.contains(char::is_whitespace))
                .filter(|m| !matches!(*m, "none" | "inherit"))
                .map(str::to_string)
        };
        if let Some(m) = pick("model") {
            // The effort of the orchestrator applies to its model. An entry
            // of `[harness.efforts]` for the same model would have no effect.
            let base = m.split('[').next().unwrap_or(&m).to_string();
            h.efforts
                .retain(|(k, _)| k.split('[').next().unwrap_or(k) != base);
            h.cfg_mut(Role::Orchestrator).model = Model::Id(m);
        }
        if let Some(m) = pick("advisorModel").filter(|m| !harness::is_haiku(m)) {
            h.cfg_mut(Role::Advisor).model = Model::Id(m);
        }
    }
    let notes = apply_conflicts(&mut h, existing, false);
    (h, notes)
}

/// Refuses the harness if git ignores a file that it writes: each generated
/// file, `CLAUDE.md` and the settings file. A team could not commit it.
pub(crate) fn refuse_ignored(repo: &Repo, h: &Harness) -> Result<(), Fatal> {
    let mut paths: Vec<String> = harness::generated_files(h)
        .into_iter()
        .map(|g| g.path)
        .collect();
    paths.push(harness::CLAUDE.to_string());
    paths.push(harness::SETTINGS.to_string());
    for p in paths {
        if repo.git.is_ignored(&p) {
            return Err(Fatal::Usage(format!(
                "git ignores {p}, so the harness files cannot be committed; remove the ignore rule, or choose the harness level none"
            )));
        }
    }
    Ok(())
}

/// The harness plan of `init`, merged into the list of changes.
pub(crate) struct HarnessRun {
    level: Level,
    /// The first version of spec-dna that has the hooks of the configuration.
    hooks_version: &'static str,
    pub(crate) plan: harness::Plan,
    notes: Vec<String>,
}

/// Plans the harness files and merges them into `changes`. The text of
/// `CLAUDE.md` after the adapter line is the input of the plan, and the two
/// changes of `CLAUDE.md` become one. Returns `None` if the harness is off and
/// nothing of it is installed.
fn plan_harness(
    repo: &Repo,
    new: &Harness,
    old: Option<&Harness>,
    notes: &[String],
    mode: harness::Mode,
    changes: &mut Vec<Change>,
) -> Result<Option<HarnessRun>, Fatal> {
    let existing = harness_existing(repo);
    let installed = harness::is_installed(&existing);
    if new.level == Level::None && !(mode == harness::Mode::Reconfigure && installed) {
        return Ok(None);
    }
    if new.level != Level::None {
        refuse_ignored(repo, new)?;
    }
    harness_run(existing, new, old, notes, mode, changes).map(Some)
}

/// The part of `plan_harness` that reads no disk and no git: the files in
/// `existing` and the changes of the adapter step give the plan of the harness.
/// `upgrade` calls it with the files of a tree.
pub(crate) fn harness_run(
    mut existing: Vec<(String, String)>,
    new: &Harness,
    old: Option<&Harness>,
    notes: &[String],
    mode: harness::Mode,
    changes: &mut Vec<Change>,
) -> Result<HarnessRun, Fatal> {
    if let Some(c) = changes.iter().find(|c| c.path == harness::CLAUDE) {
        existing.retain(|e| e.0 != harness::CLAUDE);
        existing.push((harness::CLAUDE.to_string(), c.text.clone()));
    }
    let mut plan = harness::plan(&existing, old, new, mode).map_err(Fatal::Usage)?;
    // The adapter step can create `CLAUDE.md`, and then the harness step sees a
    // file. The preview must say that the file is new.
    let claude_is_new = changes
        .iter()
        .any(|c| c.path == harness::CLAUDE && c.created);
    for c in plan
        .changes
        .iter_mut()
        .filter(|c| c.path == harness::CLAUDE)
    {
        if claude_is_new && c.action == harness::Action::Change {
            c.action = harness::Action::Create;
        }
    }
    for c in &plan.changes {
        let remove = c.action == harness::Action::Remove;
        match changes.iter_mut().find(|x| x.path == c.path) {
            Some(x) => {
                x.text = c.text.clone();
                x.remove = remove;
                x.note = format!("{}; {}", x.note, c.note);
            }
            None => changes.push(Change {
                path: c.path.clone(),
                created: c.action == harness::Action::Create,
                remove,
                text: c.text.clone(),
                note: c.note.clone(),
            }),
        }
    }
    let mut notes = notes.to_vec();
    // The file replaces a built-in subagent, so the person must know it when
    // `init` writes the file. An unchanged file gives no note.
    if let Some(def) = Role::Researcher.agent()
        && plan.changes.iter().any(|c| {
            c.path == def.path()
                && matches!(c.action, harness::Action::Create | harness::Action::Change)
        })
    {
        notes.push(EXPLORE_NOTE.to_string());
    }
    if new.level != Level::None
        && let Some((_, t)) = existing.iter().find(|e| e.0 == harness::SETTINGS)
        && has_workflow_plugin(t)
    {
        notes.push(WORKFLOW_PLUGIN_NOTE.to_string());
    }
    Ok(HarnessRun {
        level: new.level,
        hooks_version: harness::hooks_version(new),
        plan,
        notes,
    })
}

/// Computes the parts that follow a default, from the final answers: the
/// skills follow the policy that `settle_change` leaves, `review-md` follows
/// the file `REVIEW.md` of the project, and `protect-tests` follows the test
/// globs. A part that the file or the person set stays. The call is
/// idempotent.
fn settle_parts(plan: &mut Plan) {
    if plan.harness.level == Level::None {
        return;
    }
    if plan.parts.derive_skills {
        plan.harness.skills = if plan.change_policy == ChangePolicy::None {
            Vec::new()
        } else {
            // The 7 skills that are on by default (D2). The level `full` also
            // proposes the 4 that are off (D39).
            proposed_skills(plan.harness.level)
                .into_iter()
                .filter(|s| !plan.parts.unmarked_skills.contains(s))
                .collect()
        };
    }
    if plan.parts.derive_review {
        plan.harness.review_md = !plan.parts.unmarked_review;
    }
    if plan.parts.derive_protect {
        // `[evidence] tests` is empty only for the link style `verified-by`.
        // An empty list of detected globs renders no table, and the default
        // globs apply.
        plan.harness.protect_tests = if plan.style == Style::VerifiedBy {
            Protect::Off
        } else {
            Protect::FixBranches
        };
    }
}

/// The level `none` asks no question 14. A new configuration then has no
/// evals. With `--reconfigure`, the current state stays: a file that has the
/// table `[evals]` keeps it.
fn settle_evals(plan: &mut Plan) {
    if plan.harness.level == Level::None && !plan.evals_current {
        plan.evals = EvalsAnswer::None;
    }
}

/// The parts of the harness whose files exist with no marker. The default
/// leaves them out, and a note names each file. A skill or `REVIEW.md` that
/// spec-dna generated has a marker, so it is no conflict.
fn unmarked_parts(plan: &mut Plan, existing: &[(String, String)]) {
    plan.parts.unmarked_skills.clear();
    plan.parts.unmarked_review = false;
    for (path, text) in existing {
        if harness::has_marker(text) {
            continue;
        }
        if path == harness::REVIEW && plan.parts.derive_review {
            plan.parts.unmarked_review = true;
            plan.harness_notes.push(format!(
                "{path} exists, and spec-dna did not generate it: `review-md` is false. Move the file to get the generated one"
            ));
        } else if let Some(s) = Skill::ALL
            .into_iter()
            .find(|s| s.path() == *path && plan.parts.derive_skills)
        {
            plan.parts.unmarked_skills.push(s);
            plan.harness_notes.push(format!(
                "{path} exists, and spec-dna did not generate it: the skill `{}` is not in `skills`. Move the file to get the generated one",
                s.key()
            ));
        }
    }
}

/// The line of question 12 for the parts, as it is the default: `skills=none`
/// or the names with commas, then `review-md` and `protect-tests`.
fn parts_line(h: &Harness) -> String {
    let skills = if h.skills.is_empty() {
        "none".to_string()
    } else {
        h.skills
            .iter()
            .map(|s| s.key())
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "skills={skills} review-md={} protect-tests={}",
        h.review_md,
        h.protect_tests.name()
    )
}

/// Reads the names of `skills`: each name once, in the order of `Skill`.
fn parse_skills(names: &[String]) -> Result<Vec<Skill>, String> {
    let mut out: Vec<Skill> = Vec::new();
    for n in names {
        let s = Skill::from_key(n).ok_or_else(|| {
            harness::gone_skill_answer_message(n)
                .unwrap_or_else(|| format!("unknown skill `{n}`; use {}", harness::skill_keys()))
        })?;
        if out.contains(&s) {
            return Err(format!("`skills` lists `{n}` more than once"));
        }
        out.push(s);
    }
    out.sort();
    Ok(out)
}

fn parse_bool_word(what: &str, w: &str) -> Result<bool, String> {
    match w {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("`{what}` must be true or false, not `{w}`")),
    }
}

fn parse_protect(w: &str) -> Result<Protect, String> {
    Protect::from_name(w)
        .ok_or_else(|| format!("`protect-tests` must be fix-branches, always or off, not `{w}`"))
}

/// Applies the `key=value` words of the parts line of question 12. A word
/// that is not in the line keeps the default.
fn apply_parts(plan: &mut Plan, words: &[String]) -> Result<(), String> {
    for w in words {
        let (key, value) = w
            .split_once('=')
            .ok_or_else(|| format!("`{w}`: write key=value"))?;
        match key {
            "skills" => {
                let names: Vec<String> = value
                    .split(',')
                    .filter(|n| !n.is_empty() && *n != "none")
                    .map(str::to_string)
                    .collect();
                plan.harness.skills = parse_skills(&names)?;
                plan.parts.derive_skills = false;
            }
            "review-md" => {
                plan.harness.review_md = parse_bool_word("review-md", value)?;
                plan.parts.derive_review = false;
            }
            "protect-tests" => {
                plan.harness.protect_tests = parse_protect(value)?;
                plan.parts.derive_protect = false;
            }
            other => {
                return Err(format!(
                    "unknown part `{other}`; use skills, review-md, protect-tests"
                ));
            }
        }
    }
    Ok(())
}

/// The bytes of the skill listing that `skills` adds to each request. It
/// counts the skills that are on by default.
fn skill_listing_bytes(skills: &[Skill]) -> usize {
    skills
        .iter()
        .map(|s| format!("- {}: {}\n", s.name(), harness_text::description(*s)).len())
        .sum()
}

/// The skills that a level proposes (D39): the 7 that are on by default, and
/// at the level `full` also the 4 that are off.
fn proposed_skills(level: Level) -> Vec<Skill> {
    if level == Level::Full {
        Skill::ALL.to_vec()
    } else {
        Skill::default_on_list()
    }
}

/// The note for a project that also has the plugin Superpowers (D5). A spec-dna
/// skill and a skill of the plugin for the same stage compete.
const WORKFLOW_PLUGIN_NOTE: &str = "`.claude/settings.json` enables a plugin with the word `superpowers`. The description of each spec-dna skill starts with \"In a spec-dna project\", and the adapter skill `spec-dna` states that a spec-dna skill replaces a general skill of the same stage (for `write-plan`, `writing-plans`). Nothing proves that Claude Code selects the spec-dna skill first. `init` does not move a file that the plugin writes, such as `docs/superpowers/specs/`: exclude that folder from the roots.";

/// Returns true if `enabledPlugins` of a `.claude/settings.json` names a
/// plugin with the word `superpowers`, in any case, and does not set it to
/// `false`. `init` reads this file of the project only, never a file of the
/// home folder (D5).
fn has_workflow_plugin(settings: &str) -> bool {
    let Ok(doc) = json::parse(settings) else {
        return false;
    };
    match doc.get("enabledPlugins") {
        json::Value::Obj(items) => items.iter().any(|(k, v)| {
            k.to_ascii_lowercase().contains("superpowers") && *v != json::Value::Bool(false)
        }),
        _ => false,
    }
}

/// The note for `.claude/agents/Explore.md`. Only an agent with the name
/// `Explore` replaces the built-in subagent (ADR 0008).
const EXPLORE_NOTE: &str = "`.claude/agents/Explore.md` replaces the built-in `Explore` subagent of Claude Code, with its model, tools and text. The built-in subagent uses the model of the main session. To remove the file, set `researcher = { claude-code = \"none\" }` in `[harness.roles]` and run `spec-dna init --reconfigure`.";

/// Prints the harness block of the preview, or its summary after a write.
fn print_harness(h: &HarnessRun, with_files: bool) {
    println!("Agent harness: {} (for Claude Code)", h.level.name());
    for n in &h.notes {
        println!("  note: {n}");
    }
    if with_files {
        for c in &h.plan.changes {
            println!("  {} {} ({})", c.action.name(), c.path, c.note);
        }
        if h.plan.changes.is_empty() {
            println!("  no file changes");
        }
    }
    for k in &h.plan.kept {
        println!("  {k}");
    }
    for w in &h.plan.warnings {
        println!("  warning: {w}");
    }
    if h.level != Level::None {
        let what = if h.plan.skill_bytes > 0 {
            "the CLAUDE.md line, the agent descriptions and the skill descriptions"
        } else {
            "the CLAUDE.md line and the agent descriptions"
        };
        println!(
            "  The harness adds about {} bytes to each request of the main session ({what}).",
            h.plan.request_bytes
        );
        println!(
            "  Each teammate needs spec-dna {} or later on PATH for the hooks.",
            h.hooks_version
        );
    }
}

/// The `harness` object of `--json`.
fn harness_json(h: Option<&HarnessRun>, level: Level) -> String {
    let Some(h) = h else {
        return Obj::new()
            .str("level", level.name())
            .raw("files", "[]")
            .strs("kept", &[])
            .strs("warnings", &[])
            .strs("notes", &[])
            .num("request_bytes", 0)
            .end();
    };
    let files = h.plan.changes.iter().map(|c| {
        Obj::new()
            .str("path", &c.path)
            .str("action", c.action.name())
            .str("note", &c.note)
            .end()
    });
    Obj::new()
        .str("level", h.level.name())
        .raw("files", &json::array(files))
        .strs("kept", &h.plan.kept)
        .strs("warnings", &h.plan.warnings)
        .strs("notes", &h.notes)
        .num("request_bytes", h.plan.request_bytes)
        .end()
}

const RESTART: &str = "Restart Claude Code. Then check: the header shows the main model, /advisor shows the advisor, and @ lists the agents.";

/// Makes the roots fit the layer graph. A root that holds one type is not
/// written if no layer has that type. A mixed root keeps its place, but loses
/// its `default-type` and its status if no layer has the default type. In
/// both cases a note says why. `custom` changes no root: the roots of the
/// current file are the choice of the file. The call is idempotent.
fn settle(plan: &mut Plan) {
    if plan.layout || plan.graph == GraphChoice::Custom {
        return;
    }
    let graph = plan.layers();
    let has = |t: DocType| graph.layer_of_type(t).is_some();
    let choice = plan.graph.name();
    let mut notes = Vec::new();
    // The note comes from the roots that detection found, also if the graph
    // has no layer for them.
    if has_proposals(&plan.rows) && !plan.notes.iter().any(|n| n == PROPOSALS_NOTE) {
        notes.push(PROPOSALS_NOTE.to_string());
    }
    plan.rows.retain_mut(|r| {
        match r.ty {
            Some(t) if !has(t) => {
                notes.push(format!(
                    "root `{}` of type `{}` is not written: the {choice} graph has no layer with that type. Choose `default` at question 2, or add a layer with `spec-dna layer add` and a root by hand",
                    r.path,
                    t.name()
                ));
                return false;
            }
            None if !r.no_default_type && !has(r.default_type) => {
                notes.push(format!(
                    "root `{}` has no `default-type`: the {choice} graph has no layer with its detected default type `{}`. A document with no type and no layer folder gives SD303",
                    r.path,
                    r.default_type.name()
                ));
                r.no_default_type = true;
                r.status = None;
            }
            _ => {}
        }
        true
    });
    plan.notes.extend(notes);
    // A configuration needs a root. If the graph left none, the project gets
    // the root of a new project.
    if plan.rows.is_empty() {
        plan.rows = layout_rows(&plan.root);
        plan.layout = true;
        plan.notes.push(format!(
            "no detected root fits the {choice} graph: `init` writes the root `{}` of a new project",
            plan.root
        ));
    }
}

/// The note for a folder of proposals or changes. A flat folder of them
/// (`rfcs`, `proposals`) and the prefixes `RFC` and `KEP` give `research`,
/// because the type `intent` is valid only as `intent/<slug>/intent.md`.
const PROPOSALS_NOTE: &str = "a folder of proposals or changes is a `research` root: an intent is valid only as `intent/<slug>/intent.md`. To start an open proposal, run `spec-dna new intent <slug>`";

/// True if a root holds proposals or changes.
fn has_proposals(rows: &[Row]) -> bool {
    const FLAT: [&str; 6] = [
        "rfcs",
        "proposals",
        "changes",
        "tasks",
        "plans",
        "enhancements",
    ];
    const PREFIXES: [&str; 4] = ["RFC", "RFD", "KEP", "PEP"];
    rows.iter().any(|r| {
        let name = r.path.rsplit('/').next().unwrap_or(&r.path);
        r.ty == Some(DocType::Research)
            && (FLAT.contains(&name.to_ascii_lowercase().as_str())
                || PREFIXES.iter().any(|p| has_prefix(r, p)))
    })
}

/// The notes of the preview: why `init` left a root out, and how to start a
/// proposal.
fn notes(plan: &Plan) -> Vec<String> {
    let mut out = plan.notes.clone();
    if has_proposals(&plan.rows) && !out.iter().any(|n| n == PROPOSALS_NOTE) {
        out.push(PROPOSALS_NOTE.into());
    }
    out.extend(plan.codeowners().map(|l| codeowners_note(&l)));
    out
}

/// True if a legacy id of a root starts with the prefix (`RFC-{n}`).
fn has_prefix(r: &Row, prefix: &str) -> bool {
    r.legacy_ids.iter().any(|l| {
        l.split_once('|')
            .map_or(l.as_str(), |(a, _)| a)
            .starts_with(&format!("{prefix}-"))
    })
}

/// Edits a row with `key=value` words: `path`, `type` (a type or `mixed`),
/// `default-type`, `status`, `exclude`, `legacy-ids` and `legacy-keys` (lists
/// with commas).
fn edit_row(r: &mut Row, edits: &str) -> Result<(), String> {
    for w in edits.split_whitespace() {
        let (k, v) = w
            .split_once('=')
            .ok_or_else(|| format!("`{w}`: write key=value"))?;
        let items = || -> Vec<String> {
            v.split(',')
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect()
        };
        match k {
            "path" => r.path = v.trim_matches('/').to_string(),
            "type" if v == "mixed" => r.ty = None,
            "type" => {
                let t = DocType::from_name(v)
                    .ok_or_else(|| format!("unknown type `{v}`; {}", DocType::unknown_hint(v)))?;
                r.ty = Some(t);
                r.default_type = t;
                retype(r);
            }
            "default-type" => {
                r.default_type = DocType::from_name(v)
                    .ok_or_else(|| format!("unknown type `{v}`; {}", DocType::unknown_hint(v)))?;
                r.no_default_type = false;
                retype(r);
            }
            "status" => {
                if r.ty.is_none() && r.no_default_type && !v.is_empty() && v != "none" {
                    return Err(
                        "a mixed root with no default type has no status: set `default-type` first"
                            .into(),
                    );
                }
                r.status = (!v.is_empty() && v != "none").then(|| v.to_string());
            }
            "exclude" => r.exclude = items(),
            "legacy-ids" => r.legacy_ids = items(),
            "legacy-keys" => r.legacy_keys = items(),
            _ => return Err(format!("`{k}` is not a key of a row")),
        }
    }
    Ok(())
}

/// When the type of a row changes, the status, the status map and the section
/// keys keep only the values that the new types allow.
fn retype(r: &mut Row) {
    let types = r.types();
    let ok = |s: &str| types.iter().any(|t| t.statuses().contains(&s));
    if r.status
        .as_deref()
        .is_some_and(|s| !r.default_type.statuses().contains(&s))
    {
        r.status = match r.default_type {
            DocType::Decision => Some("proposed".into()),
            DocType::Intent | DocType::Spec | DocType::Plan => Some("draft".into()),
            t if t.has_status() => Some("active".into()),
            _ => None,
        };
    }
    if !r.default_type.has_status() {
        r.status = None;
    }
    r.status_map.retain(|(from, to)| {
        ok(to) && !types.iter().all(|t| t.statuses().contains(&from.as_str()))
    });
    r.required_off.retain(|t| types.contains(t));
    r.extra
        .retain(|e| !e.starts_with("required-sections") && !e.starts_with("amendment-sections"));
}

fn row_line(r: &Row) -> String {
    let ty = match r.ty {
        Some(t) => t.name().to_string(),
        None if r.no_default_type => "mixed, no default type".to_string(),
        None => format!("mixed, default {}", r.default_type.name()),
    };
    let mut extra = Vec::new();
    if let Some(s) = &r.status {
        extra.push(format!("status {s}"));
    }
    if !r.legacy_ids.is_empty() {
        extra.push(format!("ids {}", list(&r.legacy_ids)));
    }
    if !r.legacy_keys.is_empty() {
        extra.push(format!("keys {}", list(&r.legacy_keys)));
    }
    if !r.status_map.is_empty() {
        let m: Vec<String> = r
            .status_map
            .iter()
            .map(|(a, b)| format!("{a}->{b}"))
            .collect();
        extra.push(format!("map {}", m.join(" ")));
    }
    if !r.exclude.is_empty() {
        extra.push(format!("exclude {}", list(&r.exclude)));
    }
    format!(
        "{} ({ty}; {} documents{}{})",
        r.path,
        r.docs,
        if extra.is_empty() { "" } else { "; " },
        extra.join("; ")
    )
}

/// The value of the evidence answer, as the question shows it.
fn evidence_value(plan: &Plan) -> String {
    let ev = match plan.evidence {
        None => "profile",
        Some(s) => severity_name(s),
    };
    let st = match plan.style {
        Style::Covers => "covers",
        Style::VerifiedBy => "verified-by",
        Style::Both => "both",
    };
    format!("{ev}, {st}")
}

/// Checks the answer of question 7: a BCP 47 language tag. `config` owns the
/// rule.
fn check_language(tag: &str) -> Result<(), String> {
    if config::is_language_tag(tag) {
        Ok(())
    } else {
        Err(format!(
            "`{tag}`: the language must be a BCP 47 tag, such as `en`, `en-GB` or `de`"
        ))
    }
}

/// The scripts of the headings that give a language, with the tag of each.
/// A script that several languages use (Cyrillic, Han) gets the most common
/// language. The heading aliases can give a better answer ([`ALIAS_LANGUAGES`]).
const SCRIPT_LANGUAGES: [(&str, &str); 9] = [
    ("Greek", "el"),
    ("Cyrillic", "ru"),
    ("Hebrew", "he"),
    ("Arabic", "ar"),
    ("Devanagari", "hi"),
    ("Thai", "th"),
    ("Japanese kana", "ja"),
    ("Hangul", "ko"),
    ("Han", "zh"),
];

/// The scripts of [`SCRIPT_LANGUAGES`] that several languages use. The tag of
/// such a script is a guess, so the preview has a note.
const SHARED_SCRIPTS: [&str; 4] = ["Cyrillic", "Arabic", "Devanagari", "Han"];

/// The language of each heading alias that detection can find. An alias in a
/// script that the table above shares between languages gives the language
/// that the alias proves.
const ALIAS_LANGUAGES: [(&str, &str); 29] = [
    ("Решение", "ru"),
    ("Поправки", "ru"),
    ("Дополнения", "ru"),
    ("Рішення", "uk"),
    ("Entscheidung", "de"),
    ("Lösung", "de"),
    ("Nachtrag", "de"),
    ("Nachträge", "de"),
    ("Änderungen", "de"),
    ("Décision", "fr"),
    ("Amendements", "fr"),
    ("Decisión", "es"),
    ("Enmiendas", "es"),
    ("Decisão", "pt"),
    ("Emendas", "pt"),
    ("Decisione", "it"),
    ("Emendamenti", "it"),
    ("Besluit", "nl"),
    ("Wijzigingen", "nl"),
    ("Beslut", "sv"),
    ("Decyzja", "pl"),
    ("Poprawki", "pl"),
    ("Rozhodnutí", "cs"),
    ("Karar", "tr"),
    ("Değişiklikler", "tr"),
    ("決定", "ja"),
    ("决策", "zh"),
    ("决定", "zh"),
    ("결정", "ko"),
];

/// The language that question 7 proposes, from the result of detection: the
/// heading aliases first, then the most common script of the headings. A
/// Latin script with no alias is `en`, with no note. If the result gives no
/// tag, or two tags, the language is `en` and a note says why.
fn detected_language(facts: &Facts) -> (String, Option<String>) {
    let en = || config::DEFAULT_LANGUAGE.to_string();
    let aliases = facts.outcome_aliases.iter().chain(&facts.amendment_aliases);
    let mut tags: Vec<&str> = Vec::new();
    for a in aliases {
        let tag = ALIAS_LANGUAGES
            .iter()
            .find(|(w, _)| w == a)
            .map(|(_, t)| *t);
        if let Some(t) = tag
            && !tags.contains(&t)
        {
            tags.push(t);
        }
    }
    let script = facts.scripts.first().map(|s| s.0.as_str());
    match (tags.as_slice(), script) {
        ([one], _) => ((*one).to_string(), None),
        ([], None | Some("Latin")) => (en(), None),
        ([], Some(s)) => match SCRIPT_LANGUAGES.iter().find(|(n, _)| *n == s) {
            Some((_, t)) if SHARED_SCRIPTS.contains(&s) => (
                (*t).to_string(),
                Some(format!(
                    "the headings are mostly in the script `{s}`, which several languages use: question 7 proposes `{t}` as a guess. Enter the tag of the language of the text"
                )),
            ),
            Some((_, t)) => ((*t).to_string(), None),
            None => (
                en(),
                Some(format!(
                    "the headings are mostly in a script (`{s}`) that has no language tag in detection: question 7 proposes `en`. Enter the tag of the language of the text"
                )),
            ),
        },
        (many, _) => (
            en(),
            Some(format!(
                "the heading aliases belong to {} languages ({}): question 7 proposes `en`. Enter the tag of the language of the text",
                many.len(),
                many.join(", ")
            )),
        ),
    }
}

/// The value of the language answer, as the question shows it.
fn language_value(plan: &Plan) -> String {
    let aliases = |v: &[String]| {
        if v.is_empty() {
            "-".to_string()
        } else {
            list(v)
        }
    };
    format!(
        "{}; outcome: {}; amendments: {}",
        plan.language,
        aliases(&plan.outcome),
        aliases(&plan.amendments)
    )
}

fn budgets_value(b: Budgets) -> &'static str {
    match b {
        Budgets::Default => "default",
        Budgets::Relaxed => "relaxed",
        Budgets::Tight => "tight",
        Budgets::Off => "off",
    }
}

/// Asks the questions on a terminal.
fn ask(plan: &mut Plan, facts: &Facts, io: &mut Ask) -> Result<(), String> {
    io.say(&format!(
        "spec-dna init: {} repository with {} files and {} Markdown documents. Press Enter to keep a default.",
        plan.kind, facts.files, facts.markdown
    ));
    io.say("lite: warnings only. standard: broken references and budget growth are errors. strict: drift, evidence and budgets are errors, and lint gives warnings.");
    let p = io.line(
        &Question::Profile.prompt("Profile (lite, standard, strict)"),
        plan.profile.name(),
    );
    plan.profile =
        Profile::from_name(&p).ok_or(format!("profile `{p}`: use lite, standard or strict"))?;

    io.say(&format!(
        "    Documents found: {}. small: foundation, pending, rules, requirements, decisions, intent and incidents (fewer than {SMALL_LIMIT} documents). default: also interfaces, procedures and research. custom: the layers of the current spec-dna.toml.",
        plan.documents
    ));
    let small = Graph::small();
    let outside: Vec<String> = facts
        .types
        .iter()
        .filter(|t| small.layer_of_type(**t).is_none())
        .map(|t| t.name().to_string())
        .collect();
    if !outside.is_empty() && plan.graph != GraphChoice::Custom {
        io.say(&format!(
            "    Found types that the small graph has no layer for: {}. A root of such a type is not written with the small graph.",
            list(&outside)
        ));
    }
    let g = io.line(
        &Question::Graph.prompt("Layer graph (small, default, custom)"),
        plan.graph.name(),
    );
    plan.graph = parse_graph(&g, plan)?;
    settle(plan);

    io.say(&format!(
        "    Areas found: {}. An area is a folder of requirements/.",
        areas_value(&facts.areas)
    ));
    let a = io.line(
        &Question::Areas.prompt("Areas of requirements/ (names, or none)"),
        &areas_value(&plan.areas),
    );
    plan.areas = parse_areas(&words(&a))?;

    settle_change(plan);
    io.say("    three-class: a small-fix is a pull request only, a routine change has an intent and a plan, and a higher-risk change also has a spec that the owner approves. two-class: each change has an intent. none: no change process; only the checks of the layers.");
    let a = io.line(
        &Question::Policy.prompt("Change policy (three-class, two-class, none)"),
        plan.change_policy.name(),
    );
    plan.change_policy = parse_policy(&a)?;
    settle_change(plan);
    if plan.change_policy != ChangePolicy::None {
        io.say("    block: a plan with no approval is an error (SD910). warn: it is a warning. off: no report.");
        let a = io.line(
            &Question::Gate.prompt("Gate (block, warn, off)"),
            plan.change_gate.name(),
        );
        plan.change_gate = parse_gate(&a)?;
        ask_approvers(plan, io)?;
    }

    let found: Vec<String> = facts
        .tests
        .iter()
        .map(|(g, n)| format!("{g} ({n})"))
        .collect();
    io.say(&format!(
        "    Test files: {}.",
        if found.is_empty() {
            "none yet".to_string()
        } else {
            list(&found)
        }
    ));
    let ev = match plan.evidence {
        None => "profile",
        Some(s) => severity_name(s),
    };
    let st = match plan.style {
        Style::Covers => "covers",
        Style::VerifiedBy => "verified-by",
        Style::Both => "both",
    };
    let a = io.line(
        &Question::Evidence.prompt(
            "Evidence policy (profile, off, warning, error) and link style (covers, verified-by, both)",
        ),
        &format!("{ev}, {st}"),
    );
    let w = words(&a);
    plan.evidence = parse_evidence(w.first().map_or(ev, |s| s.as_str()))?;
    plan.style = parse_style(w.get(1).map_or(st, |s| s.as_str()))?;

    let scripts: Vec<String> = facts
        .scripts
        .iter()
        .take(3)
        .map(|(s, n)| format!("{s} {n}"))
        .collect();
    io.say(&format!(
        "    Heading scripts: {}. The word lists of the lint are English, so another language turns them off.",
        if scripts.is_empty() {
            "none".into()
        } else {
            list(&scripts)
        }
    ));
    let default = format!(
        "{}; outcome: {}; amendments: {}",
        plan.language,
        if plan.outcome.is_empty() {
            "-".into()
        } else {
            list(&plan.outcome)
        },
        if plan.amendments.is_empty() {
            "-".into()
        } else {
            list(&plan.amendments)
        }
    );
    let a = io.line(
        &Question::Language.prompt("Normative language and heading aliases"),
        &default,
    );
    let mut parts = a.split(';');
    plan.language = parts.next().unwrap_or("en").trim().to_string();
    check_language(&plan.language)?;
    for part in parts {
        let (k, v) = part.split_once(':').unwrap_or((part, ""));
        let v: Vec<String> = v
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty() && *s != "-")
            .map(str::to_string)
            .collect();
        match k.trim() {
            "outcome" => plan.outcome = v,
            "amendments" => plan.amendments = v,
            other => return Err(format!("`{other}`: use outcome: and amendments:")),
        }
    }

    io.say(&format!(
        "{}. Mapping: {} roots. For each root, press Enter to accept it, enter `drop` to remove it, or enter `key=value ...` to edit it (path, type, default-type, status, exclude, legacy-ids, legacy-keys).",
        Question::Mapping.number(),
        plan.rows.len()
    ));
    let mut rows = Vec::new();
    for mut r in std::mem::take(&mut plan.rows) {
        if !r.why.is_empty() {
            io.say(&format!("    # {}", r.why));
        }
        loop {
            let a = io.line(&format!("    {}", row_line(&r)), "accept");
            match a.as_str() {
                "accept" => {
                    rows.push(r);
                    break;
                }
                "drop" => break,
                edits => match edit_row(&mut r, edits) {
                    Ok(()) => continue,
                    Err(e) => io.say(&format!("    {e}")),
                },
            }
        }
    }
    plan.rows = rows;
    // An edited root can have a type that the graph has no layer for.
    settle(plan);

    io.say(&format!(
        "    Documents over the hard budget: default {}, relaxed (x2) {}, tight (x0.5) {}.",
        detect::over_budget(facts, 1.0),
        detect::over_budget(facts, 2.0),
        detect::over_budget(facts, 0.5)
    ));
    let b = io.line(
        &Question::Budgets.prompt("Budget level (default, relaxed, tight, off)"),
        match plan.budgets {
            Budgets::Default => "default",
            Budgets::Relaxed => "relaxed",
            Budgets::Tight => "tight",
            Budgets::Off => "off",
        },
    );
    plan.budgets = parse_budgets(&b)?;

    io.say(&format!(
        "    Agent files: {}.",
        if facts.agent_files.is_empty() {
            "none".into()
        } else {
            list(&facts.agent_files)
        }
    ));
    let a = io.line(
        &Question::Adapters.prompt("Agent adapters (agents-md, claude-md, skill, none)"),
        &list(&adapter_names(&plan.adapters)),
    );
    plan.adapters = parse_adapters(&words(&a))?;

    io.say(&format!(
        "    CI: {}. Hooks: {}.",
        if facts.ci.is_empty() {
            "none".into()
        } else {
            list(&facts.ci)
        },
        if facts.hooks.is_empty() {
            "none".into()
        } else {
            list(&facts.hooks)
        }
    ));
    let a = io.line(
        &Question::Ci.prompt(
            "CI step (github, gitlab, none), hook (pre-commit, husky, lefthook, none) and base branch",
        ),
        &format!("{}, {}, {}", plan.ci, plan.hook, plan.base),
    );
    let w = words(&a);
    if let Some(c) = w.first() {
        plan.ci = check_choice("CI step", c, &["github", "gitlab", "none"])?;
    }
    if let Some(h) = w.get(1) {
        plan.hook = check_choice("hook", h, &["pre-commit", "husky", "lefthook", "none"])?;
    }
    if let Some(b) = w.get(2) {
        plan.base = b.clone();
    }

    io.say("    The agent harness is a team of models for Claude Code. lean writes the main model, the advisor, `verifier`, `Explore`, `fallback-advisor`, the stage skills, `REVIEW.md`, up to 3 hooks and a model-team line in CLAUDE.md.");
    io.say("    full also writes `implementer` and `code-simplifier`, and proposes the 4 skills that are off by default (`execute`, `tdd`, `verify` and `debug`). none writes nothing. The files are for Claude Code only.");
    for n in &plan.harness_notes {
        io.say(&format!("    {n}"));
    }
    let a = io.line(
        &Question::Harness.prompt("Agent harness (none, lean, full)"),
        plan.harness.level.name(),
    );
    plan.harness.level = parse_level(&a)?;
    settle_evals(plan);
    if plan.harness.level != Level::None {
        settle_parts(plan);
        // The level `full` proposes the 4 skills that are off too (D39).
        let proposed = proposed_skills(plan.harness.level);
        io.say(&format!(
            "    The {} skills add {} bytes to each request (their descriptions). `REVIEW.md` adds none. `protect-tests` asks before an edit of a test file that git tracks, on a branch of the form `fix/...`, and it needs a test glob.",
            proposed.len(),
            skill_listing_bytes(&proposed)
        ));
        let a = io.line(
            &Question::Harness.prompt(
                "Harness parts (skills=<names or none> review-md=<true or false> protect-tests=<fix-branches, always or off>)",
            ),
            &parts_line(&plan.harness),
        );
        // A skills value has commas, so the words split at spaces only.
        let parts: Vec<String> = a.split_whitespace().map(str::to_string).collect();
        apply_parts(plan, &parts)?;
        io.say("    The main model sets the cost of each request.");
        io.say("    The advisor reads the whole context at its own price each time that it is consulted. It needs Fable access, or `fallback-advisor` gives the advice.");
        io.say("    `verifier` and `Explore` run only when the main session uses them.");
        io.say("    `none` turns a role off. Write advisor=<model>/<model of fallback-advisor> to set both.");
        let a = io.line(
            &Question::Models.prompt("Models (role=model[:effort] ...)"),
            &models_line(&plan.harness),
        );
        apply_models(&mut plan.harness, &words(&a))?;
        io.say(&format!(
            "    skeleton writes the table [evals], {dir}/check.sh and an example eval with its fixture. For the CI answer github, it also writes .github/workflows/agent-evals.yml. CI runs no eval until you set the repository variable SPEC_DNA_EVALS. The check SD918 (each incident needs an eval) needs skeleton.",
            dir = plan.evals_table.dir
        ));
        let a = io.line(
            &Question::Evals.prompt("Evals (none, skeleton)"),
            plan.evals.name(),
        );
        plan.evals =
            EvalsAnswer::from_name(&a).ok_or(format!("evals `{a}`: use none or skeleton"))?;
    }
    validate_harness(&plan.harness)?;
    Ok(())
}

/// Applies an answers file (TOML) to the plan. A key that is not in the file
/// keeps its default.
fn apply_answers(plan: &mut Plan, text: &str) -> Result<bool, String> {
    let t: toml::Table = text.parse().map_err(|e: toml::de::Error| e.to_string())?;
    let s = |k: &str| -> Result<Option<String>, String> {
        match t.get(k) {
            None => Ok(None),
            Some(toml::Value::String(v)) => Ok(Some(v.clone())),
            Some(_) => Err(format!("answers: `{k}` must be a string")),
        }
    };
    let l = |k: &str| -> Result<Option<Vec<String>>, String> {
        match t.get(k) {
            None => Ok(None),
            Some(toml::Value::Array(a)) => a
                .iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
                .map(Some)
                .ok_or_else(|| format!("answers: `{k}` must be an array of strings")),
            Some(_) => Err(format!("answers: `{k}` must be an array of strings")),
        }
    };
    const KEYS: [&str; 27] = [
        "profile",
        "graph",
        "areas",
        "change-policy",
        "gate",
        "approvers",
        "evidence",
        "evidence-style",
        "language",
        "outcome-headings",
        "amendment-headings",
        "mapping",
        "roots",
        "drop",
        "budgets",
        "adapters",
        "ci",
        "hook",
        "base",
        "harness",
        "harness-models",
        "harness-deny",
        "harness-skills",
        "harness-review-md",
        "harness-protect-tests",
        "evals",
        "write",
    ];
    if t.contains_key("types") {
        return Err(
            "answers: the key `types` ended in 0.6; use the key `graph` (\"small\", \"default\" or \"custom\"): the layers give the types".into(),
        );
    }
    if let Some(k) = t.keys().find(|k| !KEYS.contains(&k.as_str())) {
        return Err(format!(
            "answers: unknown key `{k}`; use {}",
            KEYS.join(", ")
        ));
    }
    if let Some(p) = s("profile")? {
        plan.profile = Profile::from_name(&p).ok_or(format!("answers: profile `{p}`"))?;
    }
    if let Some(g) = s("graph")? {
        plan.graph = parse_graph(&g, plan).map_err(|e| format!("answers: {e}"))?;
    }
    // `areas = []` and `areas = "none"` give one folder.
    let areas = match t.get("areas") {
        None => None,
        Some(toml::Value::String(v)) => Some(vec![v.clone()]),
        Some(_) => l("areas")?,
    };
    if let Some(v) = areas {
        plan.areas = parse_areas(&v).map_err(|e| format!("answers: {e}"))?;
    }
    if let Some(v) = s("change-policy")? {
        plan.change_policy = parse_policy(&v).map_err(|e| format!("answers: {e}"))?;
    }
    if let Some(v) = s("gate")? {
        plan.change_gate = parse_gate(&v).map_err(|e| format!("answers: {e}"))?;
    }
    // The flag `--approver` has priority over the key.
    if let Some(v) = l("approvers")? {
        let list = parse_approvers("answers: `approvers`", &v)?;
        if plan.approvers_given.is_none() {
            plan.approvers_given = Some(list);
        }
    }
    if let Some(e) = s("evidence")? {
        plan.evidence = parse_evidence(&e)?;
    }
    if let Some(e) = s("evidence-style")? {
        plan.style = parse_style(&e)?;
    }
    if let Some(v) = s("language")? {
        check_language(&v).map_err(|e| format!("answers: {e}"))?;
        plan.language = v;
    }
    if let Some(v) = l("outcome-headings")? {
        plan.outcome = v;
    }
    if let Some(v) = l("amendment-headings")? {
        plan.amendments = v;
    }
    if let Some(m) = s("mapping")?
        && m != "accept"
    {
        return Err(format!(
            "answers: mapping `{m}`: use \"accept\", or give `[[roots]]` tables"
        ));
    }
    if let Some(roots) = t.get("roots") {
        // The roots replace the detected mapping. They use the keys of
        // spec-dna.toml.
        let mut tbl = toml::Table::new();
        tbl.insert("spec-dna".into(), toml::Value::String(VERSION.into()));
        tbl.insert("roots".into(), roots.clone());
        let src = format!(
            "spec-dna = {}\n{}",
            toml_str(VERSION),
            roots_text(roots).ok_or("answers: `roots` must be an array of tables")?
        );
        let cfg = Config::parse(&src).map_err(|e| format!("answers: roots: {e}"))?;
        plan.rows = cfg.roots.iter().map(row_of_root).collect();
        for r in &mut plan.rows {
            r.why = "from the answers file".into();
        }
        plan.layout = false;
    }
    if let Some(v) = l("drop")? {
        plan.rows.retain(|r| !v.contains(&r.path));
    }
    if let Some(b) = s("budgets")? {
        plan.budgets = parse_budgets(&b)?;
    }
    if let Some(v) = l("adapters")? {
        plan.adapters = parse_adapters(&v)?;
    }
    if let Some(c) = s("ci")? {
        plan.ci = check_choice("CI step", &c, &["github", "gitlab", "none"])?;
    }
    if let Some(h) = s("hook")? {
        plan.hook = check_choice("hook", &h, &["pre-commit", "husky", "lefthook", "none"])?;
    }
    if let Some(b) = s("base")? {
        plan.base = b;
    }
    if let Some(v) = s("harness")? {
        plan.harness.level = parse_level(&v).map_err(|e| format!("answers: {e}"))?;
    }
    if let Some(v) = l("harness-deny")? {
        plan.harness.deny.clear();
        for d in v {
            if !plan.harness.deny.contains(&d) {
                plan.harness.deny.push(d);
            }
        }
    }
    if let Some(v) = l("harness-models")? {
        apply_models(&mut plan.harness, &v).map_err(|e| format!("answers: harness-models: {e}"))?;
    }
    if let Some(v) = l("harness-skills")? {
        plan.harness.skills = parse_skills(&v).map_err(|e| format!("answers: {e}"))?;
        plan.parts.derive_skills = false;
    }
    match t.get("harness-review-md") {
        None => {}
        Some(toml::Value::Boolean(b)) => {
            plan.harness.review_md = *b;
            plan.parts.derive_review = false;
        }
        Some(_) => return Err("answers: `harness-review-md` must be true or false".into()),
    }
    if let Some(v) = s("harness-protect-tests")? {
        plan.harness.protect_tests = parse_protect(&v).map_err(|e| format!("answers: {e}"))?;
        plan.parts.derive_protect = false;
    }
    if let Some(v) = s("evals")? {
        plan.evals = EvalsAnswer::from_name(&v)
            .ok_or_else(|| format!("answers: evals `{v}`: use none or skeleton"))?;
    }
    // The level `none` asks no question 12 parts and no question 14, so an
    // answer for them has no effect: say so.
    if plan.harness.level == Level::None {
        for key in [
            "harness-skills",
            "harness-review-md",
            "harness-protect-tests",
        ] {
            if t.contains_key(key) {
                return Err(format!(
                    "answers: `{key}` needs a harness level other than none"
                ));
            }
        }
        if s("evals")?.as_deref() == Some("skeleton") && !plan.evals_current {
            return Err(
                "answers: `evals` \"skeleton\" needs a harness level other than none".into(),
            );
        }
    }
    validate_harness(&plan.harness).map_err(|e| format!("answers: {e}"))?;
    match t.get("write") {
        None => Ok(true),
        Some(toml::Value::Boolean(b)) => Ok(*b),
        Some(_) => Err("answers: `write` must be true or false".into()),
    }
}

/// The `[[roots]]` tables of an answers file as TOML text.
fn roots_text(v: &toml::Value) -> Option<String> {
    let mut out = String::new();
    for item in v.as_array()? {
        out.push_str("\n[[roots]]\n");
        for (k, v) in item.as_table()? {
            out.push_str(&format!("{} = {}\n", toml_key(k), toml_value(v)));
        }
    }
    Some(out)
}

/// The exit code of an error of the answers: 3 for a role of 0.5 that does
/// not exist (the same code as the key of the table), else 2.
fn answer_error(e: String) -> Fatal {
    if GoneRole::is_word_error(&e) {
        Fatal::Config(e)
    } else {
        Fatal::Usage(e)
    }
}

/// Runs `init`.
pub fn run(repo: &Repo, o: &Opts) -> Result<u8, Fatal> {
    // A bad `--approver` stops `init` before it reads or writes anything.
    let flag_approvers = parse_approvers("`--approver`", &o.approvers).map_err(Fatal::Usage)?;
    let cfg_path = repo.config_path();
    let exists = cfg_path.exists();
    if exists {
        let cfg = repo.load_config()?;
        if cfg.pin != VERSION {
            return Err(crate::pin_mismatch(&cfg));
        }
        if !o.reconfigure {
            return adapters_only(repo, o, &cfg);
        }
    } else if o.reconfigure {
        return Err(Fatal::Usage(
            "--reconfigure needs a spec-dna.toml; run `spec-dna init` first".into(),
        ));
    }
    let (facts, files) = detect_repo(repo)?;
    let root = o.root.as_deref().unwrap_or("docs").to_string();
    let mut plan = if exists {
        let text = std::fs::read_to_string(&cfg_path).map_err(|e| Fatal::Io(e.to_string()))?;
        current(&repo.load_config()?, &text, &facts)
    } else {
        defaults(&facts, Some(&root))
    };
    let existing = harness_existing(repo);
    let was_none = plan
        .harness_old
        .as_ref()
        .is_some_and(|h| h.level == Level::None);
    if plan.harness_old.is_none() || was_none {
        // A configuration of level `none` holds no other key, so a new level
        // takes the fresh defaults. The level `none` stays the default.
        let (h, notes) = harness_defaults(repo, &existing);
        plan.harness = h;
        plan.harness_notes = notes;
        plan.parts = Parts::derived();
        if was_none {
            plan.harness.level = Level::None;
        }
    } else {
        // A new level can add a role whose agent name the project uses.
        let h = &mut plan.harness;
        plan.harness_notes = apply_conflicts(h, &existing, true);
    }
    unmarked_parts(&mut plan, &existing);
    plan.git_email = repo.git.user_email();
    // A folder `evals/` of the project is not the folder of the evals (D50).
    if !plan.evals_current
        && plan.evals_table.dir == config::DEFAULT_EVALS_DIR
        && files
            .iter()
            .any(|f| f.starts_with(&format!("{}/", config::DEFAULT_EVALS_DIR)))
    {
        plan.evals_table.dir = ".spec-dna/evals".to_string();
    }
    let before = plan.clone();
    if !flag_approvers.is_empty() {
        plan.approvers_given = Some(flag_approvers);
    }
    let terminal = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let interactive = !o.non_interactive && o.answers.is_none() && terminal;
    let mut write = true;
    if let Some(path) = &o.answers {
        let text = std::fs::read_to_string(path)
            .map_err(|e| Fatal::Usage(format!("--answers {path}: {e}")))?;
        write = apply_answers(&mut plan, &text).map_err(answer_error)?;
    } else if interactive {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        let mut out = std::io::stdout();
        let mut io = Ask {
            input: &mut input,
            out: &mut out,
        };
        ask(&mut plan, &facts, &mut io).map_err(answer_error)?;
    }
    // The answers can change the graph, so the roots follow it now. The
    // preview shows the defaults with the same rule.
    settle(&mut plan);
    settle_change(&mut plan);
    settle_approval(&mut plan);
    settle_parts(&mut plan);
    settle_evals(&mut plan);
    settle_approvals_hook(&mut plan);
    let mut offered = before.clone();
    settle(&mut offered);
    settle_change(&mut offered);
    settle_approval(&mut offered);
    settle_parts(&mut offered);
    settle_evals(&mut offered);
    settle_approvals_hook(&mut offered);
    let text = render(&plan);
    // spec-dna never writes a configuration that it cannot read.
    let parsed = Config::parse(&text)
        .map_err(|e| Fatal::Config(format!("the new spec-dna.toml is not valid: {e}")))?;
    // The hook `protect-tests` uses the test globs that `check` reads from this
    // text, so `check` finds no difference.
    let mut new_harness = plan.harness.clone();
    new_harness.tests = parsed.evidence_tests.clone();
    // The hook `protect-approvals` reads the folders of the `intent` layer, as
    // `check` reads them from this text.
    new_harness.intent_dirs = crate::lifecycle::intent_dirs(&parsed);
    let mut changes = Vec::new();
    let old = std::fs::read_to_string(&cfg_path).ok();
    if old.as_deref() != Some(text.as_str()) {
        changes.push(Change {
            path: config::FILE_NAME.into(),
            created: old.is_none(),
            remove: false,
            text: text.clone(),
            note: if old.is_some() {
                "replaced; comments of the old file are lost".into()
            } else {
                "the configuration".into()
            },
        });
    }
    let types = adapters::project_types(&plan.layers(), &plan.kept_types);
    changes.extend(adapter_changes(&disk(repo), &plan.adapters, &types)?);
    let (ci, manual) = ci_changes(repo, &plan);
    changes.extend(ci);
    // A new configuration and `--reconfigure` both replace the harness.
    let hrun = plan_harness(
        repo,
        &new_harness,
        plan.harness_old.as_ref(),
        &plan.harness_notes,
        harness::Mode::Reconfigure,
        &mut changes,
    )?;
    let (skeleton, skeleton_notes) = skeleton_changes(repo, &plan, &parsed.evidence_tests);
    changes.extend(skeleton);
    plan.notes.extend(skeleton_notes);

    let preview = o.dry_run || interactive || !write;
    let resolved = resolved_answers(&plan, &offered, &text, &changes, &manual, hrun.as_ref());
    let codeowners = plan.codeowners();
    let notes = notes(&plan);
    let warnings: Vec<String> = types_warning(&plan).into_iter().collect();
    if o.json {
        let answers = resolved.iter().map(|r| {
            Obj::new()
                .num("number", r.number)
                .str("name", r.name)
                .str("value", &r.value)
                .str("default", &r.offered)
                .bool("is_default", r.default)
                .str("writes", &r.writes)
                .end()
        });
        let rows = plan.rows.iter().map(|r| {
            Obj::new()
                .str("path", &r.path)
                .str("type", r.ty.map_or("mixed", DocType::name))
                .raw(
                    "default_type",
                    &if r.no_default_type {
                        "null".to_string()
                    } else {
                        format!("\"{}\"", r.default_type.name())
                    },
                )
                .num("documents", r.docs)
                .str("why", &r.why)
                .end()
        });
        let detected = Obj::new()
            .str("kind", plan.kind)
            .num("files", facts.files)
            .num("markdown", facts.markdown)
            .raw("roots", &crate::json::array(rows))
            .strs("stack", &facts.stack)
            .strs("apps", &facts.apps)
            .strs("agent_files", &facts.agent_files)
            .strs("ci", &facts.ci)
            .strs("hooks", &facts.hooks)
            .str("base", &facts.base)
            .num("over_budget", detect::over_budget(&facts, 1.0))
            .end();
        let written: Vec<String> = if o.dry_run || !write {
            Vec::new()
        } else {
            apply(repo, &changes)?
        };
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .bool("dry_run", o.dry_run || !write)
                .raw("detected", &detected)
                .raw("answers", &crate::json::array(answers))
                .str("config", &text)
                .raw("files", &files_json(repo, &changes))
                .strs("manual", &manual)
                .strs("notes", &notes)
                .strs("warnings", &warnings)
                .opt_str("codeowners", codeowners.as_deref())
                .raw("harness", &harness_json(hrun.as_ref(), plan.harness.level))
                .strs("written", &written)
                .strs("evals_commands", &evals_commands(&written))
                .end()
        );
        return Ok(0);
    }
    if preview {
        println!("\nHow the answers were applied:");
        for r in &resolved {
            println!("{}", r.line());
        }
        for n in &notes {
            println!("note: {n}");
        }
        for w in &warnings {
            println!("warning: {w}");
        }
        println!("\n--- spec-dna.toml ---\n{}---", text);
        println!("Files to create or change:");
        if changes.is_empty() {
            println!("  none");
        }
        for c in &changes {
            if hrun.as_ref().is_some_and(|h| {
                c.path != harness::CLAUDE && h.plan.changes.iter().any(|x| x.path == c.path)
            }) {
                continue;
            }
            println!("  {} {} ({})", c.verb(), c.path, c.note);
        }
        for c in changes.iter().filter(|c| c.path != config::FILE_NAME) {
            print!("\n{}", change_diff(repo, c));
        }
        for m in &manual {
            println!("{m}");
        }
        if let Some(h) = &hrun {
            print_harness(h, true);
        }
    }
    if o.dry_run || !write {
        println!("dry run: nothing written");
        return Ok(0);
    }
    if interactive {
        print!("Write these files? [y/N]: ");
        let _ = std::io::stdout().flush();
        let mut s = String::new();
        let _ = std::io::stdin().lock().read_line(&mut s);
        if !matches!(s.trim(), "y" | "Y" | "yes") {
            println!("nothing written");
            return Ok(0);
        }
    } else {
        println!(
            "spec-dna init: {} repository; profile {}; {} root{}{}",
            plan.kind,
            plan.profile.name(),
            plan.rows.len(),
            if plan.rows.len() == 1 { "" } else { "s" },
            if exists { "" } else { " (see spec-dna.toml)" }
        );
    }
    let written = apply(repo, &changes)?;
    report(&written, &plan.adapters, &changes);
    if !preview {
        for n in &notes {
            println!("note: {n}");
        }
        for w in &warnings {
            println!("warning: {w}");
        }
    }
    for m in &manual {
        println!("{m}");
    }
    report_harness(hrun.as_ref(), &written);
    let commands = evals_commands(&written);
    if !commands.is_empty() {
        println!("To run the evals in CI, set the secret and the variable:");
        for c in commands {
            println!("  {c}");
        }
    }
    report_commit(&written);
    Ok(0)
}

/// The 2 commands that enable the CI runs of the evals, if the write created
/// the workflow. Otherwise `init` prints nothing and the list is empty.
fn evals_commands(written: &[String]) -> Vec<String> {
    if written.iter().any(|w| w == EVALS_WORKFLOW) {
        EVALS_COMMANDS.map(str::to_string).to_vec()
    } else {
        Vec::new()
    }
}

/// Writes the changes. Returns the paths.
fn apply(repo: &Repo, changes: &[Change]) -> Result<Vec<String>, Fatal> {
    let mut written = Vec::new();
    for c in changes {
        let p = repo.root.join(&c.path);
        if c.remove {
            match std::fs::remove_file(&p) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => return Err(Fatal::Io(format!("{}: {e}", p.display()))),
            }
            harness::remove_empty_skill_dir(&repo.root, &c.path);
            written.push(c.path.clone());
            continue;
        }
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d).map_err(|e| Fatal::Io(e.to_string()))?;
        }
        crate::mv::write(&p, &c.text)?;
        written.push(c.path.clone());
    }
    Ok(written)
}

/// One answer, as the preview shows it. `number` is the number of the
/// question in the design.
struct Resolved {
    number: usize,
    name: &'static str,
    value: String,
    /// The default of the question.
    offered: String,
    /// True if the value equals the default of the question.
    default: bool,
    /// What the answer writes. Empty if it writes nothing.
    writes: String,
}

impl Resolved {
    fn line(&self) -> String {
        let state = if self.default {
            "default".to_string()
        } else {
            format!("changed from {}", self.offered)
        };
        if self.writes.is_empty() {
            format!(
                "  {}. {}: {} ({state}, writes nothing)",
                self.number, self.name, self.value
            )
        } else {
            format!(
                "  {}. {}: {} ({state}) -> {}",
                self.number, self.name, self.value, self.writes
            )
        }
    }
}

/// How `init` applied each answer. `before` is the plan that the questions
/// offered, so its values are the defaults. `text` is the new
/// configuration, and `changes` are the files to write. `hrun` is the harness
/// plan.
fn resolved_answers(
    plan: &Plan,
    before: &Plan,
    text: &str,
    changes: &[Change],
    manual: &[String],
    hrun: Option<&HarnessRun>,
) -> Vec<Resolved> {
    let graph = |p: &Plan| p.graph.name().to_string();
    // The value has the full line of each root, so a change of a root shows in
    // the line.
    let mapping = |p: &Plan| {
        format!(
            "{} root{}: {}",
            p.rows.len(),
            if p.rows.len() == 1 { "" } else { "s" },
            p.rows.iter().map(row_line).collect::<Vec<_>>().join("; ")
        )
    };
    let ci = |p: &Plan| format!("{}, {}, {}", p.ci, p.hook, p.base);
    let adapter_files: Vec<&str> = changes
        .iter()
        .map(|c| c.path.as_str())
        .filter(|p| adapters::PATHS.contains(p))
        // The harness writes `CLAUDE.md` too, and question 12 lists it.
        .filter(|p| *p != harness::CLAUDE || plan.adapters.contains(&Adapter::ClaudeMd))
        .collect();
    let evals_paths = evals::skeleton_paths(&plan.evals_table.dir);
    let ci_files: Vec<&str> = changes
        .iter()
        .map(|c| c.path.as_str())
        .filter(|p| *p != config::FILE_NAME && !adapters::PATHS.contains(p))
        // The starter files of the evals belong to question 14.
        .filter(|p| !evals_paths.iter().any(|e| e == p))
        .filter(|p| !hrun.is_some_and(|h| h.plan.changes.iter().any(|x| x.path == *p)))
        .collect();
    // The harness files that the answer writes. A removal says so.
    let harness_files: Vec<String> = hrun.map_or_else(Vec::new, |h| {
        h.plan
            .changes
            .iter()
            .map(|c| match c.action {
                harness::Action::Remove => format!("removes {}", c.path),
                _ => c.path.clone(),
            })
            .collect()
    });
    let harness_writes = {
        let mut w = vec![format!(
            "[harness] level = \"{}\"",
            plan.harness.level.name()
        )];
        w.extend(harness_files);
        w.join(", ")
    };
    let models = |p: &Plan| {
        let m = models_line(&p.harness);
        if m.is_empty() { "-".to_string() } else { m }
    };
    let has = |needle: &str| text.contains(needle);
    let mut evidence = Vec::new();
    if plan.evidence.is_some() {
        evidence.push("[rules] evidence");
    }
    if has("\n[evidence]\n") {
        evidence.push("[evidence] tests");
    }
    let mut language = Vec::new();
    if !plan.language.eq_ignore_ascii_case(config::DEFAULT_LANGUAGE) {
        language.push(format!("language = {}", toml_str(&plan.language)));
    }
    if !plan.outcome.is_empty() && has("required-sections = { decision =") {
        language.push("required-sections".to_string());
    }
    if !plan.amendments.is_empty() && has("amendment-sections = { decision =") {
        language.push("amendment-sections".to_string());
    }
    let budgets = match plan.budgets {
        Budgets::Default => String::new(),
        Budgets::Off => "[rules] budgets = \"off\"".to_string(),
        _ => "[budgets.<type>] tables".to_string(),
    };
    let mut ci_writes = ci_files.join(", ");
    if ci_writes.is_empty() && !manual.is_empty() {
        ci_writes = "text to add by hand".into();
    }
    let rows = [
        (
            Question::Profile,
            plan.profile.name().to_string(),
            before.profile.name().to_string(),
            format!("profile = \"{}\"", plan.profile.name()),
        ),
        (
            Question::Graph,
            graph(plan),
            graph(before),
            if plan.writes_layers() {
                format!("{} [layers.*] tables", plan.layers().layers().len())
            } else {
                String::new()
            },
        ),
        (
            Question::Areas,
            areas_value(&plan.areas),
            areas_value(&before.areas),
            if plan.areas.is_empty() {
                String::new()
            } else {
                format!("areas = {}", toml_list(&plan.areas))
            },
        ),
        (
            Question::Policy,
            plan.change_policy.name().to_string(),
            before.change_policy.name().to_string(),
            if plan.writes_change() {
                "[change] policy, high-risk-layers, high-risk-paths, max-fanout".to_string()
            } else {
                String::new()
            },
        ),
        (
            Question::Gate,
            plan.change_gate.name().to_string(),
            before.change_gate.name().to_string(),
            if plan.writes_change() {
                "[change] gate".to_string()
            } else {
                String::new()
            },
        ),
        (
            Question::Evidence,
            evidence_value(plan),
            evidence_value(before),
            evidence.join(", "),
        ),
        (
            Question::Language,
            language_value(plan),
            language_value(before),
            language.join(", "),
        ),
        (
            Question::Mapping,
            mapping(plan),
            mapping(before),
            if plan.rows.is_empty() {
                String::new()
            } else {
                format!("{} [[roots]] table(s)", plan.rows.len())
            },
        ),
        (
            Question::Budgets,
            budgets_value(plan.budgets).to_string(),
            budgets_value(before.budgets).to_string(),
            budgets,
        ),
        (
            Question::Adapters,
            list(&adapter_names(&plan.adapters)),
            list(&adapter_names(&before.adapters)),
            adapter_files.join(", "),
        ),
        (Question::Ci, ci(plan), ci(before), ci_writes),
        (
            Question::Harness,
            plan.harness.level.name().to_string(),
            before.harness.level.name().to_string(),
            harness_writes,
        ),
        (
            Question::Models,
            models(plan),
            models(before),
            if plan.harness.level == Level::None {
                String::new()
            } else {
                "[harness.roles] table".to_string()
            },
        ),
        (
            Question::Evals,
            plan.evals.name().to_string(),
            // The level `none` asks no question 14. Its default is `none`, and
            // the current state for a file that has the table.
            if plan.harness.level == Level::None && !plan.evals_current {
                EvalsAnswer::None.name().to_string()
            } else {
                before.evals.name().to_string()
            },
            if plan.evals == EvalsAnswer::Skeleton {
                let mut w = vec!["[evals]".to_string()];
                w.extend(
                    changes
                        .iter()
                        .map(|c| c.path.clone())
                        .filter(|p| evals_paths.contains(p)),
                );
                w.join(", ")
            } else {
                String::new()
            },
        ),
    ];
    // The rows are in the order of the table of the questions.
    let mut rows: Vec<_> = rows.into_iter().collect();
    rows.sort_by_key(|r| r.0.number());
    rows.into_iter()
        .map(|(q, value, offered, writes)| {
            let default = value == offered;
            Resolved {
                number: q.number(),
                name: q.entry().2,
                value,
                offered,
                default,
                writes,
            }
        })
        .collect()
}

/// One edit of a line diff.
enum Op<'a> {
    Keep(&'a str),
    Del(&'a str),
    Add(&'a str),
}

/// The largest table (old lines times new lines) that the diff builds. A
/// larger change is a block of removed lines and a block of added lines.
const DIFF_LIMIT: usize = 4_000_000;

fn edit_script<'a>(old: &[&'a str], new: &[&'a str]) -> Vec<Op<'a>> {
    let mut head = 0;
    while head < old.len() && head < new.len() && old[head] == new[head] {
        head += 1;
    }
    let mut tail = 0;
    while tail < old.len() - head
        && tail < new.len() - head
        && old[old.len() - 1 - tail] == new[new.len() - 1 - tail]
    {
        tail += 1;
    }
    let (a, b) = (&old[head..old.len() - tail], &new[head..new.len() - tail]);
    let mut ops: Vec<Op> = old[..head].iter().map(|l| Op::Keep(l)).collect();
    if a.len() * b.len() > DIFF_LIMIT {
        ops.extend(a.iter().map(|l| Op::Del(l)));
        ops.extend(b.iter().map(|l| Op::Add(l)));
    } else {
        // lcs[i][j] is the length of the longest common subsequence of a[i..] and
        // b[j..].
        let w = b.len() + 1;
        let mut lcs = vec![0u32; (a.len() + 1) * w];
        for i in (0..a.len()).rev() {
            for j in (0..b.len()).rev() {
                lcs[i * w + j] = if a[i] == b[j] {
                    lcs[(i + 1) * w + j + 1] + 1
                } else {
                    lcs[(i + 1) * w + j].max(lcs[i * w + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < a.len() || j < b.len() {
            if i < a.len() && j < b.len() && a[i] == b[j] {
                ops.push(Op::Keep(a[i]));
                i += 1;
                j += 1;
            } else if j == b.len() || (i < a.len() && lcs[(i + 1) * w + j] >= lcs[i * w + j + 1]) {
                ops.push(Op::Del(a[i]));
                i += 1;
            } else {
                ops.push(Op::Add(b[j]));
                j += 1;
            }
        }
    }
    ops.extend(old[old.len() - tail..].iter().map(|l| Op::Keep(l)));
    ops
}

/// The lines of context around a change in a diff.
const DIFF_CONTEXT: usize = 2;

/// A unified diff of two texts, or an empty string if they are equal. `old`
/// is `None` for a file that does not exist yet. `deleted` is true for a file
/// that `init` removes: `new` is then empty, and the new side is `/dev/null`.
pub(crate) fn unified_diff(path: &str, old: Option<&str>, new: &str, deleted: bool) -> String {
    if old == Some(new) || (deleted && old.is_none()) {
        return String::new();
    }
    // A line keeps its line ending, so a change of CRLF to LF, or of the final
    // newline, is a change of the diff.
    let old_lines: Vec<&str> = old.unwrap_or("").split_inclusive('\n').collect();
    let new_lines: Vec<&str> = new.split_inclusive('\n').collect();
    let ops = edit_script(&old_lines, &new_lines);
    // The position of each op in the old and the new text (1-based).
    let mut at = Vec::with_capacity(ops.len());
    let (mut o, mut n) = (1usize, 1usize);
    for op in &ops {
        at.push((o, n));
        match op {
            Op::Keep(_) => {
                o += 1;
                n += 1;
            }
            Op::Del(_) => o += 1,
            Op::Add(_) => n += 1,
        }
    }
    let changed: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter(|(_, op)| !matches!(op, Op::Keep(_)))
        .map(|(i, _)| i)
        .collect();
    let mut out = format!(
        "--- {}\n+++ {}\n",
        if old.is_some() {
            format!("a/{path}")
        } else {
            "/dev/null".to_string()
        },
        if deleted {
            "/dev/null".to_string()
        } else {
            format!("b/{path}")
        }
    );
    let mut k = 0;
    while k < changed.len() {
        let start = changed[k].saturating_sub(DIFF_CONTEXT);
        let mut end = changed[k];
        while k + 1 < changed.len() && changed[k + 1] <= end + 2 * DIFF_CONTEXT + 1 {
            k += 1;
            end = changed[k];
        }
        let end = (end + DIFF_CONTEXT + 1).min(ops.len());
        let count = |keep: bool, del: bool, add: bool| {
            ops[start..end]
                .iter()
                .filter(|op| match op {
                    Op::Keep(_) => keep,
                    Op::Del(_) => del,
                    Op::Add(_) => add,
                })
                .count()
        };
        let (old_n, new_n) = (count(true, true, false), count(true, false, true));
        let (o, n) = at[start];
        out.push_str(&format!(
            "@@ -{},{old_n} +{},{new_n} @@\n",
            if old_n == 0 { o - 1 } else { o },
            if new_n == 0 { n - 1 } else { n }
        ));
        for op in &ops[start..end] {
            let (mark, l) = match op {
                Op::Keep(l) => (' ', l),
                Op::Del(l) => ('-', l),
                Op::Add(l) => ('+', l),
            };
            out.push(mark);
            out.push_str(l.strip_suffix('\n').unwrap_or(l));
            out.push('\n');
            if !l.ends_with('\n') {
                out.push_str("\\ No newline at end of file\n");
            }
        }
        k += 1;
    }
    out
}

/// The `files` array of `--json`: the path, the action, the note and the diff
/// of each change. The harness files are in the list, in the same form.
fn files_json(repo: &Repo, changes: &[Change]) -> String {
    let files = changes.iter().map(|c| {
        Obj::new()
            .str("path", &c.path)
            .str("action", c.verb())
            .str("note", &c.note)
            .str("diff", &change_diff(repo, c))
            .end()
    });
    json::array(files)
}

/// The diff of one change against the file on disk. A removal is a diff to
/// `/dev/null`.
fn change_diff(repo: &Repo, c: &Change) -> String {
    let old = std::fs::read_to_string(repo.root.join(&c.path)).ok();
    let new = if c.remove { "" } else { &c.text };
    unified_diff(&c.path, old.as_deref(), new, c.remove)
}

/// The bytes that spec-dna adds to the prompt prefix of each request.
/// Claude Code reads one copy of the line (in CLAUDE.md, or in AGENTS.md
/// through an import) and the skill descriptions. Other agents read
/// AGENTS.md.
fn prefix(adapters: &[Adapter]) -> (usize, usize, usize) {
    let line = adapters::line().len() + 1;
    let skill = if adapters.contains(&Adapter::Skill) {
        adapters::skill_prefix_bytes()
    } else {
        0
    };
    let agents = if adapters.contains(&Adapter::AgentsMd) {
        line
    } else {
        0
    };
    (line, skill, agents)
}

fn report(written: &[String], adapters: &[Adapter], changes: &[Change]) {
    if written.is_empty() {
        println!("spec-dna init: everything is up to date");
    }
    for w in written {
        let removed = changes.iter().any(|c| c.remove && c.path == *w);
        println!("{} {w}", if removed { "removed" } else { "wrote" });
    }
    let (line, skill, agents) = prefix(adapters);
    println!(
        "per-request prefix: {} bytes for Claude Code (line {line}, skill description {skill}), {agents} bytes for AGENTS.md",
        line + skill
    );
}

/// Prints the `git add` command for the paths that `init` wrote.
fn report_commit(written: &[String]) {
    if !written.is_empty() {
        println!("to commit: git add {}", written.join(" "));
    }
}

/// Prints the summary of the harness after a write, and the restart line.
fn report_harness(h: Option<&HarnessRun>, written: &[String]) {
    let Some(h) = h else { return };
    print_harness(h, false);
    if h.level != Level::None && h.plan.changes.iter().any(|c| written.contains(&c.path)) {
        println!("{RESTART}");
    }
}

/// The adapters of a project with a configuration and no new answer: which
/// adapters, and the files that change. `AGENTS.md` is a target if it exists or
/// if `CLAUDE.md` does not. The skill lists the types of the layers. A hook of
/// 0.4.1 reads the working tree, and the hook of this version reads the index.
/// `init` with an existing configuration and `upgrade` both use it, so the
/// planned base gives the findings of the head (D148).
pub(crate) fn adapters_of(
    src: &ReadText,
    cfg: &Config,
) -> Result<(Vec<Adapter>, Vec<Change>), Fatal> {
    let exists = |p: &str| src(p).is_some();
    let mut chosen = vec![Adapter::ClaudeMd, Adapter::Skill];
    if exists(adapters::AGENTS) || !exists(adapters::CLAUDE) {
        chosen.insert(0, Adapter::AgentsMd);
    }
    let types = adapters::project_types(&cfg.layers, &cfg.types);
    let mut changes = adapter_changes(src, &chosen, &types)?;
    for path in HOOK_FILES {
        changes.extend(hook_upgrade(src, path));
    }
    Ok((chosen, changes))
}

/// `init` with an existing `spec-dna.toml`: the adapters, and the harness
/// files if the configuration has a `[harness]` table. It writes and never
/// removes.
fn adapters_only(repo: &Repo, o: &Opts, cfg: &Config) -> Result<u8, Fatal> {
    let (chosen, mut changes) = adapters_of(&disk(repo), cfg)?;
    let hrun = match &cfg.harness {
        Some(h) => plan_harness(repo, h, Some(h), &[], harness::Mode::Upgrade, &mut changes)?,
        None => None,
    };
    if o.dry_run {
        if o.json {
            println!(
                "{}",
                Obj::new()
                    .str("version", VERSION)
                    .bool("dry_run", true)
                    .raw("files", &files_json(repo, &changes))
                    .raw("harness", &harness_json(hrun.as_ref(), harness_level(cfg)))
                    .end()
            );
            return Ok(0);
        }
        for c in &changes {
            println!("would {} {} ({})", c.verb(), c.path, c.note);
            print!("{}", change_diff(repo, c));
        }
        if let Some(h) = &hrun {
            print_harness(h, false);
        }
        println!("dry run: nothing written");
        return Ok(0);
    }
    let written = apply(repo, &changes)?;
    if o.json {
        let (line, skill, agents) = prefix(&chosen);
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .strs("written", &written)
                .raw("harness", &harness_json(hrun.as_ref(), harness_level(cfg)))
                .raw(
                    "prefix_bytes",
                    &Obj::new()
                        .num("claude_code", line + skill)
                        .num("agents_md", agents)
                        .end()
                )
                .end()
        );
    } else {
        report(&written, &chosen, &changes);
        report_harness(hrun.as_ref(), &written);
        report_commit(&written);
    }
    Ok(0)
}

fn harness_level(cfg: &Config) -> Level {
    cfg.harness.as_ref().map_or(Level::None, |h| h.level)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan_of_empty_repo() -> Plan {
        let facts = detect::detect(
            &[],
            |_| None,
            GitInfo {
                base: "main".into(),
            },
        );
        defaults(&facts, None)
    }

    /// Runs the approvers prompt with an empty answer. Returns the text of the
    /// prompt and the plan.
    fn approvers_prompt(git_email: Option<&str>) -> (String, Plan) {
        let mut plan = plan_of_empty_repo();
        plan.git_email = git_email.map(str::to_string);
        let mut input = std::io::Cursor::new(b"\n".to_vec());
        let mut out = Vec::new();
        {
            let mut io = Ask {
                input: &mut input,
                out: &mut out,
            };
            ask_approvers(&mut plan, &mut io).unwrap();
        }
        (String::from_utf8(out).unwrap(), plan)
    }

    #[test]
    fn the_approvers_prompt_offers_the_git_email_of_a_person_only() {
        // A person e-mail is the default, and the empty answer takes it.
        let (out, plan) = approvers_prompt(Some("person@example.test"));
        assert!(out.contains("or none [person@example.test]: "), "{out}");
        assert_eq!(
            plan.approvers_given,
            Some(vec!["person@example.test".into()])
        );
        // The address of an agent, a bot and no e-mail give the default `none`.
        for email in [
            Some("noreply@anthropic.com"),
            Some("41898282+github-actions[bot]@users.noreply.github.com"),
            None,
        ] {
            let (out, plan) = approvers_prompt(email);
            assert!(out.contains("or none [none]: "), "{email:?}: {out}");
            assert_eq!(plan.approvers_given, Some(Vec::new()), "{email:?}");
        }
        // The answer is checked like a flag.
        let mut plan = plan_of_empty_repo();
        let mut input = std::io::Cursor::new(b"noreply@anthropic.com\n".to_vec());
        let mut out = Vec::new();
        let mut io = Ask {
            input: &mut input,
            out: &mut out,
        };
        let e = ask_approvers(&mut plan, &mut io).unwrap_err();
        assert!(e.contains("is the address of an agent or a bot"), "{e}");
    }

    #[test]
    fn the_parts_line_round_trips_and_keeps_an_omitted_word() {
        let mut plan = plan_of_empty_repo();
        settle_parts(&mut plan);
        let line = parts_line(&plan.harness);
        assert_eq!(
            line,
            "skills=brainstorm,specify,clarify,write-plan,review,finish,post-mortem review-md=true protect-tests=fix-branches"
        );
        let words: Vec<String> = line.split_whitespace().map(str::to_string).collect();
        let mut again = plan.clone();
        apply_parts(&mut again, &words).unwrap();
        assert_eq!(parts_line(&again.harness), line);
        // An omitted word keeps its default, and a word sets its part.
        let mut p = plan.clone();
        apply_parts(
            &mut p,
            &["skills=none".into(), "protect-tests=always".into()],
        )
        .unwrap();
        assert_eq!(
            parts_line(&p.harness),
            "skills=none review-md=true protect-tests=always"
        );
        assert!(!p.parts.derive_skills && p.parts.derive_review);
    }

    #[test]
    fn a_bad_part_word_is_an_error() {
        let mut plan = plan_of_empty_repo();
        for (w, want) in [
            ("skills=build", "unknown skill `build`"),
            ("skills=specify,specify", "`specify` more than once"),
            ("skills=intent", "use `brainstorm`"),
            ("review-md=yes", "true or false"),
            ("protect-tests=never", "fix-branches, always or off"),
            ("models=none", "unknown part `models`"),
            ("skills", "key=value"),
        ] {
            let e = apply_parts(&mut plan, &[w.to_string()]).unwrap_err();
            assert!(e.contains(want), "{w}: {e}");
        }
    }

    #[test]
    fn the_default_skills_follow_the_final_policy_and_the_link_style() {
        let mut plan = plan_of_empty_repo();
        settle_parts(&mut plan);
        assert_eq!(plan.harness.skills.len(), 7);
        // A later answer changes the default, because no one set the part.
        plan.change_policy = ChangePolicy::None;
        plan.style = Style::VerifiedBy;
        settle_parts(&mut plan);
        assert!(plan.harness.skills.is_empty());
        assert_eq!(plan.harness.protect_tests, Protect::Off);
        // A part that the answers set stays.
        plan.parts.derive_skills = false;
        plan.harness.skills = vec![Skill::Specify];
        settle_parts(&mut plan);
        assert_eq!(plan.harness.skills, vec![Skill::Specify]);
    }

    #[test]
    fn the_listing_of_the_7_skills_that_are_on_has_at_most_1250_bytes() {
        let n = skill_listing_bytes(&proposed_skills(Level::Lean));
        assert!(n > 588 && n <= 1250, "{n} bytes");
        // The 11 skills of the level `full` have 1,900 bytes or fewer.
        let n = skill_listing_bytes(&proposed_skills(Level::Full));
        assert!(n <= 1900, "{n} bytes");
    }

    #[test]
    fn a_plugin_with_the_word_superpowers_gives_the_note_of_d5() {
        let on = |k: &str| format!("{{\"enabledPlugins\":{{\"{k}\":true}}}}");
        assert!(has_workflow_plugin(&on("superpowers@x")));
        assert!(has_workflow_plugin(&on("my-Superpowers-fork@y")));
        assert!(!has_workflow_plugin(&on("other@x")));
        assert!(!has_workflow_plugin(
            "{\"enabledPlugins\":{\"superpowers@x\":false}}"
        ));
        assert!(!has_workflow_plugin("{\"permissions\":{}}"));
        assert!(!has_workflow_plugin("not json"));
    }

    #[test]
    fn pre_commit_hook_goes_at_the_end_of_repos() {
        let t =
            "repos:\n  - repo: https://example.org/hooks\n    rev: v1\n    hooks:\n      - id: x\n";
        let out = pre_commit_append(t).unwrap();
        assert!(out.ends_with("  - repo: local\n    hooks:\n      - id: spec-dna\n        name: spec-dna check\n        entry: spec-dna check --staged\n        language: system\n        pass_filenames: false\n        always_run: true\n"), "{out}");
        assert!(pre_commit_append("repos:\n  - repo: a\nci:\n  skip: []\n").is_none());
        assert!(pre_commit_append("default_stages: [commit]\n").is_none());
    }

    #[test]
    fn rows_render_as_a_valid_configuration() {
        let facts = Facts::default();
        let mut plan = defaults(&facts, Some("docs"));
        assert_eq!(plan.kind, "greenfield");
        assert_eq!(plan.rows.len(), 1);
        plan.outcome = vec!["Decisión".into()];
        plan.amendments = vec!["Enmiendas".into()];
        plan.evidence = Some(Some(Severity::Error));
        plan.budgets = Budgets::Relaxed;
        plan.language = "de".into();
        let text = render(&plan);
        let cfg = Config::parse(&text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        assert_eq!(cfg.language, "de");
        assert!(!text.contains("lint"), "{text}");
        assert_eq!(cfg.roots.len(), 1);
        assert_eq!(cfg.roots[0].path, "docs");
        assert!(cfg.types.is_empty(), "every type by default");
        assert!(cfg.allows(DocType::Interface));
        assert_eq!(
            cfg.roots[0].required_sections[DocType::Decision as usize],
            [["Decisión", "Decision Outcome", "Decision"]]
        );
        assert_eq!(
            cfg.severity(crate::diag::Code::RequirementUnverified, Some(0)),
            Some(Severity::Error)
        );
        assert_eq!(
            cfg.severity(crate::diag::Code::RequirementKeyword, Some(0)),
            None
        );
        assert_eq!(
            cfg.budgets[DocType::Requirements as usize],
            (20_000, 40_000)
        );
        let mut r = plan.rows[0].clone();
        edit_row(
            &mut r,
            "type=procedure status=active exclude=a/**,b.md legacy-ids=DEF-{n}",
        )
        .unwrap();
        assert_eq!(r.ty, Some(DocType::Procedure));
        assert_eq!(r.exclude, ["a/**", "b.md"]);
        assert!(edit_row(&mut r, "colour=red").is_err());
    }

    #[test]
    fn review_fixes_for_rows_and_rules() {
        // A type edit removes the status and the section keys of the old type.
        let mut r = blank_row("docs/adr", Some(DocType::Decision), DocType::Decision);
        r.status = Some("proposed".into());
        r.extra
            .push("required-sections = { decision = [\"Decisión\"] }".into());
        edit_row(&mut r, "type=requirements").unwrap();
        assert_eq!(r.status.as_deref(), Some("active"));
        assert!(r.extra.is_empty());
        // A removed decision section stays removed with aliases.
        let facts = Facts::default();
        let mut plan = defaults(&facts, Some("docs"));
        plan.outcome = vec!["Decisión".into()];
        plan.rows[0].required_off.push(DocType::Decision);
        let text = render(&plan);
        assert!(
            text.contains("required-sections = { decision = [] }"),
            "{text}"
        );
        // The answers replace their rule. The other rules stay.
        plan.rules = vec![
            ("lint".into(), Some(Severity::Warning)),
            ("budgets".into(), Some(Severity::Error)),
            ("evidence".into(), Some(Severity::Error)),
        ];
        plan.evidence = None;
        let text = render(&plan);
        assert!(
            text.contains("lint = \"warning\"\nbudgets = \"error\"\n"),
            "{text}"
        );
        assert!(!text.contains("evidence ="), "{text}");
        plan.evidence = Some(Some(Severity::Warning));
        plan.budgets = Budgets::Off;
        let text = render(&plan);
        assert!(
            text.contains("budgets = \"off\"\nevidence = \"warning\"\n"),
            "{text}"
        );
        Config::parse(&text).unwrap();
        // The default test globs render as no `[evidence]` table.
        plan.tests = config::DEFAULT_TEST_GLOBS.map(str::to_string).to_vec();
        assert!(!render(&plan).contains("[evidence]"));
    }

    #[test]
    fn the_test_globs_of_the_hook_are_the_globs_that_check_reads() {
        // `init` gives the hook `protect-tests` the globs that `Config::parse`
        // reads from the text that `render` writes, so `check` finds no
        // difference. 3 cases: an empty repository, a subset of hits and
        // `verified-by`.
        let globs = |plan: &Plan| {
            let mut h = Harness::new(Level::Lean);
            h.protect_tests = harness::Protect::FixBranches;
            h.tests = Config::parse(&render(plan)).unwrap().evidence_tests;
            h
        };
        let defaults_list = config::DEFAULT_TEST_GLOBS.map(str::to_string).to_vec();
        let mut plan = defaults(&Facts::default(), Some("docs"));
        // An empty repository: no hit, so `render` writes no `[evidence]`.
        plan.tests = Vec::new();
        let h = globs(&plan);
        assert_eq!(h.tests, defaults_list);
        assert!(h.protect_hook_on());
        // A subset of hits.
        plan.tests = vec!["t/**".into(), "**/*_t.*".into()];
        let h = globs(&plan);
        assert_eq!(h.tests, ["t/**", "**/*_t.*"]);
        assert!(h.protect_hook_on());
        // `verified-by` gives no glob, so no hook.
        plan.style = Style::VerifiedBy;
        let h = globs(&plan);
        assert!(h.tests.is_empty() && !h.protect_hook_on());
    }

    #[test]
    fn the_models_line_round_trips_through_its_parser() {
        let mut h = Harness::new(Level::Lean);
        assert_eq!(
            models_line(&h),
            "orchestrator=opus:xhigh advisor=fable verifier=opus:high researcher=haiku:high helper=haiku"
        );
        h.level = Level::Full;
        assert!(
            models_line(&h)
                .ends_with("helper=haiku implementer=sonnet:high code-simplifier=sonnet:high"),
            "{}",
            models_line(&h)
        );
        let edits: Vec<String> = [
            "orchestrator=sonnet:inherit",
            "advisor=opus/sonnet:high",
            "verifier=none",
            "researcher=inherit",
        ]
        .map(String::from)
        .to_vec();
        apply_models(&mut h, &edits).unwrap();
        let line = models_line(&h);
        assert!(
            line.starts_with(
                "orchestrator=sonnet:inherit advisor=opus/sonnet:high verifier=none researcher=inherit:high helper=haiku"
            ),
            "{line}"
        );
        let mut again = Harness::new(Level::Full);
        apply_models(&mut again, &words(&line)).unwrap();
        assert_eq!(again, h);
        validate_harness(&h).unwrap();
    }

    #[test]
    fn the_model_words_refuse_what_the_configuration_refuses() {
        let lean = || Harness::new(Level::Lean);
        for (word, want) in [
            ("verifier", "write role=model[:effort]"),
            ("reviewer=opus", "the role `reviewer` is now `verifier`"),
            ("spec-writer=opus", "the role `spec-writer` ended"),
            ("judge=opus", "unknown role `judge`"),
            ("implementer=opus", "role `implementer` is in level full"),
            ("helper=haiku:high", "the role `helper` has no effort"),
            ("verifier=", "the model is empty"),
            ("verifier=opus:ultra", "unknown effort `ultra`"),
            ("verifier=opus:", "unknown effort ``"),
            ("advisor=opus/haiku:ultra", "unknown effort `ultra`"),
        ] {
            let e = apply_models(&mut lean(), &[word.to_string()]).unwrap_err();
            assert!(e.contains(want), "{word}: {e}");
        }
        // A word of a role of 0.5 gives exit code 3, the other errors 2.
        for (word, code) in [
            ("reviewer=opus", 3),
            ("spec-writer=opus", 3),
            ("judge=opus", 2),
        ] {
            let e = apply_models(&mut lean(), &[word.to_string()]).unwrap_err();
            assert_eq!(answer_error(e).exit_code(), code, "{word}");
        }
        // The configuration rules apply to the result.
        for (word, want) in [
            ("advisor=haiku", "cannot be \"haiku\""),
            ("advisor=claude-haiku-4-5", "cannot be \"claude-haiku-4-5\""),
            ("orchestrator=none", "cannot be \"none\""),
            ("orchestrator=opus:max", "cannot be \"max\""),
        ] {
            let mut h = lean();
            apply_models(&mut h, &[word.to_string()]).unwrap();
            let e = validate_harness(&h).unwrap_err();
            assert!(e.contains(want), "{word}: {e}");
        }
        // The fallback agent can be haiku.
        let mut h = lean();
        apply_models(&mut h, &["advisor=opus/haiku".to_string()]).unwrap();
        validate_harness(&h).unwrap();
        // A suffix that is no effort belongs to the model id.
        let mut h = lean();
        apply_models(&mut h, &["verifier=vendor.model-v1:0".to_string()]).unwrap();
        assert_eq!(
            h.cfg(Role::Verifier).model,
            Model::Id("vendor.model-v1:0".into())
        );
        // Level none takes any role word.
        let mut h = Harness::new(Level::None);
        apply_models(&mut h, &["implementer=opus".to_string()]).unwrap();
    }

    #[test]
    fn an_agent_name_in_use_turns_off_its_role_only() {
        let own =
            |path: &str, name: &str| (path.to_string(), format!("---\nname: {name}\n---\nown\n"));
        let existing = vec![
            own(".claude/agents/mine.md", "verifier"),
            own(".claude/agents/fallback-advisor.md", "fallback-advisor"),
            own(".claude/agents/implementer.md", "implementer"),
        ];
        let mut h = Harness::new(Level::Lean);
        let notes = apply_conflicts(&mut h, &existing, false);
        assert_eq!(notes.len(), 3, "{notes:?}");
        assert_eq!(h.cfg(Role::Verifier).model, Model::None);
        assert_eq!(h.cfg(Role::Advisor).agent, Model::None);
        assert!(h.cfg(Role::Advisor).model != Model::None);
        assert_eq!(h.cfg(Role::Implementer).model, Model::None);
        // For an existing configuration only the roles outside the level go.
        let mut h = Harness::new(Level::Lean);
        let notes = apply_conflicts(&mut h, &existing, true);
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(h.cfg(Role::Verifier).model, Model::Id("opus".into()));
    }

    fn facts_with(scripts: &[(&str, usize)], outcome: &[&str]) -> Facts {
        Facts {
            scripts: scripts.iter().map(|s| (s.0.to_string(), s.1)).collect(),
            outcome_aliases: outcome.iter().map(|s| s.to_string()).collect(),
            ..Facts::default()
        }
    }

    #[test]
    fn detection_proposes_a_language_tag() {
        let tag = |f: &Facts| detected_language(f);
        assert_eq!(tag(&Facts::default()), ("en".into(), None));
        assert_eq!(tag(&facts_with(&[("Latin", 9)], &[])), ("en".into(), None));
        assert_eq!(
            tag(&facts_with(&[("Latin", 9)], &["Entscheidung"])),
            ("de".into(), None)
        );
        assert_eq!(tag(&facts_with(&[("Greek", 4)], &[])), ("el".into(), None));
        // An alias decides between the languages of one script.
        assert_eq!(
            tag(&facts_with(&[("Cyrillic", 4)], &["Рішення"])),
            ("uk".into(), None)
        );
        // A script that several languages use gives a guess, and a note.
        let (t, note) = tag(&facts_with(&[("Cyrillic", 4)], &[]));
        assert_eq!(t, "ru");
        assert!(note.is_some_and(|n| n.contains("`Cyrillic`") && n.contains("guess")));
        // No tag for the script: `en`, and a note.
        let (t, note) = tag(&facts_with(&[("other", 4)], &[]));
        assert_eq!(t, "en");
        assert!(note.is_some_and(|n| n.contains("`other`")));
        // Aliases of 2 languages: `en`, and a note.
        let (t, note) = tag(&facts_with(&[("Latin", 4)], &["Entscheidung", "Décision"]));
        assert_eq!(t, "en");
        assert!(note.is_some_and(|n| n.contains("de, fr")));
    }

    #[test]
    fn the_answer_of_question_7_is_a_language_tag() {
        for ok in ["en", "de", "en-GB", "zh-Hans", "yue"] {
            assert!(check_language(ok).is_ok(), "{ok}");
        }
        for bad in ["", "German", "e", "de_DE", "x-private"] {
            assert!(check_language(bad).is_err(), "{bad}");
        }
    }
}

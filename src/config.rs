//! `spec-dna.toml` is the project configuration (ADR 0001, "Layers").
//! docs/config.md describes it. schema/spec-dna.schema.json contains its JSON
//! Schema.

use crate::adapters::VERSION;
use crate::diag::{Code, Severity};
use crate::harness::Harness;
use crate::layer::{self, Graph};
use crate::model::DocType;
use crate::release;
use toml::{Table, Value};

pub const FILE_NAME: &str = "spec-dna.toml";

#[derive(Clone, Debug)]
pub struct Config {
    /// The pinned spec-dna version.
    pub pin: String,
    /// True for `check --lint`: the prose lint (SD801 to SD805) runs in each
    /// profile, as a warning, also if a rule turned it off. A rule that sets a
    /// severity for it keeps that severity. This is a flag of one run, not a
    /// key of the file.
    pub force_lint: bool,
    pub exclude: Vec<String>,
    pub roots: Vec<Root>,
    /// Soft and hard budgets in characters, per document type.
    pub budgets: [(u32, u32); DocType::COUNT],
    pub max_words: u32,
    pub context_budget: u32,
    pub repos: Vec<(String, String)>,
    /// URL prefixes that address this repository. A link under one of them is
    /// internal.
    pub self_urls: Vec<String>,
    /// The git source of spec-dna for `cargo install --git`. The message of a
    /// pin mismatch uses it.
    pub install_source: String,
    /// The table `[spec-dna-sha256]`: the release assets of the pin and their
    /// SHA-256 sums. `None` if there is no table. If the pin is not the
    /// version of the binary, the table holds only the asset of this binary,
    /// if it has one. spec-dna does not validate the table in this case.
    pub sha256: Option<Vec<(String, String)>>,
    /// The profile: the default severity of each diagnostic code.
    pub profile: Profile,
    /// Severity overrides for every root (`[rules]`).
    pub rules: Rules,
    /// Globs of the files where `check` reads `covers:` tags.
    pub evidence_tests: Vec<String>,
    /// The document types of the project. Empty: every type.
    pub types: Vec<DocType>,
    /// The number of characters that a document over its hard budget in the
    /// base can grow in one change. Growth up to this number gives a warning
    /// (SD703), not an error (SD702).
    pub growth: u32,
    /// For each type, the sections of project documents that state the
    /// project conventions for documents of that type (`[conventions]`), as
    /// `<doc-id or path>[#<anchor>]`.
    pub conventions: [Vec<String>; DocType::COUNT],
    /// The agent harness (`[harness]`, ADR 0004). `None`: no table.
    pub harness: Option<Harness>,
    /// The layer graph: the table `[layers]`, or the default graph for the
    /// types of the project if the file has no such table (`layers_declared`
    /// is false). The commands read it from the place rule on.
    pub layers: Graph,
    /// True if the file has the table `[layers]`.
    pub layers_declared: bool,
    /// The areas of the project: the folders of `requirements/`.
    #[allow(dead_code)]
    pub areas: Vec<String>,
    /// `[groups]`: a name and its areas, in the order of the name.
    pub groups: Vec<(String, Vec<String>)>,
    /// `[templates] dir`: the folder of the project templates, relative to the
    /// repository root, with no `./` and no final `/`. It is never part of a
    /// root: `is_excluded` is true for each path in it.
    pub templates_dir: String,
    /// `[evals]`. `None`: no table, so there are no evals.
    pub evals: Option<Evals>,
    /// The language of the normative text: a BCP 47 tag (`language`). The
    /// default is `en`. A primary subtag other than `en` turns off the checks
    /// that read English word lists (SD801 to SD805 and SD904).
    pub language: String,
    /// `[lint] history-terms`: the phrases that SD903 reports. They replace
    /// the default phrases. `None`: the key is absent, so the default
    /// phrases apply. `Some` with an empty list: no phrase.
    pub history_terms: Option<Vec<String>>,
    /// `[change] policy`. With no `[change]` table it is `None`.
    pub change_policy: ChangePolicy,
    /// `[change] gate`. `None`: the key is absent, so SD910 takes the
    /// severity of the profile.
    pub change_gate: Option<Gate>,
    /// `[change] high-risk-layers`: names of layers of the graph. The default
    /// is `interfaces` and `rules`, without the names that the graph does not
    /// have.
    pub high_risk_layers: Vec<String>,
    /// `[change] high-risk-paths`: globs from the repository root. The
    /// default is none.
    pub high_risk_paths: Vec<String>,
    /// `[change] max-fanout`: the most requirement documents that one change
    /// can touch before SD905. The default is `DEFAULT_MAX_FANOUT`.
    pub max_fanout: u32,
}

/// The default folder of the evals.
pub const DEFAULT_EVALS_DIR: &str = "evals";
pub const DEFAULT_EVALS_MIN_PASS_RATE: f64 = 0.8;
pub const DEFAULT_EVALS_RUNS: u32 = 3;
pub const DEFAULT_EVALS_MAX_TOTAL_USD: f64 = 20.0;

/// The table `[evals]` (ADR 0008).
#[derive(Clone, Debug, PartialEq)]
pub struct Evals {
    /// The folder of the evals, relative to the repository root, with no `./`
    /// and no final `/`. It is never part of a root: `is_excluded` is true
    /// for each path in it.
    pub dir: String,
    /// The share of evals that must pass, from 0 to 1.
    pub min_pass_rate: f64,
    /// The runs of an eval that sets no `runs`, from 1 to 100.
    pub runs: u32,
    /// The most that `eval run` can spend, in USD. Above 0.
    pub max_total_usd: f64,
}

impl Evals {
    fn parse(v: &Value, templates_dir: &str) -> Result<Evals, String> {
        let tbl = v.as_table().ok_or("`evals` must be a table")?;
        let mut f = Fields::new(tbl, "evals.");
        let dir = match f.opt_str("dir")? {
            Some(d) => dir_value("evals.dir", &d, "evals")?,
            None => DEFAULT_EVALS_DIR.to_string(),
        };
        if dir == templates_dir
            || dir.starts_with(&format!("{templates_dir}/"))
            || templates_dir.starts_with(&format!("{dir}/"))
        {
            return Err(format!(
                "`evals.dir` and `templates.dir` overlap (`{dir}` and `{templates_dir}`); write 2 folders, none inside the other"
            ));
        }
        let min_pass_rate = match f.take("min-pass-rate") {
            None => DEFAULT_EVALS_MIN_PASS_RATE,
            Some(v) => number(v)
                .filter(|n| (0.0..=1.0).contains(n))
                .ok_or("`evals.min-pass-rate` must be a number from 0 to 1")?,
        };
        let runs = match f.take("runs") {
            None => DEFAULT_EVALS_RUNS,
            Some(Value::Integer(n)) if (1..=100).contains(n) => *n as u32,
            Some(_) => return Err("`evals.runs` must be an integer from 1 to 100".into()),
        };
        let max_total_usd = match f.take("max-total-usd") {
            None => DEFAULT_EVALS_MAX_TOTAL_USD,
            Some(v) => number(v)
                .filter(|n| n.is_finite() && *n > 0.0)
                .ok_or("`evals.max-total-usd` must be a number above 0")?,
        };
        f.done()?;
        Ok(Evals {
            dir,
            min_pass_rate,
            runs,
            max_total_usd,
        })
    }
}

/// Returns a TOML integer or float as a number.
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(n) => Some(*n as f64),
        Value::Float(n) => Some(*n),
        _ => None,
    }
}

/// The gate of the change process (`[change] gate`, design 0.6.0, "Gates").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gate {
    Block,
    Warn,
    Off,
}

impl Gate {
    pub fn name(self) -> &'static str {
        match self {
            Gate::Block => "block",
            Gate::Warn => "warn",
            Gate::Off => "off",
        }
    }

    fn from_name(s: &str) -> Option<Gate> {
        [Gate::Block, Gate::Warn, Gate::Off]
            .into_iter()
            .find(|g| g.name() == s)
    }

    /// The severity of SD910 for this gate.
    fn severity(self) -> Option<Severity> {
        match self {
            Gate::Block => Some(Severity::Error),
            Gate::Warn => Some(Severity::Warning),
            Gate::Off => None,
        }
    }
}

/// The default of `[change] max-fanout`.
pub const DEFAULT_MAX_FANOUT: u32 = 8;

/// The default of `[change] high-risk-layers`. A layer that the graph does
/// not have is dropped.
pub const DEFAULT_HIGH_RISK_LAYERS: [&str; 2] = ["interfaces", "rules"];

/// The values of the table `[change]`.
struct ChangeTable {
    policy: ChangePolicy,
    gate: Option<Gate>,
    high_risk_layers: Vec<String>,
    high_risk_paths: Vec<String>,
    max_fanout: u32,
}

impl ChangeTable {
    /// No table: the policy is `none`.
    fn absent(graph: &Graph) -> ChangeTable {
        ChangeTable {
            policy: ChangePolicy::None,
            gate: None,
            high_risk_layers: default_high_risk_layers(graph),
            high_risk_paths: Vec::new(),
            max_fanout: DEFAULT_MAX_FANOUT,
        }
    }
}

fn default_high_risk_layers(graph: &Graph) -> Vec<String> {
    DEFAULT_HIGH_RISK_LAYERS
        .iter()
        .filter(|n| graph.index(n).is_some())
        .map(|n| n.to_string())
        .collect()
}

impl ChangeTable {
    fn parse(tbl: &Table, graph: &Graph) -> Result<ChangeTable, String> {
        let mut f = Fields::new(tbl, "change.");
        let policy = f.req_str("policy")?;
        let policy = ChangePolicy::from_name(&policy).ok_or_else(|| {
            format!(
                "`change.policy` must be \"three-class\", \"two-class\" or \"none\", not `{policy}`"
            )
        })?;
        let gate = match f.opt_str("gate")? {
            None => None,
            Some(g) => Some(Gate::from_name(&g).ok_or_else(|| {
                format!("`change.gate` must be \"block\", \"warn\" or \"off\", not `{g}`")
            })?),
        };
        let high_risk_layers = match f.take("high-risk-layers") {
            None => default_high_risk_layers(graph),
            Some(_) => {
                let names = f.opt_strs("high-risk-layers")?;
                let mut out: Vec<String> = Vec::new();
                for n in names {
                    if graph.index(&n).is_none() {
                        let have: Vec<&str> =
                            graph.layers().iter().map(|l| l.name.as_str()).collect();
                        return Err(format!(
                            "`change.high-risk-layers`: `{n}` is not a layer of the graph ({})",
                            have.join(", ")
                        ));
                    }
                    if out.contains(&n) {
                        return Err(format!(
                            "`change.high-risk-layers` lists `{n}` more than once"
                        ));
                    }
                    out.push(n);
                }
                out
            }
        };
        let high_risk_paths = high_risk_paths_value(&f.opt_strs("high-risk-paths")?)?;
        let max_fanout = f.opt_u32("max-fanout")?.unwrap_or(DEFAULT_MAX_FANOUT);
        f.done()?;
        Ok(ChangeTable {
            policy,
            gate,
            high_risk_layers,
            high_risk_paths,
            max_fanout,
        })
    }
}

/// Checks `[change] high-risk-paths`: globs from the repository root. A glob
/// is not empty, has no `\`, is not absolute, has no `.` or `..` segment,
/// and is listed once. A leading `./` is removed.
fn high_risk_paths_value(globs: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for g in globs {
        let bad = |why: &str| {
            Err(format!(
                "`change.high-risk-paths`: `{g}` {why}; write a glob from the repository root, such as `src/api/**`"
            ))
        };
        let mut v = g.trim();
        while let Some(rest) = v.strip_prefix("./") {
            v = rest;
        }
        if v.trim_matches('/').is_empty() {
            return bad("is empty");
        }
        if v.starts_with('/') || v.contains('\\') {
            return bad("is not a relative path with `/` separators");
        }
        if v.split('/').any(|s| s == "." || s == "..") {
            return bad("has a `.` or `..` segment");
        }
        if out.iter().any(|o| o == v) {
            return bad("is listed more than once");
        }
        out.push(v.to_string());
    }
    Ok(out)
}

/// The change policy (`[change] policy`, design 0.6.0, "Change classes").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChangePolicy {
    ThreeClass,
    TwoClass,
    None,
}

impl ChangePolicy {
    pub fn name(self) -> &'static str {
        match self {
            ChangePolicy::ThreeClass => "three-class",
            ChangePolicy::TwoClass => "two-class",
            ChangePolicy::None => "none",
        }
    }

    fn from_name(s: &str) -> Option<ChangePolicy> {
        [
            ChangePolicy::ThreeClass,
            ChangePolicy::TwoClass,
            ChangePolicy::None,
        ]
        .into_iter()
        .find(|p| p.name() == s)
    }
}

/// The default language of the normative text.
pub const DEFAULT_LANGUAGE: &str = "en";

/// Checks the form of a BCP 47 language tag: a primary subtag of 2 or 3
/// letters, then subtags of 1 to 8 letters or digits, separated by `-`. The
/// check does not look at the registry. The grandfathered tags and the
/// private-use tags (`i-...`, `x-...`) are not valid. The schema has the same
/// pattern.
pub fn is_language_tag(s: &str) -> bool {
    let mut parts = s.split('-');
    let primary = parts.next().unwrap_or("");
    (2..=3).contains(&primary.len())
        && primary.bytes().all(|b| b.is_ascii_alphabetic())
        && parts.all(|p| (1..=8).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric()))
}

/// The default folder of the project templates.
pub const DEFAULT_TEMPLATES_DIR: &str = ".spec-dna/templates";

/// The default growth allowance: about one amendment or a few registry rows.
pub const DEFAULT_GROWTH: u32 = 4_000;

/// The public repository of spec-dna: the install source by default.
pub const DEFAULT_INSTALL_SOURCE: &str = "https://github.com/AlexGavrilov939/spec-dna";

/// Reads `[spec-dna-sha256]`. If the pin is the version of this binary, it
/// validates the table (ADR 0003, amendment "phase 2"). Otherwise it keeps
/// only the sum of the asset of this binary.
pub(crate) fn sha256_table(
    v: Option<&Value>,
    pin: &str,
) -> Result<Option<Vec<(String, String)>>, String> {
    let Some(v) = v else {
        return Ok(None);
    };
    if pin != VERSION {
        return Ok(v.as_table().map(|t| {
            let own = release::own_asset(pin).and_then(|a| {
                t.get(&a)
                    .and_then(Value::as_str)
                    .map(|s| (a, s.to_string()))
            });
            own.into_iter().collect()
        }));
    }
    let tbl = v
        .as_table()
        .ok_or("`spec-dna-sha256` must be a table of release assets and their SHA-256 sums")?;
    let assets = release::assets(pin);
    let mut out = Vec::new();
    for (k, v) in tbl {
        if !assets.contains(k) {
            return Err(format!(
                "`spec-dna-sha256`: `{k}` is not a release asset of spec-dna {pin}; copy the table from spec-dna-pin.toml of release v{pin}"
            ));
        }
        match v.as_str() {
            Some(s)
                if s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) =>
            {
                out.push((k.clone(), s.to_string()))
            }
            _ => {
                return Err(format!(
                    "`spec-dna-sha256`: the sum of `{k}` must be 64 lowercase hexadecimal characters"
                ));
            }
        }
    }
    Ok(Some(out))
}

/// The first version whose model this binary reads (D133). A pin before it is
/// a 0.5 pin, and only `upgrade` reads such a project. It is not `VERSION`:
/// a later 0.6 release still reads the 0.6 model.
pub const MODEL_06: &str = "0.6.0";

/// The oldest pin that `upgrade` accepts (ADR 0009).
pub const PIN_OLDEST: &str = "0.4.0";

/// The last release of the 0.4 line. A project with an older pin moves to it
/// first (ADR 0003, "Pin first").
pub const PIN_FIRST_STEP: &str = "0.4.2";

/// A release version: 3 numbers with dots.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version(pub u64, pub u64, pub u64);

impl Version {
    /// Reads 3 numbers with dots, such as `0.4.2`. A number has digits only,
    /// with no sign and no leading zero, so 2 equal versions are 2 equal
    /// texts. Any other text, such as `0.5` or `0.5.0-rc1`, gives `None`.
    pub fn parse(s: &str) -> Option<Version> {
        let mut parts = s.split('.');
        let mut number = || {
            let p = parts.next()?;
            let digits = !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit());
            if !digits || (p.len() > 1 && p.starts_with('0')) {
                return None;
            }
            p.parse::<u64>().ok()
        };
        let v = Version(number()?, number()?, number()?);
        parts.next().is_none().then_some(v)
    }
}

/// The class of a pin against the version of the binary (D134). The class
/// decides what `upgrade` does and the last sentence of the pin mismatch
/// message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinClass {
    /// Not 3 numbers with dots.
    Invalid,
    /// Before 0.4.0 (`PIN_OLDEST`).
    BeforeOldest,
    /// From 0.4.0 to before 0.6.0 (`MODEL_06`): the 0.5 reader.
    Model05,
    /// From 0.6.0 to before the binary: the 0.6 parser.
    Model06,
    /// The version of the binary.
    Binary,
    /// After the binary.
    AfterBinary,
}

/// The class of `pin` when the binary is `binary`. A test gives another
/// binary to reach each class.
pub fn pin_class(pin: &str, binary: &str) -> PinClass {
    let (Some(p), Some(b)) = (Version::parse(pin), Version::parse(binary)) else {
        return PinClass::Invalid;
    };
    let limit = |v: &str| Version::parse(v).expect("a constant is a version");
    if p == b {
        PinClass::Binary
    } else if p > b {
        PinClass::AfterBinary
    } else if p < limit(PIN_OLDEST) {
        PinClass::BeforeOldest
    } else if p < limit(MODEL_06) {
        PinClass::Model05
    } else {
        PinClass::Model06
    }
}

/// Reads the key `spec-dna` of a configuration text with a line scan, for a
/// text that does not parse. It looks at the lines before the first table
/// header.
pub fn scan_pin(text: &str) -> Option<String> {
    // A byte order mark is not part of the first key.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            return None;
        }
        let Some(rest) = line.strip_prefix("spec-dna") else {
            continue;
        };
        let Some(rest) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        // A basic string or a literal string: a pin has no escape.
        let rest = rest.trim_start();
        let quote = rest.chars().next().filter(|c| matches!(c, '"' | '\''))?;
        return rest[1..].split_once(quote).map(|(v, _)| v.to_string());
    }
    None
}

/// The message of a pin mismatch (ADR 0003). `sums` is `[spec-dna-sha256]`,
/// if the file has the table. The release form needs the public install
/// source and the sum of the asset of this binary. The last sentence follows
/// the class of the pin (D134): `upgrade` moves a project that pins 0.4.0 or
/// later, and a project that pins an earlier version moves to 0.4.2 first.
pub fn pin_mismatch_message(pin: &str, source: &str, sums: Option<&[(String, String)]>) -> String {
    mismatch_message(pin, source, sums, VERSION)
}

/// `pin_mismatch_message` for a given version of the binary, so a test can
/// reach each class of the pin.
fn mismatch_message(
    pin: &str,
    source: &str,
    sums: Option<&[(String, String)]>,
    binary: &str,
) -> String {
    let release = release::own_asset(pin)
        .filter(|_| source == DEFAULT_INSTALL_SOURCE)
        .and_then(|a| sums.and_then(|s| s.iter().find(|(k, _)| *k == a).cloned()));
    let build = format!("cargo install --locked --force --git {source} --tag v{pin} spec-dna");
    let mut m = format!("spec-dna.toml pins version {pin}, but this binary is {binary}.\n");
    match release {
        Some((asset, sum)) => m.push_str(&format!(
            "  To use {pin}, download it and compare its SHA-256 with spec-dna.toml:\n    {source}/releases/download/v{pin}/{asset}\n    SHA-256: {sum}\n  Or build it from source:\n    {build}\n"
        )),
        None => m.push_str(&format!("  To use {pin}, install it:\n    {build}\n")),
    }
    match pin_class(pin, binary) {
        PinClass::BeforeOldest => m.push_str(&format!(
            "  First move the project to {PIN_FIRST_STEP} with the {PIN_FIRST_STEP} binary and `spec-dna init`, then run `spec-dna upgrade`."
        )),
        PinClass::Model05 | PinClass::Model06 if sums.is_some() => m.push_str(&format!(
            "  To move the project to {binary}, download {DEFAULT_INSTALL_SOURCE}/releases/download/v{binary}/spec-dna-pin.toml to a place outside the repository, for example the folder above it, verify it with `gh attestation verify`, and run `spec-dna upgrade --pin-file ../spec-dna-pin.toml`."
        )),
        PinClass::Model05 | PinClass::Model06 => m.push_str(&format!(
            "  To move the project to {binary}, run `spec-dna upgrade`."
        )),
        // The binary is older than the project: no sentence moves the
        // project to an older version. The pin of the binary is no mismatch.
        PinClass::AfterBinary | PinClass::Binary => {
            m.pop();
        }
        // The sentence of 0.5: the person sets the pin to the binary.
        PinClass::Invalid if sums.is_some() => m.push_str(&format!(
            "  To move the project to {binary}, replace `spec-dna` and `[spec-dna-sha256]` in spec-dna.toml with {DEFAULT_INSTALL_SOURCE}/releases/download/v{binary}/spec-dna-pin.toml and run `spec-dna init`."
        )),
        PinClass::Invalid => m.push_str(&format!(
            "  To move the project to {binary}, set `spec-dna = \"{binary}\"` in spec-dna.toml and run `spec-dna init`."
        )),
    }
    m
}

/// The pin mismatch message for a text that does not parse as a
/// configuration. It reads the install source and the sums from the TOML
/// table, if the text is valid TOML.
pub fn pin_mismatch_of_text(pin: &str, text: &str) -> String {
    let table: Table = text.parse().unwrap_or_default();
    let source = match table.get("install-source") {
        Some(Value::String(s)) if !s.trim().is_empty() && !s.contains(char::is_whitespace) => {
            s.as_str()
        }
        _ => DEFAULT_INSTALL_SOURCE,
    };
    let sums = sha256_table(table.get("spec-dna-sha256"), pin)
        .ok()
        .flatten();
    pin_mismatch_message(pin, source, sums.as_deref())
}

#[derive(Clone, Debug)]
pub struct Root {
    pub path: String,
    /// `None` for a mixed root.
    pub single_type: Option<DocType>,
    /// The type of a document that its front matter and its place give no
    /// type. A root of one type has it. A mixed root has it only if the file
    /// sets `default-type`: with none, such a document gives SD303.
    pub default_type: Option<DocType>,
    pub status: Option<String>,
    pub exclude: Vec<String>,
    pub legacy_ids: Vec<LegacyId>,
    pub legacy_keys: Vec<Pattern>,
    /// Legacy status words, each with the status that it maps to.
    pub status_map: Vec<(String, String)>,
    /// For each type (index `DocType as usize`), the headings of its required
    /// section. A document of that type needs one of them. An empty list means
    /// no required section.
    pub required_sections: [Vec<Vec<String>>; DocType::COUNT],
    /// For each type, the names of its amendment sections. A heading starts an
    /// amendment section if it is a name, or if it starts with a name and a
    /// character that is not a letter or a digit.
    pub amendment_sections: [Vec<String>; DocType::COUNT],
    /// If false, a document over the hard budget gives a warning, not an
    /// error.
    pub enforce_budgets: bool,
    /// The profile of this root, if it differs from the project profile.
    pub profile: Option<Profile>,
    /// Severity overrides for this root. They replace `[rules]`.
    pub rules: Rules,
    /// For each type, the names of its history sections (iteration logs,
    /// discussions, superseded drafts). They match as amendment sections do.
    /// `spec-dna history` moves them into a `research` document. In a
    /// decision, they are not locked text.
    pub history_sections: [Vec<String>; DocType::COUNT],
    /// For each type, the names of the sections that belong to the
    /// requirements, such as a coverage table. When requirements move, their
    /// rows move with them.
    pub companion_sections: [Vec<String>; DocType::COUNT],
    /// For each type, the names of its live sections, such as a
    /// progress section. They match as amendment sections do. In a
    /// decision, they are not locked text, and their text can change in
    /// place.
    pub live_sections: [Vec<String>; DocType::COUNT],
    /// For a root of type `research`: the namespace of its ids, a suffix of
    /// its path that no other research root has. Thus the ids of two
    /// research roots never collide.
    pub namespace: Option<String>,
}

/// The profiles (docs/profiles.md). A profile gives the default severity of
/// each diagnostic code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Profile {
    Lite,
    Standard,
    Strict,
}

impl Profile {
    pub const ALL: [Profile; 3] = [Profile::Lite, Profile::Standard, Profile::Strict];

    pub fn name(self) -> &'static str {
        match self {
            Profile::Lite => "lite",
            Profile::Standard => "standard",
            Profile::Strict => "strict",
        }
    }

    pub fn from_name(s: &str) -> Option<Profile> {
        Profile::ALL.into_iter().find(|p| p.name() == s)
    }

    /// The default severity of a code in this profile. `None` turns the code
    /// off. A code that is not in the table keeps the severity of its
    /// definition in every profile.
    pub fn severity(self, code: Code) -> Option<Severity> {
        use Severity::{Error as E, Warning as W};
        let (lite, standard, strict) = match code.code() {
            "SD101" | "SD102" | "SD103" | "SD104" | "SD105" | "SD106" | "SD201" | "SD202"
            | "SD203" | "SD204" | "SD702" => (Some(W), Some(E), Some(E)),
            "SD701" | "SD703" | "SD704" => (Some(W), Some(W), Some(E)),
            "SD606" | "SD607" | "SD608" => (None, Some(W), Some(E)),
            "SD801" | "SD802" | "SD803" | "SD804" | "SD805" => (None, None, Some(W)),
            "SD901" | "SD902" | "SD903" | "SD904" => (None, Some(W), Some(W)),
            "SD905" => (None, Some(W), Some(W)),
            "SD910" | "SD914" | "SD915" | "SD916" | "SD919" | "SD920" | "SD922" => {
                (Some(W), Some(E), Some(E))
            }
            "SD911" | "SD913" => (None, Some(W), Some(W)),
            "SD912" => (None, Some(W), Some(E)),
            "SD917" | "SD921" => (None, Some(W), Some(E)),
            "SD918" => (None, Some(W), Some(W)),
            _ => return Some(code.severity()),
        };
        match self {
            Profile::Lite => lite,
            Profile::Standard => standard,
            Profile::Strict => strict,
        }
    }
}

/// Severity overrides: a rule group or a diagnostic code, and its severity
/// (`None` is "off").
pub type Rules = Vec<(String, Option<Severity>)>;

/// The rule groups. A rule with a group name sets every code of the group.
pub const GROUPS: [(&str, &[&str]); 8] = [
    ("references", &["SD101", "SD102", "SD104", "SD106"]),
    ("ids", &["SD201", "SD202", "SD203", "SD204"]),
    ("budgets", &["SD701", "SD702", "SD703", "SD704"]),
    ("drift", &["SD606", "SD607"]),
    ("evidence", &["SD103", "SD105", "SD608"]),
    ("lint", &["SD801", "SD802", "SD803", "SD804", "SD805"]),
    ("layers", &["SD901", "SD902", "SD903", "SD904"]),
    (
        "changes",
        &[
            "SD905", "SD910", "SD911", "SD912", "SD913", "SD914", "SD915", "SD916", "SD917",
            "SD918", "SD919", "SD920", "SD921", "SD922",
        ],
    ),
];

/// True for a code of the group `changes`: SD905 and SD910 to SD922.
fn is_change_code(code: Code) -> bool {
    GROUPS
        .iter()
        .any(|g| g.0 == "changes" && g.1.contains(&code.code()))
}

/// Reads a severity value: `"off"` (`None`), `"warning"` or `"error"`.
fn severity_value(v: &Value, key: &str) -> Result<Option<Severity>, String> {
    match v.as_str() {
        Some("warning") => Ok(Some(Severity::Warning)),
        Some("error") => Ok(Some(Severity::Error)),
        Some("off") => Ok(None),
        _ => Err(format!("`{key}` must be \"off\", \"warning\" or \"error\"")),
    }
}

/// Reads a rules table: each key is a group name or a code.
fn rules_table(v: &Value, key: &str) -> Result<Rules, String> {
    let tbl = v
        .as_table()
        .ok_or_else(|| format!("`{key}` must be a table"))?;
    let mut out = Vec::new();
    for (name, sev) in tbl {
        if !GROUPS.iter().any(|g| g.0 == name) && Code::from_code(name).is_none() {
            let groups: Vec<&str> = GROUPS.iter().map(|g| g.0).collect();
            return Err(format!(
                "`{key}.{name}` is not a rule group ({}) or a diagnostic code",
                groups.join(", ")
            ));
        }
        out.push((name.clone(), severity_value(sev, &format!("{key}.{name}"))?));
    }
    Ok(out)
}

/// Returns the override of a code in one rules table. A code rule has priority
/// over a group rule.
fn rule_for(rules: &Rules, code: Code) -> Option<Option<Severity>> {
    if let Some((_, s)) = rules.iter().find(|r| r.0 == code.code()) {
        return Some(*s);
    }
    let group = GROUPS.iter().find(|g| g.1.contains(&code.code()))?.0;
    rules.iter().find(|r| r.0 == group).map(|r| r.1)
}

/// The default required section of a decision: its outcome.
pub const DEFAULT_DECISION_SECTIONS: [&str; 2] = ["Decision Outcome", "Decision"];

/// The default amendment section of a decision.
pub const DEFAULT_AMENDMENT_SECTIONS: [&str; 1] = ["Amendments"];

/// The live sections of a decision by default (design 0.6.0, "Templates").
/// Its text can change in place after the decision has the status `accepted`.
pub const DEFAULT_LIVE_SECTIONS: [&str; 1] = ["Status"];

#[derive(Clone, Debug)]
pub struct LegacyId {
    pub alias: String,
    pub file: Pattern,
}

impl LegacyId {
    /// Returns the alias for a document slug, if the slug matches.
    pub fn alias_for(&self, slug: &str) -> Option<String> {
        let n = self.file.capture(slug)?;
        let n = n.trim_start_matches('0');
        let n = if n.is_empty() { "0" } else { n };
        Some(self.alias.replace("{n}", n))
    }

    /// Returns the prefix of a `PREFIX-{n}` alias. spec-dna uses it to find
    /// bare tokens.
    pub fn token_prefix(&self) -> Option<&str> {
        let p = self.alias.strip_suffix("-{n}")?;
        (!p.is_empty()
            && p.bytes()
                .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit()))
        .then_some(p)
    }
}

/// A name pattern: literal text, `{n}` (one or more digits) and `*` (any text).
#[derive(Clone, Debug)]
pub struct Pattern {
    parts: Vec<Part>,
    /// The text of the pattern, as the configuration writes it.
    pub source: String,
}

#[derive(Clone, Debug)]
enum Part {
    Lit(String),
    Num,
    Star,
}

impl Pattern {
    pub fn new(source: &str) -> Result<Self, String> {
        let mut parts = Vec::new();
        let mut lit = String::new();
        let mut rest = source;
        while !rest.is_empty() {
            if let Some(r) = rest.strip_prefix("{n}") {
                if !lit.is_empty() {
                    parts.push(Part::Lit(std::mem::take(&mut lit)));
                }
                parts.push(Part::Num);
                rest = r;
            } else if let Some(r) = rest.strip_prefix('*') {
                if !lit.is_empty() {
                    parts.push(Part::Lit(std::mem::take(&mut lit)));
                }
                parts.push(Part::Star);
                rest = r;
            } else {
                let ch = rest.chars().next().unwrap();
                if ch == '{' || ch == '}' {
                    return Err(format!("pattern `{source}`: only `{{n}}` is a placeholder"));
                }
                lit.push(ch);
                rest = &rest[ch.len_utf8()..];
            }
        }
        if !lit.is_empty() {
            parts.push(Part::Lit(lit));
        }
        Ok(Pattern {
            parts,
            source: source.to_string(),
        })
    }

    pub fn is_match(&self, s: &str) -> bool {
        let mut cap = None;
        match_parts(&self.parts, s, &mut cap)
    }

    /// Returns the digits that the first `{n}` matches.
    pub fn capture(&self, s: &str) -> Option<String> {
        let mut cap = None;
        match_parts(&self.parts, s, &mut cap).then(|| cap.unwrap_or_default())
    }
}

fn match_parts(parts: &[Part], s: &str, cap: &mut Option<String>) -> bool {
    match parts.split_first() {
        None => s.is_empty(),
        Some((Part::Lit(l), rest)) => s
            .strip_prefix(l.as_str())
            .is_some_and(|r| match_parts(rest, r, cap)),
        Some((Part::Num, rest)) => {
            let digits = s.bytes().take_while(u8::is_ascii_digit).count();
            // Try the longest run first, so that `{n}*` keeps all digits.
            for len in (1..=digits).rev() {
                let mut c = cap.clone();
                if c.is_none() {
                    c = Some(s[..len].to_string());
                }
                if match_parts(rest, &s[len..], &mut c) {
                    *cap = c;
                    return true;
                }
            }
            false
        }
        Some((Part::Star, rest)) => s
            .char_indices()
            .map(|(i, _)| i)
            .chain([s.len()])
            .any(|i| match_parts(rest, &s[i..], cap)),
    }
}

/// The message for a type name that is not a type. For a type name of 0.5,
/// it names the new type. `key` is the key that holds the name.
pub(crate) fn unknown_type(key: &str, name: &str) -> String {
    format!(
        "{key}: unknown type `{name}`; {}",
        DocType::unknown_hint(name)
    )
}

pub const DEFAULT_BUDGETS: [(u32, u32); DocType::COUNT] = [
    (8_000, 16_000),  // product
    (8_000, 16_000),  // system
    (10_000, 20_000), // terms
    (8_000, 16_000),  // roadmap
    (8_000, 16_000),  // questions
    (3_000, 6_000),   // rule: `context` serves it first, so it stays short
    (10_000, 20_000), // requirements
    (10_000, 20_000), // interface
    (7_000, 14_000),  // decision
    (14_000, 28_000), // procedure
    (6_000, 12_000),  // incident
    (0, 0),           // research: cold material, no budget
    (6_000, 12_000),  // intent
    (10_000, 20_000), // spec
    (6_000, 12_000),  // plan
];

/// The default required sections of each type: a list of groups. A document
/// needs one heading of each group.
pub fn default_required_sections() -> [Vec<Vec<String>>; DocType::COUNT] {
    let mut out: [Vec<Vec<String>>; DocType::COUNT] = Default::default();
    let group = |h: &[&str]| h.iter().map(|s| s.to_string()).collect::<Vec<String>>();
    out[DocType::Decision as usize] = vec![group(&DEFAULT_DECISION_SECTIONS)];
    out[DocType::Rule as usize] = vec![group(&["Rules"])];
    out[DocType::Incident as usize] = vec![group(&["Summary"]), group(&["Root cause"])];
    out[DocType::Intent as usize] = vec![group(&["Intent", "Problem", "Summary"])];
    out[DocType::Spec as usize] = vec![group(&["Delta"])];
    out[DocType::Plan as usize] = vec![group(&["Files"])];
    out
}

/// The default globs of test files, for `covers:` tags. They cover the usual
/// test directories and file names of common stacks.
pub const DEFAULT_TEST_GLOBS: [&str; 10] = [
    "**/tests/**",
    "**/test/**",
    "**/__tests__/**",
    "**/*_test.*",
    "**/*_tests.*",
    "**/test_*.*",
    "**/*.test.*",
    "**/*.spec.*",
    "**/*Test.*",
    "**/*_spec.*",
];

impl Config {
    pub fn parse(text: &str) -> Result<Config, String> {
        let table: Table = text
            .parse()
            .map_err(|e: toml::de::Error| e.to_string().trim().to_string())?;
        let mut t = Fields::new(&table, "");
        let pin = t.req_str("spec-dna")?;
        let exclude = t.opt_strs("exclude")?;
        let mut roots = Vec::new();
        if let Some(v) = t.take("roots") {
            let arr = v
                .as_array()
                .ok_or("`roots` must be an array of tables (`[[roots]]`)")?;
            for (i, item) in arr.iter().enumerate() {
                let tbl = item
                    .as_table()
                    .ok_or("`roots` must be an array of tables (`[[roots]]`)")?;
                roots.push(Root::parse(tbl, &format!("roots[{i}].")).map_err(|e| e.to_string())?);
            }
        }
        if roots.is_empty() {
            return Err("at least one `[[roots]]` table is required".into());
        }
        for (i, a) in roots.iter().enumerate() {
            if roots[..i].iter().any(|b| b.path == a.path) {
                return Err(format!("root `{}` is listed twice", a.path));
            }
        }
        set_namespaces(&mut roots);
        let mut budgets = DEFAULT_BUDGETS;
        let mut growth = DEFAULT_GROWTH;
        if let Some(v) = t.take("budgets") {
            let tbl = v.as_table().ok_or("`budgets` must be a table")?;
            let mut b = Fields::new(tbl, "budgets.");
            growth = b.opt_u32("growth")?.unwrap_or(growth);
            for name in tbl.keys() {
                if DocType::from_name(name).is_none() && DocType::from_old_name(name).is_some() {
                    return Err(unknown_type(&format!("`budgets.{name}`"), name));
                }
            }
            for ty in DocType::ALL {
                if let Some(v) = b.take(ty.name()) {
                    let tbl = v
                        .as_table()
                        .ok_or_else(|| format!("`budgets.{}` must be a table", ty.name()))?;
                    let mut f = Fields::new(tbl, &format!("budgets.{}.", ty.name()));
                    let (soft, hard) = budgets[ty as usize];
                    let soft = f.opt_u32("soft")?.unwrap_or(soft);
                    let hard = f.opt_u32("hard")?.unwrap_or(hard);
                    if hard < soft {
                        return Err(format!("`budgets.{}`: `hard` is below `soft`", ty.name()));
                    }
                    budgets[ty as usize] = (soft, hard);
                    f.done()?;
                }
            }
            b.done()?;
        }
        let mut max_words = 30;
        let mut history_terms = None;
        if let Some(v) = t.take("lint") {
            let tbl = v.as_table().ok_or("`lint` must be a table")?;
            let mut f = Fields::new(tbl, "lint.");
            max_words = f.opt_u32("max-words")?.unwrap_or(max_words);
            if f.table.contains_key("history-terms") {
                let terms = f.opt_strs("history-terms")?;
                for (i, term) in terms.iter().enumerate() {
                    if term.trim().is_empty() {
                        return Err("`lint.history-terms` has an empty phrase".into());
                    }
                    if terms[..i].contains(term) {
                        return Err(format!(
                            "`lint.history-terms` lists `{term}` more than once"
                        ));
                    }
                }
                history_terms = Some(terms);
            }
            f.done()?;
        }
        // 8,000 characters (about 2,000 tokens): every later request of an
        // agent carries the output again.
        let mut context_budget = 8_000;
        if let Some(v) = t.take("context") {
            let tbl = v.as_table().ok_or("`context` must be a table")?;
            let mut f = Fields::new(tbl, "context.");
            context_budget = f.opt_u32("budget")?.unwrap_or(context_budget);
            f.done()?;
        }
        let mut repos = Vec::new();
        if let Some(v) = t.take("repos") {
            let tbl = v.as_table().ok_or("`repos` must be a table")?;
            for (k, v) in tbl {
                let s = v
                    .as_str()
                    .ok_or_else(|| format!("`repos.{k}` must be a string"))?;
                repos.push((k.clone(), s.to_string()));
            }
        }
        let self_urls = t.opt_strs("self-urls")?;
        for u in &self_urls {
            let scheme = u.starts_with("https://") || u.starts_with("http://");
            // `*` is a whole path segment after the host, never the host.
            let stars_ok = u.match_indices('*').all(|(i, _)| {
                u[..i].ends_with('/')
                    && u[..i].matches('/').count() >= 3
                    && u[i + 1..].starts_with('/')
            });
            if !scheme || !u.ends_with('/') || !stars_ok {
                return Err(format!(
                    "`self-urls`: `{u}` must be an http(s) URL that ends in `/`, with `*` only as a whole path segment"
                ));
            }
        }
        let install_source = match t.take("install-source") {
            None => DEFAULT_INSTALL_SOURCE.to_string(),
            Some(Value::String(s)) if !s.trim().is_empty() && !s.contains(char::is_whitespace) => {
                s.clone()
            }
            Some(_) => {
                return Err(
                    "`install-source` must be a git URL without spaces, such as `file:///srv/git/spec-dna` or `ssh://git@host/team/spec-dna.git`".into(),
                );
            }
        };
        let sha256 = sha256_table(t.take("spec-dna-sha256"), &pin)?;
        let profile = match t.opt_str("profile")? {
            None => Profile::Standard,
            Some(p) => Profile::from_name(&p).ok_or_else(|| {
                format!("`profile` must be \"lite\", \"standard\" or \"strict\", not `{p}`")
            })?,
        };
        let rules = match t.take("rules") {
            Some(v) => rules_table(v, "rules")?,
            None => Vec::new(),
        };
        let mut types = Vec::new();
        for name in t.opt_strs("types")? {
            let ty = DocType::from_name(&name).ok_or_else(|| unknown_type("`types`", &name))?;
            if !types.contains(&ty) {
                types.push(ty);
            }
        }
        let mut evidence_tests = DEFAULT_TEST_GLOBS.map(str::to_string).to_vec();
        if let Some(v) = t.take("evidence") {
            let tbl = v.as_table().ok_or("`evidence` must be a table")?;
            let mut f = Fields::new(tbl, "evidence.");
            if f.table.contains_key("tests") {
                evidence_tests = f.opt_strs("tests")?;
            }
            f.done()?;
        }
        let mut conventions: [Vec<String>; DocType::COUNT] = Default::default();
        if let Some(v) = t.take("conventions") {
            let tbl = v.as_table().ok_or("`conventions` must be a table")?;
            for (name, list) in tbl {
                let ty = DocType::from_name(name)
                    .ok_or_else(|| unknown_type(&format!("`conventions.{name}`"), name))?;
                let ptrs = list
                    .as_array()
                    .and_then(|a| {
                        a.iter()
                            .map(|p| p.as_str().map(str::to_string))
                            .collect::<Option<Vec<_>>>()
                    })
                    .ok_or_else(|| format!("`conventions.{name}` must be an array of strings"))?;
                if ptrs.iter().any(|p| p.trim().is_empty()) {
                    return Err(format!("`conventions.{name}` has an empty pointer"));
                }
                conventions[ty as usize] = ptrs;
            }
        }
        let harness = match t.take("harness") {
            Some(v) => Some(Harness::parse(v)?),
            None => None,
        };
        let (layers, layers_declared) = match t.take("layers") {
            Some(v) => {
                let tbl = v.as_table().ok_or("`layers` must be a table")?;
                (layer::parse_table(tbl, text)?, true)
            }
            None => (Graph::implied(&types), false),
        };
        let areas = areas_list(&t.opt_strs("areas")?)?;
        let groups = match t.take("groups") {
            Some(v) => groups_table(v, &areas)?,
            None => Vec::new(),
        };
        let templates_dir = match t.take("templates") {
            Some(v) => {
                let tbl = v.as_table().ok_or("`templates` must be a table")?;
                let mut f = Fields::new(tbl, "templates.");
                let dir = f.opt_str("dir")?;
                f.done()?;
                match dir {
                    Some(d) => templates_dir_value(&d)?,
                    None => DEFAULT_TEMPLATES_DIR.to_string(),
                }
            }
            None => DEFAULT_TEMPLATES_DIR.to_string(),
        };
        let evals = match t.take("evals") {
            Some(v) => Some(Evals::parse(v, &templates_dir)?),
            None => None,
        };
        let language = match t.opt_str("language")? {
            None => DEFAULT_LANGUAGE.to_string(),
            Some(l) if is_language_tag(&l) => l,
            Some(l) => {
                return Err(format!(
                    "`language` must be a BCP 47 language tag, such as `en`, `en-GB` or `de`, not `{l}`"
                ));
            }
        };
        let change = match t.take("change") {
            None => ChangeTable::absent(&layers),
            Some(v) => {
                let tbl = v.as_table().ok_or("`change` must be a table")?;
                ChangeTable::parse(tbl, &layers)?
            }
        };
        t.done()?;
        // The hook `protect-tests` reads the globs of the base, so the
        // harness keeps them.
        let harness = harness.map(|mut h| {
            h.tests = evidence_tests.clone();
            h
        });
        Ok(Config {
            pin,
            force_lint: false,
            exclude,
            roots,
            budgets,
            max_words,
            context_budget,
            repos,
            self_urls,
            install_source,
            sha256,
            profile,
            rules,
            evidence_tests,
            types,
            growth,
            conventions,
            harness,
            layers,
            layers_declared,
            areas,
            groups,
            templates_dir,
            evals,
            language,
            history_terms,
            change_policy: change.policy,
            change_gate: change.gate,
            high_risk_layers: change.high_risk_layers,
            high_risk_paths: change.high_risk_paths,
            max_fanout: change.max_fanout,
        })
    }

    /// True if the primary subtag of `language` is `en`. The word lists of the
    /// prose lint and of SD904 are English.
    pub fn language_is_english(&self) -> bool {
        self.language
            .split('-')
            .next()
            .is_some_and(|p| p.eq_ignore_ascii_case("en"))
    }

    /// Returns the severity of a code for a file of a root. It returns `None`
    /// for a file outside every root, or if the code is off. The order is: the
    /// rules of the root, `[rules]`, the profile of the root, the profile of
    /// the project.
    pub fn severity(&self, code: Code, root: Option<usize>) -> Option<Severity> {
        let found = self.configured_severity(code, root);
        if found.is_none() && self.force_lint && code.is_prose_lint() && root.is_some() {
            return Some(Severity::Warning);
        }
        found
    }

    fn configured_severity(&self, code: Code, root: Option<usize>) -> Option<Severity> {
        // With no change policy, the codes of the group `changes` have no
        // meaning. No rule of the file can turn them on.
        if is_change_code(code) && self.change_policy == ChangePolicy::None {
            return None;
        }
        let root = root.map(|r| &self.roots[r]);
        if let Some(s) = root.and_then(|r| rule_for(&r.rules, code)) {
            return s;
        }
        if let Some(s) = rule_for(&self.rules, code) {
            return s;
        }
        // SD910 follows the gate, and the gate follows no profile.
        if code.code() == "SD910"
            && let Some(g) = self.change_gate
        {
            return g.severity();
        }
        root.and_then(|r| r.profile)
            .unwrap_or(self.profile)
            .severity(code)
    }

    /// Returns true if a code is off in every root and outside the roots.
    pub fn is_off(&self, code: Code) -> bool {
        self.severity(code, None).is_none()
            && (0..self.roots.len()).all(|r| self.severity(code, Some(r)).is_none())
    }

    /// Returns true if the project uses a document type (`types`).
    pub fn allows(&self, ty: DocType) -> bool {
        self.types.is_empty() || self.types.contains(&ty)
    }

    /// Returns true if a path is a test file for `covers:` tags.
    pub fn is_test_file(&self, path: &str) -> bool {
        self.evidence_tests
            .iter()
            .any(|g| crate::glob::matches(g, path))
    }

    /// Returns the root of a repository path: the deepest root that contains
    /// the path.
    pub fn root_of(&self, path: &str) -> Option<usize> {
        let mut best: Option<(usize, usize)> = None;
        for (i, r) in self.roots.iter().enumerate() {
            let inside = r.path.is_empty()
                || path.starts_with(&r.path) && path.as_bytes().get(r.path.len()) == Some(&b'/');
            if inside && best.is_none_or(|(_, len)| r.path.len() > len) {
                best = Some((i, r.path.len()));
            }
        }
        best.map(|(i, _)| i)
    }

    /// Returns true if a path is in `[templates] dir`.
    pub fn in_templates_dir(&self, path: &str) -> bool {
        path.strip_prefix(&self.templates_dir)
            .is_some_and(|rest| rest.starts_with('/'))
    }

    /// Returns true if a path is in `[evals] dir`.
    pub fn in_evals_dir(&self, path: &str) -> bool {
        self.evals.as_ref().is_some_and(|e| {
            path.strip_prefix(&e.dir)
                .is_some_and(|rest| rest.starts_with('/'))
        })
    }

    /// True for a document that the harness configuration writes: `REVIEW.md`
    /// and a stage skill (step 4, D44).
    fn is_harness_doc(&self, path: &str) -> bool {
        self.harness.as_ref().is_some_and(|h| h.writes_doc(path))
    }

    pub fn is_excluded(&self, root: usize, path: &str) -> bool {
        let rel = self.roots[root].rel(path);
        self.in_templates_dir(path)
            || self.in_evals_dir(path)
            || self.is_harness_doc(path)
            || self.exclude.iter().any(|g| crate::glob::matches(g, path))
            || self.roots[root]
                .exclude
                .iter()
                .any(|g| crate::glob::matches(g, rel))
    }

    /// Returns the Markdown files that a pointer of `conventions` names by
    /// path and that are not documents. These files are outside every root, or
    /// an exclude glob removes them. spec-dna reads these files only to serve
    /// the conventions (`context`, `show --conventions`) and to check the
    /// pointers (SD106).
    pub fn convention_files(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .conventions
            .iter()
            .flatten()
            .map(|p| p.split('#').next().unwrap_or(p).trim_start_matches("./"))
            // A path that leaves the repository would break `git status`
            // (`root_paths`): such a pointer resolves to nothing (SD106).
            .filter(|p| crate::model::is_doc_path(p) && !p.starts_with('/') && !p.contains('\\'))
            .filter(|p| !p.split('/').any(|s| s == ".." || s == "." || s.is_empty()))
            .filter(|p| self.root_of(p).is_none_or(|r| self.is_excluded(r, p)))
            .map(str::to_string)
            .collect();
        v.sort();
        v.dedup();
        v
    }

    /// Returns the prefixes of legacy `PREFIX-{n}` aliases, for bare-token
    /// checks.
    pub fn token_prefixes(&self) -> Vec<String> {
        let mut v: Vec<String> = self
            .roots
            .iter()
            .flat_map(|r| {
                r.legacy_ids
                    .iter()
                    .filter_map(|l| l.token_prefix().map(str::to_string))
            })
            .collect();
        v.sort();
        v.dedup();
        v
    }
}

impl Root {
    fn parse(tbl: &Table, prefix: &str) -> Result<Root, String> {
        let mut f = Fields::new(tbl, prefix);
        let path = f.req_str("path")?;
        let path = path.trim_matches('/').to_string();
        let path = if path == "." { String::new() } else { path };
        if path.split('/').any(|s| s == ".." || s == ".") {
            return Err(format!(
                "`{prefix}path` must be a plain path inside the repository"
            ));
        }
        let ty = f.opt_str("type")?.unwrap_or_else(|| "mixed".into());
        let single_type = if ty == "mixed" {
            None
        } else {
            let t = DocType::from_name(&ty)
                .ok_or_else(|| unknown_type(&format!("`{prefix}type`"), &ty))?;
            if t.is_intent_type() {
                return Err(format!("`{prefix}type`: {}", t.intent_only_hint()));
            }
            Some(t)
        };
        let default_type = match f.opt_str("default-type")? {
            Some(d) => {
                if single_type.is_some() {
                    return Err(format!(
                        "`{prefix}default-type` is only for `type = \"mixed\"`"
                    ));
                }
                let t = DocType::from_name(&d)
                    .ok_or_else(|| unknown_type(&format!("`{prefix}default-type`"), &d))?;
                if t.is_intent_type() {
                    return Err(format!("`{prefix}default-type`: {}", t.intent_only_hint()));
                }
                Some(t)
            }
            None => single_type,
        };
        let status = f.opt_str("status")?;
        if let Some(s) = &status {
            let Some(dt) = default_type else {
                return Err(format!(
                    "`{prefix}status` needs `{prefix}default-type`: the status is for the documents of that type"
                ));
            };
            if !dt.statuses().contains(&s.as_str()) {
                return Err(format!(
                    "`{prefix}status`: `{s}` is not a status of type `{}`",
                    dt.name()
                ));
            }
        }
        let exclude = f.opt_strs("exclude")?;
        let mut legacy_ids = Vec::new();
        if let Some(v) = f.take("legacy-ids") {
            let arr = v
                .as_array()
                .ok_or_else(|| format!("`{prefix}legacy-ids` must be an array"))?;
            for item in arr {
                let (alias, file) = match item {
                    Value::String(s) => (s.clone(), format!("{s}*")),
                    Value::Table(t) => {
                        let mut lf = Fields::new(t, &format!("{prefix}legacy-ids."));
                        let alias = lf.req_str("alias")?;
                        let file = lf.opt_str("file")?.unwrap_or_else(|| format!("{alias}*"));
                        lf.done()?;
                        (alias, file)
                    }
                    _ => return Err(format!("`{prefix}legacy-ids` items are strings or tables")),
                };
                if alias.matches("{n}").count() != 1 || file.matches("{n}").count() != 1 {
                    return Err(format!(
                        "`{prefix}legacy-ids`: `{alias}` and its file pattern need exactly one `{{n}}`"
                    ));
                }
                legacy_ids.push(LegacyId {
                    file: Pattern::new(&file)?,
                    alias,
                });
            }
        }
        let legacy_keys = f
            .opt_strs("legacy-keys")?
            .iter()
            .map(|s| Pattern::new(s))
            .collect::<Result<Vec<_>, _>>()?;
        let types: Vec<DocType> = match single_type {
            Some(t) => vec![t],
            None => DocType::ALL.to_vec(),
        };
        let mut status_map = Vec::new();
        if let Some(v) = f.take("status-map") {
            let tbl = v
                .as_table()
                .ok_or_else(|| format!("`{prefix}status-map` must be a table"))?;
            for (from, to) in tbl {
                let to = to
                    .as_str()
                    .ok_or_else(|| format!("`{prefix}status-map.{from}` must be a string"))?;
                // Do not map a word that every type of the root already allows.
                if types.iter().all(|t| t.statuses().contains(&from.as_str())) {
                    return Err(format!(
                        "`{prefix}status-map.{from}`: `{from}` is already a status"
                    ));
                }
                if !types.iter().any(|t| t.statuses().contains(&to)) {
                    return Err(format!(
                        "`{prefix}status-map.{from}`: `{to}` is not a status of {}",
                        match single_type {
                            Some(t) => format!("type `{}`", t.name()),
                            None => "any type".into(),
                        }
                    ));
                }
                status_map.push((from.clone(), to.to_string()));
            }
        }
        let mut required_sections = default_required_sections();
        if let Some(v) = f.take("required-sections") {
            section_groups_per_type(
                v,
                &format!("{prefix}required-sections"),
                &types,
                &mut required_sections,
            )?;
        }
        let mut amendment_sections: [Vec<String>; DocType::COUNT] = Default::default();
        amendment_sections[DocType::Decision as usize] =
            DEFAULT_AMENDMENT_SECTIONS.map(str::to_string).to_vec();
        if let Some(v) = f.take("amendment-sections") {
            headings_per_type(
                v,
                &format!("{prefix}amendment-sections"),
                &types,
                &mut amendment_sections,
            )?;
        }
        let mut history_sections: [Vec<String>; DocType::COUNT] = Default::default();
        if let Some(v) = f.take("history-sections") {
            headings_per_type(
                v,
                &format!("{prefix}history-sections"),
                &types,
                &mut history_sections,
            )?;
        }
        let mut companion_sections: [Vec<String>; DocType::COUNT] = Default::default();
        if let Some(v) = f.take("companion-sections") {
            headings_per_type(
                v,
                &format!("{prefix}companion-sections"),
                &types,
                &mut companion_sections,
            )?;
        }
        let mut live_sections: [Vec<String>; DocType::COUNT] = Default::default();
        live_sections[DocType::Decision as usize] =
            DEFAULT_LIVE_SECTIONS.map(str::to_string).to_vec();
        if let Some(v) = f.take("live-sections") {
            headings_per_type(
                v,
                &format!("{prefix}live-sections"),
                &types,
                &mut live_sections,
            )?;
        }
        let enforce_budgets = match f.take("enforce-budgets") {
            None => true,
            Some(Value::Boolean(b)) => *b,
            Some(_) => return Err(format!("`{prefix}enforce-budgets` must be true or false")),
        };
        let profile = match f.opt_str("profile")? {
            None => None,
            Some(p) => Some(Profile::from_name(&p).ok_or_else(|| {
                format!("`{prefix}profile` must be \"lite\", \"standard\" or \"strict\", not `{p}`")
            })?),
        };
        let mut rules = match f.take("rules") {
            Some(v) => rules_table(v, &format!("{prefix}rules"))?,
            None => Vec::new(),
        };
        // `lint-severity` and `drift-severity` are the rules `lint` and
        // `drift` of 0.1 configurations. A rule in `rules` has priority.
        for (key, group) in [("lint-severity", "lint"), ("drift-severity", "drift")] {
            if let Some(v) = f.take(key) {
                let s = severity_value(v, &format!("{prefix}{key}"))?;
                if !rules.iter().any(|r| r.0 == group) {
                    rules.push((group.to_string(), s));
                }
            }
        }
        f.done()?;
        Ok(Root {
            path,
            single_type,
            default_type,
            status,
            exclude,
            legacy_ids,
            legacy_keys,
            status_map,
            required_sections,
            amendment_sections,
            enforce_budgets,
            profile,
            rules,
            history_sections,
            companion_sections,
            live_sections,
            namespace: None,
        })
    }

    /// Returns the path relative to this root.
    pub fn rel<'a>(&self, path: &'a str) -> &'a str {
        if self.path.is_empty() {
            path
        } else {
            &path[self.path.len() + 1..]
        }
    }
}

/// Checks the value of `[templates] dir`.
fn templates_dir_value(d: &str) -> Result<String, String> {
    dir_value("templates.dir", d, ".spec-dna/templates")
}

/// Checks the value of a key that names a folder (`[templates] dir`,
/// `[evals] dir`) and returns it in the form that the code compares: no `./`
/// at the start, no `/` at the end.
fn dir_value(key: &str, d: &str, example: &str) -> Result<String, String> {
    let bad = |why: &str| {
        Err(format!(
            "`{key}`: `{d}` {why}; write a folder inside the repository, such as `{example}`"
        ))
    };
    let mut dir = d.trim();
    while let Some(rest) = dir.strip_prefix("./") {
        dir = rest;
    }
    let dir = dir.trim_end_matches('/');
    if dir.is_empty() || dir == "." {
        return bad("is empty or is the repository root");
    }
    if dir.starts_with('/') || dir.contains('\\') || dir.contains(':') {
        return bad("is not a relative path with `/` separators");
    }
    if dir
        .split('/')
        .any(|s| s.is_empty() || s == "." || s == "..")
    {
        return bad("has an empty, `.` or `..` segment");
    }
    Ok(dir.to_string())
}

/// Gives each root of type `research` a namespace: the shortest suffix of
/// its path (whole segments) that no other research root has. A research
/// root at the repository root has no namespace.
fn set_namespaces(roots: &mut [Root]) {
    let paths: Vec<Option<Vec<String>>> = roots
        .iter()
        .map(|r| {
            (r.single_type == Some(DocType::Research) && !r.path.is_empty())
                .then(|| r.path.split('/').map(str::to_string).collect())
        })
        .collect();
    let suffix = |segs: &[String], n: usize| segs[segs.len().saturating_sub(n)..].join("/");
    for (i, own) in paths.iter().enumerate() {
        let Some(own) = own else { continue };
        let n = (1..=own.len())
            .find(|&n| {
                paths.iter().enumerate().all(|(j, o)| {
                    j == i || o.as_ref().is_none_or(|o| suffix(o, n) != suffix(own, n))
                })
            })
            .unwrap_or(own.len());
        roots[i].namespace = Some(suffix(own, n));
    }
}

/// Checks `areas`: each area is one plain folder name, once.
fn areas_list(areas: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for a in areas {
        if a.is_empty() || a == "." || a == ".." || a.contains(['/', '\\']) || a.trim() != a {
            return Err(format!(
                "`areas`: `{a}` must be the name of one folder of `requirements/`"
            ));
        }
        if out.contains(a) {
            return Err(format!("`areas`: `{a}` is listed twice"));
        }
        out.push(a.clone());
    }
    Ok(out)
}

/// Reads `[groups]`: each key is a group, and its value is a list of areas
/// that `areas` has.
fn groups_table(v: &Value, areas: &[String]) -> Result<Vec<(String, Vec<String>)>, String> {
    let tbl = v.as_table().ok_or("`groups` must be a table")?;
    let mut out = Vec::new();
    for (name, list) in tbl {
        let key = format!("groups.{name}");
        let items = list
            .as_array()
            .and_then(|a| {
                a.iter()
                    .map(|p| p.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or_else(|| format!("`{key}` must be an array of strings"))?;
        for a in &items {
            if !areas.contains(a) {
                return Err(format!(
                    "`{key}`: `{a}` is not in `areas`; a group names only areas of `areas`"
                ));
            }
        }
        let mut uniq: Vec<String> = Vec::new();
        for a in items {
            if !uniq.contains(&a) {
                uniq.push(a);
            }
        }
        out.push((name.clone(), uniq));
    }
    Ok(out)
}

/// Reads `required-sections` into `out`. For each type, the value is a list
/// of headings (one required section, with alternative headings) or a list of
/// such lists (several required sections). A value replaces the default of
/// its type.
fn section_groups_per_type(
    v: &Value,
    key: &str,
    types: &[DocType],
    out: &mut [Vec<Vec<String>>; DocType::COUNT],
) -> Result<(), String> {
    let tbl = v
        .as_table()
        .ok_or_else(|| format!("`{key}` must be a table"))?;
    for (name, list) in tbl {
        let key = format!("{key}.{name}");
        let ty = match DocType::from_name(name) {
            Some(t) if types.contains(&t) => t,
            Some(_) => return Err(format!("`{key}`: `{name}` is not a type of this root")),
            None => return Err(unknown_type(&format!("`{key}`"), name)),
        };
        let items = list
            .as_array()
            .ok_or_else(|| format!("`{key}` must be an array of strings or of arrays"))?;
        let strs = |a: &Vec<Value>| -> Option<Vec<String>> {
            a.iter().map(|h| h.as_str().map(str::to_string)).collect()
        };
        let groups: Vec<Vec<String>> = if items.iter().all(|i| i.is_str()) {
            if items.is_empty() {
                Vec::new()
            } else {
                vec![strs(items).unwrap_or_default()]
            }
        } else {
            items
                .iter()
                .map(|i| i.as_array().and_then(strs).filter(|g| !g.is_empty()))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    format!("`{key}` must be an array of strings or an array of non-empty arrays of strings")
                })?
        };
        if groups.iter().flatten().any(|h| h.trim().is_empty()) {
            return Err(format!("`{key}` has an empty heading"));
        }
        out[ty as usize] = groups;
    }
    Ok(())
}

/// Reads a table of heading lists per type, such as `amendment-sections`,
/// into `out`. A list replaces the default of its type.
fn headings_per_type(
    v: &Value,
    key: &str,
    types: &[DocType],
    out: &mut [Vec<String>; DocType::COUNT],
) -> Result<(), String> {
    let tbl = v
        .as_table()
        .ok_or_else(|| format!("`{key}` must be a table"))?;
    for (name, list) in tbl {
        let key = format!("{key}.{name}");
        let ty = match DocType::from_name(name) {
            Some(t) if types.contains(&t) => t,
            Some(_) => return Err(format!("`{key}`: `{name}` is not a type of this root")),
            None => return Err(unknown_type(&format!("`{key}`"), name)),
        };
        let headings = list
            .as_array()
            .and_then(|a| {
                a.iter()
                    .map(|h| h.as_str().map(str::to_string))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or_else(|| format!("`{key}` must be an array of strings"))?;
        if headings.iter().any(|h| h.trim().is_empty()) {
            return Err(format!("`{key}` has an empty heading"));
        }
        out[ty as usize] = headings;
    }
    Ok(())
}

/// Reads keys from a table and rejects unknown ones.
pub(crate) struct Fields<'a> {
    table: &'a Table,
    used: Vec<&'a str>,
    prefix: String,
}

impl<'a> Fields<'a> {
    pub(crate) fn new(table: &'a Table, prefix: &str) -> Self {
        Fields {
            table,
            used: Vec::new(),
            prefix: prefix.to_string(),
        }
    }

    pub(crate) fn take(&mut self, key: &'a str) -> Option<&'a Value> {
        self.used.push(key);
        self.table.get(key)
    }

    pub(crate) fn opt_str(&mut self, key: &'a str) -> Result<Option<String>, String> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(format!("`{}{key}` must be a string", self.prefix)),
        }
    }

    pub(crate) fn req_str(&mut self, key: &'a str) -> Result<String, String> {
        self.opt_str(key)?
            .ok_or_else(|| format!("`{}{key}` is required", self.prefix))
    }

    pub(crate) fn opt_strs(&mut self, key: &'a str) -> Result<Vec<String>, String> {
        match self.take(key) {
            None => Ok(Vec::new()),
            Some(Value::Array(a)) => a
                .iter()
                .map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .ok_or_else(|| format!("`{}{key}` must hold strings", self.prefix))
                })
                .collect(),
            Some(_) => Err(format!(
                "`{}{key}` must be an array of strings",
                self.prefix
            )),
        }
    }

    pub(crate) fn opt_u32(&mut self, key: &'a str) -> Result<Option<u32>, String> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Integer(n)) if *n >= 0 && *n <= u32::MAX as i64 => Ok(Some(*n as u32)),
            Some(_) => Err(format!(
                "`{}{key}` must be a non-negative integer",
                self.prefix
            )),
        }
    }

    pub(crate) fn done(self) -> Result<(), String> {
        for k in self.table.keys() {
            if !self.used.contains(&k.as_str()) {
                return Err(format!("unknown key `{}{k}`", self.prefix));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn convention_files_are_the_paths_outside_the_documents() {
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\n[conventions]\ninterface = [\"docs/README.md#a\", \"./CONTRIBUTING.md\", \"docs/x.md#b\", \"token\", \"../up.md\", \"/abs.md\", \"a\\\\b.md\", \"notes.txt\"]\ndecision = [\"CONTRIBUTING.md#c\"]\n[[roots]]\npath = \"docs\"\nexclude = [\"README.md\"]\n",
        )
        .unwrap();
        // docs/x.md is a document. An id, a non-Markdown file and a path that
        // leaves the repository are not convention files.
        assert_eq!(
            c.convention_files(),
            vec!["CONTRIBUTING.md".to_string(), "docs/README.md".to_string()]
        );
    }

    const FULL: &str = r#"
spec-dna = "0.1.0"
exclude = ["**/vendor/**"]

[[roots]]
path = "docs"
type = "mixed"
default-type = "requirements"
status = "draft"
legacy-keys = ["R-{n}"]

[[roots]]
path = "docs/adr"
type = "decision"
legacy-ids = ["ADR-{n}", { alias = "KEP-{n}", file = "{n}-*" }]

[budgets.requirements]
soft = 100
hard = 200

[lint]
max-words = 25

[context]
budget = 9000
"#;

    #[test]
    fn parses_a_full_config() {
        let c = Config::parse(FULL).unwrap();
        assert_eq!(c.pin, "0.1.0");
        assert_eq!(c.roots.len(), 2);
        assert_eq!(c.roots[0].default_type, Some(DocType::Requirements));
        assert_eq!(c.roots[1].single_type, Some(DocType::Decision));
        assert_eq!(c.budgets[DocType::Requirements as usize], (100, 200));
        assert_eq!(c.max_words, 25);
        assert_eq!(c.context_budget, 9000);
        assert_eq!(c.root_of("docs/adr/ADR-1-x.md"), Some(1));
        assert_eq!(c.root_of("docs/a.md"), Some(0));
        assert_eq!(c.root_of("other/a.md"), None);
        assert!(c.is_excluded(0, "docs/vendor/x.md"));
        assert_eq!(
            c.token_prefixes(),
            vec!["ADR".to_string(), "KEP".to_string()]
        );
    }

    #[test]
    fn legacy_patterns() {
        let c = Config::parse(FULL).unwrap();
        let l = &c.roots[1].legacy_ids;
        assert_eq!(l[0].alias_for("ADR-012-cache").as_deref(), Some("ADR-12"));
        assert_eq!(l[0].alias_for("cache"), None);
        assert_eq!(
            l[1].alias_for("1205-bound-tokens").as_deref(),
            Some("KEP-1205")
        );
        assert!(c.roots[0].legacy_keys[0].is_match("R-12"));
        assert!(!c.roots[0].legacy_keys[0].is_match("R-x"));
    }

    #[test]
    fn rejects_unknown_keys_and_bad_values() {
        assert!(
            Config::parse(
                "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\ntyp = \"requirements\"\n"
            )
            .unwrap_err()
            .contains("roots[0].typ")
        );
        assert!(
            Config::parse("spec-dna = \"0.1.0\"\n")
                .unwrap_err()
                .contains("roots")
        );
        assert!(
            Config::parse("[[roots]]\npath = \"d\"\n")
                .unwrap_err()
                .contains("spec-dna")
        );
        assert!(Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\ntype = \"requirements\"\nstatus = \"accepted\"\n").is_err());
    }

    #[test]
    fn status_map_required_sections_and_self_urls() {
        let c = Config::parse(
            r#"
spec-dna = "0.1.0"
self-urls = ["https://example.org/acme/specs/blob/*/"]
[[roots]]
path = "docs"
status-map = { released = "active", wip = "draft" }
required-sections = { decision = ["Lösung"], requirements = ["Requirements", "Anforderungen"] }
enforce-budgets = false
[[roots]]
path = "adr"
type = "decision"
"#,
        )
        .unwrap();
        assert_eq!(c.self_urls, ["https://example.org/acme/specs/blob/*/"]);
        let r = &c.roots[0];
        assert_eq!(
            r.status_map,
            [
                ("released".to_string(), "active".to_string()),
                ("wip".to_string(), "draft".to_string())
            ]
        );
        assert_eq!(
            r.required_sections[DocType::Decision as usize],
            [["Lösung"]]
        );
        assert_eq!(
            r.required_sections[DocType::Requirements as usize],
            [["Requirements", "Anforderungen"]]
        );
        assert!(r.required_sections[DocType::Procedure as usize].is_empty());
        assert!(!r.enforce_budgets);
        assert_eq!(
            c.roots[1].required_sections[DocType::Decision as usize],
            [DEFAULT_DECISION_SECTIONS]
        );
        assert!(c.roots[1].enforce_budgets);
        assert!(c.roots[1].status_map.is_empty());

        let err = |extra: &str| {
            Config::parse(&format!(
                "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\ntype = \"requirements\"\n{extra}\n"
            ))
            .unwrap_err()
        };
        // `active` is already a spec status. `accepted` is not a spec status.
        assert!(err("status-map = { active = \"draft\" }").contains("already a status"));
        assert!(
            err("status-map = { frozen = \"accepted\" }")
                .contains("not a status of type `requirements`")
        );
        assert!(err("status-map = { frozen = 1 }").contains("must be a string"));
        assert!(err("required-sections = { requirements = [\" \"] }").contains("empty heading"));
        assert!(
            err("required-sections = { decision = [\"Outcome\"] }")
                .contains("not a type of this root")
        );
        assert!(err("required-sections = { requirements = \"Requirements\" }").contains("array"));
        assert!(err("enforce-budgets = \"no\"").contains("true or false"));
        // An empty list removes the required section.
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\nrequired-sections = { decision = [] }\n",
        )
        .unwrap();
        assert!(c.roots[0].required_sections[DocType::Decision as usize].is_empty());
        // In a mixed root, you can map a word that some type does not allow.
        assert!(
            Config::parse(
                "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\nstatus-map = { accepted = \"active\" }\n"
            )
            .is_ok()
        );
        assert!(
            err("amendment-sections = { decision = [\"Nachtrag\"] }")
                .contains("not a type of this root")
        );
        assert!(err("amendment-sections = { requirements = [\"\"] }").contains("empty heading"));
        assert!(err("lint-severity = \"warn\"").contains("\"off\", \"warning\" or \"error\""));
        assert!(err("lint-severity = false").contains("lint-severity"));
        assert!(err("drift-severity = \"fatal\"").contains("`roots[0].drift-severity` must be"));
        assert!(err("profile = \"loose\"").contains("`roots[0].profile` must be"));
        assert!(err("rules = { drfit = \"off\" }").contains("not a rule group"));
        assert!(err("rules = { SD999 = \"off\" }").contains("not a rule group"));
        assert!(err("rules = { drift = \"loud\" }").contains("`roots[0].rules.drift` must be"));
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\ninstall-source = \"file:///srv/git/spec-dna\"\n[[roots]]\npath = \"d\"\ndrift-severity = \"error\"\n",
        )
        .unwrap();
        assert_eq!(c.install_source, "file:///srv/git/spec-dna");
        assert_eq!(
            c.severity(Code::CitedPathRemoved, Some(0)),
            Some(Severity::Error)
        );
        let c = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\n").unwrap();
        assert_eq!(c.install_source, DEFAULT_INSTALL_SOURCE);
        assert_eq!(
            c.severity(Code::CitedPathRemoved, Some(0)),
            Some(Severity::Warning)
        );
        assert!(
            Config::parse(
                "spec-dna = \"0.1.0\"\ninstall-source = \"a b\"\n[[roots]]\npath = \"d\"\n"
            )
            .unwrap_err()
            .contains("install-source")
        );
        for bad in [
            "ftp://example.org/x/",
            "https://example.org/x",
            "https://example.org/x*/",
            "https://example.org/*x/",
            "https://*/x/",
        ] {
            let e = Config::parse(&format!(
                "spec-dna = \"0.1.0\"\nself-urls = [\"{bad}\"]\n[[roots]]\npath = \"d\"\n"
            ))
            .unwrap_err();
            assert!(e.contains("self-urls"), "{bad}: {e}");
        }
    }

    #[test]
    fn amendment_sections_and_lint_severity() {
        let c = Config::parse(
            r#"
spec-dna = "0.1.0"
[[roots]]
path = "adr"
type = "decision"
amendment-sections = { decision = ["Nachtrag", "Amendments"] }
lint-severity = "off"
[[roots]]
path = "docs"
lint-severity = "error"
[[roots]]
path = "notes"
"#,
        )
        .unwrap();
        let d = DocType::Decision as usize;
        assert_eq!(c.roots[0].amendment_sections[d], ["Nachtrag", "Amendments"]);
        assert_eq!(c.severity(Code::SentenceLength, Some(0)), None);
        assert_eq!(c.roots[1].amendment_sections[d], DEFAULT_AMENDMENT_SECTIONS);
        assert!(c.roots[1].amendment_sections[DocType::Requirements as usize].is_empty());
        assert_eq!(
            c.severity(Code::SentenceLength, Some(1)),
            Some(Severity::Error)
        );
        // Without a rule, the profile decides: `standard` has no lint.
        assert_eq!(c.severity(Code::SentenceLength, Some(2)), None);
    }

    #[test]
    fn profiles_and_rules_set_severities() {
        use Severity::{Error as E, Warning as W};
        let c = Config::parse(
            r#"
spec-dna = "0.1.0"
profile = "lite"
[rules]
SD606 = "error"
[[roots]]
path = "docs"
[[roots]]
path = "strict"
profile = "strict"
rules = { lint = "off", evidence = "warning" }
[[roots]]
path = "legacy"
lint-severity = "warning"
rules = { lint = "error" }
"#,
        )
        .unwrap();
        assert_eq!(c.profile, Profile::Lite);
        let sev = |code, root| c.severity(code, root);
        // Lite: references, ids and budgets are warnings. Drift and evidence
        // are off. A code rule of `[rules]` has priority over every profile.
        assert_eq!(sev(Code::UnresolvedReference, Some(0)), Some(W));
        assert_eq!(sev(Code::DuplicateId, Some(0)), Some(W));
        assert_eq!(sev(Code::BudgetHard, Some(0)), Some(W));
        assert_eq!(sev(Code::CitedSymbolRemoved, Some(0)), None);
        assert_eq!(sev(Code::CitedPathRemoved, Some(0)), Some(E));
        assert_eq!(sev(Code::RequirementUnverified, Some(0)), None);
        assert_eq!(sev(Code::SentenceLength, Some(0)), None);
        // Codes outside the table keep their own severity.
        assert_eq!(sev(Code::DecisionFrozen, Some(0)), Some(E));
        assert_eq!(sev(Code::TokenUnresolved, Some(0)), Some(W));
        // A strict root: errors for drift, budgets and evidence, warnings for
        // the lint. Its own rules win over its profile.
        assert_eq!(sev(Code::CitedSymbolRemoved, Some(1)), Some(E));
        assert_eq!(sev(Code::BudgetHardKept, Some(1)), Some(E));
        assert_eq!(sev(Code::BudgetSoft, Some(1)), Some(E));
        assert_eq!(sev(Code::SentenceLength, Some(1)), None);
        assert_eq!(sev(Code::RequirementUnverified, Some(1)), Some(W));
        assert_eq!(sev(Code::EvidenceMissing, Some(1)), Some(W));
        // `rules` has priority over the old `lint-severity` key of the same root.
        assert_eq!(sev(Code::SentenceLength, Some(2)), Some(E));
        // Outside the roots, the project profile and rules apply.
        assert_eq!(sev(Code::AdapterModified, None), Some(E));
        // The default profile is `standard`.
        let c = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\n").unwrap();
        assert_eq!(c.profile, Profile::Standard);
        assert_eq!(c.severity(Code::BudgetHard, Some(0)), Some(E));
        assert_eq!(c.severity(Code::BudgetHardKept, Some(0)), Some(W));
        assert_eq!(c.severity(Code::CitedPathRemoved, Some(0)), Some(W));
        assert_eq!(c.severity(Code::RequirementUnverified, Some(0)), Some(W));
        assert_eq!(c.severity(Code::RequirementKeyword, Some(0)), None);
        assert!(c.is_off(Code::PassiveVoice));
        assert!(!c.is_off(Code::CitedPathRemoved));
        assert!(
            Config::parse("spec-dna = \"0.1.0\"\nprofile = \"max\"\n[[roots]]\npath = \"d\"\n")
                .unwrap_err()
                .contains("`profile` must be")
        );
    }

    #[test]
    fn required_sections_groups_and_evidence_globs() {
        let c = Config::parse(
            r#"
spec-dna = "0.1.0"
[evidence]
tests = ["spec/**"]
[[roots]]
path = "ops"
required-sections = { incident = [["Summary", "Zusammenfassung"], ["Root cause"]], procedure = [] }
"#,
        )
        .unwrap();
        let r = &c.roots[0];
        assert_eq!(
            r.required_sections[DocType::Incident as usize],
            [vec!["Summary", "Zusammenfassung"], vec!["Root cause"]]
        );
        assert!(r.required_sections[DocType::Procedure as usize].is_empty());
        assert_eq!(r.required_sections[DocType::Rule as usize], [["Rules"]]);
        assert_eq!(c.evidence_tests, ["spec/**"]);
        assert!(c.is_test_file("spec/a_spec.rb"));
        assert!(!c.is_test_file("tests/a.rs"));
        let c = Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\n").unwrap();
        assert!(c.is_test_file("tests/cli.rs"));
        assert!(c.is_test_file("src/loans_test.go"));
        assert!(c.is_test_file("web/app.test.ts"));
        assert!(c.is_test_file("lib/test_loans.py"));
        assert!(!c.is_test_file("src/loans.py"));
        let e = Config::parse(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\nrequired-sections = { requirements = [[]] }\n",
        )
        .unwrap_err();
        assert!(e.contains("non-empty arrays"), "{e}");
    }

    const SUM: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

    fn with_table(pin: &str, table: &str) -> Result<Config, String> {
        Config::parse(&format!(
            "spec-dna = \"{pin}\"\n[spec-dna-sha256]\n{table}\n[[roots]]\npath = \"d\"\n"
        ))
    }

    #[test]
    fn the_sha256_table_is_validated_for_the_pin_of_the_binary() {
        assert!(
            Config::parse(&format!(
                "spec-dna = \"{VERSION}\"\n[[roots]]\npath = \"d\"\n"
            ))
            .unwrap()
            .sha256
            .is_none()
        );
        let good = format!(
            "\"spec-dna-{VERSION}-x86_64-unknown-linux-gnu\" = \"{SUM}\"\n\"spec-dna-{VERSION}-x86_64-pc-windows-gnu.exe\" = \"{SUM}\""
        );
        let c = with_table(VERSION, &good).unwrap();
        assert_eq!(c.sha256.unwrap().len(), 2);
        let e = with_table(
            VERSION,
            &format!("\"spec-dna-0.0.1-x86_64-unknown-linux-gnu\" = \"{SUM}\""),
        )
        .unwrap_err();
        assert_eq!(
            e,
            format!(
                "`spec-dna-sha256`: `spec-dna-0.0.1-x86_64-unknown-linux-gnu` is not a release asset of spec-dna {VERSION}; copy the table from spec-dna-pin.toml of release v{VERSION}"
            )
        );
        // The Windows assets have the extension `.exe`. The others do not.
        assert!(
            with_table(
                VERSION,
                &format!("\"spec-dna-{VERSION}-x86_64-pc-windows-gnu\" = \"{SUM}\"")
            )
            .unwrap_err()
            .contains("is not a release asset")
        );
        for bad in [
            SUM.to_uppercase(),
            SUM[1..].to_string(),
            format!("{SUM}0"),
            "z".repeat(64),
        ] {
            let e = with_table(
                VERSION,
                &format!("\"spec-dna-{VERSION}-x86_64-unknown-linux-gnu\" = \"{bad}\""),
            )
            .unwrap_err();
            assert_eq!(
                e,
                format!(
                    "`spec-dna-sha256`: the sum of `spec-dna-{VERSION}-x86_64-unknown-linux-gnu` must be 64 lowercase hexadecimal characters"
                )
            );
        }
        assert!(
            with_table(
                VERSION,
                &format!("\"spec-dna-{VERSION}-x86_64-unknown-linux-gnu\" = 1")
            )
            .unwrap_err()
            .contains("64 lowercase hexadecimal")
        );
        let e = Config::parse(&format!(
            "spec-dna = \"{VERSION}\"\nspec-dna-sha256 = \"x\"\n[[roots]]\npath = \"d\"\n"
        ))
        .unwrap_err();
        assert_eq!(
            e,
            "`spec-dna-sha256` must be a table of release assets and their SHA-256 sums"
        );
    }

    #[test]
    fn the_sha256_table_of_another_pin_is_not_validated() {
        // A wrong key and a wrong sum do not matter for another version.
        let c = with_table("0.0.1", "\"nonsense\" = 1").unwrap();
        assert_eq!(c.sha256, Some(Vec::new()));
        if let Some(own) = release::own_asset("0.0.1") {
            let c = with_table("0.0.1", &format!("\"{own}\" = \"{SUM}\"\n\"other\" = 1")).unwrap();
            assert_eq!(c.sha256, Some(vec![(own, SUM.to_string())]));
        }
    }

    #[test]
    fn the_pin_is_found_by_a_line_scan() {
        assert_eq!(
            scan_pin("spec-dna = \"1.2.3\"\nfoo = ").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(
            scan_pin("# c\n\nspec-dna=\"1.2.3\" # x\n[[roots]]").as_deref(),
            Some("1.2.3")
        );
        assert_eq!(scan_pin("spec-dna-sha256 = \"x\"\n"), None);
        assert_eq!(scan_pin("[[roots]]\nspec-dna = \"1.2.3\"\n"), None);
        assert_eq!(scan_pin("spec-dna = 5\n"), None);
    }

    #[test]
    fn the_pin_mismatch_message_has_a_release_form() {
        let Some(own) = release::own_asset("0.5.0") else {
            return;
        };
        let sums = [(own.clone(), SUM.to_string())];
        let m = pin_mismatch_message("0.5.0", DEFAULT_INSTALL_SOURCE, Some(&sums));
        let src = DEFAULT_INSTALL_SOURCE;
        let want = format!(
            "spec-dna.toml pins version 0.5.0, but this binary is {VERSION}.\n  To use 0.5.0, download it and compare its SHA-256 with spec-dna.toml:\n    {src}/releases/download/v0.5.0/{own}\n    SHA-256: {SUM}\n  Or build it from source:\n    cargo install --locked --force --git {src} --tag v0.5.0 spec-dna\n  To move the project to {VERSION}, download {src}/releases/download/v{VERSION}/spec-dna-pin.toml to a place outside the repository, for example the folder above it, verify it with `gh attestation verify`, and run `spec-dna upgrade --pin-file ../spec-dna-pin.toml`."
        );
        assert_eq!(m, want);
        // A source of the project gets the build command only. The table still
        // sets the last line.
        let m = pin_mismatch_message("0.5.0", "file:///srv/x", Some(&sums));
        assert!(!m.contains("SHA-256"), "{m}");
        assert!(m.contains("--git file:///srv/x --tag v0.5.0"), "{m}");
        assert!(
            m.ends_with("run `spec-dna upgrade --pin-file ../spec-dna-pin.toml`."),
            "{m}"
        );
        // No sum of this binary, or no table: the text of phase 1.
        let m = pin_mismatch_message("0.5.0", src, Some(&[]));
        assert!(m.contains("To use 0.5.0, install it:"), "{m}");
        assert!(m.contains("spec-dna-pin.toml"), "{m}");
        let m = pin_mismatch_message("0.5.0", src, None);
        assert!(
            m.ends_with(&format!(
                "To move the project to {VERSION}, run `spec-dna upgrade`."
            )),
            "{m}"
        );
    }

    #[test]
    fn the_last_sentence_of_the_message_follows_the_class_of_the_pin() {
        let src = DEFAULT_INSTALL_SOURCE;
        let sums: &[(String, String)] = &[];
        let last = |pin: &str, table: bool, binary: &str| {
            let m = mismatch_message(pin, src, table.then_some(sums), binary);
            m.lines().last().unwrap().to_string()
        };
        // Before 0.4.0: first to 0.4.2, with or without the table.
        let first = "  First move the project to 0.4.2 with the 0.4.2 binary and `spec-dna init`, then run `spec-dna upgrade`.";
        assert_eq!(last("0.3.3", false, "0.6.0"), first);
        assert_eq!(last("0.3.3", true, "0.6.0"), first);
        // 0.4.0 to before 0.6.0, and 0.6.0 to before the binary: upgrade.
        for pin in ["0.4.0", "0.5.0", "0.5.9", "0.6.0", "0.6.2"] {
            assert_eq!(
                last(pin, false, "0.6.3"),
                "  To move the project to 0.6.3, run `spec-dna upgrade`.",
                "{pin}"
            );
            assert_eq!(
                last(pin, true, "0.6.3"),
                format!(
                    "  To move the project to 0.6.3, download {src}/releases/download/v0.6.3/spec-dna-pin.toml to a place outside the repository, for example the folder above it, verify it with `gh attestation verify`, and run `spec-dna upgrade --pin-file ../spec-dna-pin.toml`."
                ),
                "{pin}"
            );
        }
        // After the binary: no sentence to move the project.
        for table in [false, true] {
            let m = mismatch_message("0.7.0", src, table.then_some(sums), "0.6.3");
            assert!(!m.contains("To move the project"), "{m}");
            assert!(!m.contains("spec-dna upgrade"), "{m}");
            assert!(m.contains("To use 0.7.0, install it:"), "{m}");
        }
        // Not a release version: the sentence of 0.5.
        assert_eq!(
            last("0.5", false, "0.6.3"),
            "  To move the project to 0.6.3, set `spec-dna = \"0.6.3\"` in spec-dna.toml and run `spec-dna init`."
        );
        assert_eq!(
            last("0.5", true, "0.6.3"),
            format!(
                "  To move the project to 0.6.3, replace `spec-dna` and `[spec-dna-sha256]` in spec-dna.toml with {DEFAULT_INSTALL_SOURCE}/releases/download/v0.6.3/spec-dna-pin.toml and run `spec-dna init`."
            )
        );
    }

    #[test]
    fn a_version_is_3_numbers_with_dots() {
        assert_eq!(Version::parse("0.4.2"), Some(Version(0, 4, 2)));
        assert_eq!(Version::parse("10.20.30"), Some(Version(10, 20, 30)));
        for bad in [
            "",
            "0.5",
            "0.5.0.1",
            "0.5.",
            ".5.0",
            "a.b.c",
            "0.5.0-rc1",
            "0.5.0+b",
            "+0.5.0",
            "-1.0.0",
            "0.05.0",
            "00.5.0",
            " 0.5.0",
            "0.5.0 ",
            "0.5.x",
            "1_0.0.0",
            "99999999999999999999.0.0",
        ] {
            assert_eq!(Version::parse(bad), None, "{bad:?}");
        }
        // The compare is by number, not by text.
        assert!(Version::parse("0.10.0") > Version::parse("0.9.0"));
        assert!(Version::parse("0.4.2") < Version::parse("0.6.0"));
    }

    #[test]
    fn the_class_of_a_pin_follows_d134() {
        use PinClass::*;
        let class = |pin: &str| pin_class(pin, "0.6.3");
        assert_eq!(class("0.5"), Invalid);
        assert_eq!(class("abc"), Invalid);
        assert_eq!(class("0.5.0-rc1"), Invalid);
        assert_eq!(class(""), Invalid);
        assert_eq!(class("0.1.0"), BeforeOldest);
        assert_eq!(class("0.3.3"), BeforeOldest);
        assert_eq!(class("0.3.99"), BeforeOldest);
        assert_eq!(class("0.4.0"), Model05);
        assert_eq!(class("0.4.2"), Model05);
        assert_eq!(class("0.5.0"), Model05);
        assert_eq!(class("0.5.99"), Model05);
        assert_eq!(class("0.6.0"), Model06);
        assert_eq!(class("0.6.2"), Model06);
        assert_eq!(class("0.6.3"), Binary);
        assert_eq!(class("0.6.4"), AfterBinary);
        assert_eq!(class("0.7.0"), AfterBinary);
        assert_eq!(class("1.0.0"), AfterBinary);
        // The binary 0.6.0 has no class of 0.6.0 to before the binary.
        assert_eq!(pin_class("0.6.0", "0.6.0"), Binary);
        assert_eq!(pin_class("0.5.0", "0.6.0"), Model05);
        // The numbers decide: 0.10.0 is after 0.9.0.
        assert_eq!(pin_class("0.10.0", "0.9.0"), AfterBinary);
        // The class of the pin of this binary is `Binary`.
        assert_eq!(pin_class(VERSION, VERSION), Binary);
    }

    #[test]
    fn the_limits_of_the_classes_are_consistent_with_the_binary() {
        let model = Version::parse(MODEL_06).unwrap();
        let binary = Version::parse(VERSION).unwrap();
        assert!(model <= binary, "MODEL_06 is not after the binary");
        assert!(Version::parse(PIN_OLDEST).unwrap() < model);
        assert!(Version::parse(PIN_FIRST_STEP).unwrap() < model);
        assert!(Version::parse(PIN_FIRST_STEP).unwrap() >= Version::parse(PIN_OLDEST).unwrap());
    }

    #[test]
    fn the_line_scan_reads_a_literal_string_and_a_byte_order_mark() {
        assert_eq!(scan_pin("spec-dna = '0.5.0'\n").as_deref(), Some("0.5.0"));
        assert_eq!(
            scan_pin("\u{feff}spec-dna = \"0.5.0\"\r\n[[roots]]\r\n").as_deref(),
            Some("0.5.0")
        );
        assert_eq!(scan_pin("spec-dna = '0.5.0\n"), None);
        assert_eq!(scan_pin("spec-dna = 0.5.0\n"), None);
    }

    const BASE: &str = "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\n";

    /// A configuration with the top-level lines `head` before the roots, and
    /// the tables `tail` after them.
    fn with(head: &str, tail: &str) -> Result<Config, String> {
        Config::parse(&format!(
            "spec-dna = \"0.1.0\"\n{head}\n[[roots]]\npath = \"docs\"\n{tail}"
        ))
    }

    #[test]
    fn no_layers_table_gives_the_implied_graph() {
        let c = Config::parse(BASE).unwrap();
        assert!(!c.layers_declared);
        assert_eq!(c.layers, Graph::default_graph());
        assert!(c.areas.is_empty() && c.groups.is_empty());
        // A `types` list keeps the layers that hold one of its types.
        let c = with("types = [\"requirements\", \"decision\"]", "").unwrap();
        let names: Vec<&str> = c.layers.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["requirements", "decisions"]);
        // An explicit table wins, and `types` does not filter it.
        let c = with(
            "types = [\"decision\"]",
            "[layers.mine]\ntypes = [\"rule\"]\n[layers.other]\n",
        )
        .unwrap();
        assert!(c.layers_declared);
        let names: Vec<&str> = c.layers.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["mine", "other"]);
    }

    #[test]
    fn layers_keep_the_order_of_the_file() {
        let c = Config::parse(&format!(
            "{BASE}[layers.zeta]\ntypes = [\"rule\"]\n[layers.alpha]\ntypes = [\"decision\"]\nmoves-to = [\"zeta\"]\nkeep = \"until-finished\"\ncontext = \"on-request\"\nmax-docs = 3\nmax-chars = 900\nmax-started = 2\nstale-days = 7\n"
        ))
        .unwrap();
        let l = c.layers.layers();
        assert_eq!((l[0].name.as_str(), l[1].name.as_str()), ("zeta", "alpha"));
        assert_eq!(c.layers.level(1), 1);
        assert_eq!(l[1].keep, layer::Keep::UntilFinished);
        assert_eq!(l[1].context, layer::ContextMode::OnRequest);
        assert_eq!(
            (
                l[1].max_docs,
                l[1].max_chars,
                l[1].max_started,
                l[1].stale_days
            ),
            (Some(3), Some(900), Some(2), Some(7))
        );
        assert_eq!(l[0].keep, layer::Keep::Forever);
        assert_eq!(l[0].dir(), Some("zeta"));
    }

    #[test]
    fn areas_and_groups() {
        let c = with(
            "areas = [\"valves\", \"sensors\", \"reports\"]",
            "[groups]\noperations = [\"valves\", \"sensors\"]\nreporting = [\"reports\"]\n",
        )
        .unwrap();
        assert_eq!(c.areas, ["valves", "sensors", "reports"]);
        assert_eq!(c.groups.len(), 2);
        assert_eq!(
            c.groups[0],
            ("operations".into(), vec!["valves".into(), "sensors".into()])
        );
        // An area is in 2 groups: allowed.
        assert!(
            with(
                "areas = [\"a\", \"b\"]",
                "[groups]\nx = [\"a\"]\ny = [\"a\", \"b\"]\n"
            )
            .is_ok()
        );
        let e = with("areas = [\"a\"]", "[groups]\nx = [\"b\"]\n").unwrap_err();
        assert!(
            e.contains("groups.x") && e.contains("`b`") && e.contains("areas"),
            "{e}"
        );
        let e = with("", "[groups]\nx = [\"b\"]\n").unwrap_err();
        assert!(e.contains("groups.x"), "{e}");
        let e = with("", "[groups]\nx = \"a\"\n").unwrap_err();
        assert!(e.contains("groups.x") && e.contains("array"), "{e}");
        for bad in ["\"a/b\"", "\"\"", "\"..\"", "\"a\", \"a\""] {
            let e = with(&format!("areas = [{bad}]"), "").unwrap_err();
            assert!(e.contains("`areas`"), "{bad}: {e}");
        }
    }

    #[test]
    fn a_later_step_key_is_still_unknown() {
        // `[evals]` and the 3 new keys of `[harness]` come in step 4. A key
        // of step 5 or later is unknown.
        let e = with("", "[evals]\nlist = true\n").unwrap_err();
        assert!(e.contains("unknown key `evals.list`"), "{e}");
        let e = with("", "[harness]\nlevel = \"lean\"\nupgrade = true\n").unwrap_err();
        assert!(e.contains("unknown key `harness.upgrade`"), "{e}");
        let e = with("upgrade = true", "").unwrap_err();
        assert!(e.contains("unknown key `upgrade`"), "{e}");
    }

    #[test]
    fn evals_has_defaults_and_limits() {
        assert!(Config::parse(BASE).unwrap().evals.is_none());
        let e = with("", "[evals]\n").unwrap().evals.unwrap();
        assert_eq!(
            e,
            Evals {
                dir: "evals".into(),
                min_pass_rate: 0.8,
                runs: 3,
                max_total_usd: 20.0
            }
        );
        let e = with(
            "",
            "[evals]\ndir = \"./q/e/\"\nmin-pass-rate = 1\nruns = 100\nmax-total-usd = 0.25\n",
        )
        .unwrap()
        .evals
        .unwrap();
        assert_eq!(e.dir, "q/e");
        assert_eq!((e.min_pass_rate, e.runs, e.max_total_usd), (1.0, 100, 0.25));
        assert_eq!(
            with("", "[evals]\nmin-pass-rate = 0\n")
                .unwrap()
                .evals
                .unwrap()
                .min_pass_rate,
            0.0
        );
        for bad in [
            "min-pass-rate = 1.01",
            "runs = 0",
            "runs = 101",
            "max-total-usd = 0",
            "max-total-usd = inf",
            "pass-at = 1",
        ] {
            assert!(with("", &format!("[evals]\n{bad}\n")).is_err(), "{bad}");
        }
        let e = with("", "[evals]\ndir = \"..\"\n").unwrap_err();
        assert!(e.contains("`evals.dir`") && e.contains("`evals`"), "{e}");
    }

    #[test]
    fn the_evals_dir_and_the_harness_files_are_never_part_of_a_root() {
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \".\"\n[evals]\ndir = \"q/evals/\"\n[harness]\nlevel = \"lean\"\nskills = [\"spec\"]\nreview-md = true\n",
        )
        .unwrap();
        assert!(c.is_excluded(0, "q/evals/fixtures/a/notes.md"));
        assert!(!c.is_excluded(0, "q/evals.md"));
        assert!(!c.is_excluded(0, "q/evalsx/a.md"));
        assert!(c.is_excluded(0, "REVIEW.md"));
        assert!(c.is_excluded(0, ".claude/skills/spec-dna-spec/SKILL.md"));
        // A skill that the configuration does not write stays a file of the root.
        assert!(!c.is_excluded(0, ".claude/skills/spec-dna-plan/SKILL.md"));
        assert!(!c.is_excluded(0, "docs/a.md"));
        // With `review-md = false`, and at level `none`, `REVIEW.md` is a document.
        for h in [
            "[harness]\nlevel = \"lean\"\n",
            "[harness]\nlevel = \"none\"\nreview-md = true\nskills = [\"spec\"]\n",
        ] {
            let c = Config::parse(&format!(
                "spec-dna = \"0.1.0\"\n[[roots]]\npath = \".\"\n{h}"
            ))
            .unwrap();
            assert!(!c.is_excluded(0, "REVIEW.md"), "{h}");
            assert!(
                !c.is_excluded(0, ".claude/skills/spec-dna-spec/SKILL.md"),
                "{h}"
            );
        }
        // No `[evals]` table: no folder.
        let c = Config::parse(BASE).unwrap();
        assert!(!c.in_evals_dir("evals/a.md"));
    }

    #[test]
    fn language_has_a_default_and_a_form() {
        let c = Config::parse(BASE).unwrap();
        assert_eq!(c.language, "en");
        assert!(c.language_is_english());
        for (tag, english) in [
            ("en", true),
            ("EN", true),
            ("en-GB", true),
            ("en-Latn-US", true),
            ("de", false),
            ("de-AT", false),
            ("zh-Hant-TW", false),
            ("fil", false),
        ] {
            let c = with(&format!("language = \"{tag}\""), "").unwrap();
            assert_eq!(c.language, tag);
            assert_eq!(c.language_is_english(), english, "{tag}");
        }
        for bad in [
            "\"\"",
            "\"e\"",
            "\"english\"",
            "\"en_GB\"",
            "\"en-\"",
            "\"-en\"",
            "\"en--GB\"",
            "\"en-toolongsubtag\"",
            "\"x-private\"",
            "\"en GB\"",
            "\"d\u{e9}\"",
        ] {
            let e = with(&format!("language = {bad}"), "").unwrap_err();
            assert!(e.contains("`language` must be a BCP 47"), "{bad}: {e}");
        }
        for bad in ["1", "[\"en\"]"] {
            let e = with(&format!("language = {bad}"), "").unwrap_err();
            assert!(e.contains("`language` must be a string"), "{bad}: {e}");
        }
        // The key is a top-level key. A root does not have it.
        let e = Config::parse(&format!("{BASE}language = \"en\"\n")).unwrap_err();
        assert!(e.contains("unknown key `roots[0].language`"), "{e}");
    }

    #[test]
    fn history_terms_are_unique_strings_that_keep_absent_apart_from_empty() {
        let c = Config::parse(BASE).unwrap();
        assert_eq!(c.history_terms, None);
        let c = with("", "[lint]\nmax-words = 20\n").unwrap();
        assert_eq!(c.history_terms, None);
        let c = with("", "[lint]\nhistory-terms = []\n").unwrap();
        assert_eq!(c.history_terms, Some(Vec::new()));
        let c = with("", "[lint]\nhistory-terms = [\"no longer\", \"vormals\"]\n").unwrap();
        assert_eq!(
            c.history_terms,
            Some(vec!["no longer".to_string(), "vormals".to_string()])
        );
        for (value, want) in [
            ("1", "`lint.history-terms` must be an array of strings"),
            ("[1]", "`lint.history-terms` must hold strings"),
            (
                "[\"a\", \"a\"]",
                "`lint.history-terms` lists `a` more than once",
            ),
            (
                "[\"a\", \"  \"]",
                "`lint.history-terms` has an empty phrase",
            ),
            ("[\"\"]", "`lint.history-terms` has an empty phrase"),
        ] {
            let e = with("", &format!("[lint]\nhistory-terms = {value}\n")).unwrap_err();
            assert!(e.contains(want), "{value}: {e}");
        }
    }

    #[test]
    fn the_change_table_keys_and_their_defaults() {
        // No table: the policy is `none`, and the keys have their defaults.
        let c = Config::parse(BASE).unwrap();
        assert_eq!(c.change_policy, ChangePolicy::None);
        assert_eq!(c.change_gate, None);
        assert_eq!(c.max_fanout, 8);
        assert!(c.high_risk_paths.is_empty());
        for (value, want) in [
            ("three-class", ChangePolicy::ThreeClass),
            ("two-class", ChangePolicy::TwoClass),
            ("none", ChangePolicy::None),
        ] {
            let c = with("", &format!("[change]\npolicy = \"{value}\"\n")).unwrap();
            assert_eq!(c.change_policy, want);
            assert_eq!(want.name(), value);
            // The other keys keep their defaults.
            assert_eq!(c.change_gate, None);
            assert_eq!(c.max_fanout, DEFAULT_MAX_FANOUT);
        }
        for (tail, want) in [
            ("[change]\n", "`change.policy` is required"),
            (
                "[change]\ngate = \"block\"\n",
                "`change.policy` is required",
            ),
            ("[change]\npolicy = 1\n", "`change.policy` must be a string"),
            (
                "[change]\npolicy = \"four-class\"\n",
                "`change.policy` must be \"three-class\", \"two-class\" or \"none\", not `four-class`",
            ),
            ("[change]\npolicy = \"\"\n", "`change.policy` must be"),
        ] {
            let e = with("", tail).unwrap_err();
            assert!(e.contains(want), "{tail}: {e}");
        }
        let e = with("change = true", "").unwrap_err();
        assert!(e.contains("`change` must be a table"), "{e}");
        // An invented key is still an unknown key.
        let e = with("", "[change]\npolicy = \"none\"\ninvented = 1\n").unwrap_err();
        assert!(e.contains("unknown key `change.invented`"), "{e}");
        assert!(!e.contains("later step"), "{e}");
    }

    #[test]
    fn each_change_key_has_valid_and_invalid_values() {
        let table = |keys: &str| format!("[change]\npolicy = \"three-class\"\n{keys}\n");
        let ok = |keys: &str| with("", &table(keys)).unwrap();
        let err = |keys: &str| with("", &table(keys)).unwrap_err();
        // gate
        for (v, want) in [
            ("block", Gate::Block),
            ("warn", Gate::Warn),
            ("off", Gate::Off),
        ] {
            let c = ok(&format!("gate = \"{v}\""));
            assert_eq!(c.change_gate, Some(want));
            assert_eq!(want.name(), v);
        }
        for bad in ["gate = \"error\"", "gate = \"\"", "gate = \"Block\""] {
            let e = err(bad);
            assert!(e.contains("`change.gate` must be"), "{bad}: {e}");
        }
        assert!(err("gate = true").contains("`change.gate` must be a string"));
        // max-fanout
        assert_eq!(ok("max-fanout = 3").max_fanout, 3);
        assert_eq!(ok("max-fanout = 0").max_fanout, 0);
        for bad in ["max-fanout = -1", "max-fanout = \"8\"", "max-fanout = 1.5"] {
            let e = err(bad);
            assert!(
                e.contains("`change.max-fanout` must be a non-negative integer"),
                "{bad}: {e}"
            );
        }
        // high-risk-paths
        assert_eq!(
            ok("high-risk-paths = [\"src/api/**\", \"./db/*.sql\"]").high_risk_paths,
            ["src/api/**", "db/*.sql"]
        );
        assert!(ok("high-risk-paths = []").high_risk_paths.is_empty());
        for (bad, want) in [
            ("high-risk-paths = [\"\"]", "is empty"),
            ("high-risk-paths = [\"/abs/**\"]", "relative path"),
            ("high-risk-paths = [\"a\\\\b\"]", "relative path"),
            ("high-risk-paths = [\"../up/**\"]", "`.` or `..` segment"),
            ("high-risk-paths = [\"a/**\", \"./a/**\"]", "more than once"),
            ("high-risk-paths = [1]", "must hold strings"),
            ("high-risk-paths = \"src\"", "must be an array of strings"),
        ] {
            let e = err(bad);
            assert!(e.contains("`change.high-risk-paths`"), "{bad}: {e}");
            assert!(e.contains(want), "{bad}: {e}");
        }
        // high-risk-layers: the default graph has both layers of the default.
        let layers = "[layers.rules]\ntypes = [\"rule\"]\n[layers.other]\ntypes = [\"decision\"]\n";
        let c = with("", &format!("{layers}{}", table(""))).unwrap();
        // The default names only the layers that exist: `rules` here.
        assert_eq!(c.high_risk_layers, ["rules"]);
        let c = with("", &table("")).unwrap();
        assert_eq!(c.high_risk_layers, ["interfaces", "rules"]);
        let c = with(
            "",
            &format!("{layers}{}", table("high-risk-layers = [\"other\"]")),
        )
        .unwrap();
        assert_eq!(c.high_risk_layers, ["other"]);
        assert!(ok("high-risk-layers = []").high_risk_layers.is_empty());
        let e = with(
            "",
            &format!("{layers}{}", table("high-risk-layers = [\"nope\"]")),
        )
        .unwrap_err();
        assert!(
            e.contains("`change.high-risk-layers`: `nope` is not a layer of the graph"),
            "{e}"
        );
        assert!(e.contains("rules, other"), "{e}");
        let e = err("high-risk-layers = [\"rules\", \"rules\"]");
        assert!(e.contains("lists `rules` more than once"), "{e}");
        assert!(err("high-risk-layers = 3").contains("must be an array of strings"));
        // A project with no `[change]` table has the default layers of the
        // graph.
        let c = with("", layers).unwrap();
        assert_eq!(c.high_risk_layers, ["rules"]);
    }

    #[test]
    fn the_new_codes_follow_the_profile_table_and_the_new_groups() {
        use Severity::{Error as E, Warning as W};
        let by = |profile: &str, code: Code| {
            let c = with(
                &format!("profile = \"{profile}\""),
                "[change]\npolicy = \"three-class\"\n",
            )
            .unwrap();
            c.severity(code, Some(0))
        };
        for code in [
            Code::LinkDirection,
            Code::LayerLimit,
            Code::HistoryInRequirements,
            Code::TermForbidden,
        ] {
            assert_eq!(by("lite", code), None, "{}", code.code());
            assert_eq!(by("standard", code), Some(W), "{}", code.code());
            assert_eq!(by("strict", code), Some(W), "{}", code.code());
        }
        assert_eq!(by("lite", Code::TruthChangedWithoutChange), None);
        assert_eq!(by("standard", Code::TruthChangedWithoutChange), Some(W));
        assert_eq!(by("strict", Code::TruthChangedWithoutChange), Some(E));
        for (code, name) in [
            (Code::LinkDirection, "link-direction"),
            (Code::LayerLimit, "layer-limit"),
            (Code::HistoryInRequirements, "history-in-requirements"),
            (Code::TermForbidden, "term-forbidden"),
            (
                Code::TruthChangedWithoutChange,
                "truth-changed-without-change",
            ),
            (Code::QuestionOpen, "question-open"),
        ] {
            assert_eq!(code.name(), name);
            assert_eq!(Code::from_code(code.code()), Some(code));
        }
        // The group `layers` holds SD901 to SD904. The group `changes` holds
        // SD905 and SD910 to SD922.
        let c = with(
            "profile = \"lite\"\n[rules]\nlayers = \"error\"\nchanges = \"warning\"\nSD902 = \"off\"",
            "[change]\npolicy = \"two-class\"\n",
        )
        .unwrap();
        assert_eq!(c.severity(Code::LinkDirection, Some(0)), Some(E));
        assert_eq!(c.severity(Code::LayerLimit, Some(0)), None);
        assert_eq!(c.severity(Code::TermForbidden, Some(0)), Some(E));
        assert_eq!(
            c.severity(Code::TruthChangedWithoutChange, Some(0)),
            Some(W)
        );
    }

    #[test]
    fn the_profile_rows_of_the_change_codes() {
        use Severity::{Error as E, Warning as W};
        // The design table: code, lite, standard, strict.
        type Sev = Option<Severity>;
        let rows: [(Code, Sev, Sev, Sev); 13] = [
            (Code::FanOut, None, Some(W), Some(W)),
            (Code::GateApprovalMissing, Some(W), Some(E), Some(E)),
            (Code::ChangeStale, None, Some(W), Some(W)),
            (Code::StartedLimit, None, Some(W), Some(E)),
            (Code::OutsidePlan, None, Some(W), Some(W)),
            (Code::DeltaInvalid, Some(W), Some(E), Some(E)),
            (Code::FinishIncomplete, Some(W), Some(E), Some(E)),
            (Code::DeltaEvidenceMissing, Some(W), Some(E), Some(E)),
            (Code::ClassTooLow, None, Some(W), Some(E)),
            (Code::ConcernOpen, Some(W), Some(E), Some(E)),
            (Code::QuestionOpen, Some(W), Some(E), Some(E)),
            (Code::DecisionNeededUnaccepted, Some(W), Some(E), Some(E)),
            (Code::IncidentEvalMissing, None, Some(W), Some(W)),
        ];
        for (code, lite, standard, strict) in rows {
            for (profile, want) in [("lite", lite), ("standard", standard), ("strict", strict)] {
                let c = with(
                    &format!("profile = \"{profile}\""),
                    "[change]\npolicy = \"three-class\"\n",
                )
                .unwrap();
                assert_eq!(c.severity(code, Some(0)), want, "{} {profile}", code.code());
                assert_eq!(c.severity(code, None), want, "{} {profile}", code.code());
            }
            assert!(
                GROUPS
                    .iter()
                    .any(|g| g.0 == "changes" && g.1.contains(&code.code()))
            );
        }
        // SD918 (step 4) is in the group `changes`, and it needs `[evals]`.
        assert_eq!(Code::from_code("SD918"), Some(Code::IncidentEvalMissing));
        assert_eq!(Code::IncidentEvalMissing.name(), "incident-eval-missing");
        // SD506 (step 4) is in no group, and it keeps its severity in each
        // profile.
        assert_eq!(Code::from_code("SD506"), Some(Code::EvalsInvalid));
        assert_eq!(Code::EvalsInvalid.name(), "evals-invalid");
        assert!(!GROUPS.iter().any(|g| g.1.contains(&"SD506")));
        for profile in ["lite", "standard", "strict"] {
            let c = with(&format!("profile = \"{profile}\""), "").unwrap();
            assert_eq!(
                c.severity(Code::EvalsInvalid, Some(0)),
                Some(E),
                "{profile}"
            );
        }
        // SD911 is a state code: it is never new against the base (K4).
        assert!(Code::ChangeStale.is_state());
        assert!(!Code::GateApprovalMissing.is_state());
        assert!(!Code::TruthChangedWithoutChange.is_state());
        assert_eq!(Code::ChangeStale.name(), "change-stale");
        assert_eq!(Code::GateApprovalMissing.name(), "gate-approval-missing");
    }

    #[test]
    fn sd910_follows_the_gate_the_profile_and_the_rules() {
        use Severity::{Error as E, Warning as W};
        // The expected severity by the order of the design: the policy, the
        // rules, the gate, the profile.
        let profile_col = |p: &str| match p {
            "lite" => Some(W),
            _ => Some(E),
        };
        let gate_col = |g: &str| match g {
            "block" => Some(E),
            "warn" => Some(W),
            "off" => None,
            _ => unreachable!(),
        };
        // The rule text, the rule that SD910 gets, the rule that another code
        // of the group `changes` gets.
        type Rule = Option<Option<Severity>>;
        let rule_sets: [(&str, Rule, Rule); 7] = [
            ("", None, None),
            ("SD910 = \"off\"\n", Some(None), None),
            ("SD910 = \"warning\"\n", Some(Some(W)), None),
            ("SD910 = \"error\"\n", Some(Some(E)), None),
            ("changes = \"off\"\n", Some(None), Some(None)),
            ("changes = \"error\"\n", Some(Some(E)), Some(Some(E))),
            // A code rule has priority over a group rule.
            (
                "changes = \"error\"\nSD910 = \"warning\"\n",
                Some(Some(W)),
                Some(Some(E)),
            ),
        ];
        for profile in ["lite", "standard", "strict"] {
            for gate in [None, Some("block"), Some("warn"), Some("off")] {
                for (rules, rule, group_rule) in &rule_sets {
                    let gate_line = gate.map_or(String::new(), |g| format!("gate = \"{g}\"\n"));
                    let text = format!(
                        "spec-dna = \"0.1.0\"\nprofile = \"{profile}\"\n[change]\npolicy = \"three-class\"\n{gate_line}[rules]\n{rules}[[roots]]\npath = \"docs\"\n"
                    );
                    let c = Config::parse(&text).unwrap();
                    let want = match (rule, gate) {
                        (Some(r), _) => *r,
                        (None, Some(g)) => gate_col(g),
                        (None, None) => profile_col(profile),
                    };
                    assert_eq!(
                        c.severity(Code::GateApprovalMissing, Some(0)),
                        want,
                        "{profile} gate={gate:?} rules={rules:?}"
                    );
                    // The gate does not change any other code.
                    assert_eq!(
                        c.severity(Code::DeltaInvalid, Some(0)),
                        group_rule.unwrap_or_else(|| profile_col(profile)),
                        "{profile} gate={gate:?} rules={rules:?}"
                    );
                }
            }
        }
        // A rule of a root has priority over `[rules]` and over the gate.
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n[rules]\nSD910 = \"error\"\n[[roots]]\npath = \"docs\"\nrules = { SD910 = \"off\" }\n",
        )
        .unwrap();
        assert_eq!(c.severity(Code::GateApprovalMissing, Some(0)), None);
        assert_eq!(c.severity(Code::GateApprovalMissing, None), Some(E));
    }

    #[test]
    fn the_policy_none_turns_off_every_change_code_before_the_rules() {
        let codes = [
            Code::FanOut,
            Code::GateApprovalMissing,
            Code::ChangeStale,
            Code::StartedLimit,
            Code::OutsidePlan,
            Code::DeltaInvalid,
            Code::FinishIncomplete,
            Code::DeltaEvidenceMissing,
            Code::ClassTooLow,
            Code::ConcernOpen,
            Code::QuestionOpen,
            Code::DecisionNeededUnaccepted,
            Code::TruthChangedWithoutChange,
        ];
        for change in [
            "",
            "[change]\npolicy = \"none\"\n",
            "[change]\npolicy = \"none\"\ngate = \"block\"\n",
        ] {
            for rules in [
                "",
                "[rules]\nchanges = \"error\"\n",
                "[rules]\nSD910 = \"error\"\nSD914 = \"error\"\nSD905 = \"error\"\n",
            ] {
                let text = format!(
                    "spec-dna = \"0.1.0\"\nprofile = \"strict\"\n{change}{rules}[[roots]]\npath = \"docs\"\nrules = {{ changes = \"error\", SD911 = \"error\" }}\n"
                );
                let c = Config::parse(&text).unwrap();
                for code in codes {
                    assert_eq!(
                        c.severity(code, Some(0)),
                        None,
                        "{} {change:?}",
                        code.code()
                    );
                    assert_eq!(c.severity(code, None), None, "{} {change:?}", code.code());
                    assert!(c.is_off(code));
                }
            }
        }
        // The other codes keep their rules.
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\n[rules]\nSD701 = \"error\"\n[[roots]]\npath = \"docs\"\n",
        )
        .unwrap();
        assert_eq!(c.severity(Code::BudgetSoft, Some(0)), Some(Severity::Error));
    }

    #[test]
    fn no_rule_turns_sd921_on_without_a_change_policy() {
        let rule_sets = [
            "[rules]\nchanges = \"error\"\n",
            "[rules]\nSD921 = \"error\"\n",
            "[rules]\nSD921 = \"warning\"\nchanges = \"error\"\n",
        ];
        for rules in rule_sets {
            for profile in ["lite", "standard", "strict"] {
                let parse = |change: &str| {
                    Config::parse(&format!(
                        "spec-dna = \"0.1.0\"\nprofile = \"{profile}\"\n{change}{rules}[[roots]]\npath = \"docs\"\n"
                    ))
                    .unwrap()
                };
                // No table: the policy is `none`.
                let c = parse("");
                assert_eq!(c.severity(Code::TruthChangedWithoutChange, Some(0)), None);
                assert_eq!(c.severity(Code::TruthChangedWithoutChange, None), None);
                assert!(c.is_off(Code::TruthChangedWithoutChange));
                // The same with an explicit `none`.
                let c = parse("[change]\npolicy = \"none\"\n");
                assert_eq!(c.severity(Code::TruthChangedWithoutChange, Some(0)), None);
                // With a policy, the same rule turns it on, also in `lite`.
                let c = parse("[change]\npolicy = \"three-class\"\n");
                assert!(
                    c.severity(Code::TruthChangedWithoutChange, Some(0))
                        .is_some()
                );
            }
        }
        // A rule of a root cannot turn it on either.
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"docs\"\nrules = { SD921 = \"error\", changes = \"error\" }\n",
        )
        .unwrap();
        assert_eq!(c.severity(Code::TruthChangedWithoutChange, Some(0)), None);
        // The other codes keep their rules under `none`.
        let c = Config::parse(
            "spec-dna = \"0.1.0\"\n[rules]\nlayers = \"error\"\n[[roots]]\npath = \"docs\"\n",
        )
        .unwrap();
        assert_eq!(
            c.severity(Code::LinkDirection, Some(0)),
            Some(Severity::Error)
        );
    }

    const SCHEMA: &str = include_str!("../schema/spec-dna.schema.json");

    /// The schema and the parser accept the same keys. For each key of the
    /// schema, the parser must not call the key unknown, and an invented key
    /// must be unknown.
    #[test]
    fn the_schema_and_the_parser_accept_the_same_keys() {
        let schema = crate::json::parse(SCHEMA).unwrap();
        let keys = |v: &crate::json::Value| -> Vec<String> {
            match v.get("properties") {
                crate::json::Value::Obj(f) => f.iter().map(|(k, _)| k.clone()).collect(),
                _ => Vec::new(),
            }
        };
        let is_unknown = |e: &str| e.contains("unknown key");

        // The top level.
        let top = keys(&schema);
        for k in [
            "areas",
            "groups",
            "layers",
            "templates",
            "language",
            "change",
        ] {
            assert!(top.iter().any(|t| t == k), "the schema lacks `{k}`");
        }
        for k in &top {
            // `with` writes these 2 keys.
            if k == "spec-dna" || k == "roots" {
                continue;
            }
            // A value of the wrong type is an error that is not "unknown key".
            // A key can also accept the value `true`.
            if let Err(e) = with(&format!("{k} = true"), "") {
                assert!(!is_unknown(&e), "the parser does not know `{k}`: {e}");
            }
        }
        let e = Config::parse("invented = true\nspec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\n")
            .unwrap_err();
        assert!(is_unknown(&e), "{e}");

        // The keys of `[change]` and `[lint]` that step 2 adds.
        let change = schema.get("properties").get("change");
        let mut ckeys = keys(change);
        ckeys.sort();
        assert_eq!(
            ckeys,
            [
                "gate",
                "high-risk-layers",
                "high-risk-paths",
                "max-fanout",
                "policy"
            ]
        );
        for (k, bad) in [
            ("gate", "1"),
            ("high-risk-layers", "1"),
            ("high-risk-paths", "1"),
            ("max-fanout", "\"x\""),
        ] {
            let e = with("", &format!("[change]\npolicy = \"none\"\n{k} = {bad}\n")).unwrap_err();
            assert!(
                !is_unknown(&e),
                "the parser does not know `change.{k}`: {e}"
            );
        }
        assert_eq!(
            change.get("additionalProperties"),
            &crate::json::Value::Bool(false)
        );
        let e = with("", "[change]\npolicy = \"none\"\ninvented = true\n").unwrap_err();
        assert!(is_unknown(&e), "{e}");
        let e = with("", "[change]\npolicy = true\n").unwrap_err();
        assert!(
            !is_unknown(&e),
            "the parser does not know `change.policy`: {e}"
        );
        let lint = schema.get("properties").get("lint");
        assert!(keys(lint).iter().any(|k| k == "history-terms"));
        let e = with("", "[lint]\nhistory-terms = true\n").unwrap_err();
        assert!(
            !is_unknown(&e),
            "the parser does not know `lint.history-terms`: {e}"
        );
        // The keys of `[evals]`: exactly the 4 keys of step 4.
        let evals = schema.get("properties").get("evals");
        let mut ekeys = keys(evals);
        ekeys.sort();
        assert_eq!(ekeys, ["dir", "max-total-usd", "min-pass-rate", "runs"]);
        for k in &ekeys {
            let e = with("", &format!("[evals]\n{k} = true\n")).unwrap_err();
            assert!(!is_unknown(&e), "the parser does not know `evals.{k}`: {e}");
        }
        assert_eq!(
            evals.get("additionalProperties"),
            &crate::json::Value::Bool(false)
        );
        let e = with("", "[evals]\ninvented = true\n").unwrap_err();
        assert!(is_unknown(&e), "{e}");
        // The keys of `[harness]`.
        let harness = schema.get("properties").get("harness");
        let mut hkeys = keys(harness);
        hkeys.sort();
        assert_eq!(
            hkeys,
            [
                "context-meter",
                "deny",
                "efforts",
                "fanout-cap",
                "level",
                "protect-tests",
                "review-md",
                "roles",
                "skills",
                "subagent-meter",
            ]
        );
        for k in &hkeys {
            let e = with("", &format!("[harness]\nlevel = \"lean\"\n{k} = \"x\"\n")).unwrap_err();
            assert!(
                !is_unknown(&e),
                "the parser does not know `harness.{k}`: {e}"
            );
        }
        assert_eq!(
            harness.get("additionalProperties"),
            &crate::json::Value::Bool(false)
        );
        // The roles of `[harness.roles]`. The roles of 0.5 that ended are not
        // in the schema, and the parser gives an error that is not "unknown".
        let mut rkeys = keys(harness.get("properties").get("roles"));
        rkeys.sort();
        let mut roles: Vec<&str> = crate::harness::Role::ALL.iter().map(|r| r.key()).collect();
        roles.sort();
        assert_eq!(rkeys, roles);
        for gone in ["reviewer", "spec-writer"] {
            assert!(!rkeys.iter().any(|k| k == gone), "{gone}");
            let toml = format!("[harness]\nlevel = \"lean\"\n[harness.roles]\n{gone} = {{}}\n");
            let e = with("", &toml).unwrap_err();
            assert!(!is_unknown(&e), "{gone}: {e}");
            assert!(
                e.contains(&format!("`harness.roles.{gone}`")),
                "{gone}: {e}"
            );
        }
        // The rule groups of the schema are the groups of the parser.
        let pattern = match schema
            .get("$defs")
            .get("rules")
            .get("propertyNames")
            .get("pattern")
        {
            crate::json::Value::Str(p) => p.clone(),
            other => panic!("no pattern: {other:?}"),
        };
        for (name, _) in GROUPS {
            assert!(
                pattern.contains(name),
                "the schema lacks the group `{name}`"
            );
            assert!(with("", &format!("[rules]\n{name} = \"off\"\n")).is_ok());
        }

        // The keys of a layer: exactly the 12 keys of the design.
        let layer = &schema.get("$defs").get("layer");
        let mut lkeys = keys(layer);
        lkeys.sort();
        let mut want = [
            "types",
            "dir",
            "files",
            "paths",
            "moves-to",
            "linked-from",
            "keep",
            "context",
            "max-docs",
            "max-chars",
            "max-started",
            "stale-days",
        ]
        .map(String::from)
        .to_vec();
        want.sort();
        assert_eq!(lkeys, want);
        for k in &lkeys {
            let e = with("", &format!("[layers.a]\n{k} = true\n")).unwrap_err();
            assert!(
                !is_unknown(&e),
                "the parser does not know `layers.a.{k}`: {e}"
            );
        }
        let e = with("", "[layers.a]\ninvented = true\n").unwrap_err();
        assert!(is_unknown(&e), "{e}");
        // The schema forbids other keys of a layer, as the parser does.
        assert_eq!(
            layer.get("additionalProperties"),
            &crate::json::Value::Bool(false)
        );
    }
}

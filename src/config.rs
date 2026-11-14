//! `spec-dna.toml` is the project configuration (ADR 0001, "Layers").
//! docs/config.md describes it. schema/spec-dna.schema.json contains its JSON
//! Schema.

use crate::adapters::VERSION;
use crate::diag::{Code, Severity};
use crate::harness::Harness;
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
}

/// The default growth allowance: about one amendment or a few registry rows.
pub const DEFAULT_GROWTH: u32 = 4_000;

/// The public repository of spec-dna: the install source by default.
pub const DEFAULT_INSTALL_SOURCE: &str = "https://github.com/AlexGavrilov939/spec-dna";

/// Reads `[spec-dna-sha256]`. If the pin is the version of this binary, it
/// validates the table (ADR 0003, amendment "phase 2"). Otherwise it keeps
/// only the sum of the asset of this binary.
fn sha256_table(v: Option<&Value>, pin: &str) -> Result<Option<Vec<(String, String)>>, String> {
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

/// Reads the key `spec-dna` of a configuration text with a line scan, for a
/// text that does not parse. It looks at the lines before the first table
/// header.
pub fn scan_pin(text: &str) -> Option<String> {
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
        let rest = rest.trim_start().strip_prefix('"')?;
        return rest.split_once('"').map(|(v, _)| v.to_string());
    }
    None
}

/// The message of a pin mismatch (ADR 0003). `sums` is `[spec-dna-sha256]`,
/// if the file has the table. The release form needs the public install
/// source and the sum of the asset of this binary.
pub fn pin_mismatch_message(pin: &str, source: &str, sums: Option<&[(String, String)]>) -> String {
    let release = release::own_asset(pin)
        .filter(|_| source == DEFAULT_INSTALL_SOURCE)
        .and_then(|a| sums.and_then(|s| s.iter().find(|(k, _)| *k == a).cloned()));
    let build = format!("cargo install --locked --force --git {source} --tag v{pin} spec-dna");
    let mut m = format!("spec-dna.toml pins version {pin}, but this binary is {VERSION}.\n");
    match release {
        Some((asset, sum)) => m.push_str(&format!(
            "  To use {pin}, download it and compare its SHA-256 with spec-dna.toml:\n    {source}/releases/download/v{pin}/{asset}\n    SHA-256: {sum}\n  Or build it from source:\n    {build}\n"
        )),
        None => m.push_str(&format!("  To use {pin}, install it:\n    {build}\n")),
    }
    if sums.is_some() {
        m.push_str(&format!(
            "  To move the project to {VERSION}, replace `spec-dna` and `[spec-dna-sha256]` in spec-dna.toml with {DEFAULT_INSTALL_SOURCE}/releases/download/v{VERSION}/spec-dna-pin.toml and run `spec-dna init`."
        ));
    } else {
        m.push_str(&format!(
            "  To move the project to {VERSION}, set `spec-dna = \"{VERSION}\"` in spec-dna.toml and run `spec-dna init`."
        ));
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
    pub default_type: DocType,
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
    /// discussions, superseded notes). They match as amendment sections do.
    /// `spec-dna history` moves them into a `reference` document. In a
    /// decision, they are not frozen text.
    pub history_sections: [Vec<String>; DocType::COUNT],
    /// For each type, the names of the sections that belong to the
    /// requirements, such as a coverage table. When requirements move, their
    /// rows move with them.
    pub companion_sections: [Vec<String>; DocType::COUNT],
    /// For each type, the names of its live sections, such as a
    /// progress section. They match as amendment sections do. In a
    /// decision, they are not frozen text, and their text can change in
    /// place.
    pub live_sections: [Vec<String>; DocType::COUNT],
    /// For a root of type `reference`: the namespace of its ids, a suffix of
    /// its path that no other reference root has. Thus the ids of two
    /// reference roots never collide.
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
pub const GROUPS: [(&str, &[&str]); 6] = [
    ("references", &["SD101", "SD102", "SD104", "SD106"]),
    ("ids", &["SD201", "SD202", "SD203", "SD204"]),
    ("budgets", &["SD701", "SD702", "SD703", "SD704"]),
    ("drift", &["SD606", "SD607"]),
    ("evidence", &["SD103", "SD105", "SD608"]),
    ("lint", &["SD801", "SD802", "SD803", "SD804", "SD805"]),
];

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

pub const DEFAULT_BUDGETS: [(u32, u32); DocType::COUNT] = [
    (10_000, 20_000), // spec
    (10_000, 20_000), // contract
    (14_000, 28_000), // guide
    (7_000, 14_000),  // decision
    (10_000, 20_000), // change
    (0, 0),           // note: no budget
    (3_000, 6_000),   // invariant: `context` serves it first, so it stays short
    (8_000, 16_000),  // runbook
    (6_000, 12_000),  // incident
    (0, 0),           // reference: cold material, no budget
];

/// The default required sections of each type: a list of groups. A document
/// needs one heading of each group.
pub fn default_required_sections() -> [Vec<Vec<String>>; DocType::COUNT] {
    let mut out: [Vec<Vec<String>>; DocType::COUNT] = Default::default();
    let group = |h: &[&str]| h.iter().map(|s| s.to_string()).collect::<Vec<String>>();
    out[DocType::Decision as usize] = vec![group(&DEFAULT_DECISION_SECTIONS)];
    out[DocType::Invariant as usize] = vec![group(&["Rules"])];
    out[DocType::Runbook as usize] = vec![group(&["Steps"])];
    out[DocType::Incident as usize] = vec![group(&["Summary"]), group(&["Root cause"])];
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
        if let Some(v) = t.take("lint") {
            let tbl = v.as_table().ok_or("`lint` must be a table")?;
            let mut f = Fields::new(tbl, "lint.");
            max_words = f.opt_u32("max-words")?.unwrap_or(max_words);
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
            let ty = DocType::from_name(&name).ok_or_else(|| {
                format!("`types`: unknown type `{name}`; use {}", DocType::names())
            })?;
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
                let ty = DocType::from_name(name).ok_or_else(|| {
                    format!(
                        "`conventions.{name}`: unknown type `{name}`; use {}",
                        DocType::names()
                    )
                })?;
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
        t.done()?;
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
        })
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
        let root = root.map(|r| &self.roots[r]);
        if let Some(s) = root.and_then(|r| rule_for(&r.rules, code)) {
            return s;
        }
        if let Some(s) = rule_for(&self.rules, code) {
            return s;
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

    pub fn is_excluded(&self, root: usize, path: &str) -> bool {
        let rel = self.roots[root].rel(path);
        self.exclude.iter().any(|g| crate::glob::matches(g, path))
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
            Some(
                DocType::from_name(&ty)
                    .ok_or_else(|| format!("`{prefix}type`: unknown type `{ty}`"))?,
            )
        };
        let default_type = match f.opt_str("default-type")? {
            Some(d) => {
                if single_type.is_some() {
                    return Err(format!(
                        "`{prefix}default-type` is only for `type = \"mixed\"`"
                    ));
                }
                DocType::from_name(&d)
                    .ok_or_else(|| format!("`{prefix}default-type`: unknown type `{d}`"))?
            }
            None => single_type.unwrap_or(DocType::Note),
        };
        let status = f.opt_str("status")?;
        if let Some(s) = &status
            && !default_type.statuses().contains(&s.as_str())
        {
            return Err(format!(
                "`{prefix}status`: `{s}` is not a status of type `{}`",
                default_type.name()
            ));
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

/// Gives each root of type `reference` a namespace: the shortest suffix of
/// its path (whole segments) that no other reference root has. A reference
/// root at the repository root has no namespace.
fn set_namespaces(roots: &mut [Root]) {
    let paths: Vec<Option<Vec<String>>> = roots
        .iter()
        .map(|r| {
            (r.single_type == Some(DocType::Reference) && !r.path.is_empty())
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
        let ty = DocType::from_name(name)
            .filter(|t| types.contains(t))
            .ok_or_else(|| format!("`{key}`: `{name}` is not a type of this root"))?;
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
        let ty = DocType::from_name(name)
            .filter(|t| types.contains(t))
            .ok_or_else(|| format!("`{key}`: `{name}` is not a type of this root"))?;
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
            "spec-dna = \"0.1.0\"\n[conventions]\ncontract = [\"docs/README.md#a\", \"./CONTRIBUTING.md\", \"docs/x.md#b\", \"token\", \"../up.md\", \"/abs.md\", \"a\\\\b.md\", \"notes.txt\"]\ndecision = [\"CONTRIBUTING.md#c\"]\n[[roots]]\npath = \"docs\"\nexclude = [\"README.md\"]\n",
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
default-type = "spec"
status = "draft"
legacy-keys = ["R-{n}"]

[[roots]]
path = "docs/adr"
type = "decision"
legacy-ids = ["ADR-{n}", { alias = "KEP-{n}", file = "{n}-*" }]

[budgets.spec]
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
        assert_eq!(c.roots[0].default_type, DocType::Spec);
        assert_eq!(c.roots[1].single_type, Some(DocType::Decision));
        assert_eq!(c.budgets[DocType::Spec as usize], (100, 200));
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
            Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\ntyp = \"spec\"\n")
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
        assert!(Config::parse("spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\ntype = \"spec\"\nstatus = \"accepted\"\n").is_err());
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
required-sections = { decision = ["Lösung"], spec = ["Requirements", "Anforderungen"] }
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
            r.required_sections[DocType::Spec as usize],
            [["Requirements", "Anforderungen"]]
        );
        assert!(r.required_sections[DocType::Guide as usize].is_empty());
        assert!(!r.enforce_budgets);
        assert_eq!(
            c.roots[1].required_sections[DocType::Decision as usize],
            [DEFAULT_DECISION_SECTIONS]
        );
        assert!(c.roots[1].enforce_budgets);
        assert!(c.roots[1].status_map.is_empty());

        let err = |extra: &str| {
            Config::parse(&format!(
                "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\ntype = \"spec\"\n{extra}\n"
            ))
            .unwrap_err()
        };
        // `active` is already a spec status. `accepted` is not a spec status.
        assert!(err("status-map = { active = \"draft\" }").contains("already a status"));
        assert!(
            err("status-map = { frozen = \"accepted\" }").contains("not a status of type `spec`")
        );
        assert!(err("status-map = { frozen = 1 }").contains("must be a string"));
        assert!(err("required-sections = { spec = [\" \"] }").contains("empty heading"));
        assert!(
            err("required-sections = { decision = [\"Outcome\"] }")
                .contains("not a type of this root")
        );
        assert!(err("required-sections = { spec = \"Requirements\" }").contains("array"));
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
        assert!(err("amendment-sections = { spec = [\"\"] }").contains("empty heading"));
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
        assert!(c.roots[1].amendment_sections[DocType::Spec as usize].is_empty());
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
required-sections = { incident = [["Summary", "Zusammenfassung"], ["Root cause"]], runbook = [] }
"#,
        )
        .unwrap();
        let r = &c.roots[0];
        assert_eq!(
            r.required_sections[DocType::Incident as usize],
            [vec!["Summary", "Zusammenfassung"], vec!["Root cause"]]
        );
        assert!(r.required_sections[DocType::Runbook as usize].is_empty());
        assert_eq!(
            r.required_sections[DocType::Invariant as usize],
            [["Rules"]]
        );
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
            "spec-dna = \"0.1.0\"\n[[roots]]\npath = \"d\"\nrequired-sections = { spec = [[]] }\n",
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
        let Some(own) = release::own_asset("0.0.1") else {
            return;
        };
        let sums = [(own.clone(), SUM.to_string())];
        let m = pin_mismatch_message("0.0.1", DEFAULT_INSTALL_SOURCE, Some(&sums));
        let src = DEFAULT_INSTALL_SOURCE;
        let want = format!(
            "spec-dna.toml pins version 0.0.1, but this binary is {VERSION}.\n  To use 0.0.1, download it and compare its SHA-256 with spec-dna.toml:\n    {src}/releases/download/v0.0.1/{own}\n    SHA-256: {SUM}\n  Or build it from source:\n    cargo install --locked --force --git {src} --tag v0.0.1 spec-dna\n  To move the project to {VERSION}, replace `spec-dna` and `[spec-dna-sha256]` in spec-dna.toml with {src}/releases/download/v{VERSION}/spec-dna-pin.toml and run `spec-dna init`."
        );
        assert_eq!(m, want);
        // A source of the project gets the build command only. The table still
        // sets the last line.
        let m = pin_mismatch_message("0.0.1", "file:///srv/x", Some(&sums));
        assert!(!m.contains("SHA-256"), "{m}");
        assert!(m.contains("--git file:///srv/x --tag v0.0.1"), "{m}");
        assert!(m.ends_with("pin.toml and run `spec-dna init`."), "{m}");
        // No sum of this binary, or no table: the text of phase 1.
        let m = pin_mismatch_message("0.0.1", src, Some(&[]));
        assert!(m.contains("To use 0.0.1, install it:"), "{m}");
        assert!(m.contains("spec-dna-pin.toml"), "{m}");
        let m = pin_mismatch_message("0.0.1", src, None);
        assert!(
            m.ends_with(&format!(
                "set `spec-dna = \"{VERSION}\"` in spec-dna.toml and run `spec-dna init`."
            )),
            "{m}"
        );
    }
}

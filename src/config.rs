//! `spec-dna.toml` is the project configuration (ADR 0001, "Layers").
//! docs/config.md describes it. schema/spec-dna.schema.json contains its JSON
//! Schema.

use crate::diag::Severity;
use crate::model::DocType;
use toml::{Table, Value};

pub const FILE_NAME: &str = "spec-dna.toml";

#[derive(Clone, Debug)]
pub struct Config {
    /// The pinned spec-dna version.
    pub pin: String,
    pub exclude: Vec<String>,
    pub roots: Vec<Root>,
    /// Soft and hard budgets in characters, per document type.
    pub budgets: [(u32, u32); 6],
    pub max_words: u32,
    pub context_budget: u32,
    pub repos: Vec<(String, String)>,
    /// URL prefixes that address this repository. A link under one of them is
    /// internal.
    pub self_urls: Vec<String>,
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
    pub required_sections: [Vec<String>; 6],
    /// For each type, the names of its amendment sections. A heading that is
    /// a name, or that starts with a name and a character that is not a letter
    /// or a digit, starts an amendment section.
    pub amendment_sections: [Vec<String>; 6],
    /// If false, a document over the hard budget gives a warning, not an
    /// error.
    pub enforce_budgets: bool,
    /// The severity of the requirement lint (SD801 to SD805). `None` turns
    /// the lint off.
    pub lint_severity: Option<Severity>,
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
        Ok(Pattern { parts })
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

pub const DEFAULT_BUDGETS: [(u32, u32); 6] = [
    (10_000, 20_000), // spec
    (10_000, 20_000), // contract
    (14_000, 28_000), // guide
    (7_000, 14_000),  // decision
    (10_000, 20_000), // change
    (0, 0),           // note: no budget
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
        let mut budgets = DEFAULT_BUDGETS;
        if let Some(v) = t.take("budgets") {
            let tbl = v.as_table().ok_or("`budgets` must be a table")?;
            let mut b = Fields::new(tbl, "budgets.");
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
        let mut context_budget = 12_000;
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
        t.done()?;
        Ok(Config {
            pin,
            exclude,
            roots,
            budgets,
            max_words,
            context_budget,
            repos,
            self_urls,
        })
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
        let mut required_sections: [Vec<String>; 6] = Default::default();
        required_sections[DocType::Decision as usize] =
            DEFAULT_DECISION_SECTIONS.map(str::to_string).to_vec();
        if let Some(v) = f.take("required-sections") {
            headings_per_type(
                v,
                &format!("{prefix}required-sections"),
                &types,
                &mut required_sections,
            )?;
        }
        let mut amendment_sections: [Vec<String>; 6] = Default::default();
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
        let enforce_budgets = match f.take("enforce-budgets") {
            None => true,
            Some(Value::Boolean(b)) => *b,
            Some(_) => return Err(format!("`{prefix}enforce-budgets` must be true or false")),
        };
        let lint_severity = match f.take("lint-severity").map(|v| v.as_str()) {
            None | Some(Some("warning")) => Some(Severity::Warning),
            Some(Some("error")) => Some(Severity::Error),
            Some(Some("off")) => None,
            Some(_) => {
                return Err(format!(
                    "`{prefix}lint-severity` must be \"off\", \"warning\" or \"error\""
                ));
            }
        };
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
            lint_severity,
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

/// Reads a table of heading lists per type, such as `required-sections`, into
/// `out`. A list replaces the default of its type.
fn headings_per_type(
    v: &Value,
    key: &str,
    types: &[DocType],
    out: &mut [Vec<String>; 6],
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
struct Fields<'a> {
    table: &'a Table,
    used: Vec<&'a str>,
    prefix: String,
}

impl<'a> Fields<'a> {
    fn new(table: &'a Table, prefix: &str) -> Self {
        Fields {
            table,
            used: Vec::new(),
            prefix: prefix.to_string(),
        }
    }

    fn take(&mut self, key: &'a str) -> Option<&'a Value> {
        self.used.push(key);
        self.table.get(key)
    }

    fn opt_str(&mut self, key: &'a str) -> Result<Option<String>, String> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::String(s)) => Ok(Some(s.clone())),
            Some(_) => Err(format!("`{}{key}` must be a string", self.prefix)),
        }
    }

    fn req_str(&mut self, key: &'a str) -> Result<String, String> {
        self.opt_str(key)?
            .ok_or_else(|| format!("`{}{key}` is required", self.prefix))
    }

    fn opt_strs(&mut self, key: &'a str) -> Result<Vec<String>, String> {
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

    fn opt_u32(&mut self, key: &'a str) -> Result<Option<u32>, String> {
        match self.take(key) {
            None => Ok(None),
            Some(Value::Integer(n)) if *n >= 0 && *n <= u32::MAX as i64 => Ok(Some(*n as u32)),
            Some(_) => Err(format!(
                "`{}{key}` must be a non-negative integer",
                self.prefix
            )),
        }
    }

    fn done(self) -> Result<(), String> {
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
        assert_eq!(r.required_sections[DocType::Decision as usize], ["Lösung"]);
        assert_eq!(
            r.required_sections[DocType::Spec as usize],
            ["Requirements", "Anforderungen"]
        );
        assert!(r.required_sections[DocType::Guide as usize].is_empty());
        assert!(!r.enforce_budgets);
        assert_eq!(
            c.roots[1].required_sections[DocType::Decision as usize],
            DEFAULT_DECISION_SECTIONS
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
        assert_eq!(c.roots[0].lint_severity, None);
        assert_eq!(c.roots[1].amendment_sections[d], DEFAULT_AMENDMENT_SECTIONS);
        assert!(c.roots[1].amendment_sections[DocType::Spec as usize].is_empty());
        assert_eq!(c.roots[1].lint_severity, Some(Severity::Error));
        assert_eq!(c.roots[2].lint_severity, Some(Severity::Warning));
    }
}

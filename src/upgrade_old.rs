//! The 0.5 names and the reader of a 0.4 or 0.5 `spec-dna.toml` (D137). The
//! binary keeps the 0.5 names in this one place: `DocType::from_old_name` reads
//! the table of the types, so the hint of SD303 and the plan of `upgrade`
//! cannot differ. No command accepts an old name as input. A 0.4 file is a 0.5
//! file with no `[harness]` table, so one reader serves both.

use crate::model::DocType;
use toml::{Table, Value};

/// A 0.5 type, and the 0.6 type that replaces it. The name `spec` is also a
/// 0.6 type name, so a name in a file or a front matter is a 0.5 name only
/// where the 0.5 rule says so.
pub const TYPES: [(&str, DocType); 10] = [
    ("spec", DocType::Requirements),
    ("contract", DocType::Interface),
    ("guide", DocType::Procedure),
    ("decision", DocType::Decision),
    ("change", DocType::Intent),
    ("note", DocType::Research),
    ("invariant", DocType::Rule),
    ("runbook", DocType::Procedure),
    ("incident", DocType::Incident),
    ("reference", DocType::Research),
];

/// The 0.6 type of a 0.5 type name.
pub fn type_of(old: &str) -> Option<DocType> {
    TYPES.iter().find(|(n, _)| *n == old).map(|(_, t)| *t)
}

/// The 0.6 type of a name that 0.6 no longer has. The result is for a hint.
/// `spec`, `decision` and `incident` are 0.6 names, so they have no entry.
pub fn renamed(old: &str) -> Option<DocType> {
    if DocType::from_name(old).is_some() {
        return None;
    }
    type_of(old)
}

/// What a root of one 0.5 type becomes (D140).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootTo {
    /// A root of this 0.6 type.
    Type(DocType),
    /// A root with `type = "mixed"` and `default-type = "research"`. A root of
    /// type `research` adds a namespace to each id, which a `note` root and a
    /// `change` root never had.
    MixedResearch,
}

/// The 0.6 root that a 0.5 `roots.type` gives.
pub fn root_to(old: &str) -> Option<RootTo> {
    match old {
        "note" | "change" => Some(RootTo::MixedResearch),
        _ => type_of(old).map(RootTo::Type),
    }
}

/// The 0.6 type of a 0.5 `roots.default-type`. A `change` becomes `research`,
/// because `intent` is valid only in its layer (D140).
pub fn default_type_to(old: &str) -> Option<DocType> {
    match old {
        "change" => Some(DocType::Research),
        _ => type_of(old),
    }
}

/// The 0.5 statuses of a `change`, and the 0.6 status of an `intent` that
/// replaces each. A `done` change becomes a `research` document in place,
/// which has no status.
pub const CHANGE_STATUSES: [(&str, Option<&str>); 4] = [
    ("proposed", Some("draft")),
    ("accepted", Some("accepted")),
    ("rejected", Some("closed")),
    ("done", None),
];

/// The 0.6 status of a 0.5 change status: `None` for a word that is not a
/// change status, `Some(None)` for `done`.
pub fn change_status(old: &str) -> Option<Option<&'static str>> {
    CHANGE_STATUSES
        .iter()
        .find(|(n, _)| *n == old)
        .map(|(_, new)| *new)
}

/// A role key of 0.5 and the key that replaces it. `spec-writer` ended.
pub const ROLES: [(&str, Option<&str>); 2] =
    [("reviewer", Some("verifier")), ("spec-writer", None)];

/// The default budget of a 0.5 type, in characters: soft and hard. `None`: the
/// type has no budget.
pub fn budget_of(old: &str) -> Option<(u32, u32)> {
    match old {
        "spec" | "contract" | "change" => Some((10_000, 20_000)),
        "guide" => Some((14_000, 28_000)),
        "decision" => Some((7_000, 14_000)),
        "invariant" => Some((3_000, 6_000)),
        "runbook" => Some((8_000, 16_000)),
        "incident" => Some((6_000, 12_000)),
        _ => None,
    }
}

/// The default required sections of a 0.5 type: a list of groups. A document
/// needs one heading of each group.
pub fn required_of(old: &str) -> Vec<Vec<&'static str>> {
    match old {
        "decision" => vec![vec!["Decision Outcome", "Decision"]],
        "invariant" => vec![vec!["Rules"]],
        "runbook" => vec![vec!["Steps"]],
        "incident" => vec![vec!["Summary"], vec!["Root cause"]],
        _ => Vec::new(),
    }
}

/// The 5 tables of a root that have a type for each key.
pub const SECTION_TABLES: [&str; 5] = [
    "required-sections",
    "amendment-sections",
    "history-sections",
    "companion-sections",
    "live-sections",
];

/// The keys of the 0.5.0 schema.
pub const TOP_KEYS: [&str; 16] = [
    "spec-dna",
    "profile",
    "rules",
    "types",
    "evidence",
    "exclude",
    "self-urls",
    "install-source",
    "spec-dna-sha256",
    "roots",
    "budgets",
    "lint",
    "context",
    "conventions",
    "repos",
    "harness",
];

pub const ROOT_KEYS: [&str; 18] = [
    "path",
    "type",
    "default-type",
    "status",
    "exclude",
    "legacy-ids",
    "legacy-keys",
    "status-map",
    "required-sections",
    "amendment-sections",
    "live-sections",
    "history-sections",
    "companion-sections",
    "enforce-budgets",
    "lint-severity",
    "drift-severity",
    "profile",
    "rules",
];

pub const HARNESS_KEYS: [&str; 7] = [
    "level",
    "deny",
    "context-meter",
    "subagent-meter",
    "fanout-cap",
    "roles",
    "efforts",
];

/// The roles of 0.5, each with its keys.
pub const HARNESS_ROLES: [(&str, &[&str]); 7] = [
    ("orchestrator", &["claude-code", "effort", "fallback"]),
    ("advisor", &["claude-code", "effort", "agent"]),
    ("reviewer", &["claude-code", "effort"]),
    ("researcher", &["claude-code", "effort"]),
    ("implementer", &["claude-code", "effort"]),
    ("spec-writer", &["claude-code", "effort"]),
    ("helper", &["claude-code"]),
];

/// The kind of a 0.5 root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind<'a> {
    Mixed,
    /// A root of one 0.5 type.
    Single(&'a str),
}

/// A `[[roots]]` table of 0.5. The reader keeps what the plan needs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldRoot {
    /// The path as `Config` reads it: no `/` at the ends, and `` for `.`.
    pub path: String,
    pub ty: Option<String>,
    pub default_type: Option<String>,
    pub status: Option<String>,
    /// The words and the targets of `status-map`.
    pub status_map: Vec<(String, String)>,
    /// For each table of `SECTION_TABLES`: `None` if the root has no such
    /// key, else the type keys of the table.
    pub sections: [Option<Vec<String>>; 5],
}

impl OldRoot {
    pub fn kind(&self) -> Kind<'_> {
        match self.ty.as_deref() {
            None | Some("mixed") => Kind::Mixed,
            Some(t) => Kind::Single(t),
        }
    }

    /// The 0.5 type of a document with no `type` field: the type of a root of
    /// one type, else `default-type`, else `note`.
    pub fn default_old_type(&self) -> &str {
        match self.kind() {
            Kind::Single(t) => t,
            Kind::Mixed => self.default_type.as_deref().unwrap_or("note"),
        }
    }

    /// The type keys of a table of `SECTION_TABLES`, empty if the root has no
    /// such table.
    pub fn section_keys(&self, table: &str) -> &[String] {
        SECTION_TABLES
            .iter()
            .position(|t| *t == table)
            .and_then(|i| self.sections[i].as_deref())
            .unwrap_or(&[])
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldHarness {
    pub level: String,
    /// The keys of `[harness.roles]`.
    pub roles: Vec<String>,
}

/// A 0.4 or 0.5 `spec-dna.toml`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OldConfig {
    pub pin: String,
    /// True if the file has `[spec-dna-sha256]`.
    pub has_sums: bool,
    /// The key `types`.
    pub types: Option<Vec<String>>,
    pub roots: Vec<OldRoot>,
    /// The type keys of `[budgets]`: the key `growth` is not one.
    pub budgets: Vec<String>,
    /// The type keys of `[conventions]`.
    pub conventions: Vec<String>,
    pub harness: Option<OldHarness>,
}

impl OldConfig {
    /// Reads a text with the keys of the 0.5.0 schema, and no other key. A 0.6
    /// key, such as `[layers]`, is an error.
    pub fn read(text: &str) -> Result<OldConfig, String> {
        let table: Table = text
            .parse()
            .map_err(|e: toml::de::Error| e.to_string().trim().to_string())?;
        check_keys(&table, &TOP_KEYS, "")?;
        let pin = match table.get("spec-dna") {
            Some(Value::String(s)) => s.clone(),
            Some(_) => return Err("`spec-dna` must be a string".into()),
            None => return Err("`spec-dna` is required".into()),
        };
        if let Some(v) = table.get("profile") {
            profile(v, "profile")?;
        }
        if let Some(v) = table.get("rules") {
            rules(v, "rules")?;
        }
        let types = match table.get("types") {
            None => None,
            Some(v) => {
                let names = strings(v, "types")?;
                for n in &names {
                    old_type(n, "`types`")?;
                }
                Some(names)
            }
        };
        if let Some(v) = table.get("evidence") {
            let t = as_table(v, "evidence")?;
            check_keys(t, &["tests"], "evidence.")?;
            if let Some(v) = t.get("tests") {
                strings(v, "evidence.tests")?;
            }
        }
        for key in ["exclude", "self-urls"] {
            if let Some(v) = table.get(key) {
                strings(v, key)?;
            }
        }
        if let Some(v) = table.get("install-source")
            && !v.is_str()
        {
            return Err("`install-source` must be a string".into());
        }
        let has_sums = match table.get("spec-dna-sha256") {
            None => false,
            Some(v) => {
                let t = as_table(v, "spec-dna-sha256")?;
                if let Some((k, _)) = t.iter().find(|(_, v)| !v.is_str()) {
                    return Err(format!("`spec-dna-sha256.{k}` must be a string"));
                }
                true
            }
        };
        let mut roots = Vec::new();
        match table.get("roots") {
            Some(Value::Array(a)) => {
                for (i, item) in a.iter().enumerate() {
                    let t = item
                        .as_table()
                        .ok_or("`roots` must be an array of tables (`[[roots]]`)")?;
                    roots.push(read_root(t, &format!("roots[{i}].")).map_err(|e| e.to_string())?);
                }
            }
            Some(_) => return Err("`roots` must be an array of tables (`[[roots]]`)".into()),
            None => {}
        }
        if roots.is_empty() {
            return Err("at least one `[[roots]]` table is required".into());
        }
        let mut budgets = Vec::new();
        if let Some(v) = table.get("budgets") {
            let t = as_table(v, "budgets")?;
            for (k, v) in t {
                if k == "growth" {
                    non_negative(v, "budgets.growth")?;
                    continue;
                }
                if type_of(k).is_none() {
                    return Err(unknown(&format!("budgets.{k}"), "budgets"));
                }
                let bt = as_table(v, &format!("budgets.{k}"))?;
                check_keys(bt, &["soft", "hard"], &format!("budgets.{k}."))?;
                for (bk, bv) in bt {
                    non_negative(bv, &format!("budgets.{k}.{bk}"))?;
                }
                budgets.push(k.clone());
            }
        }
        if let Some(v) = table.get("lint") {
            let t = as_table(v, "lint")?;
            check_keys(t, &["max-words"], "lint.")?;
            if let Some(v) = t.get("max-words") {
                non_negative(v, "lint.max-words")?;
            }
        }
        if let Some(v) = table.get("context") {
            let t = as_table(v, "context")?;
            check_keys(t, &["budget"], "context.")?;
            if let Some(v) = t.get("budget") {
                non_negative(v, "context.budget")?;
            }
        }
        let mut conventions = Vec::new();
        if let Some(v) = table.get("conventions") {
            for (k, v) in as_table(v, "conventions")? {
                old_type(k, &format!("`conventions.{k}`"))?;
                strings(v, &format!("conventions.{k}"))?;
                conventions.push(k.clone());
            }
        }
        if let Some(v) = table.get("repos") {
            for (k, v) in as_table(v, "repos")? {
                if !v.is_str() {
                    return Err(format!("`repos.{k}` must be a string"));
                }
            }
        }
        let harness = match table.get("harness") {
            None => None,
            Some(v) => Some(read_harness(v)?),
        };
        Ok(OldConfig {
            pin,
            has_sums,
            types,
            roots,
            budgets,
            conventions,
            harness,
        })
    }
}

// --- the reader ---

/// The error of a key that 0.5 does not have. A key of 0.6 gets a hint.
fn unknown(key: &str, table: &str) -> String {
    let new_in_06 = matches!(
        key,
        "areas" | "groups" | "layers" | "templates" | "language" | "change" | "evals"
    ) || key == "lint.history-terms"
        || matches!(
            key,
            "harness.skills" | "harness.review-md" | "harness.protect-tests"
        );
    if new_in_06 {
        format!("unknown key `{key}`: 0.5 has no such key, it is new in 0.6")
    } else if table.is_empty() {
        format!("unknown key `{key}`")
    } else {
        format!("unknown key `{key}` in `{table}`: not a key of 0.5")
    }
}

/// Refuses each key of the table that is not in the list. `prefix` is the
/// path of the table with a final `.`, or empty for the top level.
fn check_keys(t: &Table, allowed: &[&str], prefix: &str) -> Result<(), String> {
    for k in t.keys() {
        if !allowed.contains(&k.as_str()) {
            let table = prefix.trim_end_matches('.');
            return Err(unknown(&format!("{prefix}{k}"), table));
        }
    }
    Ok(())
}

fn as_table<'a>(v: &'a Value, key: &str) -> Result<&'a Table, String> {
    v.as_table()
        .ok_or_else(|| format!("`{key}` must be a table"))
}

fn strings(v: &Value, key: &str) -> Result<Vec<String>, String> {
    v.as_array()
        .and_then(|a| {
            a.iter()
                .map(|x| x.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        })
        .ok_or_else(|| format!("`{key}` must be an array of strings"))
}

fn non_negative(v: &Value, key: &str) -> Result<(), String> {
    match v {
        Value::Integer(n) if *n >= 0 => Ok(()),
        _ => Err(format!("`{key}` must be a non-negative integer")),
    }
}

fn string_of(v: &Value, key: &str) -> Result<String, String> {
    v.as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("`{key}` must be a string"))
}

fn profile(v: &Value, key: &str) -> Result<(), String> {
    match v.as_str() {
        Some("lite" | "standard" | "strict") => Ok(()),
        _ => Err(format!(
            "`{key}` must be \"lite\", \"standard\" or \"strict\""
        )),
    }
}

fn severity(v: &Value, key: &str) -> Result<(), String> {
    match v.as_str() {
        Some("off" | "warning" | "error") => Ok(()),
        _ => Err(format!("`{key}` must be \"off\", \"warning\" or \"error\"")),
    }
}

/// `[rules]`: a rule group or a code, with a severity.
fn rules(v: &Value, key: &str) -> Result<(), String> {
    for (k, v) in as_table(v, key)? {
        let code =
            k.len() == 5 && k.starts_with("SD") && k[2..].bytes().all(|b| b.is_ascii_digit());
        let group = matches!(
            k.as_str(),
            "references" | "ids" | "budgets" | "drift" | "evidence" | "lint"
        );
        if !code && !group {
            return Err(format!("`{key}.{k}` is not a rule group or a code"));
        }
        severity(v, &format!("{key}.{k}"))?;
    }
    Ok(())
}

/// Checks a 0.5 type name. `place` is the key, with its quotes.
fn old_type(name: &str, place: &str) -> Result<(), String> {
    if type_of(name).is_some() {
        return Ok(());
    }
    Err(format!(
        "{place}: `{name}` is not a type of 0.5; the types are {}",
        TYPES.map(|(n, _)| n).join(", ")
    ))
}

/// A list of headings or a list of such lists.
fn headings(v: &Value, key: &str) -> Result<(), String> {
    let bad = || format!("`{key}` must be a list of headings, or a list of such lists");
    let a = v.as_array().ok_or_else(bad)?;
    let flat = a.iter().all(Value::is_str);
    let nested = a.iter().all(|x| {
        x.as_array()
            .is_some_and(|g| !g.is_empty() && g.iter().all(Value::is_str))
    });
    if flat || nested { Ok(()) } else { Err(bad()) }
}

fn read_root(t: &Table, prefix: &str) -> Result<OldRoot, String> {
    check_keys(t, &ROOT_KEYS, prefix)?;
    let path = string_of(
        t.get("path")
            .ok_or_else(|| format!("`{prefix}path` is required"))?,
        &format!("{prefix}path"),
    )?;
    let path = path.trim_matches('/');
    let path = if path == "." { "" } else { path };
    let ty = match t.get("type") {
        None => None,
        Some(v) => {
            let s = string_of(v, &format!("{prefix}type"))?;
            if s != "mixed" {
                old_type(&s, &format!("`{prefix}type`"))?;
            }
            Some(s)
        }
    };
    let default_type = match t.get("default-type") {
        None => None,
        Some(v) => {
            let s = string_of(v, &format!("{prefix}default-type"))?;
            old_type(&s, &format!("`{prefix}default-type`"))?;
            Some(s)
        }
    };
    let status = match t.get("status") {
        None => None,
        Some(v) => Some(string_of(v, &format!("{prefix}status"))?),
    };
    for key in ["exclude", "legacy-keys"] {
        if let Some(v) = t.get(key) {
            strings(v, &format!("{prefix}{key}"))?;
        }
    }
    if let Some(v) = t.get("legacy-ids") {
        let key = format!("{prefix}legacy-ids");
        let a = v
            .as_array()
            .ok_or_else(|| format!("`{key}` must be an array"))?;
        for item in a {
            match item {
                Value::String(_) => {}
                Value::Table(it) => {
                    check_keys(it, &["alias", "file"], &format!("{key}."))?;
                    if !it.get("alias").is_some_and(Value::is_str) {
                        return Err(format!("`{key}.alias` is required, and a string"));
                    }
                    if it.get("file").is_some_and(|f| !f.is_str()) {
                        return Err(format!("`{key}.file` must be a string"));
                    }
                }
                _ => return Err(format!("`{key}` items are strings or tables")),
            }
        }
    }
    let mut status_map = Vec::new();
    if let Some(v) = t.get("status-map") {
        let key = format!("{prefix}status-map");
        for (from, to) in as_table(v, &key)? {
            status_map.push((from.clone(), string_of(to, &format!("{key}.{from}"))?));
        }
    }
    let mut sections: [Option<Vec<String>>; 5] = Default::default();
    for (i, name) in SECTION_TABLES.iter().enumerate() {
        let Some(v) = t.get(*name) else {
            continue;
        };
        let key = format!("{prefix}{name}");
        let mut keys = Vec::new();
        for (ty, list) in as_table(v, &key)? {
            old_type(ty, &format!("`{key}.{ty}`"))?;
            if i == 0 {
                headings(list, &format!("{key}.{ty}"))?;
            } else {
                strings(list, &format!("{key}.{ty}"))?;
            }
            keys.push(ty.clone());
        }
        sections[i] = Some(keys);
    }
    if let Some(v) = t.get("enforce-budgets")
        && !v.is_bool()
    {
        return Err(format!("`{prefix}enforce-budgets` must be true or false"));
    }
    if let Some(v) = t.get("profile") {
        profile(v, &format!("{prefix}profile"))?;
    }
    if let Some(v) = t.get("rules") {
        rules(v, &format!("{prefix}rules"))?;
    }
    for key in ["lint-severity", "drift-severity"] {
        if let Some(v) = t.get(key) {
            severity(v, &format!("{prefix}{key}"))?;
        }
    }
    Ok(OldRoot {
        path: path.to_string(),
        ty,
        default_type,
        status,
        status_map,
        sections,
    })
}

fn read_harness(v: &Value) -> Result<OldHarness, String> {
    let t = as_table(v, "harness")?;
    check_keys(t, &HARNESS_KEYS, "harness.")?;
    let level = string_of(
        t.get("level").ok_or("`harness.level` is required")?,
        "harness.level",
    )?;
    if !matches!(level.as_str(), "none" | "lean" | "full") {
        return Err(format!(
            "`harness.level` must be \"none\", \"lean\" or \"full\", not `{level}`"
        ));
    }
    if let Some(v) = t.get("deny") {
        strings(v, "harness.deny")?;
    }
    if let Some(v) = t.get("context-meter")
        && !v
            .as_array()
            .is_some_and(|a| a.iter().all(|x| matches!(x, Value::Integer(n) if *n > 0)))
    {
        return Err("`harness.context-meter` must be an array of positive integers".into());
    }
    for key in ["subagent-meter", "fanout-cap"] {
        if let Some(v) = t.get(key) {
            non_negative(v, &format!("harness.{key}"))?;
        }
    }
    let mut roles = Vec::new();
    if let Some(v) = t.get("roles") {
        for (role, rv) in as_table(v, "harness.roles")? {
            let Some((_, role_keys)) = HARNESS_ROLES.iter().find(|(n, _)| n == role) else {
                return Err(unknown(&format!("harness.roles.{role}"), "harness.roles"));
            };
            let rt = as_table(rv, &format!("harness.roles.{role}"))?;
            check_keys(rt, role_keys, &format!("harness.roles.{role}."))?;
            for (k, v) in rt {
                let ok = if k == "fallback" {
                    strings(v, "").is_ok()
                } else {
                    v.is_str()
                };
                if !ok {
                    return Err(format!(
                        "`harness.roles.{role}.{k}` has a value of the wrong type"
                    ));
                }
            }
            roles.push(role.clone());
        }
    }
    if let Some(v) = t.get("efforts") {
        for (m, e) in as_table(v, "harness.efforts")? {
            if !e.is_str() {
                return Err(format!("`harness.efforts.{m}` must be a string"));
            }
        }
    }
    Ok(OldHarness { level, roles })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{self, Value};

    const SCHEMA_05: &str = include_str!("../tests/fixtures/upgrade/schema-0.5.0.json");

    fn fixture(rel: &str) -> String {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/upgrade")
            .join(rel);
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    fn keys(v: &Value) -> Vec<String> {
        match v.get("properties") {
            Value::Obj(f) => f.iter().map(|(k, _)| k.clone()).collect(),
            _ => Vec::new(),
        }
    }

    fn sorted(mut v: Vec<String>) -> Vec<String> {
        v.sort();
        v
    }

    fn of(keys: &[&str]) -> Vec<String> {
        sorted(keys.iter().map(|k| k.to_string()).collect())
    }

    // --- names ---

    #[test]
    fn the_table_of_types_has_the_10_types_of_0_5() {
        let names: Vec<&str> = TYPES.iter().map(|(n, _)| *n).collect();
        assert_eq!(
            sorted(names.iter().map(|n| n.to_string()).collect()),
            of(&[
                "change",
                "contract",
                "decision",
                "guide",
                "incident",
                "invariant",
                "note",
                "reference",
                "runbook",
                "spec"
            ])
        );
        let schema = json::parse(SCHEMA_05).unwrap();
        let listed: Vec<String> = schema
            .get("$defs")
            .get("type")
            .get("enum")
            .as_arr()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect();
        assert_eq!(
            sorted(listed),
            sorted(names.iter().map(|n| n.to_string()).collect())
        );
        for (old, new) in TYPES {
            assert_eq!(type_of(old), Some(new), "{old}");
        }
        assert_eq!(type_of("requirements"), None);
        assert_eq!(type_of("intent"), None);
    }

    #[test]
    fn a_renamed_type_is_a_name_that_0_6_does_not_have() {
        for (old, new) in [
            ("contract", DocType::Interface),
            ("invariant", DocType::Rule),
            ("guide", DocType::Procedure),
            ("runbook", DocType::Procedure),
            ("note", DocType::Research),
            ("reference", DocType::Research),
            ("change", DocType::Intent),
        ] {
            assert_eq!(renamed(old), Some(new), "{old}");
            assert_eq!(DocType::from_old_name(old), Some(new), "{old}");
        }
        for same in ["spec", "decision", "incident", "requirements", "bogus"] {
            assert_eq!(renamed(same), None, "{same}");
            assert_eq!(DocType::from_old_name(same), None, "{same}");
        }
    }

    #[test]
    fn a_root_of_one_type_follows_d140() {
        assert_eq!(root_to("spec"), Some(RootTo::Type(DocType::Requirements)));
        assert_eq!(root_to("runbook"), Some(RootTo::Type(DocType::Procedure)));
        assert_eq!(root_to("reference"), Some(RootTo::Type(DocType::Research)));
        assert_eq!(root_to("decision"), Some(RootTo::Type(DocType::Decision)));
        // No namespace for these two: a `research` root adds one to each id.
        assert_eq!(root_to("note"), Some(RootTo::MixedResearch));
        assert_eq!(root_to("change"), Some(RootTo::MixedResearch));
        assert_eq!(root_to("mixed"), None);
        assert_eq!(root_to("intent"), None);
        // A default type: a change becomes research.
        assert_eq!(default_type_to("change"), Some(DocType::Research));
        assert_eq!(default_type_to("note"), Some(DocType::Research));
        assert_eq!(default_type_to("spec"), Some(DocType::Requirements));
        assert_eq!(default_type_to("guide"), Some(DocType::Procedure));
        assert_eq!(default_type_to("intent"), None);
    }

    #[test]
    fn the_change_statuses_follow_the_mapping() {
        assert_eq!(change_status("proposed"), Some(Some("draft")));
        assert_eq!(change_status("accepted"), Some(Some("accepted")));
        assert_eq!(change_status("rejected"), Some(Some("closed")));
        // A `done` change becomes research, which has no status.
        assert_eq!(change_status("done"), Some(None));
        assert_eq!(change_status("active"), None);
        assert_eq!(change_status("closed"), None);
        // The 0.6 statuses of an intent are statuses of the type.
        for (_, new) in CHANGE_STATUSES {
            if let Some(n) = new {
                assert!(DocType::Intent.statuses().contains(&n), "{n}");
            }
        }
    }

    #[test]
    fn the_roles_of_0_5_follow_the_harness_table() {
        assert_eq!(
            ROLES,
            [("reviewer", Some("verifier")), ("spec-writer", None)]
        );
        let schema = json::parse(SCHEMA_05).unwrap();
        let roles = keys(
            schema
                .get("properties")
                .get("harness")
                .get("properties")
                .get("roles"),
        );
        for (old, _) in ROLES {
            assert!(roles.iter().any(|r| r == old), "{old}");
        }
    }

    #[test]
    fn the_defaults_of_0_5_are_the_table_of_docs_types() {
        // docs/types.md of 0.5.0, "budget" and "required sections".
        let budgets = [
            ("spec", Some((10_000, 20_000))),
            ("contract", Some((10_000, 20_000))),
            ("guide", Some((14_000, 28_000))),
            ("decision", Some((7_000, 14_000))),
            ("change", Some((10_000, 20_000))),
            ("note", None),
            ("invariant", Some((3_000, 6_000))),
            ("runbook", Some((8_000, 16_000))),
            ("incident", Some((6_000, 12_000))),
            ("reference", None),
        ];
        for (old, want) in budgets {
            assert_eq!(budget_of(old), want, "{old}");
        }
        assert_eq!(
            required_of("decision"),
            vec![vec!["Decision Outcome", "Decision"]]
        );
        assert_eq!(required_of("invariant"), vec![vec!["Rules"]]);
        assert_eq!(required_of("runbook"), vec![vec!["Steps"]]);
        assert_eq!(
            required_of("incident"),
            vec![vec!["Summary"], vec!["Root cause"]]
        );
        for none in ["spec", "contract", "guide", "change", "note", "reference"] {
            assert!(required_of(none).is_empty(), "{none}");
        }
    }

    // --- the keys of the 0.5.0 schema ---

    #[test]
    fn the_key_lists_are_the_keys_of_the_0_5_0_schema() {
        let schema = json::parse(SCHEMA_05).unwrap();
        assert_eq!(sorted(keys(&schema)), of(&TOP_KEYS));
        let root = schema.get("$defs").get("root");
        assert_eq!(sorted(keys(root)), of(&ROOT_KEYS));
        let harness = schema.get("properties").get("harness");
        assert_eq!(sorted(keys(harness)), of(&HARNESS_KEYS));
        let roles = harness.get("properties").get("roles");
        assert_eq!(sorted(keys(roles)), of(&HARNESS_ROLES.map(|(r, _)| r)));
        for (role, role_keys) in HARNESS_ROLES {
            assert_eq!(
                sorted(keys(roles.get("properties").get(role))),
                of(role_keys),
                "{role}"
            );
        }
        // The type keys of `[budgets]`, with `growth`.
        let mut budget_keys = keys(schema.get("properties").get("budgets"));
        budget_keys.sort();
        let mut want: Vec<String> = TYPES.iter().map(|(n, _)| n.to_string()).collect();
        want.push("growth".into());
        assert_eq!(budget_keys, sorted(want));
        // `evidence`, `lint` and `context` have one key each.
        let one = |k: &str| keys(schema.get("properties").get(k));
        assert_eq!(one("evidence"), ["tests"]);
        assert_eq!(one("lint"), ["max-words"]);
        assert_eq!(one("context"), ["budget"]);
    }

    // --- the reader ---

    /// The example of `docs/config.md` of 0.5.0, with each key.
    const FULL: &str = r#"spec-dna = "0.5.0"
profile = "standard"
types = ["spec", "decision", "invariant", "runbook", "incident", "reference"]
exclude = ["**/vendor/**"]
self-urls = ["https://example.org/acme/specs/blob/*/"]
install-source = "ssh://git@example.org/tools/spec-dna.git"

[spec-dna-sha256]
"spec-dna-0.5.0-x86_64-unknown-linux-gnu" = "0000000000000000000000000000000000000000000000000000000000000000"

[rules]
drift = "error"

[evidence]
tests = ["tests/**", "**/*_test.go"]

[[roots]]
path = "docs/"
type = "mixed"
default-type = "spec"
status = "draft"
exclude = ["drafts/**"]
legacy-ids = ["ADR-{n}", { alias = "KEP-{n}", file = "{n}-*" }]
legacy-keys = ["R-{n}"]
status-map = { released = "active", wip = "draft" }
required-sections = { decision = ["Decision Outcome", "Decision"], incident = [["Summary"], ["Root cause"]] }
amendment-sections = { decision = ["Amendments", "Nachtrag"] }
history-sections = { guide = ["Iteration log"], decision = ["Discussion"] }
companion-sections = { spec = ["Coverage table"] }
live-sections = { decision = ["Progress"] }
enforce-budgets = true
profile = "lite"
rules = { lint = "off" }
lint-severity = "off"
drift-severity = "warning"

[[roots]]
path = "."
type = "runbook"

[budgets]
growth = 4000

[budgets.spec]
soft = 10000
hard = 20000

[lint]
max-words = 30

[context]
budget = 8000

[repos]
platform = "https://example.org/platform.git"

[conventions]
contract = ["docs/README.md#contract-changes"]
decision = ["docs/adr/README.md#how-to-amend"]

[harness]
level = "full"
deny = ["NotebookEdit"]
context-meter = [300000, 600000]
subagent-meter = 250000
fanout-cap = 10

[harness.roles]
orchestrator = { claude-code = "opus", effort = "xhigh", fallback = ["opus", "sonnet"] }
advisor = { claude-code = "fable", agent = "opus", effort = "xhigh" }
reviewer = { claude-code = "opus", effort = "high" }
researcher = { claude-code = "haiku", effort = "high" }
implementer = { claude-code = "sonnet", effort = "high" }
spec-writer = { claude-code = "sonnet", effort = "high" }
helper = { claude-code = "haiku" }

[harness.efforts]
sonnet = "high"
"#;

    #[test]
    fn the_reader_accepts_each_key_of_the_0_5_0_schema() {
        let old = OldConfig::read(FULL).unwrap();
        assert_eq!(old.pin, "0.5.0");
        assert!(old.has_sums);
        assert_eq!(
            old.types.as_deref(),
            Some(
                &[
                    "spec",
                    "decision",
                    "invariant",
                    "runbook",
                    "incident",
                    "reference"
                ]
                .map(String::from)[..]
            )
        );
        assert_eq!(old.roots.len(), 2);
        let r = &old.roots[0];
        assert_eq!(r.path, "docs");
        assert_eq!(r.kind(), Kind::Mixed);
        assert_eq!(r.default_old_type(), "spec");
        assert_eq!(r.status.as_deref(), Some("draft"));
        assert_eq!(
            r.status_map,
            [
                ("released".to_string(), "active".to_string()),
                ("wip".to_string(), "draft".to_string())
            ]
        );
        assert_eq!(
            r.section_keys("required-sections"),
            ["decision", "incident"]
        );
        assert_eq!(r.section_keys("history-sections"), ["decision", "guide"]);
        assert_eq!(r.section_keys("live-sections"), ["decision"]);
        assert!(r.sections.iter().all(Option::is_some));
        // `.` is the whole repository.
        assert_eq!(old.roots[1].path, "");
        assert_eq!(old.roots[1].kind(), Kind::Single("runbook"));
        assert_eq!(old.roots[1].default_old_type(), "runbook");
        assert!(old.roots[1].sections.iter().all(Option::is_none));
        assert!(old.roots[1].section_keys("required-sections").is_empty());
        assert_eq!(old.budgets, ["spec"]);
        assert_eq!(old.conventions, ["contract", "decision"]);
        let h = old.harness.unwrap();
        assert_eq!(h.level, "full");
        assert_eq!(
            sorted(h.roles),
            of(&[
                "advisor",
                "helper",
                "implementer",
                "orchestrator",
                "researcher",
                "reviewer",
                "spec-writer"
            ])
        );
    }

    #[test]
    fn a_root_with_no_type_is_a_mixed_root_with_the_default_note() {
        let old = OldConfig::read("spec-dna = \"0.4.2\"\n[[roots]]\npath = \"docs\"\n").unwrap();
        assert_eq!(old.roots[0].kind(), Kind::Mixed);
        assert_eq!(old.roots[0].default_old_type(), "note");
        assert!(old.harness.is_none());
        assert!(!old.has_sums);
        assert!(old.types.is_none());
    }

    #[test]
    fn the_reader_reads_each_fixture_of_the_upgrade() {
        for (rel, pin, roots) in [
            ("v04/spec-dna.toml", "0.4.2", 2),
            ("v05/spec-dna.toml", "0.5.0", 4),
            ("brownfield/spec-dna.toml", "0.5.0", 7),
        ] {
            let old = OldConfig::read(&fixture(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"));
            assert_eq!(old.pin, pin, "{rel}");
            assert_eq!(old.roots.len(), roots, "{rel}");
        }
        let text = format!(
            "{}{}",
            fixture("v05/spec-dna.toml"),
            fixture("harness-full/spec-dna.toml.append")
        );
        let old = OldConfig::read(&text).unwrap();
        assert_eq!(old.harness.unwrap().level, "full");
    }

    #[test]
    fn the_reader_refuses_each_key_that_0_5_does_not_have() {
        let head = "spec-dna = \"0.6.0\"\n";
        let root = "[[roots]]\npath = \"docs\"\n";
        let refused = [
            // The top level.
            format!("{head}areas = [\"a\"]\n{root}"),
            format!("{head}language = \"en\"\n{root}"),
            format!("{head}{root}[layers.rules]\ntypes = [\"rule\"]\n"),
            format!("{head}{root}[groups]\nx = [\"a\"]\n"),
            format!("{head}{root}[change]\npolicy = \"none\"\n"),
            format!("{head}{root}[templates]\ndir = \"t\"\n"),
            format!("{head}{root}[evals]\ndir = \"evals\"\n"),
            format!("{head}{root}[lint]\nhistory-terms = [\"x\"]\n"),
            format!("{head}{root}[budgets]\nbogus = 1\n"),
            // A root.
            format!("{head}[[roots]]\npath = \"docs\"\nbogus = 1\n"),
            // The harness: the keys and the roles of 0.6.
            format!("{head}{root}[harness]\nlevel = \"lean\"\nskills = []\n"),
            format!("{head}{root}[harness]\nlevel = \"lean\"\nreview-md = false\n"),
            format!("{head}{root}[harness]\nlevel = \"lean\"\nprotect-tests = \"off\"\n"),
            format!("{head}{root}[harness]\nlevel = \"lean\"\n[harness.roles]\nverifier = {{}}\n"),
            format!(
                "{head}{root}[harness]\nlevel = \"full\"\n[harness.roles]\ncode-simplifier = {{}}\n"
            ),
            format!(
                "{head}{root}[harness]\nlevel = \"lean\"\n[harness.roles]\nhelper = {{ effort = \"high\" }}\n"
            ),
        ];
        for text in &refused {
            let e = OldConfig::read(text).unwrap_err();
            assert!(
                e.contains("unknown key") || e.contains("not a key"),
                "{text}\n{e}"
            );
        }
        let e = OldConfig::read(&format!("{head}{root}[layers.rules]\n")).unwrap_err();
        assert!(e.contains("`layers`"), "{e}");
    }

    #[test]
    fn the_reader_refuses_a_name_that_0_5_does_not_have() {
        let head = "spec-dna = \"0.5.0\"\n";
        for text in [
            format!("{head}types = [\"requirements\"]\n[[roots]]\npath = \"d\"\n"),
            format!("{head}[[roots]]\npath = \"d\"\ntype = \"procedure\"\n"),
            format!("{head}[[roots]]\npath = \"d\"\ndefault-type = \"intent\"\n"),
            format!(
                "{head}[[roots]]\npath = \"d\"\nrequired-sections = {{ rule = [\"Rules\"] }}\n"
            ),
            format!("{head}[[roots]]\npath = \"d\"\n[budgets.research]\nsoft = 1\n"),
            format!("{head}[[roots]]\npath = \"d\"\n[conventions]\nplan = [\"x\"]\n"),
            format!("{head}[[roots]]\npath = \"d\"\n[harness]\nlevel = \"max\"\n"),
        ] {
            let e = OldConfig::read(&text).unwrap_err();
            assert!(!e.is_empty(), "{text}");
        }
    }

    #[test]
    fn the_reader_refuses_a_bad_value_and_no_roots() {
        for text in [
            "spec-dna = 5\n[[roots]]\npath = \"d\"\n",
            "[[roots]]\npath = \"d\"\n",
            "spec-dna = \"0.5.0\"\n",
            "spec-dna = \"0.5.0\"\n[[roots]]\n",
            "spec-dna = \"0.5.0\"\nroots = 3\n",
            "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\nstatus-map = { a = 1 }\n",
            "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\nrules = { lint = \"loud\" }\n",
            "spec-dna = \"0.5.0\"\nprofile = \"loose\"\n[[roots]]\npath = \"d\"\n",
            "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\n[budgets.spec]\nsoft = \"x\"\n",
            "this is not toml",
        ] {
            assert!(OldConfig::read(text).is_err(), "{text}");
        }
    }
}

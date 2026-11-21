//! The new `spec-dna.toml` of `upgrade` (D138 to D140, D147). The plan edits the
//! text of a 0.4 or 0.5 file in 2 passes, and each edit is a splice at a span
//! (`src/config_text.rs`), so each comment and each other byte stay.
//!
//! - Pass 1 renames. It needs no fact about the documents. Its result is a
//!   valid 0.6 file, so `Config::parse` reads it and gives the roots (step 2 of
//!   the plan). It also merges the keys of 2 old types that become one, by
//!   their explicit values.
//! - Pass 2 adds. It takes the facts of the documents (`Facts`): the types in
//!   use, the roots with a migrated change, a change over the budget, and the
//!   roles whose agent file has no marker. It adds the keys that keep a 0.5
//!   document as it was, the layers, the policy and the parts of the harness.
//!   It also finishes the merge: the weaker value of the types in use.

use crate::config::{Config, DEFAULT_BUDGETS, Profile, Version};
use crate::config_text::{
    Seg, append_tables, at, delete_key, delete_table, insert_keys, k, key_text, line_ending, quote,
    rename_key, render_value, replace_strings, set_value,
};
use crate::harness::{Level, Role, Skill};
use crate::layer::{Graph, Layer, render_graph};
use crate::model::DocType;
use crate::repo::Fatal;
use crate::upgrade_old::{
    self as names, Kind, OldConfig, OldRoot, RootTo, SECTION_TABLES, change_status,
    default_type_to, root_to, type_of,
};
use std::collections::{BTreeMap, BTreeSet};
use toml::{Table, Value};

/// The facts of the documents that pass 2 needs. The caller reads them from
/// the tree (step 3 of the plan).
#[derive(Clone, Debug, Default)]
pub struct Facts {
    /// The 0.5 types of the documents: the types in use.
    pub old_types: BTreeSet<String>,
    /// The paths of the roots that hold a change that moves (D144).
    pub change_roots: BTreeSet<String>,
    /// A change that moves has more than 12,000 characters.
    pub change_over_budget: bool,
    /// A `done` change stays in place as a `research` document (D142).
    pub done_changes: bool,
    /// The keys of the roles whose agent file has no marker (D147).
    pub unmarked_roles: BTreeSet<String>,
    /// The types of the top files that `--layout` makes (D43): `roadmap`,
    /// `product`, `questions`, `terms` or `system`. The plan adds each of
    /// them to the types in use. Sorted, with no type twice.
    pub top_types: Vec<DocType>,
    /// The areas of `--areas` (D47), sorted. The plan writes the key `areas`.
    pub areas: BTreeSet<String>,
}

#[derive(Clone, Debug)]
pub struct ConfigOpts {
    /// The new pin: the version of the binary.
    pub pin: String,
    /// The sums of `spec-dna-pin.toml` of that version (`--pin-file`).
    pub sums: Option<Vec<(String, String)>>,
}

/// The result of pass 1.
#[derive(Clone, Debug)]
pub struct Pass1 {
    /// A valid 0.6 file.
    pub text: String,
    /// The table `[spec-dna-sha256]` was not in the file, and pass 1 added it.
    pub sums_added: bool,
    /// For each root whose `status` key ended: its path and the word. Each
    /// moved change of the root gets this word as its status (D140).
    pub root_statuses: Vec<(String, String)>,
    /// The explicit budget of each 0.5 type of `[budgets]`, with the 0.5
    /// default for a missing number. Pass 2 needs them, because pass 1 gave
    /// the keys their new names.
    explicit_budgets: BTreeMap<String, (u32, u32)>,
    /// For each root: the explicit required sections of each 0.5 type.
    explicit_required: Vec<BTreeMap<String, Vec<Vec<String>>>>,
}

/// The result of the plan.
#[derive(Clone, Debug)]
pub struct ConfigPlan {
    pub text: String,
    pub sums_added: bool,
    #[allow(dead_code)] // Only the unit tests read it (D193).
    pub root_statuses: Vec<(String, String)>,
    /// The 0.6 types in use: the layers of the file hold them.
    #[allow(dead_code)] // Only the unit tests read it.
    pub types: Vec<DocType>,
}

// --- helpers ---

fn invalid(e: impl std::fmt::Display) -> Fatal {
    Fatal::Config(format!(
        "upgrade cannot give a valid spec-dna.toml ({e}); write the change by hand, commit the change, then run `spec-dna upgrade` again"
    ))
}

fn parse_table(text: &str) -> Result<Table, Fatal> {
    text.parse::<Table>()
        .map_err(|e| Fatal::Config(format!("spec-dna.toml does not parse: {e}")))
}

/// The table at `path` of a text.
fn table_at(text: &str, path: &[Seg]) -> Result<Option<Table>, Fatal> {
    let mut cur = Value::Table(parse_table(text)?);
    for seg in path {
        let next = match (seg, &cur) {
            (Seg::Key(n), Value::Table(t)) => t.get(*n).cloned(),
            (Seg::At(i), Value::Array(a)) => a.get(*i).cloned(),
            _ => None,
        };
        match next {
            Some(v) => cur = v,
            None => return Ok(None),
        }
    }
    Ok(match cur {
        Value::Table(t) => Some(t),
        _ => None,
    })
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn str_array(items: &[String]) -> Value {
    Value::Array(items.iter().map(|s| Value::String(s.clone())).collect())
}

/// A list of headings, or a list of such lists, as groups.
fn groups_of(v: &Value) -> Vec<Vec<String>> {
    let Some(a) = v.as_array() else {
        return Vec::new();
    };
    if a.iter().all(Value::is_str) {
        if a.is_empty() {
            Vec::new()
        } else {
            vec![strs(v)]
        }
    } else {
        a.iter().map(strs).collect()
    }
}

fn groups_value(groups: &[Vec<String>]) -> Value {
    match groups {
        [one] => str_array(one),
        many => Value::Array(many.iter().map(|g| str_array(g)).collect()),
    }
}

fn norm(g: &[String]) -> Vec<String> {
    let mut n: Vec<String> = g.iter().map(|s| s.to_lowercase()).collect();
    n.sort();
    n
}

/// The groups that each list has.
fn shared(lists: &[Vec<Vec<String>>]) -> Vec<Vec<String>> {
    let Some((first, rest)) = lists.split_first() else {
        return Vec::new();
    };
    first
        .iter()
        .filter(|g| rest.iter().all(|l| l.iter().any(|h| norm(h) == norm(g))))
        .cloned()
        .collect()
}

fn union(lists: &[Vec<String>]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for l in lists {
        for s in l {
            if !out.contains(s) {
                out.push(s.clone());
            }
        }
    }
    out
}

/// The path `parent` and the key `key`.
fn join<'a>(parent: &[Seg<'a>], key: &'a str) -> Vec<Seg<'a>> {
    let mut p = parent.to_vec();
    p.push(k(key));
    p
}

/// The keys of a table by the 0.6 name that they get: a group of more than 1
/// key is a merge. The order is the order of the first key of a group.
fn groups_by_new(keys: &[String]) -> Vec<(&'static str, Vec<&str>)> {
    let mut out: Vec<(&'static str, Vec<&str>)> = Vec::new();
    for key in keys {
        let Some(new) = type_of(key).map(DocType::name) else {
            continue;
        };
        match out.iter_mut().find(|(n, _)| *n == new) {
            Some((_, olds)) => olds.push(key),
            None => out.push((new, vec![key])),
        }
    }
    out
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Merge {
    /// The required sections that each type has.
    Shared,
    /// The names of each type.
    Union,
}

/// Gives each key of the table at `path` its 0.6 name. The keys of 2 old types
/// that become one are merged: the shared required sections, or the union of
/// the names. A merge of required sections with no shared one writes no key.
/// The function edits the keys one by one, and if the table is an inline
/// table, which cannot lose a key at its span, it writes the table again.
fn merge_keys(orig: &str, path: &[Seg], keys: &[String], kind: Merge) -> Result<String, Fatal> {
    let groups = groups_by_new(keys);
    let rename_only = |text: &str| -> Result<String, Fatal> {
        let mut t = text.to_string();
        for (new, olds) in &groups {
            if *new != olds[0] {
                t = rename_key(&t, &join(path, olds[0]), new)?;
            }
        }
        Ok(t)
    };
    if groups.iter().all(|(_, o)| o.len() == 1) {
        return rename_only(orig);
    }
    let table = table_at(orig, path)?.unwrap_or_default();
    let merged = |olds: &[&str]| -> Option<Value> {
        let values: Vec<&Value> = olds.iter().filter_map(|o| table.get(*o)).collect();
        match kind {
            Merge::Union => Some(str_array(&union(
                &values.iter().map(|v| strs(v)).collect::<Vec<_>>(),
            ))),
            Merge::Shared => {
                let lists: Vec<Vec<Vec<String>>> = values.iter().map(|v| groups_of(v)).collect();
                let s = shared(&lists);
                if s.is_empty() {
                    None
                } else {
                    Some(groups_value(&s))
                }
            }
        }
    };
    // The keys one by one.
    let one_by_one = || -> Result<String, Fatal> {
        let mut t = orig.to_string();
        for (new, olds) in &groups {
            if olds.len() == 1 {
                if *new != olds[0] {
                    t = rename_key(&t, &join(path, olds[0]), new)?;
                }
                continue;
            }
            for other in &olds[1..] {
                t = delete_key(&t, &join(path, other))?;
            }
            match merged(olds) {
                Some(v) => {
                    t = set_value(&t, &join(path, olds[0]), &v)?;
                    if *new != olds[0] {
                        t = rename_key(&t, &join(path, olds[0]), new)?;
                    }
                }
                None => t = delete_key(&t, &join(path, olds[0]))?,
            }
        }
        if table_at(&t, path)?.is_some_and(|x| x.is_empty()) {
            t = delete_key(&t, path).or_else(|_| delete_table(&t, path))?;
        }
        Ok(t)
    };
    // The table again.
    let whole = || -> Result<String, Fatal> {
        let mut out = Table::new();
        for (new, olds) in &groups {
            let v = if olds.len() == 1 {
                table.get(olds[0]).cloned()
            } else {
                merged(olds)
            };
            if let Some(v) = v {
                out.insert((*new).to_string(), v);
            }
        }
        if out.is_empty() {
            delete_key(orig, path)
        } else {
            set_value(orig, path, &Value::Table(out))
        }
    };
    one_by_one().or_else(|_| whole())
}

// --- pass 1 ---

/// The weaker budget: the larger numbers, and no budget if one has none.
fn weaker(budgets: &[Option<(u32, u32)>]) -> Option<(u32, u32)> {
    let mut out = (0, 0);
    for b in budgets {
        let (s, h) = (*b)?;
        out = (out.0.max(s), out.1.max(h));
    }
    Some(out)
}

/// The explicit budgets of the old text. A missing number is the 0.5 default of
/// the type, or 0 for a type with no budget.
fn explicit_budgets(orig: &Table, old: &OldConfig) -> BTreeMap<String, (u32, u32)> {
    let mut out = BTreeMap::new();
    for ty in &old.budgets {
        let Some(t) = orig
            .get("budgets")
            .and_then(Value::as_table)
            .and_then(|b| b.get(ty))
            .and_then(Value::as_table)
        else {
            continue;
        };
        let def = names::budget_of(ty).unwrap_or((0, 0));
        let num = |key: &str, d: u32| {
            t.get(key)
                .and_then(Value::as_integer)
                .map_or(d, |n| n as u32)
        };
        out.insert(ty.clone(), (num("soft", def.0), num("hard", def.1)));
    }
    out
}

fn explicit_required(orig: &Table, old: &OldConfig) -> Vec<BTreeMap<String, Vec<Vec<String>>>> {
    old.roots
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let mut m = BTreeMap::new();
            let table = orig
                .get("roots")
                .and_then(Value::as_array)
                .and_then(|a| a.get(i))
                .and_then(|r| r.get(SECTION_TABLES[0]))
                .and_then(Value::as_table);
            for key in r.section_keys(SECTION_TABLES[0]) {
                if let Some(v) = table.and_then(|t| t.get(key)) {
                    m.insert(key.clone(), groups_of(v));
                }
            }
            m
        })
        .collect()
}

pub fn pass1(text: &str, old: &OldConfig, opts: &ConfigOpts) -> Result<Pass1, Fatal> {
    let orig = parse_table(text)?;
    let explicit_budgets = explicit_budgets(&orig, old);
    let explicit_required = explicit_required(&orig, old);
    let mut t = text.to_string();
    t = replace_strings(&t, &[k("spec-dna")], &|_| Some(opts.pin.clone()))?;
    let sums_added = edit_sums(&mut t, old, opts)?;
    if let Some(types) = &old.types {
        t = edit_types(&t, types)?;
    }
    let mut root_statuses = Vec::new();
    for (i, r) in old.roots.iter().enumerate() {
        t = edit_root(&t, i, r, &mut root_statuses)?;
    }
    t = edit_budgets(&t, old, &explicit_budgets)?;
    if !old.conventions.is_empty() {
        t = merge_keys(&t, &[k("conventions")], &old.conventions, Merge::Union)?;
    }
    if let Some(h) = &old.harness {
        for role in &h.roles {
            match names::ROLES.iter().find(|(o, _)| o == role) {
                Some((o, Some(new))) => {
                    t = rename_key(&t, &[k("harness"), k("roles"), k(o)], new)?;
                }
                Some((o, None)) => t = delete_key(&t, &[k("harness"), k("roles"), k(o)])?,
                None => {}
            }
        }
    }
    Config::parse(&t).map_err(invalid)?;
    Ok(Pass1 {
        text: t,
        sums_added,
        root_statuses,
        explicit_budgets,
        explicit_required,
    })
}

/// The table `[spec-dna-sha256]`: the sums of the pin file (D135).
fn edit_sums(t: &mut String, old: &OldConfig, opts: &ConfigOpts) -> Result<bool, Fatal> {
    let path = [k("spec-dna-sha256")];
    match (&opts.sums, old.has_sums) {
        (None, true) if old.pin != opts.pin => Err(Fatal::Config(
            "spec-dna.toml has `[spec-dna-sha256]`: give the file `spec-dna-pin.toml` of the release with `--pin-file`".into(),
        )),
        (None, _) => Ok(false),
        (Some(sums), true) => {
            let table = table_at(t, &[])?
                .and_then(|r| r.get("spec-dna-sha256").and_then(Value::as_table).cloned())
                .unwrap_or_default();
            let prefix = format!("spec-dna-{}-", old.pin);
            for (key, sum) in &table {
                let new = key
                    .strip_prefix(&prefix)
                    .map(|s| format!("spec-dna-{}-{s}", opts.pin))
                    .and_then(|n| sums.iter().find(|(k, _)| *k == n));
                let Some((new_key, new_sum)) = new else {
                    *t = delete_key(t, &join(&path, key))?;
                    continue;
                };
                let mut at_key = key.as_str();
                if new_key != key {
                    *t = rename_key(t, &join(&path, key), new_key)?;
                    at_key = new_key;
                }
                if sum.as_str() != Some(new_sum.as_str()) {
                    *t = set_value(t, &join(&path, at_key), &Value::String(new_sum.clone()))?;
                }
            }
            let now = table_at(t, &path)?.unwrap_or_default();
            let missing: Vec<(&str, Value)> = sums
                .iter()
                .filter(|(k, _)| !now.contains_key(k))
                .map(|(k, v)| (k.as_str(), Value::String(v.clone())))
                .collect();
            if !missing.is_empty() {
                *t = insert_keys(t, &path, &missing)?;
            }
            Ok(false)
        }
        (Some(sums), false) => {
            let mut block = String::from("[spec-dna-sha256]\n");
            for (name, sum) in sums {
                block.push_str(&format!("{} = {}\n", key_text(name), quote(sum)));
            }
            let eol = line_ending(t);
            append_tables(t, &block, eol);
            Ok(true)
        }
    }
}

/// The 0.6 names that a 0.5 type gives to the key `types`. `change` gives the
/// 3 types of the intent layer, which hold together (D14).
fn types_of(old: &str) -> Vec<String> {
    match type_of(old) {
        Some(DocType::Intent) => INTENT_TYPES.map(str::to_string).to_vec(),
        Some(d) => vec![d.name().to_string()],
        None => Vec::new(),
    }
}

/// The types of the intent layer: a change policy needs all of them.
pub const INTENT_TYPES: [&str; 3] = ["intent", "spec", "plan"];

/// The key `types`: the new names, with no name twice.
fn edit_types(t: &str, types: &[String]) -> Result<String, Fatal> {
    let mut mapped: Vec<String> = Vec::new();
    for o in types {
        for n in types_of(o) {
            if !mapped.contains(&n) {
                mapped.push(n);
            }
        }
    }
    // The list keeps its length only if no name merges and no name grows.
    let grows = types.iter().any(|o| type_of(o) == Some(DocType::Intent));
    if mapped.len() == types.len() && !grows {
        replace_strings(t, &[k("types")], &|s| {
            type_of(s).map(|d| d.name().to_string())
        })
    } else {
        // `set_value` changes an array of many lines at each element, so it
        // accepts a new list that has the same length by chance. That would
        // move the comments to other names. A list of another length makes
        // `set_value` refuse such an array, so a probe finds it.
        let mut probe = mapped.clone();
        probe.push(String::new());
        set_value(t, &[k("types")], &str_array(&probe))?;
        set_value(t, &[k("types")], &str_array(&mapped))
    }
}

/// The keys of the root `i` that name a type or a status.
fn edit_root(
    text: &str,
    i: usize,
    r: &OldRoot,
    statuses: &mut Vec<(String, String)>,
) -> Result<String, Fatal> {
    let mut t = text.to_string();
    let root = [k("roots"), at(i)];
    if let Kind::Single(old_ty) = r.kind() {
        let new = match root_to(old_ty) {
            Some(RootTo::Type(ty)) => ty.name(),
            Some(RootTo::MixedResearch) => "mixed",
            None => return Err(invalid(format!("`{old_ty}` is not a type of 0.5"))),
        };
        t = replace_strings(&t, &join(&root, "type"), &|_| Some(new.to_string()))?;
    }
    if let Some(d) = &r.default_type {
        let new = default_type_to(d).map(DocType::name);
        t = replace_strings(&t, &join(&root, "default-type"), &|_| {
            new.map(str::to_string)
        })?;
    }
    // A change has statuses that its new type, a `research`, has not.
    if r.default_old_type() == "change"
        && let Some(word) = &r.status
    {
        t = delete_key(&t, &join(&root, "status"))?;
        statuses.push((r.path.clone(), word.clone()));
    }
    if !r.status_map.is_empty() {
        t = edit_status_map(&t, i, r)?;
    }
    for (idx, name) in SECTION_TABLES.iter().enumerate() {
        if let Some(keys) = &r.sections[idx] {
            let kind = if idx == 0 {
                Merge::Shared
            } else {
                Merge::Union
            };
            t = merge_keys(&t, &join(&root, name), keys, kind)?;
        }
    }
    Ok(t)
}

/// The targets of `status-map`. In a root of one type `change`, a target is a
/// change status and gets its 0.6 name. In another root, `proposed`, `accepted`
/// and `rejected` are also decision statuses, so they stay. Each root loses an
/// entry to `done`, because no 0.6 type has that status.
fn edit_status_map(text: &str, i: usize, r: &OldRoot) -> Result<String, Fatal> {
    let path = [k("roots"), at(i), k("status-map")];
    let only_change = r.kind() == Kind::Single("change");
    let mut renames: Vec<(&str, &str)> = Vec::new();
    let mut deletes: Vec<&str> = Vec::new();
    for (word, to) in &r.status_map {
        match change_status(to) {
            Some(None) => deletes.push(word),
            Some(Some(new)) if only_change && new != to => renames.push((word, new)),
            _ => {}
        }
    }
    let mut t = text.to_string();
    if deletes.is_empty() {
        for (word, new) in renames {
            t = replace_strings(&t, &join(&path, word), &|_| Some(new.to_string()))?;
        }
        return Ok(t);
    }
    let mut table = table_at(&t, &path)?.unwrap_or_default();
    for d in &deletes {
        table.remove(*d);
    }
    for (word, new) in renames {
        table.insert(word.to_string(), Value::String(new.to_string()));
    }
    if table.is_empty() {
        delete_key(&t, &path)
    } else {
        set_value(&t, &path, &Value::Table(table))
    }
}

/// The budgets: each key gets its 0.6 name. The budgets of 2 old types that
/// become one are merged by their explicit values (pass 2 adds the types in
/// use that have no key).
fn edit_budgets(
    text: &str,
    old: &OldConfig,
    explicit: &BTreeMap<String, (u32, u32)>,
) -> Result<String, Fatal> {
    let mut t = text.to_string();
    for (new, olds) in groups_by_new(&old.budgets) {
        let path = [k("budgets")];
        if olds.len() > 1 {
            for other in &olds[1..] {
                t = delete_table(&t, &join(&path, other))?;
            }
        }
        if new != olds[0] {
            t = rename_key(&t, &join(&path, olds[0]), new)?;
        }
        if olds.len() > 1 {
            let merged = weaker(
                &olds
                    .iter()
                    .map(|o| explicit.get(*o).copied())
                    .collect::<Vec<_>>(),
            );
            if let Some(b) = merged {
                t = ensure_budget(&t, new, b)?;
            }
        }
    }
    Ok(t)
}

/// Gives the table `[budgets.<name>]` the numbers `soft` and `hard`.
fn ensure_budget(text: &str, name: &str, (soft, hard): (u32, u32)) -> Result<String, Fatal> {
    let path = [k("budgets"), k(name)];
    let mut t = text.to_string();
    let Some(table) = table_at(&t, &path)? else {
        let eol = line_ending(&t);
        let block = format!("[budgets.{name}]\nsoft = {soft}\nhard = {hard}\n");
        append_tables(&mut t, &block, eol);
        return Ok(t);
    };
    let mut missing: Vec<(&str, Value)> = Vec::new();
    for (key, v) in [("soft", soft), ("hard", hard)] {
        let new = Value::Integer(i64::from(v));
        match table.get(key) {
            Some(x) if *x == new => {}
            Some(_) => t = set_value(&t, &join(&path, key), &new)?,
            None => missing.push((key, new)),
        }
    }
    if !missing.is_empty() {
        t = insert_keys(&t, &path, &missing)?;
    }
    Ok(t)
}

// --- pass 2 ---

/// The 0.5 types in use: the key `types`, or the types of the documents and of
/// the roots.
fn in_use_old(old: &OldConfig, facts: &Facts) -> BTreeSet<String> {
    if let Some(types) = &old.types {
        return types.iter().cloned().collect();
    }
    let mut s = facts.old_types.clone();
    for r in &old.roots {
        if let Kind::Single(t) = r.kind() {
            s.insert(t.to_string());
        }
        if let Some(d) = &r.default_type {
            s.insert(d.clone());
        }
    }
    s
}

/// The 0.6 types in use, in the order of `DocType`.
pub fn types_in_use(old: &OldConfig, facts: &Facts) -> Vec<DocType> {
    let mut out: Vec<DocType> = in_use_old(old, facts)
        .iter()
        .filter_map(|o| type_of(o))
        .collect();
    // A change that moves needs its layer.
    if !facts.change_roots.is_empty() {
        out.push(DocType::Intent);
    }
    // A `done` change stays in place as a `research` document.
    if facts.done_changes {
        out.push(DocType::Research);
    }
    // A top file needs the layer of its type (D43).
    out.extend(facts.top_types.iter().copied());
    out.sort_by_key(|t| *t as usize);
    out.dedup();
    out
}

/// The layer graph of the types in use, with no limit key.
pub fn graph_without_limits(types: &[DocType]) -> Result<Graph, Fatal> {
    let layers: Vec<Layer> = Graph::implied(types)
        .layers()
        .iter()
        .cloned()
        .map(|mut l| {
            l.max_docs = None;
            l.max_chars = None;
            l.max_started = None;
            l.stale_days = None;
            l
        })
        .collect();
    Graph::new(layers).map_err(invalid)
}

pub fn pass2(
    p1: &Pass1,
    old: &OldConfig,
    facts: &Facts,
    _opts: &ConfigOpts,
) -> Result<ConfigPlan, Fatal> {
    let types = types_in_use(old, facts);
    let in_use = in_use_old(old, facts);
    let mut t = p1.text.clone();
    t = finish_budget_merges(&t, p1, &in_use)?;
    for (i, r) in old.roots.iter().enumerate() {
        t = finish_required_merge(&t, i, r, p1, &in_use)?;
        t = add_root_keys(&t, i, r, facts)?;
    }
    if facts.change_over_budget {
        let now = Config::parse(&t).map_err(invalid)?.budgets[DocType::Intent as usize];
        let want = (now.0.max(10_000), now.1.max(20_000));
        if now != want {
            t = ensure_budget(&t, DocType::Intent.name(), want)?;
        }
    }
    // The key `types` of the project must allow the type of a `done` change.
    if old.types.is_some() && facts.done_changes {
        let mut list = table_at(&t, &[])?
            .and_then(|r| r.get("types").map(strs))
            .unwrap_or_default();
        let research = DocType::Research.name().to_string();
        // An empty list means that every type is valid: it stays.
        if !list.is_empty() && !list.contains(&research) {
            list.push(research);
            t = set_value(&t, &[k("types")], &str_array(&list))?;
        }
    }
    // The key `types` must allow the type of each top file (D43). An empty
    // list means that every type is valid: it stays.
    if old.types.is_some() && !facts.top_types.is_empty() {
        let mut list = table_at(&t, &[])?
            .and_then(|r| r.get("types").map(strs))
            .unwrap_or_default();
        let before = list.len();
        if before > 0 {
            for ty in &facts.top_types {
                if !list.iter().any(|l| l == ty.name()) {
                    list.push(ty.name().to_string());
                }
            }
            if list.len() != before {
                t = set_value(&t, &[k("types")], &str_array(&list))?;
            }
        }
    }
    // The areas of `--areas`, sorted (D47). A 0.5 file has no key `areas`.
    if !facts.areas.is_empty() {
        let list: Vec<String> = facts.areas.iter().cloned().collect();
        t = insert_keys(&t, &[], &[("areas", str_array(&list))])?;
    }
    t = edit_harness(&t, old, facts)?;
    let graph = graph_without_limits(&types)?;
    let mut block = render_graph(&graph);
    block.push_str("\n[change]\npolicy = \"none\"\n");
    let eol = line_ending(&t);
    append_tables(&mut t, &block, eol);
    // The result is the plan, so it must give what the plan says.
    let c = Config::parse(&t).map_err(invalid)?;
    let paths = |c: &Config| c.roots.iter().map(|r| r.path.clone()).collect::<Vec<_>>();
    let ok = c.layers_declared
        && c.layers == graph
        && c.change_policy.name() == "none"
        && c.areas.iter().eq(facts.areas.iter())
        && paths(&c) == old.roots.iter().map(|r| r.path.clone()).collect::<Vec<_>>()
        && c.harness
            .as_ref()
            .is_none_or(|h| h.skills.is_empty() && !h.review_md && h.protect_tests.name() == "off");
    if !ok {
        return Err(invalid(
            "the file does not give the planned roots and layers",
        ));
    }
    Ok(ConfigPlan {
        text: t,
        sums_added: p1.sums_added,
        root_statuses: p1.root_statuses.clone(),
        types,
    })
}

/// The edits of a 0.6 file beyond the pin (D15, D34), and the edits that the
/// plan cannot make.
#[derive(Clone, Debug, Default)]
pub struct Repair {
    pub text: String,
    /// One line for each edit that the plan made, for the report.
    pub edits: Vec<String>,
    /// One line for each edit that the plan cannot make: the person makes it
    /// by hand.
    pub by_hand: Vec<String>,
    /// One line for each edit that the plan could not make and that stops
    /// nothing: the report gives it as a warning.
    pub warnings: Vec<String>,
}

/// The skill names of 0.6 that end in 0.7.0, with the names that replace them
/// (D4). `review` stays.
const SKILL_RENAMES: [(&str, &[&str]); 3] = [
    ("intent", &["brainstorm"]),
    ("spec", &["specify", "clarify"]),
    ("plan", &["write-plan"]),
];

/// The skills that a release adds, each with the first release that has it, in
/// the order of D2. A project of 0.6 had no way to choose the skills of 0.7.0
/// (ADR 0023, amendment of 2026-11-19). Since 0.7.0, no release adds a skill: the
/// skill `accept` of 0.8.0 part A merged into `review` (design 0.8.0, D71).
const NEW_SKILLS: [(Skill, Version); 6] = [
    (Skill::Execute, Version(0, 7, 0)),
    (Skill::Tdd, Version(0, 7, 0)),
    (Skill::Verify, Version(0, 7, 0)),
    (Skill::Finish, Version(0, 7, 0)),
    (Skill::Debug, Version(0, 7, 0)),
    (Skill::PostMortem, Version(0, 7, 0)),
];

/// The start of the report line that names the skills that `upgrade` adds. SD503
/// does not count such an edit of `spec-dna.toml` (`config::pin_upgrade_only_change`),
/// and the line `next:` of `upgrade` reads the start to say so.
pub const SKILLS_ADDED: &str = "`skills` of `[harness]`: added ";

/// "0.7.0", "0.8.0" or "0.7.0 and 0.8.0": the releases that gave the skills.
fn since_label(skills: &[Skill]) -> String {
    let mut from: Vec<Version> = NEW_SKILLS
        .iter()
        .filter(|(s, _)| skills.contains(s))
        .map(|(_, v)| *v)
        .collect();
    from.sort();
    from.dedup();
    let names: Vec<String> = from
        .iter()
        .map(|Version(a, b, c)| format!("{a}.{b}.{c}"))
        .collect();
    names.join(" and ")
}

/// The skills of `NEW_SKILLS` that the text needs, with the words of the report
/// that name the reason. A release after the pin of the text added each such
/// skill, `init` proposes it for the profile and the level of the text, and
/// `list` lacks it. The result is empty when the text has no valid pin or level,
/// when the list is empty, or when the level or the change policy is `none`.
/// A text that does not parse has no result.
fn new_skills(table: &Table, list: &[String], on: &dyn Fn(Skill) -> bool) -> (Vec<Skill>, String) {
    let none = (Vec::new(), String::new());
    let pin = table
        .get("spec-dna")
        .and_then(Value::as_str)
        .and_then(Version::parse);
    let level = table
        .get("harness")
        .and_then(|h| h.get("level"))
        .and_then(Value::as_str)
        .and_then(Level::from_name);
    let profile = match table.get("profile") {
        None => Some(Profile::Standard),
        Some(p) => p.as_str().and_then(Profile::from_name),
    };
    // `init` writes no skill for the policy `none`, and a file with no
    // `[change]` table has that policy.
    let policy = table
        .get("change")
        .and_then(|c| c.get("policy"))
        .and_then(Value::as_str);
    let (Some(pin), false, Some(level), Some(profile)) = (pin, list.is_empty(), level, profile)
    else {
        return none;
    };
    if level == Level::None || policy.is_none_or(|p| p == "none") {
        return none;
    }
    let proposed = crate::init::profile_skills(profile, level, on);
    let added: Vec<Skill> = NEW_SKILLS
        .into_iter()
        .filter(|(s, from)| {
            pin < *from && proposed.contains(s) && !list.iter().any(|x| x == s.key())
        })
        .map(|(s, _)| s)
        .collect();
    let why = if level == Level::Full {
        "on at the level `full`".to_string()
    } else {
        format!("on by default for the profile `{}`", profile.name())
    };
    (added, why)
}

/// Renames the skills of 0.6 in `[harness] skills` (D4, D41, ADR 0023):
/// `intent` is `brainstorm`, `spec` is `specify` and `clarify`, and `plan` is
/// `write-plan`. A new name that is on by default (`Skill::default_on`) goes
/// into the list. A new name that is off by default is left out, and a line of
/// `edits` names it and the fix. A name that the list already has stays, also
/// when its default is off, because the person chose it. A name is not added
/// twice, and the new list has the order of D2. Only the value of `skills` in
/// `[harness]` changes: the words `intent`, `spec` and `plan` are also the
/// names of types. The call is before the first parse of the file, because the
/// parser gives exit code 3 for an old name. A list with no old name, and a
/// text that does not parse, stay as they are, so a second run changes
/// nothing.
///
/// A list that is not empty gets a new skill in this case: the file pins a
/// version before the release that gave the skill, and `init` proposes the
/// skill for the profile and the level of the file (ADR 0023, amendment of
/// 2026-11-19). The skills are `tdd`, `execute`, `verify`, `finish`, `debug` and
/// `post-mortem` (0.7.0). A list with no old
/// name keeps its order, and the new names follow in the order of D2. An empty
/// list stays empty. A list of many lines that no rename touches stays as it
/// is, and a warning gives the names to add by hand.
pub fn rename_skills(text: &str) -> Repair {
    rename_skills_with(text, &|s| s.default_on())
}

/// `rename_skills` with the default of a skill given by `on`, so a test can
/// pass a fake table.
pub fn rename_skills_with(text: &str, on: &dyn Fn(Skill) -> bool) -> Repair {
    let mut out = Repair {
        text: text.to_string(),
        ..Repair::default()
    };
    let Ok(table) = text.parse::<Table>() else {
        return out;
    };
    let Some(list) = table
        .get("harness")
        .and_then(|h| h.get("skills"))
        .filter(|v| v.is_array())
        .map(strs)
    else {
        return out;
    };
    let old_of = |n: &str| SKILL_RENAMES.iter().find(|r| r.0 == n);
    let found: Vec<&(&str, &[&str])> = list.iter().filter_map(|n| old_of(n)).collect();
    let (added, why) = new_skills(&table, &list, on);
    if found.is_empty() && added.is_empty() {
        return out;
    }
    // A new name goes into the list if it is on by default or if the list
    // has it already.
    let wanted =
        |name: &str| list.iter().any(|x| x == name) || Skill::from_key(name).is_some_and(on);
    let mut new: Vec<String> = Vec::new();
    for n in &list {
        let names: Vec<&str> = match old_of(n) {
            Some((_, to)) => to.iter().copied().filter(|t| wanted(t)).collect(),
            None => vec![n.as_str()],
        };
        for name in names {
            if !new.iter().any(|x| x == name) {
                new.push(name.to_string());
            }
        }
    }
    // The skills that are new in 0.7.0 follow the old list.
    let added: Vec<Skill> = added
        .into_iter()
        .filter(|s| !new.iter().any(|x| x == s.key()))
        .collect();
    new.extend(added.iter().map(|s| s.key().to_string()));
    // A rename gives the order of D2. A list with no old name keeps its order.
    // An unknown name stays at the end, and the parser reports it.
    if !found.is_empty() {
        new.sort_by_key(|n| Skill::from_key(n).map_or(usize::MAX, |s| s as usize));
    }
    let added_names: Vec<String> = added.iter().map(|s| s.key().to_string()).collect();
    let said: Vec<String> = found
        .iter()
        .map(|(from, to)| {
            let kept: Vec<String> = to
                .iter()
                .filter(|t| new.iter().any(|x| x == *t))
                .map(|t| t.to_string())
                .collect();
            if kept.is_empty() {
                format!("`{from}` ended")
            } else {
                format!("`{from}` is now {}", code_list(&kept))
            }
        })
        .collect();
    // One line for each new name that the rule leaves out.
    let left_out: Vec<(&str, &str)> = found
        .iter()
        .flat_map(|(from, to)| to.iter().map(move |t| (*from, *t)))
        .filter(|(_, t)| !new.iter().any(|x| x == t))
        .collect();
    // The list of the rename alone: the list that 0.7.0 gave before the new
    // skills joined it.
    let rename_only: Vec<String> = new
        .iter()
        .filter(|n| !added_names.contains(n))
        .cloned()
        .collect();
    let path = [k("harness"), k("skills")];
    // A list of many lines can take the rename and not the longer list. Then
    // the rename goes in, as in 0.7.0, and a warning names the new skills.
    let (spliced, added_in) = match set_value(text, &path, &str_array(&new)) {
        Ok(t) => (Ok(t), true),
        Err(e) if !found.is_empty() && !added.is_empty() => {
            match set_value(text, &path, &str_array(&rename_only)) {
                Ok(t) => (Ok(t), false),
                Err(_) => (Err(e), false),
            }
        }
        Err(e) => (Err(e), true),
    };
    let since = since_label(&added);
    let not_added = |why: &str| {
        format!(
            "`skills` of `[harness]` is a list of many lines, so upgrade did not add {}, new in {since} and {why}; add {} by hand to turn {} on",
            code_list(&added_names),
            if added_names.len() == 1 {
                "the name"
            } else {
                "the names"
            },
            if added_names.len() == 1 {
                "its skill"
            } else {
                "each skill"
            },
        )
    };
    match spliced {
        Ok(t) => {
            out.text = t;
            if !found.is_empty() {
                out.edits.push(format!(
                    "`skills` of `[harness]` got the names of 0.7.0: {}",
                    said.join(", ")
                ));
            }
            for (from, to) in left_out {
                out.edits.push(format!(
                    "`skills`: `{from}` ended, and `{to}` is off by default in 0.7.0 (no verdict `on`); add \"{to}\" to `skills` of [harness] to turn it on"
                ));
            }
            if !added.is_empty() && added_in {
                out.edits.push(format!(
                    "{SKILLS_ADDED}{}, new in {since} and {why}; remove a name to turn its skill off",
                    code_list(&added_names)
                ));
            } else if !added.is_empty() {
                out.warnings.push(not_added(&why));
            }
        }
        // A list of many lines that no rename touches: the upgrade went
        // before, so it goes now, and the report names the names to add.
        Err(_) if found.is_empty() => out.warnings.push(not_added(&why)),
        Err(_) => {
            let quoted: Vec<String> = rename_only.iter().map(|n| format!("\"{n}\"")).collect();
            let mut message = format!(
                "`skills` of `[harness]` has names of 0.6 ({}); the list has many lines, so change it by hand to [{}], then run `spec-dna upgrade` again",
                said.join(", "),
                quoted.join(", ")
            );
            if !left_out.is_empty() {
                let names: Vec<String> = left_out.iter().map(|(_, t)| t.to_string()).collect();
                message.push_str(&format!(
                    ". The list leaves out {}, which {} off by default in 0.7.0 (no verdict `on`); add {} to turn {} on",
                    code_list(&names),
                    if names.len() == 1 { "is" } else { "are" },
                    if names.len() == 1 { "it" } else { "them" },
                    if names.len() == 1 { "it" } else { "each" },
                ));
            }
            out.by_hand.push(message)
        }
    }
    out
}

/// "`a`", "`a` and `b`" or "`a`, `b` and `c`".
fn code_list(items: &[String]) -> String {
    let q: Vec<String> = items.iter().map(|i| format!("`{i}`")).collect();
    match q.split_last() {
        None => String::new(),
        Some((last, [])) => last.clone(),
        Some((last, head)) => format!("{} and {last}", head.join(", ")),
    }
}

/// The 2 repairs of a 0.6 file. Both add values only, and both are exact rules.
/// The caller reads `cfg` from `text`.
///
/// - D15. The file has the key `types`, the change policy is not `none`, and
///   the list lacks a type of the intent layer. The plan adds the absent types.
/// - D34. The `linked-from` of `[layers.pending]` is exactly `["intent"]`, the
///   value of `init` 0.6.0. The plan adds `requirements` and `decisions`, the
///   layers that the graph has.
///
/// A list of many lines keeps its comments only if its length stays, so the
/// plan cannot add to it. It leaves the list and names the edit.
pub fn repair_current(text: &str, cfg: &Config) -> Result<Repair, Fatal> {
    let table = parse_table(text)?;
    let mut out = Repair {
        text: text.to_string(),
        ..Repair::default()
    };
    let policy = cfg.change_policy.name();
    if policy != "none"
        && let Some(list) = table
            .get("types")
            .filter(|v| v.is_array())
            .map(strs)
            // An empty list means that every type is valid: it stays.
            .filter(|l| !l.is_empty())
    {
        let absent: Vec<String> = INTENT_TYPES
            .iter()
            .filter(|n| !list.iter().any(|l| l == *n))
            .map(|n| n.to_string())
            .collect();
        if !absent.is_empty() {
            let all = [list, absent.clone()].concat();
            match set_value(&out.text, &[k("types")], &str_array(&all)) {
                Ok(t) => {
                    out.text = t;
                    out.edits.push(format!(
                        "`types` got {}: the change policy `{policy}` needs the types `intent`, `spec` and `plan`",
                        code_list(&absent)
                    ));
                }
                Err(_) => out.by_hand.push(format!(
                    "`types` lacks {}, which the change policy `{policy}` needs; the list has many lines, so add the types by hand",
                    code_list(&absent)
                )),
            }
        }
    }
    let links = table
        .get("layers")
        .and_then(|l| l.get("pending"))
        .and_then(|p| p.get("linked-from"))
        .map(strs)
        .filter(|l| l == &["intent"]);
    if let Some(mut links) = links {
        let add: Vec<String> = ["requirements", "decisions"]
            .iter()
            .filter(|n| cfg.layers.index(n).is_some())
            .map(|n| n.to_string())
            .collect();
        if !add.is_empty() {
            links.extend(add.iter().cloned());
            let path = [k("layers"), k("pending"), k("linked-from")];
            match set_value(&out.text, &path, &str_array(&links)) {
                Ok(t) => {
                    out.text = t;
                    out.edits.push(format!(
                        "`linked-from` of `[layers.pending]` got {}: `init` 0.6.0 wrote only `intent`",
                        code_list(&add)
                    ));
                }
                Err(_) => out.by_hand.push(format!(
                    "`linked-from` of `[layers.pending]` is only `intent`; add {} by hand",
                    code_list(&add)
                )),
            }
        }
    }
    if out.text != text {
        Config::parse(&out.text).map_err(invalid)?;
    }
    Ok(out)
}

/// The budget of 2 old types that become one is the weaker value of the types
/// in use, also of a type that has no key (D139). A key is only in the file if
/// its value is not the 0.6 default.
fn finish_budget_merges(
    text: &str,
    p1: &Pass1,
    in_use: &BTreeSet<String>,
) -> Result<String, Fatal> {
    let mut t = text.to_string();
    for (ty, olds) in [
        (DocType::Procedure, ["guide", "runbook"]),
        (DocType::Research, ["note", "reference"]),
    ] {
        let present: Vec<&str> = olds
            .iter()
            .copied()
            .filter(|o| p1.explicit_budgets.contains_key(*o) || in_use.contains(*o))
            .collect();
        if present.len() < 2 {
            continue;
        }
        let values: Vec<Option<(u32, u32)>> = present
            .iter()
            .map(|o| {
                p1.explicit_budgets
                    .get(*o)
                    .copied()
                    .or_else(|| names::budget_of(o))
            })
            .collect();
        let want = weaker(&values).unwrap_or(DEFAULT_BUDGETS[ty as usize]);
        let now = Config::parse(&t).map_err(invalid)?.budgets[ty as usize];
        // A type with no budget has the default (0, 0).
        let want = if values.contains(&None) { (0, 0) } else { want };
        if now == want {
            continue;
        }
        t = if want == DEFAULT_BUDGETS[ty as usize] {
            delete_table(&t, &[k("budgets"), k(ty.name())])?
        } else {
            ensure_budget(&t, ty.name(), want)?
        };
    }
    Ok(t)
}

/// The required sections of `guide` and `runbook` in a root that has both: the
/// sections that both have, with the 0.5 default of a type with no key.
fn finish_required_merge(
    text: &str,
    i: usize,
    r: &OldRoot,
    p1: &Pass1,
    in_use: &BTreeSet<String>,
) -> Result<String, Fatal> {
    if r.kind() != Kind::Mixed {
        return Ok(text.to_string());
    }
    let explicit = &p1.explicit_required[i];
    let olds = ["guide", "runbook"];
    let present: Vec<&str> = olds
        .iter()
        .copied()
        .filter(|o| explicit.contains_key(*o) || in_use.contains(*o))
        .collect();
    if present.len() < 2 || !olds.iter().any(|o| explicit.contains_key(*o)) {
        return Ok(text.to_string());
    }
    let lists: Vec<Vec<Vec<String>>> = present
        .iter()
        .map(|o| {
            explicit.get(*o).cloned().unwrap_or_else(|| {
                names::required_of(o)
                    .iter()
                    .map(|g| g.iter().map(|s| s.to_string()).collect())
                    .collect()
            })
        })
        .collect();
    let want = shared(&lists);
    let now = Config::parse(text).map_err(invalid)?.roots[i].required_sections
        [DocType::Procedure as usize]
        .clone();
    let same = |a: &[Vec<String>], b: &[Vec<String>]| {
        let n = |x: &[Vec<String>]| {
            let mut v: Vec<Vec<String>> = x.iter().map(|g| norm(g)).collect();
            v.sort();
            v
        };
        n(a) == n(b)
    };
    if same(&now, &want) {
        return Ok(text.to_string());
    }
    let path = [k("roots"), at(i), k("required-sections")];
    let mut table = table_at(text, &path)?.unwrap_or_default();
    if want.is_empty() {
        table.remove("procedure");
    } else {
        table.insert("procedure".into(), groups_value(&want));
    }
    if table.is_empty() {
        delete_key(text, &path)
    } else {
        set_value(text, &path, &Value::Table(table))
    }
}

/// The keys that keep a 0.5 document as it was (D139): the default type, the
/// required sections and the live sections of a root.
fn add_root_keys(text: &str, i: usize, r: &OldRoot, facts: &Facts) -> Result<String, Fatal> {
    let mut t = text.to_string();
    let path = [k("roots"), at(i)];
    let root = table_at(&t, &path)?.unwrap_or_default();
    let mixed = root
        .get("type")
        .and_then(Value::as_str)
        .is_none_or(|x| x == "mixed");
    let mut entries: Vec<(&str, Value)> = Vec::new();
    if mixed && !root.contains_key("default-type") {
        entries.push(("default-type", Value::String("research".into())));
    }
    // The sections that a 0.5 document needs, and the 0.6 default does not give.
    let mut want = Table::new();
    if mixed && facts.change_roots.contains(&r.path) {
        want.insert("intent".into(), Value::Array(Vec::new()));
    }
    if r.kind() == Kind::Single("runbook") {
        want.insert("procedure".into(), str_array(&["Steps".to_string()]));
    }
    if !want.is_empty() {
        match root.get("required-sections").and_then(Value::as_table) {
            Some(table) => {
                let mut new = table.clone();
                for (key, v) in want {
                    new.entry(key).or_insert(v);
                }
                if &new != table {
                    t = set_value(&t, &join(&path, "required-sections"), &Value::Table(new))?;
                }
            }
            None => entries.push(("required-sections", Value::Table(want))),
        }
    }
    // The 0.6 default `{ decision = ["Status"] }` is for a decision. A root
    // that can hold a decision turns it off (D188). `{}` would keep it.
    let decision_root = root.get("type").and_then(Value::as_str) == Some("decision");
    if !root.contains_key("live-sections") && (mixed || decision_root) {
        let mut off = Table::new();
        off.insert("decision".into(), Value::Array(Vec::new()));
        entries.push(("live-sections", Value::Table(off)));
    }
    if !entries.is_empty() {
        t = insert_keys(&t, &path, &entries)?;
    }
    Ok(t)
}

/// The parts of the harness that 0.6 adds, off, and the roles that stay off.
fn edit_harness(text: &str, old: &OldConfig, facts: &Facts) -> Result<String, Fatal> {
    let Some(h) = &old.harness else {
        return Ok(text.to_string());
    };
    let mut t = text.to_string();
    let parts = [
        ("skills", Value::Array(Vec::new())),
        ("review-md", Value::Boolean(false)),
        ("protect-tests", Value::String("off".into())),
    ];
    t = insert_keys(&t, &[k("harness")], &parts)?;
    let level = Level::from_name(&h.level).unwrap_or(Level::None);
    // The role `code-simplifier` is new. The role of an agent file with no
    // marker is off, so `upgrade` never replaces that file (D147).
    let mut adds: Vec<(&'static str, Table)> = Vec::new();
    for role in Role::ALL {
        if !level.includes(role) {
            continue;
        }
        let key = role.key();
        let mut tbl = Table::new();
        if role == Role::CodeSimplifier {
            tbl.insert("claude-code".into(), Value::String("none".into()));
        }
        if facts.unmarked_roles.contains(key) {
            match role {
                Role::Advisor => {
                    tbl.insert("agent".into(), Value::String("none".into()));
                }
                Role::Orchestrator | Role::Helper => {}
                _ => {
                    tbl.insert("claude-code".into(), Value::String("none".into()));
                }
            }
        }
        if !tbl.is_empty() {
            adds.push((key, tbl));
        }
    }
    if adds.is_empty() {
        return Ok(t);
    }
    let roles_path = [k("harness"), k("roles")];
    match table_at(&t, &roles_path)? {
        Some(roles) => {
            let mut missing: Vec<(&str, Value)> = Vec::new();
            for (key, tbl) in adds {
                match roles.get(key).and_then(Value::as_table) {
                    Some(existing) => {
                        let mut new = existing.clone();
                        for (a, b) in tbl {
                            new.insert(a, b);
                        }
                        t = set_value(&t, &join(&roles_path, key), &Value::Table(new))?;
                    }
                    None => missing.push((key, Value::Table(tbl))),
                }
            }
            if !missing.is_empty() {
                t = insert_keys(&t, &roles_path, &missing)?;
            }
        }
        None => {
            let mut block = String::from("[harness.roles]\n");
            for (key, tbl) in adds {
                block.push_str(&format!(
                    "{} = {}\n",
                    key_text(key),
                    render_value(&Value::Table(tbl))
                ));
            }
            let eol = line_ending(&t);
            append_tables(&mut t, &block, eol);
        }
    }
    Ok(t)
}

/// Pass 1, then pass 2. The command calls the 2 passes itself, because the
/// documents need the result of pass 1 between them.
#[cfg(test)]
pub fn plan(
    text: &str,
    old: &OldConfig,
    facts: &Facts,
    opts: &ConfigOpts,
) -> Result<ConfigPlan, Fatal> {
    pass2(&pass1(text, old, opts)?, old, facts, opts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn tests_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests")
    }

    fn read(path: PathBuf) -> String {
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    }

    fn opts() -> ConfigOpts {
        ConfigOpts {
            pin: "0.6.0".into(),
            sums: None,
        }
    }

    fn facts(old_types: &[&str], change_roots: &[&str]) -> Facts {
        Facts {
            old_types: old_types.iter().map(|s| s.to_string()).collect(),
            change_roots: change_roots.iter().map(|s| s.to_string()).collect(),
            ..Facts::default()
        }
    }

    fn message(e: &Fatal) -> String {
        match e {
            Fatal::Config(m) => m.clone(),
            other => panic!("not a configuration error: {other:?}"),
        }
    }

    fn try_plan(text: &str, f: &Facts, o: &ConfigOpts) -> Result<ConfigPlan, Fatal> {
        let old = OldConfig::read(text).unwrap_or_else(|e| panic!("{e}\n{text}"));
        plan(text, &old, f, o)
    }

    fn go(text: &str, f: &Facts) -> ConfigPlan {
        try_plan(text, f, &opts()).unwrap_or_else(|e| panic!("{}\n{text}", message(&e)))
    }

    fn cfg(text: &str) -> Config {
        Config::parse(text).unwrap_or_else(|e| panic!("{e}\n{text}"))
    }

    const ROOT: &str = "[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\nlive-sections = {}\n";

    fn with_root(rest: &str) -> String {
        format!("spec-dna = \"0.5.0\"\n{ROOT}{rest}")
    }

    // --- the golden pairs ---

    /// The facts of the documents of a fixture, from the file that the 0.5.0
    /// binary recorded (`tests/fixtures/upgrade/recorded/`). The 0.5 type of a
    /// document is the type in the record. A change that moves is a change
    /// whose status is not `done`, and its root is the deepest root of its
    /// path. No change is over the budget in the fixtures.
    fn recorded_facts(case: &str, old: &OldConfig) -> Facts {
        let rec = read(tests_dir().join(format!("fixtures/upgrade/recorded/{case}.txt")));
        let mut f = Facts::default();
        for line in rec.lines().filter_map(|l| l.strip_prefix("doc ")) {
            let parts: Vec<&str> = line.split(' ').collect();
            let (path, ty, status) = (parts[0], parts[2], parts[3]);
            f.old_types.insert(ty.to_string());
            if ty == "change" && status == "done" {
                f.done_changes = true;
            }
            if ty == "change" && status != "done" {
                let root = old
                    .roots
                    .iter()
                    .filter(|r| r.path.is_empty() || path.starts_with(&format!("{}/", r.path)))
                    .max_by_key(|r| r.path.len())
                    .expect("a root holds the document");
                f.change_roots.insert(root.path.clone());
            }
        }
        f
    }

    /// The pairs of the plan: the text before and after, for each fixture. The
    /// files were written by hand from the rules of D138 to D140, so
    /// `UPDATE_GOLDEN=1` is only for a change of the rules, after a review.
    #[test]
    fn the_plan_gives_the_golden_pairs() {
        let dir = tests_dir().join("golden/upgrade/config");
        let fixture = |name: &str| read(tests_dir().join("fixtures/upgrade").join(name));
        let cases: [(&str, String); 5] = [
            ("v04", fixture("v04/spec-dna.toml")),
            ("v05", fixture("v05/spec-dna.toml")),
            ("brownfield", fixture("brownfield/spec-dna.toml")),
            (
                "harness-lean",
                fixture("v05/spec-dna.toml") + &fixture("harness-lean/spec-dna.toml.append"),
            ),
            (
                "harness-full",
                fixture("v05/spec-dna.toml") + &fixture("harness-full/spec-dna.toml.append"),
            ),
        ];
        for (name, before) in cases {
            assert_eq!(
                read(dir.join(format!("{name}.before.toml"))),
                before,
                "{name}: the before file is the fixture"
            );
            let old = OldConfig::read(&before).unwrap();
            let f = recorded_facts(name, &old);
            let got = plan(&before, &old, &f, &opts()).unwrap_or_else(|e| {
                panic!("{name}: {}", message(&e));
            });
            let path = dir.join(format!("{name}.after.toml"));
            if std::env::var_os("UPDATE_GOLDEN").is_some() {
                std::fs::write(&path, &got.text).unwrap();
            }
            assert_eq!(
                got.text,
                read(path),
                "{name}: run with UPDATE_GOLDEN=1 to accept"
            );
        }
    }

    /// What the key `live-sections` is for (D139, D188): no live section for a
    /// 0.5 decision. A root that cannot hold a decision has no key.
    #[test]
    fn the_plan_gives_no_live_section_to_a_decision() {
        let dir = tests_dir().join("golden/upgrade/config");
        let before = read(dir.join("v05.before.toml"));
        let old = OldConfig::read(&before).unwrap();
        let f = recorded_facts("v05", &old);
        let c = cfg(&plan(&before, &old, &f, &opts()).unwrap().text);
        for r in &c.roots {
            assert!(
                r.live_sections.iter().all(Vec::is_empty),
                "{} has a live section",
                r.path
            );
        }
    }

    #[test]
    fn each_comment_of_a_fixture_stays() {
        let dir = tests_dir().join("golden/upgrade/config");
        for name in ["v04", "v05", "brownfield", "harness-lean", "harness-full"] {
            let before = read(dir.join(format!("{name}.before.toml")));
            let after = read(dir.join(format!("{name}.after.toml")));
            let comments: Vec<&str> = before.lines().filter(|l| l.starts_with('#')).collect();
            assert!(!comments.is_empty(), "{name}");
            let old = OldConfig::read(&before).unwrap();
            let f = recorded_facts(name, &old);
            let got = plan(&before, &old, &f, &opts()).unwrap().text;
            for c in comments {
                assert!(got.lines().any(|l| l == c), "{name}: lost `{c}`");
                assert!(after.lines().any(|l| l == c), "{name}: golden lost `{c}`");
            }
        }
    }

    /// The property of the plan: the new file loads with the 0.6 loader, and
    /// it has the parts that the plan promises. The first pass loads alone too.
    #[test]
    fn the_new_text_loads_with_the_0_6_loader() {
        let dir = tests_dir().join("golden/upgrade/config");
        for name in ["v04", "v05", "brownfield", "harness-lean", "harness-full"] {
            let before = read(dir.join(format!("{name}.before.toml")));
            let old = OldConfig::read(&before).unwrap();
            let f = recorded_facts(name, &old);
            let p1 = pass1(&before, &old, &opts()).unwrap_or_else(|e| panic!("{}", message(&e)));
            let c1 = cfg(&p1.text);
            assert_eq!(c1.pin, "0.6.0", "{name}");
            assert!(!c1.layers_declared, "{name}: pass 1 adds no layer");
            let paths = |c: &Config| c.roots.iter().map(|r| r.path.clone()).collect::<Vec<_>>();
            assert_eq!(
                paths(&c1),
                old.roots.iter().map(|r| r.path.clone()).collect::<Vec<_>>(),
                "{name}: each document stays in its 0.5 root"
            );
            let got = pass2(&p1, &old, &f, &opts()).unwrap();
            let c = cfg(&got.text);
            assert_eq!(paths(&c), paths(&c1), "{name}");
            assert!(c.layers_declared, "{name}");
            assert_eq!(c.change_policy.name(), "none", "{name}");
            for l in c.layers.layers() {
                assert_eq!(
                    (l.max_docs, l.max_chars, l.max_started, l.stale_days),
                    (None, None, None, None),
                    "{name}: the layer {} has no limit",
                    l.name
                );
            }
            // A root that can hold a decision has the key `live-sections`, and
            // it has no live section (D139, D188).
            let table = parse_table(&got.text).unwrap();
            for (i, r) in table["roots"].as_array().unwrap().iter().enumerate() {
                let can = c.roots[i]
                    .single_type
                    .is_none_or(|t| t == DocType::Decision);
                assert_eq!(r.get("live-sections").is_some(), can, "{name}: root {i}");
            }
            if let Some(h) = &c.harness {
                assert!(h.skills.is_empty() && !h.review_md, "{name}");
                assert_eq!(h.protect_tests.name(), "off", "{name}");
            }
            assert!(c.evals.is_none(), "{name}: no evals");
        }
    }

    #[test]
    fn a_file_with_crlf_gives_crlf_in_each_line() {
        let dir = tests_dir().join("golden/upgrade/config");
        let before = read(dir.join("harness-full.before.toml")).replace('\n', "\r\n");
        let old = OldConfig::read(&before).unwrap();
        let f = recorded_facts("harness-full", &old);
        let got = plan(&before, &old, &f, &opts()).unwrap().text;
        assert!(!got.replace("\r\n", "").contains('\n'), "a bare line feed");
        assert_eq!(
            got.replace("\r\n", "\n"),
            read(dir.join("harness-full.after.toml"))
        );
    }

    // --- the pin and the sums ---

    #[test]
    fn the_pin_keeps_its_quote_style() {
        let t = "spec-dna = '0.5.0'\n[[roots]]\npath = \"d\"\n";
        let got = go(t, &facts(&["spec"], &[]));
        assert!(got.text.starts_with("spec-dna = '0.6.0'\n"), "{}", got.text);
    }

    fn sums_for(version: &str, sum: char) -> Vec<(String, String)> {
        crate::release::assets(version)
            .into_iter()
            .map(|a| (a, sum.to_string().repeat(64)))
            .collect()
    }

    const OLD_ASSET: &str = "spec-dna-0.5.0-x86_64-unknown-linux-gnu";
    /// The version of this binary: `config` accepts sums only for this pin.
    const V: &str = crate::adapters::VERSION;

    #[test]
    fn a_sums_table_gets_the_sums_of_the_pin_file() {
        let t = format!(
            "spec-dna = \"0.5.0\"\n\n# The sums.\n[spec-dna-sha256]\n\"{OLD_ASSET}\" = \"{}\"\n\n[[roots]]\npath = \"d\"\n",
            "0".repeat(64)
        );
        let o = ConfigOpts {
            pin: V.into(),
            sums: Some(sums_for(V, 'a')),
        };
        let got = try_plan(&t, &facts(&["spec"], &[]), &o).unwrap();
        assert!(!got.sums_added);
        assert!(got.text.contains("# The sums.\n[spec-dna-sha256]\n"));
        assert!(!got.text.contains("0.5.0"), "{}", got.text);
        let c = cfg(&got.text);
        let sums = c.sha256.unwrap();
        assert_eq!(sums.len(), 7);
        assert!(
            sums.iter()
                .all(|(k, v)| k.contains(V) && v == &"a".repeat(64))
        );
    }

    #[test]
    fn a_sums_table_with_no_pin_file_stops_the_plan() {
        let t = format!(
            "spec-dna = \"0.5.0\"\n[spec-dna-sha256]\n\"{OLD_ASSET}\" = \"{}\"\n[[roots]]\npath = \"d\"\n",
            "0".repeat(64)
        );
        let old = OldConfig::read(&t).unwrap();
        let e = plan(&t, &old, &facts(&["spec"], &[]), &opts()).unwrap_err();
        assert_eq!(e.exit_code(), 3);
        assert!(message(&e).contains("--pin-file"), "{}", message(&e));
    }

    #[test]
    fn a_file_with_the_new_pin_and_the_old_names_keeps_its_sums() {
        // The 0.5 docs told a person to set the pin by hand (D137): the pin is
        // the new one, and the sums are the sums of that pin.
        let sums = sums_for(V, 'c');
        let mut t = format!("spec-dna = \"{V}\"\n[spec-dna-sha256]\n");
        for (name, sum) in &sums {
            t.push_str(&format!("\"{name}\" = \"{sum}\"\n"));
        }
        t.push_str("[[roots]]\npath = \"d\"\ntype = \"guide\"\n");
        let o = ConfigOpts {
            pin: V.into(),
            sums: None,
        };
        let got = try_plan(&t, &facts(&["guide"], &[]), &o).unwrap();
        let mut got_sums = cfg(&got.text).sha256.unwrap();
        got_sums.sort();
        let mut want = sums;
        want.sort();
        assert_eq!(got_sums, want);
        assert!(!got.sums_added);
        assert!(got.text.contains("type = \"procedure\"\n"), "{}", got.text);
    }

    #[test]
    fn a_pin_file_adds_the_table_when_the_file_has_none() {
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\n";
        let o = ConfigOpts {
            pin: V.into(),
            sums: Some(sums_for(V, 'b')),
        };
        let got = try_plan(t, &facts(&["spec"], &[]), &o).unwrap();
        assert!(got.sums_added);
        let c = cfg(&got.text);
        assert_eq!(c.sha256.unwrap().len(), 7);
        // No pin file, no table.
        let got = go(t, &facts(&["spec"], &[]));
        assert!(!got.sums_added);
        assert!(cfg(&got.text).sha256.is_none());
    }

    // --- the types ---

    #[test]
    fn the_key_types_gets_the_new_names_with_no_duplicate() {
        let t = "spec-dna = \"0.5.0\"\ntypes = [\"spec\", \"guide\", \"runbook\", \"change\"] # kept\n[[roots]]\npath = \"d\"\n";
        let got = go(t, &facts(&[], &[]));
        // `change` becomes the 3 types of the intent layer (D14).
        assert!(
            got.text.contains(
                "types = [\"requirements\", \"procedure\", \"intent\", \"spec\", \"plan\"] # kept\n"
            ),
            "{}",
            got.text
        );
        let c = cfg(&got.text);
        assert_eq!(c.types.len(), 5);
        // The types of the key are the types in use.
        let names: Vec<&str> = c.layers.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["requirements", "procedures", "intent"]);
    }

    #[test]
    fn a_list_of_many_lines_keeps_its_comments_when_no_name_merges() {
        let t = "spec-dna = \"0.5.0\"\ntypes = [\n  \"spec\", # one\n  \"invariant\",\n]\n[[roots]]\npath = \"d\"\n";
        let got = go(t, &facts(&[], &[]));
        assert!(
            got.text
                .contains("types = [\n  \"requirements\", # one\n  \"rule\",\n]\n"),
            "{}",
            got.text
        );
        // A list of many lines with a merge is a hand edit.
        let t = "spec-dna = \"0.5.0\"\ntypes = [\n  \"guide\",\n  \"runbook\",\n]\n[[roots]]\npath = \"d\"\n";
        let old = OldConfig::read(t).unwrap();
        let e = plan(t, &old, &facts(&[], &[]), &opts()).unwrap_err();
        assert_eq!(e.exit_code(), 3);
        assert!(
            message(&e).contains("cannot edit this form"),
            "{}",
            message(&e)
        );
    }

    #[test]
    fn a_list_of_many_lines_with_change_is_a_hand_edit() {
        // `change` becomes 3 names, so the list gets longer (D14). A list of
        // many lines keeps its comments only if its length stays.
        let t = "spec-dna = \"0.5.0\"\ntypes = [\n  \"spec\", # one\n  \"change\",\n]\n[[roots]]\npath = \"d\"\n";
        let old = OldConfig::read(t).unwrap();
        let e = plan(t, &old, &facts(&[], &[]), &opts()).unwrap_err();
        assert_eq!(e.exit_code(), 3);
        assert!(
            message(&e).contains("cannot edit this form"),
            "{}",
            message(&e)
        );
        // 2 merges and the growth of `change` give a list of the same length.
        // `set_value` would take it at each element, so the plan refuses it.
        let t = "spec-dna = \"0.5.0\"\ntypes = [\n  \"guide\",\n  \"runbook\",\n  \"note\",\n  \"reference\",\n  \"change\",\n]\n[[roots]]\npath = \"d\"\n";
        let old = OldConfig::read(t).unwrap();
        let e = plan(t, &old, &facts(&[], &[]), &opts()).unwrap_err();
        assert_eq!(e.exit_code(), 3);
        // A list of one line keeps its comment.
        let t =
            "spec-dna = \"0.5.0\"\ntypes = [\"spec\", \"change\"] # one\n[[roots]]\npath = \"d\"\n";
        let got = go(t, &facts(&[], &[]));
        assert!(
            got.text
                .contains("types = [\"requirements\", \"intent\", \"spec\", \"plan\"] # one\n"),
            "{}",
            got.text
        );
    }

    /// A 0.6 file of the small graph: `head` (the keys before the first table)
    /// and `[change] policy`.
    fn file_06(head: &str, policy: &str, pending_links: &str) -> String {
        let graph = render_graph(&Graph::small());
        let at = graph.find("[layers.pending]").unwrap();
        let end = at + graph[at..].find("linked-from = ").unwrap();
        let line_end = end + graph[end..].find('\n').unwrap();
        let graph = format!(
            "{}linked-from = {pending_links}{}",
            &graph[..end],
            &graph[line_end..]
        );
        format!(
            "spec-dna = \"0.6.0\"\n{head}[change]\npolicy = \"{policy}\"\n\n{graph}\n[[roots]]\npath = \"docs\"\ndefault-type = \"requirements\"\ntype = \"mixed\"\n"
        )
    }

    #[test]
    fn a_06_file_gets_the_types_of_the_policy_and_the_links_of_pending() {
        let t = file_06(
            "types = [\"requirements\", \"plan\"] # kept\n",
            "two-class",
            "[\"intent\"]",
        );
        let r = repair_current(&t, &cfg(&t)).unwrap();
        assert!(
            r.text
                .contains("types = [\"requirements\", \"plan\", \"intent\", \"spec\"] # kept\n"),
            "{}",
            r.text
        );
        assert!(
            r.text
                .contains("linked-from = [\"intent\", \"requirements\", \"decisions\"]\n"),
            "{}",
            r.text
        );
        assert_eq!(r.edits.len(), 2, "{:?}", r.edits);
        assert!(r.by_hand.is_empty());
        // The result is the fixed point: a second repair changes nothing.
        let again = repair_current(&r.text, &cfg(&r.text)).unwrap();
        assert_eq!(again.text, r.text);
        assert!(again.edits.is_empty());
    }

    #[test]
    fn a_06_file_keeps_each_value_that_is_not_an_exact_match() {
        // The policy `none`, no key `types`, and another list of links.
        for (head, policy, links) in [
            (
                "types = [\"requirements\"]\n",
                "none",
                "[\"intent\", \"rules\"]",
            ),
            ("", "three-class", "[]"),
            ("", "three-class", "[\"requirements\"]"),
            (
                "types = [\"intent\", \"spec\", \"plan\"]\n",
                "three-class",
                "[\"intent\", \"decisions\"]",
            ),
        ] {
            let t = file_06(head, policy, links);
            let r = repair_current(&t, &cfg(&t)).unwrap();
            assert_eq!(r.text, t, "{head} {policy} {links}");
            assert!(r.edits.is_empty() && r.by_hand.is_empty());
        }
    }

    #[test]
    fn an_empty_types_list_of_a_06_file_stays_as_it_is() {
        // An empty list means that every type is valid.
        let t = file_06("types = []\n", "three-class", "[\"intent\"]");
        let r = repair_current(&t, &cfg(&t)).unwrap();
        assert!(r.text.contains("types = []\n"), "{}", r.text);
        assert!(
            r.edits.iter().all(|e| !e.starts_with("`types`")),
            "{:?}",
            r.edits
        );
        assert!(r.by_hand.is_empty(), "{:?}", r.by_hand);
    }

    #[test]
    fn an_empty_types_list_of_a_05_file_stays_as_it_is() {
        let mut f = facts(&["spec", "change"], &["docs"]);
        f.done_changes = true;
        let t =
            "spec-dna = \"0.5.0\"\ntypes = []\n[[roots]]\npath = \"docs\"\nlive-sections = {}\n";
        let got = go(t, &f);
        assert!(got.text.contains("types = []\n"), "{}", got.text);
        assert!(cfg(&got.text).allows(DocType::Research), "{}", got.text);
    }

    #[test]
    fn a_06_list_of_many_lines_is_named_and_not_edited() {
        let t = file_06(
            "types = [\n  \"requirements\", # kept\n  \"intent\",\n]\n",
            "three-class",
            "[\"intent\", \"requirements\", \"decisions\"]",
        );
        let r = repair_current(&t, &cfg(&t)).unwrap();
        assert_eq!(r.text, t);
        assert_eq!(r.by_hand.len(), 1, "{:?}", r.by_hand);
        assert!(
            r.by_hand[0].starts_with("`types` lacks `spec` and `plan`"),
            "{:?}",
            r.by_hand
        );
    }

    #[test]
    fn a_layer_for_each_type_in_use_and_the_intent_layer_for_a_moved_change() {
        // The types in use come from the documents and the roots.
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\ntype = \"spec\"\n[[roots]]\npath = \"adr\"\ntype = \"decision\"\n";
        let got = go(t, &facts(&[], &[]));
        let c = cfg(&got.text);
        let names: Vec<&str> = c.layers.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["requirements", "decisions"]);
        assert_eq!(got.types, [DocType::Requirements, DocType::Decision]);
        // A moved change needs the intent layer, also with no `change` type.
        let got = go(t, &facts(&["spec", "decision"], &["docs"]));
        let names: Vec<String> = cfg(&got.text)
            .layers
            .layers()
            .iter()
            .map(|l| l.name.clone())
            .collect();
        assert_eq!(names, ["requirements", "decisions", "intent"]);
    }

    #[test]
    fn a_top_file_adds_its_type_and_its_layer_and_the_areas_go_into_the_key_areas() {
        let mut f = facts(&["spec", "note"], &[]);
        f.top_types = vec![DocType::Terms, DocType::Roadmap];
        f.areas = ["valves".to_string(), "pumps".to_string()].into();
        // No key `types`: the layers of the top files are in the graph.
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\nlive-sections = {}\n";
        let got = go(t, &f);
        let c = cfg(&got.text);
        let names: Vec<&str> = c.layers.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["foundation", "requirements", "research", "pending"]);
        assert!(got.types.contains(&DocType::Terms) && got.types.contains(&DocType::Roadmap));
        // The key `areas` is sorted, and it comes before the first table.
        assert_eq!(c.areas, ["pumps", "valves"]);
        assert!(
            got.text.find("areas = [\"pumps\", \"valves\"]\n") < got.text.find("[[roots]]"),
            "{}",
            got.text
        );
        // The key `types` of the project allows the types of the top files.
        let t = "spec-dna = \"0.5.0\"\ntypes = [\"spec\", \"note\"]\n[[roots]]\npath = \"docs\"\nlive-sections = {}\n";
        let got = go(t, &f);
        let c = cfg(&got.text);
        assert!(
            c.allows(DocType::Terms) && c.allows(DocType::Roadmap),
            "{}",
            got.text
        );
        assert!(
            got.text
                .contains("types = [\"requirements\", \"research\", \"terms\", \"roadmap\"]\n"),
            "{}",
            got.text
        );
        // No top file and no area: no key, and no such layer.
        f.top_types.clear();
        f.areas.clear();
        let got = go(t, &f);
        assert!(!got.text.contains("areas ="), "{}", got.text);
        assert!(!got.types.contains(&DocType::Terms));
    }

    #[test]
    fn a_done_change_is_research_so_the_type_is_in_use_and_allowed() {
        let mut f = facts(&["spec", "change"], &["docs"]);
        f.done_changes = true;
        // No key `types`: a layer holds `research`.
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"docs\"\nlive-sections = {}\n";
        let got = go(t, &f);
        let c = cfg(&got.text);
        let names: Vec<&str> = c.layers.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["requirements", "research", "intent"]);
        assert!(got.types.contains(&DocType::Research));
        // Without a done change, no such layer.
        f.done_changes = false;
        let got = go(t, &f);
        assert!(!got.types.contains(&DocType::Research));
        // The key `types`: the project allows `research`.
        f.done_changes = true;
        let t = "spec-dna = \"0.5.0\"\ntypes = [\"spec\", \"change\"]\n[[roots]]\npath = \"docs\"\nlive-sections = {}\n";
        let got = go(t, &f);
        let c = cfg(&got.text);
        assert!(c.allows(DocType::Research), "{}", got.text);
        assert!(c.allows(DocType::Intent) && c.allows(DocType::Requirements));
        assert!(
            got.text.contains(
                "types = [\"requirements\", \"intent\", \"spec\", \"plan\", \"research\"]\n"
            ),
            "{}",
            got.text
        );
        // A type list that has `research` stays as it is.
        let t = "spec-dna = \"0.5.0\"\ntypes = [\"spec\", \"change\", \"note\"]\n[[roots]]\npath = \"docs\"\nlive-sections = {}\n";
        let got = go(t, &f);
        assert!(
            got.text.contains(
                "types = [\"requirements\", \"intent\", \"spec\", \"plan\", \"research\"]\n"
            ),
            "{}",
            got.text
        );
        // A list of many lines, with no `research`, is a hand edit.
        let t = "spec-dna = \"0.5.0\"\ntypes = [\n  \"spec\",\n  \"change\",\n]\n[[roots]]\npath = \"docs\"\nlive-sections = {}\n";
        let old = OldConfig::read(t).unwrap();
        let e = plan(t, &old, &f, &opts()).unwrap_err();
        assert_eq!(e.exit_code(), 3);
        assert!(message(&e).contains("`types`"), "{}", message(&e));
    }

    // --- the roots ---

    #[test]
    fn a_root_follows_the_rules_of_d140() {
        let t = "spec-dna = \"0.5.0\"\n\
[[roots]]\npath = \"a\"\ntype = \"note\"\n\
[[roots]]\npath = \"b\"\ntype = \"reference\"\n\
[[roots]]\npath = \"c\"\ntype = \"change\"\nstatus = \"proposed\"\n\
[[roots]]\npath = \"d\"\ntype = \"guide\"\n\
[[roots]]\npath = \"e\"\ntype = \"mixed\"\ndefault-type = \"change\"\nstatus = \"accepted\"\n\
[[roots]]\npath = \"f\"\ndefault-type = \"note\"\n\
[[roots]]\npath = \"g\"\n\
[[roots]]\npath = \"h\"\ntype = \"decision\"\nstatus = \"proposed\"\n\
[[roots]]\npath = \"i\"\ndefault-type = \"spec\"\nstatus = \"active\"\n";
        let got = go(t, &facts(&["note"], &[]));
        let c = cfg(&got.text);
        let kinds: Vec<_> = c
            .roots
            .iter()
            .map(|r| {
                (
                    r.path.clone(),
                    r.single_type.map(DocType::name),
                    r.default_type.map(DocType::name),
                    r.status.as_deref(),
                )
            })
            .collect();
        let want = [
            ("a", None, Some("research"), None),
            ("b", Some("research"), Some("research"), None),
            ("c", None, Some("research"), None),
            ("d", Some("procedure"), Some("procedure"), None),
            ("e", None, Some("research"), None),
            ("f", None, Some("research"), None),
            ("g", None, Some("research"), None),
            ("h", Some("decision"), Some("decision"), Some("proposed")),
            ("i", None, Some("requirements"), Some("active")),
        ];
        for (k, w) in kinds.iter().zip(want) {
            assert_eq!((k.0.as_str(), k.1, k.2, k.3), w);
        }
        // The change statuses of the root go, and the moved change gets them.
        assert_eq!(
            got.root_statuses,
            [
                ("c".to_string(), "proposed".to_string()),
                ("e".to_string(), "accepted".to_string())
            ]
        );
    }

    #[test]
    fn a_root_gets_live_sections_and_the_keys_for_a_moved_change() {
        let t = "spec-dna = \"0.5.0\"\n\
[[roots]]\npath = \"docs\"\ndefault-type = \"spec\"\n\
[[roots]]\npath = \"keep\"\ndefault-type = \"spec\"\nlive-sections = { decision = [\"Progress\"] } # mine\n\
[[roots]]\npath = \"rfcs\"\ntype = \"change\"\n";
        let got = go(t, &facts(&["spec", "change"], &["docs", "rfcs"]));
        let c = cfg(&got.text);
        assert!(got.text.contains(
            "path = \"docs\"\ndefault-type = \"requirements\"\nrequired-sections = { intent = [] }\nlive-sections = { decision = [] }\n"
        ), "{}", got.text);
        assert!(
            got.text
                .contains("live-sections = { decision = [\"Progress\"] } # mine\n")
        );
        assert_eq!(
            c.roots[1].live_sections[DocType::Decision as usize],
            ["Progress"]
        );
        assert!(c.roots[2].required_sections[DocType::Intent as usize].is_empty());
        assert!(c.roots[0].required_sections[DocType::Intent as usize].is_empty());
        assert!(!c.roots[1].required_sections[DocType::Intent as usize].is_empty());
    }

    #[test]
    fn only_a_root_that_can_hold_a_decision_gets_live_sections() {
        let t = "spec-dna = \"0.5.0\"\n\
[[roots]]\npath = \"adr\"\ntype = \"decision\"\n\
[[roots]]\npath = \"mix\"\n\
[[roots]]\npath = \"specs\"\ntype = \"spec\"\n\
[[roots]]\npath = \"ops\"\ntype = \"runbook\"\n\
[[roots]]\npath = \"notes\"\ntype = \"note\"\n\
[[roots]]\npath = \"refs\"\ntype = \"reference\"\n";
        let got = go(t, &facts(&["decision", "spec"], &[]));
        let table = parse_table(&got.text).unwrap();
        let has: Vec<bool> = table["roots"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r.get("live-sections").is_some())
            .collect();
        // A decision root, and the mixed roots (`notes` is mixed now).
        assert_eq!(has, [true, true, false, false, true, false]);
        assert!(got.text.contains("live-sections = { decision = [] }\n"));
        assert!(!got.text.contains("live-sections = {}"), "{}", got.text);
        // Each live section of a decision is off in a root that has the key.
        let c = cfg(&got.text);
        for i in [0, 1, 4] {
            assert!(c.roots[i].live_sections.iter().all(Vec::is_empty), "{i}");
        }
    }

    #[test]
    fn the_required_sections_of_a_moved_change_keep_the_user_value() {
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"r\"\ntype = \"mixed\"\ndefault-type = \"spec\"\nlive-sections = {}\nrequired-sections = { change = [\"Goal\"], decision = [\"Why\"] }\n[[roots]]\npath = \"s\"\ndefault-type = \"spec\"\nlive-sections = {}\nrequired-sections = { decision = [\"Why\"] }\n";
        let got = go(t, &facts(&["spec", "change"], &["r", "s"]));
        let c = cfg(&got.text);
        assert_eq!(
            c.roots[0].required_sections[DocType::Intent as usize],
            [["Goal"]]
        );
        assert_eq!(
            c.roots[0].required_sections[DocType::Decision as usize],
            [["Why"]]
        );
        assert!(c.roots[1].required_sections[DocType::Intent as usize].is_empty());
        assert!(
            got.text
                .contains("required-sections = { decision = [\"Why\"], intent = [] }"),
            "{}",
            got.text
        );
        assert!(!got.text.contains("change = "), "{}", got.text);
    }

    #[test]
    fn a_root_of_runbooks_keeps_the_steps_and_a_mixed_root_does_not() {
        let t = "spec-dna = \"0.5.0\"\n\
[[roots]]\npath = \"ops\"\ntype = \"runbook\"\n\
[[roots]]\npath = \"ops2\"\ntype = \"runbook\"\nrequired-sections = { runbook = [\"Restart\"] }\n\
[[roots]]\npath = \"mix\"\ndefault-type = \"spec\"\n";
        let got = go(t, &facts(&["runbook", "spec"], &[]));
        let c = cfg(&got.text);
        let p = DocType::Procedure as usize;
        assert_eq!(c.roots[0].required_sections[p], [["Steps"]]);
        assert_eq!(c.roots[1].required_sections[p], [["Restart"]]);
        assert!(c.roots[2].required_sections[p].is_empty());
    }

    #[test]
    fn a_status_word_goes_only_where_the_type_is_a_change() {
        // A decision root keeps `proposed`, and a spec root keeps `active`.
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"adr\"\ntype = \"decision\"\nstatus = \"proposed\"\nlive-sections = {}\n";
        let got = go(t, &facts(&["decision"], &[]));
        assert!(got.text.contains("status = \"proposed\"\n"));
        assert!(got.root_statuses.is_empty());
        assert_eq!(cfg(&got.text).roots[0].status.as_deref(), Some("proposed"));
    }

    #[test]
    fn the_status_map_follows_the_change_statuses_of_a_change_root() {
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"rfcs\"\ntype = \"change\"\nstatus-map = { wip = \"proposed\", ok = \"accepted\", dropped = \"rejected\" }\nlive-sections = {}\n";
        let got = go(t, &facts(&["change"], &["rfcs"]));
        // Only the names change: the order of the keys stays.
        assert!(
            got.text.contains(
                "status-map = { wip = \"draft\", ok = \"accepted\", dropped = \"closed\" }\n"
            ),
            "{}",
            got.text
        );
        let c = cfg(&got.text);
        assert_eq!(c.roots[0].status_map.len(), 3);
    }

    #[test]
    fn a_status_map_entry_to_done_goes_and_a_decision_target_stays() {
        let t = "spec-dna = \"0.5.0\"\n\
[[roots]]\npath = \"rfcs\"\ntype = \"change\"\nstatus-map = { fin = \"done\", wip = \"proposed\" }\nlive-sections = {}\n\
[[roots]]\npath = \"mix\"\ndefault-type = \"decision\"\nstatus-map = { fin = \"done\", old = \"rejected\" }\nlive-sections = {}\n\
[[roots]]\npath = \"only\"\ndefault-type = \"spec\"\nstatus-map = { fin = \"done\" }\nlive-sections = {}\n";
        let got = go(t, &facts(&["change", "decision"], &["rfcs"]));
        let c = cfg(&got.text);
        assert_eq!(
            c.roots[0].status_map,
            [("wip".to_string(), "draft".to_string())]
        );
        // A mixed root: `rejected` is also a status of a decision.
        assert_eq!(
            c.roots[1].status_map,
            [("old".to_string(), "rejected".to_string())]
        );
        assert!(c.roots[2].status_map.is_empty());
        assert!(!got.text.contains("done"), "{}", got.text);
        assert!(!got.text.contains("status-map = {}"), "{}", got.text);
    }

    // --- budgets ---

    fn budget(text: &str, t: DocType) -> (u32, u32) {
        cfg(text).budgets[t as usize]
    }

    #[test]
    fn two_budgets_that_merge_keep_the_larger_one() {
        let t = with_root(
            "[budgets.guide]\nsoft = 3000\nhard = 6000\n\n[budgets.runbook]\nsoft = 2000\nhard = 4000\n",
        );
        let got = go(&t, &facts(&["guide", "runbook"], &[]));
        assert_eq!(budget(&got.text, DocType::Procedure), (3000, 6000));
        assert!(
            got.text
                .contains("[budgets.procedure]\nsoft = 3000\nhard = 6000\n")
        );
        assert!(!got.text.contains("runbook"), "{}", got.text);
        // The other order of the sizes: a value of each key.
        let t = with_root(
            "[budgets.guide]\nsoft = 3000\nhard = 9000\n\n[budgets.runbook]\nsoft = 5000\nhard = 7000\n",
        );
        let got = go(&t, &facts(&["guide", "runbook"], &[]));
        assert_eq!(budget(&got.text, DocType::Procedure), (5000, 9000));
    }

    #[test]
    fn a_budget_of_a_type_in_use_without_a_key_counts_with_its_default() {
        // Explicit runbook 2000/4000, and guides in use with the 0.5 default
        // 14,000/28,000, which is the 0.6 default: the key goes.
        let t = with_root("[budgets.runbook]\nsoft = 2000\nhard = 4000\n");
        let got = go(&t, &facts(&["guide", "runbook"], &[]));
        assert_eq!(budget(&got.text, DocType::Procedure), (14_000, 28_000));
        assert!(!got.text.contains("[budgets."), "{}", got.text);
        // Runbooks alone: the key stays, with the new name.
        let got = go(&t, &facts(&["runbook"], &[]));
        assert_eq!(budget(&got.text, DocType::Procedure), (2000, 4000));
        assert!(
            got.text
                .contains("[budgets.procedure]\nsoft = 2000\nhard = 4000\n")
        );
        // A larger explicit guide budget wins over the default of the runbook.
        let t = with_root("[budgets.guide]\nsoft = 20000\nhard = 40000\n");
        let got = go(&t, &facts(&["guide", "runbook"], &[]));
        assert_eq!(budget(&got.text, DocType::Procedure), (20_000, 40_000));
    }

    #[test]
    fn a_note_budget_goes_when_references_have_none() {
        // A reference has no budget, so the merged type has none.
        let t = with_root("[budgets.note]\nsoft = 300\nhard = 600\n");
        let got = go(&t, &facts(&["note", "reference"], &[]));
        assert_eq!(budget(&got.text, DocType::Research), (0, 0));
        assert!(!got.text.contains("[budgets."), "{}", got.text);
        // Notes alone: the key stays, with the new name.
        let got = go(&t, &facts(&["note"], &[]));
        assert_eq!(budget(&got.text, DocType::Research), (300, 600));
        assert!(
            got.text
                .contains("[budgets.research]\nsoft = 300\nhard = 600\n")
        );
    }

    #[test]
    fn a_budget_key_gets_the_new_name_and_its_comment_stays() {
        let t = with_root(
            "[budgets]\ngrowth = 100\n# The contracts.\n[budgets.contract]\nsoft = 5\nhard = 9 # nine\n[budgets.change]\nsoft = 1\nhard = 2\n",
        );
        let got = go(&t, &facts(&["contract", "change"], &[]));
        assert!(
            got.text
                .contains("# The contracts.\n[budgets.interface]\nsoft = 5\nhard = 9 # nine\n")
        );
        assert_eq!(budget(&got.text, DocType::Interface), (5, 9));
        assert_eq!(budget(&got.text, DocType::Intent), (1, 2));
        assert_eq!(cfg(&got.text).growth, 100);
    }

    #[test]
    fn a_moved_change_over_the_budget_gets_a_budget_for_the_intent() {
        let t = with_root("");
        let mut f = facts(&["change"], &["docs"]);
        let got = go(&t, &f);
        assert_eq!(budget(&got.text, DocType::Intent), (6000, 12_000));
        f.change_over_budget = true;
        let got = go(&t, &f);
        assert_eq!(budget(&got.text, DocType::Intent), (10_000, 20_000));
        assert!(
            got.text
                .contains("[budgets.intent]\nsoft = 10000\nhard = 20000\n")
        );
        // A larger key of the user stays.
        let t = with_root("[budgets.change]\nsoft = 11000\nhard = 15000\n");
        let got = go(&t, &f);
        assert_eq!(budget(&got.text, DocType::Intent), (11_000, 20_000));
    }

    // --- the sections that merge ---

    #[test]
    fn the_required_sections_of_2_merged_types_are_the_shared_ones() {
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\ndefault-type = \"spec\"\nlive-sections = {}\nrequired-sections = { guide = [\"Steps\"], runbook = [[\"Steps\"], [\"Restart\"]] }\n";
        let got = go(t, &facts(&["guide", "runbook"], &[]));
        let c = cfg(&got.text);
        assert_eq!(
            c.roots[0].required_sections[DocType::Procedure as usize],
            [["Steps"]]
        );
        assert!(!got.text.contains("guide") && !got.text.contains("runbook"));
        // No shared section: no requirement, and no key.
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\ndefault-type = \"spec\"\nlive-sections = {}\nrequired-sections = { guide = [\"A\"], runbook = [\"B\"] }\n";
        let got = go(t, &facts(&["guide", "runbook"], &[]));
        let c = cfg(&got.text);
        assert!(c.roots[0].required_sections[DocType::Procedure as usize].is_empty());
    }

    #[test]
    fn a_runbook_requirement_goes_in_a_mixed_root_that_has_guides_too() {
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\ndefault-type = \"spec\"\nlive-sections = {}\nrequired-sections = { runbook = [\"Restart\"] }\n";
        // Guides have no required section: the weaker value is none.
        let got = go(t, &facts(&["guide", "runbook"], &[]));
        let c = cfg(&got.text);
        assert!(c.roots[0].required_sections[DocType::Procedure as usize].is_empty());
        assert!(!got.text.contains("required-sections"), "{}", got.text);
        // With no guide in use, the requirement stays.
        let got = go(t, &facts(&["runbook"], &[]));
        assert_eq!(
            cfg(&got.text).roots[0].required_sections[DocType::Procedure as usize],
            [["Restart"]]
        );
    }

    #[test]
    fn the_names_of_2_merged_types_get_the_union() {
        let t = "spec-dna = \"0.5.0\"\n[[roots]]\npath = \"d\"\ndefault-type = \"spec\"\nlive-sections = {}\nhistory-sections = { guide = [\"Log\", \"Notes\"], runbook = [\"Notes\", \"Old\"] }\ncompanion-sections = { spec = [\"Table\"] }\namendment-sections = { note = [\"A\"], reference = [\"B\"] }\n";
        let got = go(t, &facts(&["guide", "runbook"], &[]));
        let r = &cfg(&got.text).roots[0];
        assert_eq!(
            r.history_sections[DocType::Procedure as usize],
            ["Log", "Notes", "Old"]
        );
        assert_eq!(
            r.companion_sections[DocType::Requirements as usize],
            ["Table"]
        );
        assert_eq!(r.amendment_sections[DocType::Research as usize], ["A", "B"]);
    }

    #[test]
    fn the_conventions_of_2_merged_types_are_the_union() {
        let t = with_root(
            "[conventions]\nguide = [\"a.md#x\"]\nrunbook = [\"b.md\", \"a.md#x\"]\ncontract = [\"c.md\"]\n",
        );
        let got = go(&t, &facts(&["guide", "runbook", "contract"], &[]));
        let c = cfg(&got.text);
        assert_eq!(
            c.conventions[DocType::Procedure as usize],
            ["a.md#x", "b.md"]
        );
        assert_eq!(c.conventions[DocType::Interface as usize], ["c.md"]);
    }

    // --- the harness ---

    const HARNESS: &str = "[harness]\nlevel = \"lean\"\n# The roles.\n[harness.roles]\nreviewer = { claude-code = \"opus\", effort = \"high\" } # the checker\nhelper = { claude-code = \"haiku\" }\n";

    #[test]
    fn the_harness_gets_the_new_role_names_and_the_parts_off() {
        let t = with_root(HARNESS);
        let got = go(&t, &facts(&["spec"], &[]));
        assert!(
            got.text.contains(
                "[harness]\nlevel = \"lean\"\nskills = []\nreview-md = false\nprotect-tests = \"off\"\n# The roles.\n[harness.roles]\nverifier = { claude-code = \"opus\", effort = \"high\" } # the checker\nhelper = { claude-code = \"haiku\" }\n"
            ),
            "{}",
            got.text
        );
        let h = cfg(&got.text).harness.unwrap();
        assert_eq!(h.level.name(), "lean");
        assert_eq!(h.cfg(crate::harness::Role::Verifier).effort.name(), "high");
    }

    #[test]
    fn the_role_spec_writer_ends_and_a_full_harness_gets_the_code_simplifier_off() {
        let t = with_root(
            "[harness]\nlevel = \"full\"\n[harness.roles]\nspec-writer = { claude-code = \"sonnet\" }\nimplementer = { claude-code = \"sonnet\" }\n",
        );
        let got = go(&t, &facts(&["spec"], &[]));
        assert!(!got.text.contains("spec-writer"), "{}", got.text);
        assert!(got.text.contains("implementer = { claude-code = \"sonnet\" }\ncode-simplifier = { claude-code = \"none\" }\n"), "{}", got.text);
        let h = cfg(&got.text).harness.unwrap();
        assert!(!h.on(crate::harness::Role::CodeSimplifier));
        assert!(h.on(crate::harness::Role::Implementer));
        // A lean harness has no such role, and level `none` gets only the parts.
        for level in ["lean", "none"] {
            let t = with_root(&format!("[harness]\nlevel = \"{level}\"\n"));
            let got = go(&t, &facts(&["spec"], &[]));
            assert!(!got.text.contains("code-simplifier"), "{level}");
            assert!(got.text.contains("skills = []\n"), "{level}");
        }
    }

    #[test]
    fn a_role_with_no_marker_is_off() {
        let t = with_root(
            "[harness]\nlevel = \"full\"\n[harness.roles]\nimplementer = { claude-code = \"sonnet\", effort = \"high\" }\nadvisor = { claude-code = \"fable\" }\n",
        );
        let mut f = facts(&["spec"], &[]);
        f.unmarked_roles = ["implementer", "advisor", "researcher", "verifier"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let got = go(&t, &f);
        let h = cfg(&got.text).harness.unwrap();
        use crate::harness::Role;
        assert!(!h.on(Role::Implementer));
        assert!(!h.on(Role::Researcher));
        assert!(!h.on(Role::Verifier));
        // The advisor keeps its model, and its agent file is off.
        assert!(h.on(Role::Advisor));
        assert_eq!(h.cfg(Role::Advisor).agent.name(), "none");
        // The role keeps its effort.
        assert_eq!(h.cfg(Role::Implementer).effort.name(), "high");
        // A role that the level does not include gets no entry.
        let t = with_root("[harness]\nlevel = \"lean\"\n");
        let got = go(&t, &f);
        assert!(!got.text.contains("implementer"), "{}", got.text);
    }

    #[test]
    fn a_file_with_no_harness_gets_none() {
        let got = go(&with_root(""), &facts(&["spec"], &[]));
        assert!(cfg(&got.text).harness.is_none());
        assert!(!got.text.contains("[harness"), "{}", got.text);
    }

    // --- the file as a whole ---

    #[test]
    fn the_plan_writes_no_evals_no_limit_and_the_policy_none() {
        let got = go(
            &with_root(""),
            &facts(&["spec", "decision", "change"], &["docs"]),
        );
        let c = cfg(&got.text);
        assert!(c.evals.is_none());
        assert_eq!(c.change_policy.name(), "none");
        assert!(
            got.text.ends_with("[change]\npolicy = \"none\"\n"),
            "{}",
            got.text
        );
        assert!(!got.text.contains("max-docs") && !got.text.contains("stale-days"));
        assert!(!got.text.contains("[evals]") && !got.text.contains("[templates]"));
    }

    #[test]
    fn a_text_that_cannot_be_edited_stops_with_exit_code_3() {
        // A root in a form that no splice covers: it needs new keys.
        let t = "spec-dna = \"0.5.0\"\nroots = [{ path = \"d\", type = \"mixed\" }]\n";
        let old = OldConfig::read(t).unwrap();
        let e = plan(t, &old, &facts(&["guide"], &[]), &opts()).unwrap_err();
        assert_eq!(e.exit_code(), 3, "{}", message(&e));
    }

    #[test]
    fn the_root_of_a_kind_that_the_reader_gives_is_what_the_plan_reads() {
        let old = OldConfig::read(&with_root("")).unwrap();
        assert_eq!(old.roots[0].kind(), Kind::Mixed);
        let t = types_in_use(&old, &facts(&["change", "note"], &[]));
        assert_eq!(
            t,
            [DocType::Requirements, DocType::Research, DocType::Intent]
        );
        let g = graph_without_limits(&t).unwrap();
        assert!(g.layers().iter().all(|l: &Layer| l.max_docs.is_none()));
    }

    #[test]
    fn the_names_of_0_6_in_skills_change_and_the_types_stay() {
        let t = "spec-dna = \"0.6.1\"\ntypes = [\"intent\", \"spec\", \"plan\"]\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\"plan\", \"review\", \"intent\", \"spec\"]  # mine\n";
        let r = rename_skills_with(t, &|_| true);
        assert_eq!(
            r.text,
            "spec-dna = \"0.6.1\"\ntypes = [\"intent\", \"spec\", \"plan\"]\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\"brainstorm\", \"specify\", \"clarify\", \"write-plan\", \"execute\", \"tdd\", \"review\", \"verify\", \"finish\", \"debug\", \"post-mortem\"]  # mine\n"
        );
        // The rename and the skills that are new in 0.7.0.
        assert_eq!(r.edits.len(), 2, "{:?}", r.edits);
        assert!(r.by_hand.is_empty());
        // A second call changes nothing.
        let again = rename_skills_with(&r.text, &|_| true);
        assert_eq!(again.text, r.text);
        assert!(again.edits.is_empty());
    }

    /// The default of the fake table of ADR 0023, for each value of the
    /// constant. `brainstorm` and `tdd` are `on`, `clarify` and `debug` are
    /// `inconclusive`, `review` is `off`, and the other skills have no verdict.
    fn fake_default(needs_on: bool) -> impl Fn(Skill) -> bool {
        use crate::verdict::{Verdict, on_by_default};
        move |s| {
            let verdict = match s {
                Skill::Brainstorm | Skill::Tdd => Some(Verdict::On),
                Skill::Clarify | Skill::Debug => Some(Verdict::Inconclusive),
                Skill::Review => Some(Verdict::Off),
                _ => None,
            };
            on_by_default(s.in_d2(), verdict, needs_on)
        }
    }

    #[test]
    fn the_rename_follows_the_rule_with_a_fake_table() {
        let t = "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\"intent\", \"spec\", \"plan\", \"review\"]\n";
        // `true`: only `brainstorm` and `tdd` have the verdict `on`. `review`
        // stays, although it is off by default, because the person chose it.
        let r = rename_skills_with(t, &fake_default(true));
        assert_eq!(
            r.text,
            "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\"brainstorm\", \"tdd\", \"review\"]\n"
        );
        // The line of the rename, one line for each name that is left out and
        // the line of `tdd`, which is new in 0.7.0.
        assert_eq!(r.edits.len(), 5, "{:?}", r.edits);
        let said = r.edits.join("\n");
        for name in ["specify", "clarify", "write-plan"] {
            assert!(
                said.contains(&format!("`{name}` is off by default")),
                "{said}"
            );
        }
        assert!(
            said.contains("add \"write-plan\" to `skills` of [harness]"),
            "{said}"
        );
        assert!(r.by_hand.is_empty());
        // A second call changes nothing.
        let again = rename_skills_with(&r.text, &fake_default(true));
        assert_eq!(again.text, r.text);
        assert!(again.edits.is_empty());

        // `false`: D33. Each new name of D2 stays, and no name is left out.
        let r = rename_skills_with(t, &fake_default(false));
        assert_eq!(
            r.text,
            "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\"brainstorm\", \"specify\", \"clarify\", \"write-plan\", \"tdd\", \"review\", \"finish\", \"post-mortem\"]\n"
        );
        assert_eq!(r.edits.len(), 2, "{:?}", r.edits);
        assert!(!r.edits[0].contains("off by default"), "{:?}", r.edits);
    }

    #[test]
    fn a_name_that_the_list_holds_stays_also_when_its_default_is_off() {
        let t = "[harness]\nskills = [\"specify\", \"spec\", \"tdd\"]\n";
        let r = rename_skills_with(t, &fake_default(true));
        // `specify` was in the list: it stays. `spec` ended: `clarify` is off.
        // The text has no pin, so no new skill joins the list.
        assert_eq!(r.text, "[harness]\nskills = [\"specify\", \"tdd\"]\n");
        assert_eq!(r.edits.len(), 2, "{:?}", r.edits);
    }

    #[test]
    fn a_list_of_many_lines_gets_the_filtered_list_by_hand() {
        let t = "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\n  \"intent\",\n  \"spec\",\n  \"plan\",\n  \"review\",\n]\n";
        let r = rename_skills_with(t, &fake_default(true));
        assert_eq!(r.text, t);
        assert_eq!(r.by_hand.len(), 1, "{:?}", r.by_hand);
        assert!(
            r.by_hand[0].contains("[\"brainstorm\", \"review\"]"),
            "{:?}",
            r.by_hand
        );
    }

    #[test]
    fn a_name_that_the_list_already_has_is_not_added_twice() {
        let t = "[harness]\nskills = [\"specify\", \"spec\"]\n";
        let r = rename_skills_with(t, &|_| true);
        assert_eq!(r.text, "[harness]\nskills = [\"specify\", \"clarify\"]\n");
        // The same with a pin and a level: a name is not added twice, and the
        // skills that are new in 0.7.0 follow in the order of D2.
        let t = "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\"specify\", \"spec\", \"tdd\"]\n";
        let r = rename_skills_with(t, &|_| true);
        assert!(
            r.text.contains(
                "skills = [\"specify\", \"clarify\", \"execute\", \"tdd\", \"verify\", \"finish\", \"debug\", \"post-mortem\"]"
            ),
            "{}",
            r.text
        );
    }

    #[test]
    fn no_old_name_or_no_valid_text_changes_nothing() {
        for t in [
            "[harness]\nskills = [\"review\"]\n",
            "types = [\"intent\"]\n",
            "[harness]\nlevel = \"lean\"\n",
            "this is not toml",
        ] {
            let r = rename_skills_with(t, &|_| true);
            assert_eq!(r.text, t);
            assert!(r.edits.is_empty() && r.by_hand.is_empty());
        }
    }

    #[test]
    fn a_list_of_many_lines_with_a_new_length_is_left_to_the_person() {
        let t = "[harness]\nskills = [\n  \"spec\",  # the spec\n  \"review\",\n]\n";
        let r = rename_skills_with(t, &|_| true);
        assert_eq!(r.text, t);
        assert_eq!(r.by_hand.len(), 1, "{:?}", r.by_hand);
    }

    /// A 0.6.1 file with the given profile line, level and `skills` line.
    fn file_0_6_1(profile: &str, level: &str, skills: &str) -> String {
        format!(
            "spec-dna = \"0.6.1\"\n{profile}\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"{level}\"\nskills = {skills}\n"
        )
    }

    fn skills_line(text: &str) -> String {
        text.lines()
            .find(|l| l.starts_with("skills"))
            .unwrap_or_default()
            .to_string()
    }

    #[test]
    fn the_skills_that_are_new_in_0_7_0_join_a_list_with_no_old_name() {
        // `standard` (the default profile) with the fake table of D33: `tdd`,
        // `finish` and `post-mortem`. `review` keeps its place.
        let t = file_0_6_1("", "lean", "[\"review\"]");
        let r = rename_skills_with(&t, &fake_default(false));
        assert_eq!(
            skills_line(&r.text),
            "skills = [\"review\", \"tdd\", \"finish\", \"post-mortem\"]"
        );
        assert_eq!(r.edits.len(), 1, "{:?}", r.edits);
        assert_eq!(
            r.edits[0],
            "`skills` of `[harness]`: added `tdd`, `finish` and `post-mortem`, new in 0.7.0 and on by default for the profile `standard`; remove a name to turn its skill off"
        );
        // A second call changes nothing.
        let again = rename_skills_with(&r.text, &fake_default(false));
        assert_eq!(again.text, r.text);
        assert!(again.edits.is_empty(), "{:?}", again.edits);
    }

    #[test]
    fn the_profile_lite_adds_only_finish() {
        let t = file_0_6_1("profile = \"lite\"\n", "lean", "[\"review\"]");
        let r = rename_skills_with(&t, &fake_default(false));
        assert_eq!(skills_line(&r.text), "skills = [\"review\", \"finish\"]");
        assert!(
            r.edits[0].contains("added `finish`, new in 0.7.0"),
            "{:?}",
            r.edits
        );
        assert!(r.edits[0].contains("the profile `lite`"), "{:?}", r.edits);
    }

    #[test]
    fn the_level_full_adds_each_skill_that_is_new_in_0_7_0() {
        let t = file_0_6_1("", "full", "[\"review\"]");
        let r = rename_skills_with(&t, &fake_default(false));
        assert_eq!(
            skills_line(&r.text),
            "skills = [\"review\", \"execute\", \"tdd\", \"verify\", \"finish\", \"debug\", \"post-mortem\"]"
        );
    }

    #[test]
    fn a_pin_of_0_7_0_or_later_adds_no_skill() {
        // No release after 0.7.0 adds a skill: the skill `accept` of 0.8.0 part A
        // merged into `review` (design 0.8.0, D71).
        for pin in ["0.7.0", "0.7.1", "0.8.0"] {
            for (profile, level) in [
                ("", "lean"),
                ("", "full"),
                ("profile = \"lite\"\n", "lean"),
                ("profile = \"lite\"\n", "full"),
            ] {
                let t =
                    file_0_6_1(profile, level, "[\"review\", \"finish\"]").replace("0.6.1", pin);
                let r = rename_skills_with(&t, &fake_default(false));
                assert_eq!(r.text, t, "{pin} {profile} {level}");
                assert!(r.edits.is_empty(), "{pin} {profile} {level}: {:?}", r.edits);
            }
        }
    }

    #[test]
    fn an_empty_list_a_new_pin_and_the_level_none_stay() {
        for t in [
            file_0_6_1("", "lean", "[]"),
            file_0_6_1("", "none", "[\"review\"]"),
            file_0_6_1("", "lean", "[\"review\"]").replace("0.6.1", "0.8.0"),
            "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\n".to_string(),
        ] {
            let r = rename_skills_with(&t, &fake_default(false));
            assert_eq!(r.text, t);
            assert!(r.edits.is_empty() && r.by_hand.is_empty(), "{t}");
        }
    }

    #[test]
    fn a_list_of_many_lines_with_no_old_name_is_a_warning_and_no_stop() {
        let t = "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\n  \"review\",  # mine\n]\n";
        let r = rename_skills_with(t, &fake_default(false));
        assert_eq!(r.text, t);
        // `source_of` stops for `by_hand`, and an upgrade of such a list
        // worked before, so the advice is a warning.
        assert!(r.by_hand.is_empty(), "{:?}", r.by_hand);
        assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
        assert!(
            r.warnings[0].contains("`tdd`, `finish` and `post-mortem`"),
            "{:?}",
            r.warnings
        );
    }

    #[test]
    fn a_list_of_many_lines_that_takes_the_rename_and_not_the_new_skills_gets_a_warning() {
        // The rename keeps the length of the list, so the splice works, as in
        // 0.7.0. The new skills do not fit, so a warning names them.
        let t = "spec-dna = \"0.6.1\"\n\n[change]\npolicy = \"three-class\"\n\n[harness]\nlevel = \"lean\"\nskills = [\n  \"intent\",  # mine\n  \"review\",\n]\n";
        let r = rename_skills_with(t, &fake_default(false));
        assert!(r.by_hand.is_empty(), "{:?}", r.by_hand);
        assert_eq!(
            r.text,
            t.replace("\"intent\"", "\"brainstorm\""),
            "the rename alone"
        );
        assert_eq!(r.edits.len(), 1, "{:?}", r.edits);
        assert_eq!(r.warnings.len(), 1, "{:?}", r.warnings);
        assert!(
            r.warnings[0].contains("did not add `tdd`, `finish` and `post-mortem`"),
            "{:?}",
            r.warnings
        );
        // A second call changes nothing but the warning.
        let again = rename_skills_with(&r.text, &fake_default(false));
        assert_eq!(again.text, r.text);
        assert!(again.edits.is_empty(), "{:?}", again.edits);
    }

    #[test]
    fn the_change_policy_none_adds_no_skill() {
        // `init` writes no skill for the policy `none`, and a file with no
        // `[change]` table has that policy.
        for change in ["[change]\npolicy = \"none\"\n\n", ""] {
            let t = format!(
                "spec-dna = \"0.6.1\"\n\n{change}[harness]\nlevel = \"lean\"\nskills = [\"review\"]\n"
            );
            let r = rename_skills_with(&t, &fake_default(false));
            assert_eq!(r.text, t, "{change}");
            assert!(r.edits.is_empty() && r.warnings.is_empty(), "{change}");
        }
    }
}

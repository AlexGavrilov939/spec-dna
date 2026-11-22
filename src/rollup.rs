//! The roll-up (design 0.8.0, D50 to D53). A summary of an area, and a
//! `system` or `product` document with the field `rests-on`, store pins of the
//! documents below them: `<doc-id>#<key>@<16 hex>` for a requirement and
//! `<doc-id>@<16 hex>` for a document. This module reads the pins, computes the
//! frontier of a document (the items that it must pin), matches the pins with
//! the current hashes, and writes the field. The functions are pure: `check`,
//! `refresh`, `new summary` and later `finish`, `next` and `context` share them.
//!
//! The gate checks the pins, never the meaning of a summary.

use crate::change::hash_matches;
use crate::config::Config;
use crate::hash::hex;
use crate::model::{Corpus, Doc, DocType, Res};
use std::collections::{BTreeSet, HashSet};

/// The number of hexadecimal digits of the hash of a pin.
const HASH_DIGITS: usize = 16;

/// One pin: the id of a document, the key of a requirement if the pin names
/// one, and the hash.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    pub id: String,
    pub key: Option<String>,
    pub hash: String,
}

impl Pin {
    /// The id of the target without the hash: `doc#key` or `doc`.
    pub fn target(&self) -> String {
        match &self.key {
            Some(k) => format!("{}#{k}", self.id),
            None => self.id.clone(),
        }
    }

    /// The text of the pin in the front matter.
    pub fn text(&self) -> String {
        format!("{}@{}", self.target(), self.hash)
    }
}

/// Reads a pin. The error continues the sentence "the pin is not in the pin
/// form: ...".
pub fn parse_pin(text: &str) -> Result<Pin, String> {
    let t = text.trim();
    let Some((head, hash)) = t.rsplit_once('@') else {
        return Err("it has no `@<hash>` at its end".to_string());
    };
    if hash.len() != HASH_DIGITS || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!(
            "the hash must be {HASH_DIGITS} hexadecimal digits, as `spec-dna show` prints it"
        ));
    }
    let (id, key) = match head.split_once('#') {
        Some((id, key)) => (id, Some(key)),
        None => (head, None),
    };
    if id.is_empty() || id.contains(char::is_whitespace) || id.contains(':') {
        return Err(
            "the id must be a document id with no space and no `:` (a pin names no other repository)"
                .to_string(),
        );
    }
    if let Some(k) = key
        && (k.is_empty() || k.contains(['#', '@']) || k.contains(char::is_whitespace))
    {
        return Err("the key after `#` must be a requirement key".to_string());
    }
    Ok(Pin {
        id: id.to_string(),
        key: key.map(str::to_string),
        hash: hash.to_ascii_lowercase(),
    })
}

/// A thing that a roll-up document pins: a document of a lower level, or a
/// requirement. `Doc` holds the index of a document in the corpus, and `Req`
/// holds that index and the index of the requirement in `parsed.reqs`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Item {
    Doc(u32),
    Req(u32, usize),
}

impl Item {
    /// The id of the item: `doc#key` for a requirement, else `doc`.
    pub fn id(self, c: &Corpus) -> String {
        match self {
            Item::Doc(d) => c.docs[d as usize].id.clone(),
            Item::Req(d, r) => {
                let doc = &c.docs[d as usize];
                format!("{}#{}", doc.id, doc.parsed.reqs[r].key)
            }
        }
    }

    /// The current hash of the item as a pin writes it: the hash that `show`
    /// prints for a requirement, and the `frozen_hash` of a document.
    pub fn hash(self, c: &Corpus) -> String {
        match self {
            Item::Doc(d) => hex(&c.docs[d as usize].frozen_hash),
            Item::Req(d, r) => hex(&c.docs[d as usize].parsed.reqs[r].hash),
        }
    }

    /// The pin of the item, with the canonical id and the current hash.
    pub fn pin(self, c: &Corpus) -> Pin {
        let (id, key) = match self {
            Item::Doc(d) => (c.docs[d as usize].id.clone(), None),
            Item::Req(d, r) => {
                let doc = &c.docs[d as usize];
                (doc.id.clone(), Some(doc.parsed.reqs[r].key.clone()))
            }
        };
        Pin {
            id,
            key,
            hash: self.hash(c),
        }
    }

    /// True if the pin hash names the current text of the item.
    pub fn matches(self, c: &Corpus, hash: &str) -> bool {
        match self {
            Item::Doc(d) => hash.eq_ignore_ascii_case(&hex(&c.docs[d as usize].frozen_hash)),
            Item::Req(d, r) => hash_matches(&c.docs[d as usize].parsed.reqs[r], hash),
        }
    }
}

/// The level of a roll-up document. The summary of an area has the folders of
/// the area, from the root of the layer `requirements` down. The root area has
/// none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Level {
    Summary(Vec<String>),
    System,
    Product,
}

/// What a check found. The profile and the rules give the severity of the
/// codes SD641 to SD644 that `check` maps from these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// SD641: a pin no longer holds.
    Stale,
    /// SD642: an item of the frontier has no pin.
    Incomplete,
    /// SD643: a pin has a bad form, is ambiguous, names an item of the same or
    /// a higher level, or sits in a type that has no pins.
    Invalid,
    /// SD644: a summary names no folder of the layer `requirements`.
    NoArea,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub kind: Kind,
    pub line: u32,
    /// The pin without its hash, or the id of the missing item.
    pub target: String,
    pub message: String,
}

impl Finding {
    /// True for a finding that `refresh` and `finish` treat as "stale": the
    /// text under a pin changed, or an item has no pin. This is the one
    /// definition of "is stale" for `check`, `refresh` and `finish`.
    pub fn is_stale(&self) -> bool {
        matches!(self.kind, Kind::Stale | Kind::Incomplete)
    }
}

/// The folders of a path, without the file: `a/b/c.md` gives `a`, `b`.
fn folders(rel: &str) -> Vec<String> {
    let mut parts: Vec<&str> = rel.split('/').collect();
    parts.pop();
    parts.into_iter().map(str::to_string).collect()
}

/// True if `inner` is `outer` or a deeper folder of it.
fn within(inner: &[String], outer: &[String]) -> bool {
    inner.starts_with(outer)
}

/// True for a document that holds roll-up pins by its type: a summary, or a
/// `system` or `product` document that has the field `rests-on`. The field can
/// be empty.
pub fn is_rollup(d: &Doc) -> bool {
    d.moved_to.is_none()
        && !d.type_failed
        && match d.ty {
            DocType::Summary => true,
            DocType::System | DocType::Product => d.rests_on.is_some(),
            _ => false,
        }
}

/// The index of the layer that has the type `requirements`.
fn requirements_layer(config: &Config) -> Option<usize> {
    config.layers.layer_of_type(DocType::Requirements)
}

/// The folder of the layer `requirements`, if the layer has one.
fn requirements_dir(config: &Config) -> Option<&str> {
    requirements_layer(config).and_then(|i| config.layers.layers()[i].dir())
}

/// The path of a document inside its root.
fn rel_path<'a>(config: &'a Config, d: &'a Doc) -> &'a str {
    config
        .roots
        .get(d.root)
        .filter(|r| r.path.is_empty() || d.path.starts_with(&format!("{}/", r.path)))
        .map_or(d.path.as_str(), |r| r.rel(&d.path))
}

/// The area of a summary: the path under the folder of its layer, with no
/// extension, as the folders of the area. `None` if the document is not under
/// that folder.
pub fn summary_area(config: &Config, d: &Doc) -> Option<Vec<String>> {
    let layer = config.layers.layer_of_type(DocType::Summary)?;
    let dir = config.layers.layers()[layer].dir()?;
    let rest = rel_path(config, d).strip_prefix(dir)?.strip_prefix('/')?;
    let stem = rest.rsplit_once('.').map_or(rest, |(s, _)| s);
    if stem.is_empty() {
        return None;
    }
    Some(stem.split('/').map(str::to_string).collect())
}

/// The folders of a requirement document under the folder of the layer
/// `requirements`, or `None` if the document is not in that layer. A document
/// that a `paths` glob puts in the layer but that is not under the folder is in
/// the root area.
fn requirement_area(config: &Config, d: &Doc) -> Option<Vec<String>> {
    let layer = requirements_layer(config)?;
    if crate::check::doc_layer(config, d) != Some(layer) {
        return None;
    }
    let rel = rel_path(config, d);
    match requirements_dir(config).and_then(|dir| rel.strip_prefix(dir)?.strip_prefix('/')) {
        Some(rest) => Some(folders(rest)),
        None => Some(Vec::new()),
    }
}

/// True if the tree has the folder of an area under the folder of the layer
/// `requirements`. The root area always has one.
pub fn area_exists(config: &Config, c: &Corpus, area: &[String]) -> bool {
    if area.is_empty() {
        return true;
    }
    let Some(dir) = requirements_dir(config) else {
        return false;
    };
    config.roots.iter().any(|r| {
        let mut path = String::new();
        for part in [r.path.as_str(), dir]
            .into_iter()
            .chain(area.iter().map(String::as_str))
        {
            if part.is_empty() {
                continue;
            }
            if !path.is_empty() {
                path.push('/');
            }
            path.push_str(part);
        }
        c.dirs.contains(&path)
    })
}

/// What a pin resolves to in one tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    Item(Item),
    /// The pin names no item of the tree: the requirement or the document was
    /// removed.
    Missing,
    /// The pin names something that is no item, or names two things.
    Invalid(String),
}

/// Resolves a pin as a reference: the aliases of a moved document, the inline
/// aliases of a moved key, the section aliases of a split and the legacy ids.
pub fn resolve(c: &Corpus, pin: &Pin) -> Resolved {
    match &pin.key {
        None => match c.resolve(&pin.id) {
            Res::Found(d, _, _) => Resolved::Item(Item::Doc(d)),
            Res::Missing => Resolved::Missing,
            Res::Ambiguous(_) => Resolved::Invalid(format!(
                "`{}` is ambiguous: an alias names more than one document",
                pin.id
            )),
            Res::CrossRepo(_) => Resolved::Invalid("it names another repository".to_string()),
        },
        Some(key) => match c.resolve(&format!("{}#{key}", pin.id)) {
            Res::Found(d, Some(k), true) => {
                let doc = &c.docs[d as usize];
                match doc.req(&k) {
                    Some(r) => Resolved::Item(Item::Req(d, r)),
                    None => Resolved::Invalid(format!(
                        "`{}#{key}` names a section, not a requirement",
                        pin.id
                    )),
                }
            }
            Res::Found(_, _, _) | Res::Missing => Resolved::Missing,
            Res::Ambiguous(_) => Resolved::Invalid(format!(
                "`{}#{key}` is ambiguous: an alias names more than one document",
                pin.id
            )),
            Res::CrossRepo(_) => Resolved::Invalid("it names another repository".to_string()),
        },
    }
}

/// The documents and the requirements that the checks of the roll-up read,
/// computed once for one tree.
pub struct Rollup<'a> {
    config: &'a Config,
    c: &'a Corpus,
    /// The active requirement documents of the layer `requirements`, with the
    /// folders of each one.
    reqs: Vec<(u32, Vec<String>)>,
    /// The summaries that have an area with a folder: the document and its area.
    summaries: Vec<(u32, Vec<String>)>,
}

impl<'a> Rollup<'a> {
    /// The roll-up of a tree. A tree with no roll-up document and no field
    /// `rests-on` has nothing to read, so the scan of the requirements is
    /// skipped (a project with no summary pays nothing).
    pub fn build(config: &'a Config, c: &'a Corpus) -> Rollup<'a> {
        if c.docs.iter().any(|d| d.rests_on.is_some() || is_rollup(d)) {
            Rollup::full(config, c)
        } else {
            Rollup {
                config,
                c,
                reqs: Vec::new(),
                summaries: Vec::new(),
            }
        }
    }

    /// The roll-up of a tree with the requirements read. `new summary` uses it
    /// for a tree that has no summary yet.
    pub fn full(config: &'a Config, c: &'a Corpus) -> Rollup<'a> {
        let mut reqs = Vec::new();
        let mut summaries = Vec::new();
        for (i, d) in c.docs.iter().enumerate() {
            let i = i as u32;
            if d.moved_to.is_some() || d.type_failed {
                continue;
            }
            if d.ty == DocType::Summary {
                if let Some(a) = summary_area(config, d)
                    && area_exists(config, c, &a)
                {
                    summaries.push((i, a));
                }
            } else if crate::evidence::needs_evidence(d)
                && let Some(a) = requirement_area(config, d)
            {
                reqs.push((i, a));
            }
        }
        Rollup {
            config,
            c,
            reqs,
            summaries,
        }
    }

    /// The level of a roll-up document, or `None` for a summary that has no
    /// area under the folder of its layer.
    pub fn level(&self, d: &Doc) -> Option<Level> {
        match d.ty {
            DocType::Summary => summary_area(self.config, d).map(Level::Summary),
            DocType::System => Some(Level::System),
            _ => Some(Level::Product),
        }
    }

    /// The frontier of a level (D51): the items that a document of the level
    /// must pin, sorted by id. A summary of a sub-area that has a folder stands
    /// for the requirements of that sub-area. The highest such summary counts.
    pub fn frontier(&self, level: &Level) -> Vec<Item> {
        let mut items: Vec<Item> = match level {
            Level::Product => {
                let systems: Vec<Item> = self
                    .c
                    .docs
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.ty == DocType::System && is_rollup(d))
                    .map(|(i, _)| Item::Doc(i as u32))
                    .collect();
                if systems.is_empty() {
                    self.area_frontier(&[])
                } else {
                    systems
                }
            }
            Level::System => self.area_frontier(&[]),
            Level::Summary(a) => self.area_frontier(a),
        };
        items.sort_by_cached_key(|it| it.id(self.c));
        items.dedup();
        items
    }

    fn area_frontier(&self, area: &[String]) -> Vec<Item> {
        // The summaries of the sub-areas of the area.
        let below: Vec<&(u32, Vec<String>)> = self
            .summaries
            .iter()
            .filter(|(_, b)| b.len() > area.len() && within(b, area))
            .collect();
        let mut items: Vec<Item> = below
            .iter()
            .filter(|(_, b)| !below.iter().any(|(_, x)| x.len() < b.len() && within(b, x)))
            .map(|(d, _)| Item::Doc(*d))
            .collect();
        for (d, a) in &self.reqs {
            if within(a, area) && !below.iter().any(|(_, x)| within(a, x)) {
                let doc = &self.c.docs[*d as usize];
                items.extend(doc.reqs.iter().map(|&r| Item::Req(*d, r)));
            }
        }
        items
    }

    /// Why no frontier of a document of `level` can ever hold the item, or
    /// `None` if a frontier could hold it (design 0.8.0, D52). A frontier holds
    /// a requirement of a document of the layer `requirements`, a summary
    /// below the document (a `summary` pins a summary of a deeper area, a
    /// `system` document a summary, and a `product` document a summary or a
    /// `system` document). Any other target gives SD643. A target that a
    /// frontier could hold and now does not gives SD641: it moved, or a
    /// summary of a sub-area stands for it, or a `system` document lost its
    /// `rests-on`.
    fn cannot_be_held(&self, level: &Level, item: Item) -> Option<String> {
        let doc = &self.c.docs[match item {
            Item::Doc(t) | Item::Req(t, _) => t as usize,
        }];
        match item {
            Item::Req(..) => {
                if doc.ty.has_requirements() && requirement_area(self.config, doc).is_some() {
                    None
                } else {
                    Some(format!(
                        "a requirement of a `{}` document outside the layer `requirements`, which no frontier holds",
                        doc.ty.name()
                    ))
                }
            }
            Item::Doc(_) => match doc.ty {
                DocType::Summary => match level {
                    Level::Summary(a) => (!summary_area(self.config, doc)
                        .is_some_and(|b| b.len() > a.len() && within(&b, a)))
                    .then(|| "a document of the same or a higher level".to_string()),
                    Level::System | Level::Product => None,
                },
                DocType::System if matches!(level, Level::Product) => None,
                DocType::System | DocType::Product => {
                    Some("a document of the same or a higher level".to_string())
                }
                other => Some(format!(
                    "a `{}` document, and no frontier holds a whole document of that type",
                    other.name()
                )),
            },
        }
    }

    /// The findings of one roll-up document: its pins against its frontier
    /// and the current hashes. Each stale pin is one finding with the pin as
    /// its target, so that the gate sees a second stale pin of a document as
    /// new.
    pub fn findings(&self, di: u32) -> Vec<Finding> {
        let d = &self.c.docs[di as usize];
        let Some(level) = self.level(d) else {
            return vec![self.no_area(d)];
        };
        if let Level::Summary(a) = &level
            && !area_exists(self.config, self.c, a)
        {
            return vec![self.no_area(d)];
        }
        let mut out = Vec::new();
        let frontier = self.frontier(&level);
        let mut covered: HashSet<Item> = HashSet::new();
        let fix = format!("spec-dna refresh {}", d.id);
        let line = d
            .rests_on
            .as_ref()
            .and_then(|v| v.first())
            .map_or(1, |p| p.1);
        for (text, line) in d.rests_on.iter().flatten() {
            let finding = |kind: Kind, target: &str, message: String| Finding {
                kind,
                line: *line,
                target: target.to_string(),
                message,
            };
            let pin = match parse_pin(text) {
                Ok(p) => p,
                Err(why) => {
                    out.push(finding(
                        Kind::Invalid,
                        text.trim(),
                        format!(
                            "the pin `{}` is not in the pin form `<doc-id>#<key>@<16 hex>` or `<doc-id>@<16 hex>`: {why}",
                            text.trim()
                        ),
                    ));
                    continue;
                }
            };
            let target = pin.target();
            let item = match resolve(self.c, &pin) {
                Resolved::Invalid(why) => {
                    out.push(finding(
                        Kind::Invalid,
                        &target,
                        format!("the pin `{target}` is invalid: {why}"),
                    ));
                    continue;
                }
                Resolved::Missing => {
                    out.push(finding(
                        Kind::Stale,
                        &target,
                        format!(
                            "the pin `{target}` is stale: the target was removed; edit the summary, then run `{fix}`"
                        ),
                    ));
                    continue;
                }
                Resolved::Item(it) => it,
            };
            let id = item.id(self.c);
            if let Some(why) = self.cannot_be_held(&level, item) {
                out.push(finding(
                    Kind::Invalid,
                    &id,
                    format!(
                        "the pin `{id}` names {why}; a pin names an item that the frontier of this document can hold"
                    ),
                ));
                continue;
            }
            if let Item::Req(t, _) = item
                && !crate::evidence::needs_evidence(&self.c.docs[t as usize])
            {
                out.push(finding(
                    Kind::Stale,
                    &id,
                    format!(
                        "the pin `{id}` is stale: the requirement is not active; edit the summary, then run `{fix}`"
                    ),
                ));
                continue;
            }
            if !frontier.contains(&item) {
                out.push(finding(
                    Kind::Stale,
                    &id,
                    format!(
                        "the pin `{id}` is stale: it is no longer in the frontier of this document (it moved to another area, or a summary of a sub-area now stands for it); edit the summary, then run `{fix}`"
                    ),
                ));
                continue;
            }
            covered.insert(item);
            if !item.matches(self.c, &pin.hash) {
                out.push(finding(
                    Kind::Stale,
                    &id,
                    format!(
                        "the pin `{id}` is stale: its text changed; read it, edit the summary, then run `{fix}`"
                    ),
                ));
            }
        }
        for item in frontier {
            if !covered.contains(&item) {
                let id = item.id(self.c);
                out.push(Finding {
                    kind: Kind::Incomplete,
                    line,
                    target: id.clone(),
                    message: format!(
                        "`{id}` has no pin; read it, edit the summary, then run `{fix}`"
                    ),
                });
            }
        }
        out
    }

    fn no_area(&self, d: &Doc) -> Finding {
        let dir = requirements_dir(self.config);
        let message = match (summary_area(self.config, d), dir) {
            (Some(a), Some(dir)) => format!(
                "the summary names the area `{0}`, and the layer `requirements` has no folder `{dir}/{0}`; move the summary to the path of an existing area",
                a.join("/")
            ),
            (None, _) => {
                "the summary is not a file under the folder of its layer, so it names no area"
                    .to_string()
            }
            (Some(a), None) => format!(
                "the summary names the area `{}`, and no layer `requirements` with a folder exists",
                a.join("/")
            ),
        };
        Finding {
            kind: Kind::NoArea,
            line: 1,
            target: String::new(),
            message,
        }
    }

    /// The findings of each roll-up document, and of each document that has the
    /// field `rests-on` and no place for it. A document of another type is
    /// reported once for each pin (SD643), or once if it has no pin.
    pub fn all(&self) -> Vec<(u32, Finding)> {
        let mut out = Vec::new();
        for (i, d) in self.c.docs.iter().enumerate() {
            let i = i as u32;
            if is_rollup(d) {
                // A summary in a project with no layer for it has SD303 only.
                if d.ty == DocType::Summary
                    && self.config.layers.layer_of_type(DocType::Summary).is_none()
                {
                    continue;
                }
                out.extend(self.findings(i).into_iter().map(|f| (i, f)));
            } else if let Some(pins) = &d.rests_on
                && d.moved_to.is_none()
            {
                let place = format!(
                    "a `{}` document has no pins; `rests-on` is for a `summary`, and for a `system` or `product` document",
                    d.ty.name()
                );
                let mut seen: HashSet<&str> = HashSet::new();
                for (text, line) in pins {
                    if seen.insert(text.trim()) {
                        out.push((
                            i,
                            Finding {
                                kind: Kind::Invalid,
                                line: *line,
                                target: text.trim().to_string(),
                                message: format!("the pin `{}`: {place}", text.trim()),
                            },
                        ));
                    }
                }
                if pins.is_empty() {
                    out.push((
                        i,
                        Finding {
                            kind: Kind::Invalid,
                            line: 1,
                            target: "rests-on".to_string(),
                            message: format!("`rests-on` is empty: {place}"),
                        },
                    ));
                }
            }
        }
        out
    }

    /// The findings of a roll-up document that make it stale: the shared
    /// "is stale" rule of `check`, `refresh` and `finish`.
    pub fn stale(&self, di: u32) -> Vec<Finding> {
        self.findings(di)
            .into_iter()
            .filter(Finding::is_stale)
            .collect()
    }

    /// The area of the document that a delta line names, or `None` if the
    /// requirements of the document are in no frontier. An active requirements
    /// document of the layer `requirements` has the area of its folders. An id
    /// that names no document is a new document (an `ADD` line): its area is
    /// the folders of the id under the folder of the layer.
    fn target_area(&self, id: &str) -> Option<Vec<String>> {
        match self.c.resolve(id) {
            Res::Found(d, None, _) => self
                .reqs
                .iter()
                .find(|(i, _)| *i == d)
                .map(|(_, a)| a.clone()),
            Res::Missing => {
                let dir: Vec<String> = requirements_dir(self.config)?
                    .split('/')
                    .map(str::to_string)
                    .collect();
                let mut parts = folders(id);
                parts.starts_with(&dir).then(|| parts.split_off(dir.len()))
            }
            _ => None,
        }
    }

    /// True if the frontier of `level` holds the requirements of a document in
    /// the area `area`. It mirrors `area_frontier`: a summary of a sub-area
    /// stands for the requirements below it. A `product` document that has
    /// `system` documents with `rests-on` pins those, so it holds no
    /// requirement.
    fn holds_area(&self, level: &Level, area: &[String]) -> bool {
        let stood_for = |root: &[String]| {
            self.summaries
                .iter()
                .any(|(_, b)| b.len() > root.len() && within(b, root) && within(area, b))
        };
        match level {
            Level::Summary(a) => within(area, a) && !stood_for(a),
            Level::System => !stood_for(&[]),
            Level::Product => {
                !self
                    .c
                    .docs
                    .iter()
                    .any(|d| d.ty == DocType::System && is_rollup(d))
                    && !stood_for(&[])
            }
        }
    }

    /// The roll-up documents that a delta makes due (design 0.8.0, D54): each
    /// one whose frontier holds a requirement of a document that a line of the
    /// delta names. `targets` are the ids of those documents. The area of the
    /// document decides, so an `ADD` line (a requirement that the tree does not
    /// have yet) and a `REMOVE` line (one that the result no longer has) count
    /// as a `CHANGE` line does. `next` and `finish` share this function.
    pub fn due(&self, targets: &[String]) -> Vec<u32> {
        let areas: Vec<Vec<String>> = targets.iter().filter_map(|t| self.target_area(t)).collect();
        if areas.is_empty() {
            return Vec::new();
        }
        (0..self.c.docs.len() as u32)
            .filter(|&i| is_rollup(&self.c.docs[i as usize]))
            .filter(|&i| has_area(self.config, self.c, &self.c.docs[i as usize]))
            .filter(|&i| {
                self.level(&self.c.docs[i as usize])
                    .is_some_and(|l| areas.iter().any(|a| self.holds_area(&l, a)))
            })
            .collect()
    }

    /// The pins that a document of `level` must have now: the pins of its
    /// frontier with the current hashes, sorted by id.
    pub fn pins_for(&self, level: &Level) -> Vec<Pin> {
        self.frontier(level)
            .into_iter()
            .map(|it| it.pin(self.c))
            .collect()
    }
}

/// The key of a pin for the compare of 2 trees (SD645): the target as the
/// older tree names it, and the hash.
type PinKey = (String, String);

/// The pins of a document with their resolved targets. `names_in` is the tree
/// whose ids the keys use: a target of this tree that the tree `names_in` has
/// under an older id (an alias) gets that id. Thus a pin that `refresh`
/// rewrites from an alias to the canonical id is no change.
fn pin_keys(c: &Corpus, d: &Doc, names_in: Option<&Corpus>) -> BTreeSet<PinKey> {
    d.rests_on
        .iter()
        .flatten()
        .filter_map(|(text, _)| parse_pin(text).ok())
        .map(|pin| {
            let id = match resolve(c, &pin) {
                Resolved::Item(it) => {
                    let (doc, key) = match it {
                        Item::Doc(t) => (&c.docs[t as usize], None),
                        Item::Req(t, r) => {
                            let doc = &c.docs[t as usize];
                            (doc, Some(doc.parsed.reqs[r].key.clone()))
                        }
                    };
                    let name = match names_in.and_then(|b| crate::check::head_to_base(b, doc)) {
                        Some(b) => b.id.clone(),
                        None => doc.id.clone(),
                    };
                    match key {
                        Some(k) => format!("{name}#{k}"),
                        None => name,
                    }
                }
                _ => pin.target(),
            };
            (id, pin.hash)
        })
        .collect()
}

/// SD645: the documents of `head` whose set of pins differs from the set of the
/// same document in `base`, while their text is the same. The compare uses the
/// resolved targets and the hashes. A document that `base` does not have is new,
/// and it gives no finding. The result holds the indices of the documents of
/// `head`.
pub fn repinned(head: &Corpus, base: &Corpus) -> Vec<u32> {
    let mut out = Vec::new();
    for (i, h) in head.docs.iter().enumerate() {
        if !is_rollup(h) {
            // A `system` or `product` document that lost its `rests-on` and
            // kept its text is a repin by deletion: it has no finding of its
            // own any more, so the removal is the claim.
            if matches!(h.ty, DocType::System | DocType::Product)
                && h.rests_on.is_none()
                && h.moved_to.is_none()
                && !h.type_failed
                && let Some(b) = crate::check::head_to_base(base, h)
                && is_rollup(b)
                && b.frozen_hash == h.frozen_hash
            {
                out.push(i as u32);
            }
            continue;
        }
        let Some(b) = crate::check::head_to_base(base, h) else {
            continue;
        };
        if !is_rollup(b) || h.frozen_hash != b.frozen_hash {
            continue;
        }
        if pin_keys(head, h, Some(base)) != pin_keys(base, b, None) {
            out.push(i as u32);
        }
    }
    out
}

/// The folders of an area name that the user gives to `new summary`:
/// `valves` or `valves/drives`. The error is a message for exit code 2.
pub fn parse_area(name: &str) -> Result<Vec<String>, String> {
    let area: Vec<String> = name
        .trim_matches('/')
        .split('/')
        .map(str::to_string)
        .collect();
    if area.iter().any(|p| p.is_empty() || p == "." || p == "..") {
        return Err(format!(
            "`{name}` is not an area: write the folders under the folder of the layer `requirements`, such as `valves` or `valves/drives`"
        ));
    }
    Ok(area)
}

/// The pins of a new summary of `area`: the frontier of the area in the tree,
/// sorted by id. The error says that the layer `requirements` has no folder for
/// the area.
pub fn new_summary_pins(
    config: &Config,
    c: &Corpus,
    area: &[String],
) -> Result<Vec<String>, String> {
    if !area_exists(config, c, area) {
        return Err(match requirements_dir(config) {
            Some(dir) => format!(
                "the layer `requirements` has no folder `{dir}/{}`; an area is a folder of that layer",
                area.join("/")
            ),
            None => "no layer with the type `requirements` has a folder, so there is no area; run `spec-dna layer add requirements --types requirements`".to_string(),
        });
    }
    let r = Rollup::full(config, c);
    Ok(r.pins_for(&Level::Summary(area.to_vec()))
        .iter()
        .map(Pin::text)
        .collect())
}

/// The block of the field `rests-on` as front matter lines.
fn render_field(pins: &[String], eol: &str) -> String {
    if pins.is_empty() {
        return format!("rests-on: []{eol}");
    }
    let mut out = format!("rests-on:{eol}");
    for p in pins {
        out.push_str(&format!("  - {p}{eol}"));
    }
    out
}

/// Sets the field `rests-on` of a document. It changes the lines of that field
/// only: the other fields, the body and the line ends stay as they were. A
/// document with no front matter gets one. A field that is absent comes last.
/// The block list, the flow list and a scalar are the forms that it replaces.
pub fn set_rests_on(text: &str, pins: &[String]) -> String {
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let block = render_field(pins, eol);
    let Some((fm, _)) = crate::frontmatter::split(text) else {
        return format!("---{eol}{block}---{eol}{text}");
    };
    let start = 3 + if text[3..].starts_with("\r\n") { 2 } else { 1 };
    let lines: Vec<&str> = fm.split_inclusive('\n').collect();
    fn is_key(l: &str) -> Option<&str> {
        let t = l.trim_end();
        t.strip_prefix("rests-on:")
            .or_else(|| t.strip_prefix("\"rests-on\":"))
    }
    let mut out = String::from(&text[..start]);
    let Some(at) = lines.iter().position(|l| is_key(l).is_some()) else {
        out.push_str(fm);
        if !fm.is_empty() && !fm.ends_with('\n') {
            out.push_str(eol);
        }
        out.push_str(&block);
        out.push_str(&text[start + fm.len()..]);
        return out;
    };
    // The end of the field: the lines of a block list, or the line itself.
    let rest = is_key(lines[at]).unwrap_or("");
    let value_on_line = !strip_comment(rest).trim().is_empty();
    let mut end = at + 1;
    if !value_on_line {
        let mut last = at;
        for (i, l) in lines.iter().enumerate().skip(at + 1) {
            let t = l.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if l.starts_with([' ', '\t']) || t == "-" || t.starts_with("- ") {
                last = i;
            } else {
                break;
            }
        }
        end = last + 1;
    }
    for l in &lines[..at] {
        out.push_str(l);
    }
    out.push_str(&block);
    for l in &lines[end..] {
        out.push_str(l);
    }
    out.push_str(&text[start + fm.len()..]);
    out
}

/// Removes a ` #comment` of a plain value.
fn strip_comment(s: &str) -> &str {
    match s.find(" #") {
        Some(i) => &s[..i],
        None => s,
    }
}

/// Why `refresh` cannot take a document that the user named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectError {
    /// The id names no document (exit code 5).
    NotFound(String),
    /// The id names a document that is not a roll-up document, a summary that
    /// has no area, or more than one document (exit code 2).
    Usage(String),
}

/// True if `refresh` can write the pins of the document: a summary needs an
/// area that has a folder (SD644 names the others), and the other roll-up
/// documents need nothing.
fn has_area(config: &Config, c: &Corpus, d: &Doc) -> bool {
    match d.ty {
        DocType::Summary => summary_area(config, d).is_some_and(|a| area_exists(config, c, &a)),
        _ => true,
    }
}

/// The roll-up documents that the ids name, in the order of the ids. All ids
/// are checked before the caller writes anything.
pub fn select(config: &Config, c: &Corpus, ids: &[String]) -> Result<Vec<u32>, SelectError> {
    let mut out: Vec<u32> = Vec::new();
    for id in ids {
        let d = match c.resolve(id) {
            Res::Found(d, None, _) => d,
            Res::Found(_, Some(_), _) => {
                return Err(SelectError::Usage(format!(
                    "`{id}` names a section or a requirement, not a roll-up document"
                )));
            }
            Res::Ambiguous(_) => {
                return Err(SelectError::Usage(format!(
                    "`{id}` is ambiguous: an alias names more than one document"
                )));
            }
            Res::Missing | Res::CrossRepo(_) => {
                return Err(SelectError::NotFound(format!("`{id}` names no document")));
            }
        };
        if !is_rollup(&c.docs[d as usize]) {
            let doc = &c.docs[d as usize];
            let why = match doc.ty {
                DocType::System | DocType::Product => {
                    "it has no field `rests-on`; write `rests-on: []` in its front matter to make it one"
                }
                _ => "only a summary, and a `system` or `product` document with `rests-on`, is one",
            };
            return Err(SelectError::Usage(format!(
                "`{}` is not a roll-up document: {why}",
                doc.id
            )));
        }
        if !has_area(config, c, &c.docs[d as usize]) {
            return Err(SelectError::Usage(format!(
                "`{}` names no area: a summary is the file `<summaries folder>/<area>.md` of a folder of the layer `requirements` (SD644), and no pin can fix its path",
                c.docs[d as usize].id
            )));
        }
        if !out.contains(&d) {
            out.push(d);
        }
    }
    Ok(out)
}

/// The roll-up documents that `refresh --all` takes: each one that has a
/// finding, except a summary that has no area (SD644 names it, and no pin can
/// fix it).
pub fn select_all(roll: &Rollup) -> Vec<u32> {
    (0..roll.c.docs.len() as u32)
        .filter(|&i| is_rollup(&roll.c.docs[i as usize]))
        .filter(|&i| {
            has_area(roll.config, roll.c, &roll.c.docs[i as usize]) && !roll.findings(i).is_empty()
        })
        .collect()
}

/// What `refresh` does with one document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// The pins are written.
    Refreshed,
    /// The pins are written because of `--holds`: the guard would refuse them.
    Held,
    /// The guard refuses: the document has stale pins and its text is the text
    /// of the base.
    Refused,
}

/// The result of `plan` for one document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Planned {
    pub doc: u32,
    pub id: String,
    pub path: String,
    pub verdict: Verdict,
    /// The new text of the file. It is the old text for a refusal.
    pub text: String,
    pub pins: usize,
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
    /// The targets of the stale findings: the pins that make a refusal.
    pub stale: Vec<String>,
}

/// Plans `refresh` for one roll-up document (D53). `text` is the file as it is
/// now, and `base` is the tree of the guard: the base of the change checks, or
/// `HEAD`. The new text has the frontier of the document with the current
/// hashes and the canonical ids, sorted. It changes the front matter only.
///
/// The guard: a document that has a stale pin (the shared rule `Finding::
/// is_stale`) and has the `frozen_hash` of the base is not refreshed, unless
/// `holds` is true. A document that the base does not have, or has as a
/// document that is no roll-up document, has no guard.
pub fn plan(roll: &Rollup, base: Option<&Corpus>, di: u32, holds: bool, text: &str) -> Planned {
    let c = roll.c;
    let d = &c.docs[di as usize];
    let stale: Vec<String> = roll.stale(di).into_iter().map(|f| f.target).collect();
    let in_base = base.and_then(|b| crate::check::head_to_base(b, d));
    // The base copy must be a roll-up document too. A `system` or `product`
    // document that gets its first `rests-on` has no pins in the base, so its
    // first pins are no claim that a text still holds.
    let guarded = !stale.is_empty()
        && in_base.is_some_and(|b| is_rollup(b) && b.frozen_hash == d.frozen_hash);
    let verdict = match (guarded, holds) {
        (true, false) => Verdict::Refused,
        (true, true) => Verdict::Held,
        (false, _) => Verdict::Refreshed,
    };
    let level = roll
        .level(d)
        .expect("select and select_all keep summaries that have an area");
    let pins = roll.pins_for(&level);
    let items = roll.frontier(&level);
    // The old pins by the item that they resolve to.
    let old: Vec<(String, Option<Item>)> = d
        .rests_on
        .iter()
        .flatten()
        .map(|(t, _)| {
            let item = parse_pin(t).ok().and_then(|p| match resolve(c, &p) {
                Resolved::Item(it) => Some(it),
                _ => None,
            });
            (t.trim().to_ascii_lowercase(), item)
        })
        .collect();
    let (mut added, mut changed) = (0, 0);
    for (item, pin) in items.iter().zip(&pins) {
        let same: Vec<&(String, Option<Item>)> =
            old.iter().filter(|(_, it)| *it == Some(*item)).collect();
        if same.is_empty() {
            added += 1;
        } else if !same.iter().any(|(t, _)| *t == pin.text()) {
            changed += 1;
        }
    }
    let removed = old
        .iter()
        .filter(|(_, it)| it.is_none_or(|it| !items.contains(&it)))
        .count();
    let texts: Vec<String> = pins.iter().map(Pin::text).collect();
    Planned {
        doc: di,
        id: d.id.clone(),
        path: d.path.clone(),
        verdict,
        text: match verdict {
            Verdict::Refused => text.to_string(),
            _ => set_rests_on(text, &texts),
        },
        pins: pins.len(),
        added,
        changed,
        removed,
        stale: match verdict {
            Verdict::Refreshed => Vec::new(),
            _ => stale,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layer::{Graph, render_graph};
    use std::sync::Arc;

    /// The default graph, which has the layer `summaries`, and one mixed root.
    fn config() -> Config {
        Config::parse(&format!(
            "spec-dna = \"0.1.0\"\n\n{}\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n",
            render_graph(&Graph::default_graph())
        ))
        .unwrap()
    }

    fn corpus(cfg: &Config, docs: &[(&str, &str)]) -> Corpus {
        let files = docs.iter().map(|(p, _)| p.to_string()).collect();
        let parsed = docs
            .iter()
            .map(|(p, t)| (p.to_string(), Arc::new(crate::parse::parse(t))))
            .collect();
        Corpus::build(cfg, files, parsed)
    }

    /// An active requirements document with the requirement `key`.
    fn req(key: &str) -> String {
        format!(
            "---\nstatus: active\n---\n# T\n\n## Requirements\n\n- **{key}**: The valve SHALL close {key}.\n"
        )
    }

    const SUMMARY: &str = "# S\n\nText.\n";

    /// The ids of the frontier of a level.
    fn ids(c: &Corpus, cfg: &Config, level: &Level) -> Vec<String> {
        Rollup::full(cfg, c)
            .frontier(level)
            .into_iter()
            .map(|i| i.id(c))
            .collect()
    }

    fn area(s: &str) -> Level {
        Level::Summary(
            s.split('/')
                .filter(|p| !p.is_empty())
                .map(str::to_string)
                .collect(),
        )
    }

    #[test]
    fn a_pin_has_the_form_id_key_and_16_hex() {
        let p = parse_pin("requirements/valves/closing#max-angle@0B7e2d91c4a5f310").unwrap();
        assert_eq!(p.id, "requirements/valves/closing");
        assert_eq!(p.key.as_deref(), Some("max-angle"));
        assert_eq!(p.hash, "0b7e2d91c4a5f310");
        assert_eq!(
            p.text(),
            "requirements/valves/closing#max-angle@0b7e2d91c4a5f310"
        );
        let d = parse_pin("summaries/valves@9a1c44e0d2b7f6a3").unwrap();
        assert_eq!(
            (d.key.clone(), d.target()),
            (None, "summaries/valves".to_string())
        );
        for bad in [
            "",
            "a/b",
            "a/b@123",
            "a/b@0b7e2d91c4a5f31",
            "a/b@0b7e2d91c4a5f3100",
            "a/b@zzzzzzzzzzzzzzzz",
            "@0b7e2d91c4a5f310",
            "other:a/b@0b7e2d91c4a5f310",
            "a b#k@0b7e2d91c4a5f310",
            "a/b#@0b7e2d91c4a5f310",
            "a/b#k#j@0b7e2d91c4a5f310",
        ] {
            assert!(parse_pin(bad).is_err(), "`{bad}` is a pin");
        }
    }

    #[test]
    fn the_frontier_of_a_nested_area_follows_the_summaries_of_the_sub_areas() {
        let cfg = config();
        let mut docs: Vec<(&str, String)> = vec![
            ("docs/requirements/valves/closing.md", req("close")),
            ("docs/requirements/valves/drives/motor.md", req("spin")),
            ("docs/requirements/valves/drives/gear/box.md", req("mesh")),
            ("docs/requirements/zones.md", req("zone")),
            // A draft and a retired requirement are not active.
            (
                "docs/requirements/valves/draft.md",
                req("later").replace("active", "draft"),
            ),
            (
                "docs/requirements/valves/old.md",
                req("gone").replace("active", "retired"),
            ),
        ];
        let build = |docs: &[(&str, String)]| {
            let v: Vec<(&str, &str)> = docs.iter().map(|(p, t)| (*p, t.as_str())).collect();
            corpus(&cfg, &v)
        };
        // No summary: each active requirement under the folder.
        let c = build(&docs);
        assert_eq!(
            ids(&c, &cfg, &area("valves")),
            [
                "requirements/valves/closing#close",
                "requirements/valves/drives/gear/box#mesh",
                "requirements/valves/drives/motor#spin"
            ]
        );
        assert_eq!(ids(&c, &cfg, &area("valves/drives")).len(), 2);
        // The root area is the whole layer.
        assert_eq!(ids(&c, &cfg, &Level::System).len(), 4);
        assert_eq!(
            ids(&c, &cfg, &Level::Product),
            ids(&c, &cfg, &Level::System)
        );
        // A summary of a sub-area stands for its requirements.
        docs.push(("docs/summaries/valves/drives.md", SUMMARY.to_string()));
        let c = build(&docs);
        assert_eq!(
            ids(&c, &cfg, &area("valves")),
            [
                "requirements/valves/closing#close",
                "summaries/valves/drives"
            ]
        );
        // A summary of a deeper area under it does not change the item of
        // `valves`: the highest summary counts. The sub-area itself pins the
        // deeper one.
        docs.push(("docs/summaries/valves/drives/gear.md", SUMMARY.to_string()));
        let c = build(&docs);
        assert_eq!(
            ids(&c, &cfg, &area("valves")),
            [
                "requirements/valves/closing#close",
                "summaries/valves/drives"
            ]
        );
        assert_eq!(
            ids(&c, &cfg, &area("valves/drives")),
            [
                "requirements/valves/drives/motor#spin",
                "summaries/valves/drives/gear"
            ]
        );
        // The system rests on the highest summaries and on the requirements
        // that no summary covers.
        assert_eq!(
            ids(&c, &cfg, &Level::System),
            [
                "requirements/valves/closing#close",
                "requirements/zones#zone",
                "summaries/valves/drives"
            ]
        );
        docs.push(("docs/summaries/valves.md", SUMMARY.to_string()));
        let c = build(&docs);
        assert_eq!(
            ids(&c, &cfg, &Level::System),
            ["requirements/zones#zone", "summaries/valves"]
        );
        // The product rests on each `system` document that has the field. With
        // none, it rests on the layer.
        docs.push((
            "docs/system.md",
            "---\nrests-on: []\n---\n# System\n".to_string(),
        ));
        let c = build(&docs);
        assert_eq!(ids(&c, &cfg, &Level::Product), ["system"]);
        // A summary of an area with no folder is in no frontier.
        docs.push(("docs/summaries/nope.md", SUMMARY.to_string()));
        let c = build(&docs);
        assert!(
            !ids(&c, &cfg, &Level::System)
                .iter()
                .any(|i| i.contains("nope"))
        );
        assert!(!ids(&c, &cfg, &area("")).iter().any(|i| i.contains("nope")));
    }

    #[test]
    fn a_pin_resolves_through_a_document_alias_a_key_alias_and_a_section_alias() {
        let cfg = config();
        let moved = "---\nstatus: active\naliases: [requirements/valves/closing, requirements/valves/split#part-key]\n---\n# T\n\n## Requirements\n\n- **close**: The valve SHALL close. <!-- aliases: requirements/valves/closing#ttl -->\n- **part-key**: The valve SHALL open.\n";
        let c = corpus(&cfg, &[("docs/requirements/valves/shutoff.md", moved)]);
        let want = |key: &str| {
            let d = &c.docs[0];
            Resolved::Item(Item::Req(0, d.req(key).unwrap()))
        };
        let pin = |id: &str, key: &str| Pin {
            id: id.to_string(),
            key: Some(key.to_string()),
            hash: "0".repeat(16),
        };
        // The canonical id, an alias of the document, an alias of the key, and
        // the section alias of a split.
        assert_eq!(
            resolve(&c, &pin("requirements/valves/shutoff", "close")),
            want("close")
        );
        assert_eq!(
            resolve(&c, &pin("requirements/valves/closing", "close")),
            want("close")
        );
        assert_eq!(
            resolve(&c, &pin("requirements/valves/closing", "ttl")),
            want("close")
        );
        assert_eq!(
            resolve(&c, &pin("requirements/valves/split", "part-key")),
            want("part-key")
        );
        // The item has the canonical id and the current hash.
        let it = Item::Req(0, c.docs[0].req("close").unwrap());
        assert_eq!(it.id(&c), "requirements/valves/shutoff#close");
        assert_eq!(it.pin(&c).hash.len(), 16);
        assert!(it.matches(&c, &it.pin(&c).hash));
        assert!(!it.matches(&c, "0000000000000000"));
        // Nothing, a section that is no requirement, and a removed key.
        assert_eq!(
            resolve(&c, &pin("requirements/valves/nope", "close")),
            Resolved::Missing
        );
        assert_eq!(
            resolve(&c, &pin("requirements/valves/shutoff", "gone")),
            Resolved::Missing
        );
        assert!(matches!(
            resolve(&c, &pin("requirements/valves/shutoff", "requirements")),
            Resolved::Invalid(_)
        ));
        // A document pin.
        let d = Pin {
            id: "requirements/valves/closing".to_string(),
            key: None,
            hash: "0".repeat(16),
        };
        assert_eq!(resolve(&c, &d), Resolved::Item(Item::Doc(0)));
        // 2 documents with one alias: ambiguous.
        let other = "---\nstatus: active\naliases: [requirements/valves/closing]\n---\n# T\n\n## Requirements\n\n- **x**: The valve SHALL x.\n";
        let c = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/shutoff.md", moved),
                ("docs/requirements/valves/other.md", other),
            ],
        );
        assert!(matches!(resolve(&c, &d), Resolved::Invalid(m) if m.contains("ambiguous")));
    }

    fn kinds(c: &Corpus, cfg: &Config, path: &str) -> Vec<(Kind, String)> {
        let r = Rollup::build(cfg, c);
        let i = c.by_path[path];
        r.findings(i)
            .into_iter()
            .map(|f| (f.kind, f.target))
            .collect()
    }

    #[test]
    fn a_pin_that_names_the_same_or_a_higher_level_is_invalid_and_no_cycle_passes() {
        let cfg = config();
        let h = "0".repeat(16);
        let drives = format!(
            "---\nrests-on:\n  - summaries/valves@{h}\n  - summaries/valves/drives@{h}\n---\n# D\n"
        );
        let valves = format!("---\nrests-on:\n  - summaries/valves/drives@{h}\n---\n# V\n");
        let system = format!("---\nrests-on:\n  - product@{h}\n  - system@{h}\n---\n# S\n");
        let product = format!("---\nrests-on:\n  - system@{h}\n---\n# P\n");
        let c = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/drives/motor.md", &req("spin")),
                ("docs/summaries/valves.md", &valves),
                ("docs/summaries/valves/drives.md", &drives),
                ("docs/system.md", &system),
                ("docs/product.md", &product),
            ],
        );
        // The parent and the summary itself.
        let k = kinds(&c, &cfg, "docs/summaries/valves/drives.md");
        assert_eq!(
            k.iter().filter(|(k, _)| *k == Kind::Invalid).count(),
            2,
            "{k:?}"
        );
        // The summary of the parent pins the child: that is allowed.
        let k = kinds(&c, &cfg, "docs/summaries/valves.md");
        assert!(!k.iter().any(|(k, _)| *k == Kind::Invalid), "{k:?}");
        // A system pins no product and no system.
        let k = kinds(&c, &cfg, "docs/system.md");
        assert_eq!(
            k.iter().filter(|(k, _)| *k == Kind::Invalid).count(),
            2,
            "{k:?}"
        );
        // A product pins a system.
        let k = kinds(&c, &cfg, "docs/product.md");
        assert!(!k.iter().any(|(k, _)| *k == Kind::Invalid), "{k:?}");
    }

    #[test]
    fn a_stale_pin_names_its_reason_and_the_fix() {
        let cfg = config();
        let base = corpus(
            &cfg,
            &[("docs/requirements/valves/closing.md", &req("close"))],
        );
        let r = Rollup::full(&cfg, &base);
        let pins = r.pins_for(&area("valves"));
        assert_eq!(pins.len(), 1);
        let text = pins[0].text();
        let summary = format!("---\nrests-on:\n  - {text}\n---\n# S\n");
        let c = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/closing.md", &req("close")),
                ("docs/summaries/valves.md", &summary),
            ],
        );
        assert!(kinds(&c, &cfg, "docs/summaries/valves.md").is_empty());
        // The text changes: stale, and the message names the fix.
        let changed = req("close").replace("SHALL close close", "SHALL close at once");
        let c = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/closing.md", &changed),
                ("docs/summaries/valves.md", &summary),
            ],
        );
        let r = Rollup::build(&cfg, &c);
        let f = r.findings(c.by_path["docs/summaries/valves.md"]);
        assert_eq!(f.len(), 1, "{f:?}");
        assert_eq!(f[0].kind, Kind::Stale);
        assert!(f[0].is_stale());
        assert_eq!(f[0].target, "requirements/valves/closing#close");
        assert!(
            f[0].message.contains("its text changed"),
            "{}",
            f[0].message
        );
        assert!(
            f[0].message.contains("spec-dna refresh summaries/valves"),
            "{}",
            f[0].message
        );
        assert_eq!(r.stale(c.by_path["docs/summaries/valves.md"]), f);
        // The summary is a roll-up document and the requirement is not.
        assert!(is_rollup(
            &c.docs[c.by_path["docs/summaries/valves.md"] as usize]
        ));
        assert!(!is_rollup(
            &c.docs[c.by_path["docs/requirements/valves/closing.md"] as usize]
        ));
    }

    #[test]
    fn the_frozen_hash_of_a_summary_ignores_link_destinations_and_the_pins() {
        let cfg = config();
        let a = "---\nrests-on:\n  - requirements/a#k@0000000000000000\n---\n# S\n\nSee [the valves](../requirements/valves.md#close).\n";
        let b = "---\nrests-on:\n  - requirements/a#k@1111111111111111\n  - requirements/b#j@2222222222222222\n---\n# S\n\nSee [the valves](../requirements/valves/closing.md#close).\n";
        let c = "---\nrests-on: []\n---\n# S\n\nSee [the valves](../requirements/valves.md#close) and more.\n";
        let k = corpus(
            &cfg,
            &[
                ("docs/summaries/a.md", a),
                ("docs/summaries/b.md", b),
                ("docs/summaries/c.md", c),
            ],
        );
        let h = |i: usize| k.docs[i].frozen_hash;
        assert_eq!(h(0), h(1));
        assert_ne!(h(0), h(2));
    }

    #[test]
    fn set_rests_on_changes_the_lines_of_the_field_only() {
        let pins = [
            "a/b#k@0000000000000000".to_string(),
            "c@1111111111111111".to_string(),
        ];
        let block = "rests-on:\n  - a/b#k@0000000000000000\n  - c@1111111111111111\n";
        // A block list, with other fields and a comment around it.
        let old = "---\n# note\nowner: me\nrests-on:\n  - x@2222222222222222\n  - y@3333333333333333\naliases: [q]\n---\n# T\n\nBody.\n";
        assert_eq!(
            set_rests_on(old, &pins),
            format!("---\n# note\nowner: me\n{block}aliases: [q]\n---\n# T\n\nBody.\n")
        );
        // A block list at the margin, a flow list and an empty value.
        for form in [
            "rests-on:\n- x@2222222222222222\n",
            "rests-on: [x@2222222222222222]\n",
            "rests-on: x@2222222222222222\n",
            "rests-on:\n",
            "rests-on: [] # none\n",
        ] {
            let t = format!("---\n{form}owner: me\n---\n# T\n");
            assert_eq!(
                set_rests_on(&t, &pins),
                format!("---\n{block}owner: me\n---\n# T\n"),
                "{form}"
            );
        }
        // The field is the last one: the closing line stays.
        let t = "---\nowner: me\nrests-on:\n  - x@2222222222222222\n---\n# T\n";
        assert_eq!(
            set_rests_on(t, &[]),
            "---\nowner: me\nrests-on: []\n---\n# T\n"
        );
        // An absent field comes last. A text with no front matter gets one.
        assert_eq!(
            set_rests_on("---\nowner: me\n---\n# T\n", &pins),
            format!("---\nowner: me\n{block}---\n# T\n")
        );
        assert_eq!(
            set_rests_on("# T\n\nBody.\n", &pins),
            format!("---\n{block}---\n# T\n\nBody.\n")
        );
        // The line ends of the file stay: a file with CRLF gets CRLF.
        let crlf = "---\r\nowner: me\r\nrests-on:\r\n  - x@2222222222222222\r\n---\r\n# T\r\n";
        assert_eq!(
            set_rests_on(crlf, &pins),
            "---\r\nowner: me\r\nrests-on:\r\n  - a/b#k@0000000000000000\r\n  - c@1111111111111111\r\n---\r\n# T\r\n"
        );
        // The result reads back as the same pins.
        let out = set_rests_on(old, &pins);
        let c = corpus(&config(), &[("docs/summaries/a.md", &out)]);
        let read: Vec<String> = c.docs[0]
            .rests_on
            .iter()
            .flatten()
            .map(|(p, _)| p.clone())
            .collect();
        assert_eq!(read, pins);
    }

    #[test]
    fn a_pin_that_refresh_rewrites_from_an_alias_to_the_canonical_id_is_no_change() {
        let cfg = config();
        let old_req = req("close");
        let r0 = corpus(&cfg, &[("docs/requirements/valves/closing.md", &old_req)]);
        let pin0 = Rollup::full(&cfg, &r0).pins_for(&area("valves"))[0].text();
        let body = "# S\n\nText.\n";
        let base = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/closing.md", &old_req),
                (
                    "docs/summaries/valves.md",
                    &format!("---\nrests-on:\n  - {pin0}\n---\n{body}"),
                ),
            ],
        );
        // The document moves; the old id is an alias; the pin names the new id.
        let moved = old_req.replace(
            "status: active\n",
            "status: active\naliases: [requirements/valves/closing]\n",
        );
        let new_pin = pin0.replace("valves/closing", "valves/shutoff");
        let head = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/shutoff.md", &moved),
                (
                    "docs/summaries/valves.md",
                    &format!("---\nrests-on:\n  - {new_pin}\n---\n{body}"),
                ),
            ],
        );
        assert!(repinned(&head, &base).is_empty());
        // A pin with another hash, and the same text: a claim.
        let other = new_pin.replace(&new_pin[new_pin.len() - 16..], "ffffffffffffffff");
        let head = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/shutoff.md", &moved),
                (
                    "docs/summaries/valves.md",
                    &format!("---\nrests-on:\n  - {other}\n---\n{body}"),
                ),
            ],
        );
        assert_eq!(repinned(&head, &base).len(), 1);
        // The same pins and another text: no claim.
        let head = corpus(
            &cfg,
            &[
                ("docs/requirements/valves/shutoff.md", &moved),
                (
                    "docs/summaries/valves.md",
                    &format!("---\nrests-on:\n  - {other}\n---\n# S\n\nNew text.\n"),
                ),
            ],
        );
        assert!(repinned(&head, &base).is_empty());
        // A new document has no base.
        assert!(repinned(&base, &corpus(&cfg, &[])).is_empty());
    }
    /// The roll-up documents that the delta of the ids makes due, as ids.
    fn due_ids(c: &Corpus, cfg: &Config, targets: &[&str]) -> Vec<String> {
        let t: Vec<String> = targets.iter().map(|s| s.to_string()).collect();
        Rollup::build(cfg, c)
            .due(&t)
            .into_iter()
            .map(|i| c.docs[i as usize].id.clone())
            .collect()
    }

    #[test]
    fn a_summary_is_due_when_its_frontier_holds_a_requirement_of_the_delta() {
        let cfg = config();
        let pinned = "---\nrests-on: []\n---\n# S\n\nText.\n";
        let docs = [
            ("docs/requirements/valves/closing.md", req("close")),
            ("docs/requirements/valves/drives/motor.md", req("spin")),
            ("docs/requirements/zones.md", req("zone")),
            ("docs/summaries/valves.md", pinned.to_string()),
            ("docs/summaries/valves/drives.md", pinned.to_string()),
            (
                "docs/system.md",
                "---\nrests-on: []\n---\n# Sys\n".to_string(),
            ),
        ];
        let v: Vec<(&str, &str)> = docs.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let c = corpus(&cfg, &v);
        // The requirement is in the frontier of `valves` only, not of
        // `valves/drives` (it is not below it), and not of `system` (the
        // summary `valves` stands for it).
        assert_eq!(
            due_ids(&c, &cfg, &["requirements/valves/closing"]),
            ["summaries/valves"]
        );
        // A requirement under the sub-summary is in the frontier of that one.
        assert_eq!(
            due_ids(&c, &cfg, &["requirements/valves/drives/motor"]),
            ["summaries/valves/drives"]
        );
        // A requirement that no summary stands for is in the frontier of
        // `system`: the flat case.
        assert_eq!(due_ids(&c, &cfg, &["requirements/zones"]), ["system"]);
        // Two lines: both documents are due, once each.
        assert_eq!(
            due_ids(
                &c,
                &cfg,
                &["requirements/zones", "requirements/valves/closing"]
            ),
            ["summaries/valves", "system"]
        );
        // An unknown id that is not under the folder of the layer, and a
        // document that is no requirements document, are in no frontier.
        assert!(due_ids(&c, &cfg, &["decisions/none", "summaries/valves"]).is_empty());
        assert!(due_ids(&c, &cfg, &[]).is_empty());
    }

    #[test]
    fn an_added_requirement_is_due_by_the_area_of_its_new_document() {
        let cfg = config();
        let pinned = "---\nrests-on: []\n---\n# S\n\nText.\n";
        let docs = [
            ("docs/requirements/valves/closing.md", req("close")),
            ("docs/summaries/valves.md", pinned.to_string()),
            (
                "docs/system.md",
                "---\nrests-on: []\n---\n# Sys\n".to_string(),
            ),
        ];
        let v: Vec<(&str, &str)> = docs.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let c = corpus(&cfg, &v);
        // A new document of the area `valves`, and one of a new area.
        assert_eq!(
            due_ids(&c, &cfg, &["requirements/valves/gaskets"]),
            ["summaries/valves"]
        );
        assert_eq!(due_ids(&c, &cfg, &["requirements/pumps/seal"]), ["system"]);
    }

    #[test]
    fn a_product_with_systems_holds_no_requirement_and_a_tree_with_no_rollup_has_no_due() {
        let cfg = config();
        let docs = [
            ("docs/requirements/zones.md", req("zone")),
            (
                "docs/system.md",
                "---\nrests-on: []\n---\n# Sys\n".to_string(),
            ),
            (
                "docs/product.md",
                "---\nrests-on: []\n---\n# Prod\n".to_string(),
            ),
        ];
        let v: Vec<(&str, &str)> = docs.iter().map(|(p, t)| (*p, t.as_str())).collect();
        let c = corpus(&cfg, &v);
        assert_eq!(due_ids(&c, &cfg, &["requirements/zones"]), ["system"]);
        let c = corpus(&cfg, &[("docs/requirements/zones.md", &req("zone"))]);
        assert!(due_ids(&c, &cfg, &["requirements/zones"]).is_empty());
    }
}

//! The layer graph (ADR 0005, docs/design/0.6.0.md "Layer graph").
//!
//! A layer is a named set of documents. The `moves-to` edges join the layers
//! into a graph without a cycle. The level of a layer comes from the edges.
//! This module parses `[layers.<name>]`, checks the graph and answers the
//! questions that the other modules ask: the level, "more stable", the context
//! order and the layer of a document. The parser of the configuration calls it.
//! The place rule reads it. The commands `layer` and `context` and the checks of
//! the graph read the rest in later slices.

use crate::config::{Fields, unknown_type};
use crate::config_text::{Splice, append_tables, line_ending, quote};
use crate::glob;
use crate::model::DocType;
use toml::Table;
use toml::de::DeTable;

/// How long a document of a layer stays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// A document stays. SD202 reports a deletion.
    Forever,
    /// A person can delete a finished document. SD202 stays silent.
    UntilFinished,
}

impl Keep {
    pub fn from_name(s: &str) -> Option<Keep> {
        match s {
            "forever" => Some(Keep::Forever),
            "until-finished" => Some(Keep::UntilFinished),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Keep::Forever => "forever",
            Keep::UntilFinished => "until-finished",
        }
    }
}

/// Whether `context` can serve the layer without a named target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextMode {
    Always,
    OnRequest,
}

impl ContextMode {
    pub fn from_name(s: &str) -> Option<ContextMode> {
        match s {
            "always" => Some(ContextMode::Always),
            "on-request" => Some(ContextMode::OnRequest),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            ContextMode::Always => "always",
            ContextMode::OnRequest => "on-request",
        }
    }
}

/// One layer, as the file gives it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Layer {
    pub name: String,
    pub types: Vec<DocType>,
    /// The key `dir`, if the file has it. Use [`Layer::dir`].
    pub dir_key: Option<String>,
    pub files: Vec<String>,
    pub paths: Vec<String>,
    pub moves_to: Vec<String>,
    pub linked_from: Vec<String>,
    pub keep: Keep,
    pub context: ContextMode,
    pub max_docs: Option<u32>,
    pub max_chars: Option<u32>,
    pub max_started: Option<u32>,
    pub stale_days: Option<u32>,
}

impl Layer {
    /// A layer with the defaults of every key.
    pub fn new(name: &str) -> Layer {
        Layer {
            name: name.to_string(),
            types: Vec::new(),
            dir_key: None,
            files: Vec::new(),
            paths: Vec::new(),
            moves_to: Vec::new(),
            linked_from: Vec::new(),
            keep: Keep::Forever,
            context: ContextMode::Always,
            max_docs: None,
            max_chars: None,
            max_started: None,
            stale_days: None,
        }
    }

    /// The folder of the layer, relative to the root: the key `dir`, or the
    /// name of the layer. A layer with `files` has no folder.
    pub fn dir(&self) -> Option<&str> {
        if !self.files.is_empty() {
            return None;
        }
        Some(self.dir_key.as_deref().unwrap_or(&self.name))
    }

    /// Reads the table `[layers.<name>]`. Keys that name another layer are
    /// not checked here: [`Graph::new`] checks them.
    fn parse(name: &str, tbl: &Table) -> Result<Layer, String> {
        let prefix = format!("layers.{name}.");
        let mut f = Fields::new(tbl, &prefix);
        let mut l = Layer::new(name);
        for ty in f.opt_strs("types")? {
            let t = DocType::from_name(&ty)
                .ok_or_else(|| unknown_type(&format!("`{prefix}types`"), &ty))?;
            if !l.types.contains(&t) {
                l.types.push(t);
            }
        }
        if let Some(dir) = f.opt_str("dir")? {
            l.dir_key = Some(check_dir(&dir).map_err(|e| format!("`{prefix}dir` {e}"))?);
        }
        l.files = f.opt_strs("files")?;
        l.paths = f.opt_strs("paths")?;
        for (key, list) in [("files", &l.files), ("paths", &l.paths)] {
            if list.iter().any(|s| s.trim().is_empty()) {
                return Err(format!("`{prefix}{key}` has an empty entry"));
            }
        }
        for file in &l.files {
            check_file(file).map_err(|e| format!("`{prefix}files` {e}"))?;
        }
        l.moves_to = dedup(f.opt_strs("moves-to")?);
        l.linked_from = dedup(f.opt_strs("linked-from")?);
        if let Some(k) = f.opt_str("keep")? {
            l.keep = Keep::from_name(&k).ok_or_else(|| {
                format!("`{prefix}keep` must be \"forever\" or \"until-finished\", not `{k}`")
            })?;
        }
        if let Some(c) = f.opt_str("context")? {
            l.context = ContextMode::from_name(&c).ok_or_else(|| {
                format!("`{prefix}context` must be \"always\" or \"on-request\", not `{c}`")
            })?;
        }
        l.max_docs = f.opt_u32("max-docs")?;
        l.max_chars = f.opt_u32("max-chars")?;
        l.max_started = f.opt_u32("max-started")?;
        l.stale_days = f.opt_u32("stale-days")?;
        f.done()?;
        Ok(l)
    }
}

/// Checks the value of `dir` and returns the folder without a closing `/`.
/// The error continues the sentence "`dir` ...".
pub fn check_dir(dir: &str) -> Result<String, String> {
    let d = dir.trim_matches('/');
    if d.is_empty()
        || dir.starts_with('/')
        || d.contains('\\')
        || d.split('/').any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err("must be a plain folder inside the root, such as `requirements`".to_string());
    }
    Ok(d.to_string())
}

/// Checks one entry of `files`: a path inside the root, such as `terms.md`.
/// The error continues the sentence "`files` ...".
pub fn check_file(file: &str) -> Result<(), String> {
    let f = file.trim_start_matches("./");
    if f.is_empty()
        || f.starts_with('/')
        || f.contains(['\\', ':'])
        || f.split('/').any(|s| s.is_empty() || s == "." || s == "..")
    {
        return Err(format!(
            "has the entry `{file}`: it must be a relative path inside the root, such as `terms.md`, with no `..`, no `\\` and no `:`"
        ));
    }
    Ok(())
}

fn dedup(v: Vec<String>) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for s in v {
        if !out.contains(&s) {
            out.push(s);
        }
    }
    out
}

/// The rule that put a document in its layer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    /// A glob of `paths` matches the path of the document.
    Paths,
    /// The document is a top file (`files`) of the layer of its type.
    Files,
    /// The layer has the type of the document.
    Type,
}

impl Rule {
    pub fn name(self) -> &'static str {
        match self {
            Rule::Paths => "paths",
            Rule::Files => "files",
            Rule::Type => "type",
        }
    }
}

/// The layers of a project in file order, with their levels. The graph has no
/// cycle, and each edge names a layer of the graph.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Graph {
    layers: Vec<Layer>,
    levels: Vec<u32>,
}

impl Graph {
    /// Checks the layers and makes the graph. The error names the key.
    pub fn new(layers: Vec<Layer>) -> Result<Graph, String> {
        for (i, l) in layers.iter().enumerate() {
            if layers[..i].iter().any(|o| o.name == l.name) {
                return Err(format!("`layers.{}` is defined twice", l.name));
            }
            if !l.files.is_empty() && l.dir_key.is_some() {
                return Err(format!(
                    "`layers.{}`: `files` and `dir` cannot be together; a layer with `files` has no `dir`",
                    l.name
                ));
            }
            for ty in &l.types {
                if let Some(o) = layers[..i].iter().find(|o| o.types.contains(ty)) {
                    return Err(format!(
                        "`layers.{}.types`: type `{}` is already in layer `{}`; a type is in one layer at most",
                        l.name,
                        ty.name(),
                        o.name
                    ));
                }
            }
            for file in &l.files {
                let f = file.trim_start_matches("./");
                if let Some(o) = layers[..i]
                    .iter()
                    .find(|o| o.files.iter().any(|x| x.trim_start_matches("./") == f))
                {
                    return Err(format!(
                        "`layers.{}.files`: the file `{file}` is already a top file of layer `{}`",
                        l.name, o.name
                    ));
                }
            }
            if let Some(d) = l.dir()
                && let Some(o) = layers[..i].iter().find(|o| o.dir() == Some(d))
            {
                return Err(format!(
                    "`layers.{}.dir`: the folder `{d}` is already the folder of layer `{}`",
                    l.name, o.name
                ));
            }
            for (key, edges) in [("moves-to", &l.moves_to), ("linked-from", &l.linked_from)] {
                if let Some(e) = edges.iter().find(|e| !layers.iter().any(|o| &o.name == *e)) {
                    return Err(format!(
                        "`layers.{}.{key}`: layer `{e}` is not in `[layers]`",
                        l.name
                    ));
                }
            }
        }
        let mut g = Graph {
            layers,
            levels: Vec::new(),
        };
        g.levels = g.find_levels()?;
        Ok(g)
    }

    /// The level of each layer. It fails on a cycle of `moves-to` edges.
    fn find_levels(&self) -> Result<Vec<u32>, String> {
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            New,
            Open,
            Done(u32),
        }
        fn visit(
            g: &Graph,
            i: usize,
            marks: &mut [Mark],
            path: &mut Vec<usize>,
        ) -> Result<u32, String> {
            match marks[i] {
                Mark::Done(n) => return Ok(n),
                Mark::Open => {
                    let from = path.iter().position(|&p| p == i).unwrap_or(0);
                    let mut names: Vec<&str> = path[from..]
                        .iter()
                        .map(|&p| g.layers[p].name.as_str())
                        .collect();
                    names.push(&g.layers[i].name);
                    return Err(format!(
                        "`layers.{}.moves-to`: the edges make a cycle: {}",
                        g.layers[*path.last().unwrap_or(&i)].name,
                        names.join(" -> ")
                    ));
                }
                Mark::New => {}
            }
            marks[i] = Mark::Open;
            path.push(i);
            let mut level = 0;
            for e in &g.layers[i].moves_to {
                // `new` has checked that the layer exists.
                let j = g.index(e).unwrap_or(i);
                level = level.max(visit(g, j, marks, path)? + 1);
            }
            path.pop();
            marks[i] = Mark::Done(level);
            Ok(level)
        }
        let mut marks = vec![Mark::New; self.layers.len()];
        let mut out = Vec::new();
        for i in 0..self.layers.len() {
            out.push(visit(self, i, &mut marks, &mut Vec::new())?);
        }
        Ok(out)
    }

    pub fn layers(&self) -> &[Layer] {
        &self.layers
    }

    /// The position of a layer in the file.
    pub fn index(&self, name: &str) -> Option<usize> {
        self.layers.iter().position(|l| l.name == name)
    }

    /// The level of the layer at position `i`: the number of edges on its
    /// longest `moves-to` path to a layer with no edge out. Level 0 is the
    /// most stable.
    pub fn level(&self, i: usize) -> u32 {
        self.levels[i]
    }

    /// True if layer `x` is more stable than layer `y`: `x` is reachable from
    /// `y` by `moves-to` edges.
    pub fn more_stable(&self, x: usize, y: usize) -> bool {
        let mut seen = vec![false; self.layers.len()];
        let mut todo = vec![y];
        while let Some(i) = todo.pop() {
            for e in &self.layers[i].moves_to {
                let Some(j) = self.index(e) else { continue };
                if j == x {
                    return true;
                }
                if !seen[j] {
                    seen[j] = true;
                    todo.push(j);
                }
            }
        }
        false
    }

    /// The positions of the layers in the order of `context`: by level from
    /// 0 up, and the file order inside one level.
    pub fn context_order(&self) -> Vec<usize> {
        let mut v: Vec<usize> = (0..self.layers.len()).collect();
        v.sort_by_key(|&i| (self.levels[i], i));
        v
    }

    /// The layer of a document: the first layer in the file whose `paths`
    /// match the path, else the layer that has the type of the document. The
    /// path is relative to the root.
    pub fn layer_of(&self, rel_path: &str, ty: DocType) -> Option<usize> {
        self.placement(rel_path, ty).map(|(i, _)| i)
    }

    /// The layer of a document, and the rule that put the document there: a
    /// glob of `paths`, else the type of the document. A document that is a
    /// top file of the layer of its type (`files`) has the rule `files`.
    pub fn placement(&self, rel_path: &str, ty: DocType) -> Option<(usize, Rule)> {
        if let Some(i) = self
            .layers
            .iter()
            .position(|l| l.paths.iter().any(|g| glob::matches(g, rel_path)))
        {
            return Some((i, Rule::Paths));
        }
        let i = self.layer_of_type(ty)?;
        let top = self.layers[i]
            .files
            .iter()
            .any(|f| f.trim_start_matches("./") == rel_path);
        Some((i, if top { Rule::Files } else { Rule::Type }))
    }

    /// The layer that has a type.
    pub fn layer_of_type(&self, ty: DocType) -> Option<usize> {
        self.layers.iter().position(|l| l.types.contains(&ty))
    }

    /// The type that the place of a document gives (design, "Place gives the
    /// type"), or `None`. `rel` is the path relative to the root. The order
    /// is: a top file of a layer, then the folder of a layer with one type,
    /// then the file name in the folder of a layer with more types. The front
    /// matter and `default-type` come before and after this rule: the caller
    /// orders them.
    pub fn place_type(&self, rel: &str) -> Option<DocType> {
        let stem = file_stem(rel);
        let named = |l: &Layer| DocType::from_name(stem).filter(|t| l.types.contains(t));
        if let Some(l) = self
            .layers
            .iter()
            .find(|l| l.files.iter().any(|f| f.trim_start_matches("./") == rel))
        {
            return named(l).or(match l.types.as_slice() {
                [t] => Some(*t),
                _ => None,
            });
        }
        // The deepest folder wins: a layer can have a folder inside the
        // folder of another layer.
        let l = self
            .layers
            .iter()
            .filter_map(|l| {
                let d = l.dir()?;
                rel.strip_prefix(d)
                    .is_some_and(|r| r.starts_with('/'))
                    .then_some((d.len(), l))
            })
            .max_by_key(|(n, _)| *n)?
            .1;
        match l.types.as_slice() {
            [] => None,
            [t] => Some(*t),
            _ => named(l),
        }
    }

    /// True if `rel` is a valid place for a type of one change: the file
    /// `<dir>/<slug>/<type>.md` in the folder of the layer that has the type.
    pub fn is_intent_place(&self, ty: DocType, rel: &str) -> bool {
        let Some(dir) = self.layer_of_type(ty).and_then(|i| self.layers[i].dir()) else {
            return false;
        };
        let Some(rest) = rel.strip_prefix(dir).and_then(|r| r.strip_prefix('/')) else {
            return false;
        };
        matches!(rest.split_once('/'), Some((slug, file))
            if !slug.is_empty() && file == format!("{}.md", ty.name()))
    }

    /// The graph of a project that has no `[layers]` table: the default
    /// graph. A non-empty `types` list keeps only the layers that hold one of
    /// these types, and drops the edges to the other layers.
    pub fn implied(types: &[DocType]) -> Graph {
        let keep = |d: &LayerDef| types.is_empty() || d.types.iter().any(|t| types.contains(t));
        let names: Vec<&str> = DEFAULT_GRAPH
            .iter()
            .filter(|d| keep(d))
            .map(|d| d.name)
            .collect();
        build(DEFAULT_GRAPH.iter().filter(|d| keep(d)), &names)
    }

    /// The graph that `init` writes for a project with fewer than 30
    /// documents.
    pub fn small() -> Graph {
        build(SMALL_GRAPH.iter(), &SMALL_NAMES)
    }

    /// The graph that `init` writes for a project with 30 documents or more.
    pub fn default_graph() -> Graph {
        let names: Vec<&str> = DEFAULT_GRAPH.iter().map(|d| d.name).collect();
        build(DEFAULT_GRAPH.iter(), &names)
    }

    /// The graph of a size name: `small` or `default`.
    pub fn of_size(size: &str) -> Option<Graph> {
        match size {
            "small" => Some(Graph::small()),
            "default" => Some(Graph::default_graph()),
            _ => None,
        }
    }
}

/// The file name of a path without its folder and its extension.
fn file_stem(rel: &str) -> &str {
    let file = rel.rsplit('/').next().unwrap_or(rel);
    file.rsplit_once('.').map_or(file, |(s, _)| s)
}

/// Builds a graph from constant definitions. An edge to a layer that is not in
/// `names` drops.
fn build<'a>(defs: impl Iterator<Item = &'a LayerDef>, names: &[&str]) -> Graph {
    let layers = defs
        .map(|d| {
            let edges = |e: &[&str]| {
                e.iter()
                    .filter(|n| names.contains(n))
                    .map(|n| n.to_string())
                    .collect()
            };
            let mut l = Layer::new(d.name);
            l.types = d.types.to_vec();
            l.files = d.files.iter().map(|s| s.to_string()).collect();
            l.moves_to = edges(d.moves_to);
            l.linked_from = edges(d.linked_from);
            l.keep = d.keep;
            l.context = d.context;
            l.max_docs = d.max_docs;
            l.max_chars = d.max_chars;
            l.max_started = d.max_started;
            l.stale_days = d.stale_days;
            l
        })
        .collect();
    Graph::new(layers).expect("the constant graphs are valid")
}

/// A layer of a constant graph.
struct LayerDef {
    name: &'static str,
    types: &'static [DocType],
    files: &'static [&'static str],
    moves_to: &'static [&'static str],
    linked_from: &'static [&'static str],
    keep: Keep,
    context: ContextMode,
    max_docs: Option<u32>,
    max_chars: Option<u32>,
    max_started: Option<u32>,
    stale_days: Option<u32>,
}

/// A definition with the defaults. The constants below override keys.
const BASE: LayerDef = LayerDef {
    name: "",
    types: &[],
    files: &[],
    moves_to: &[],
    linked_from: &[],
    keep: Keep::Forever,
    context: ContextMode::Always,
    max_docs: None,
    max_chars: None,
    max_started: None,
    stale_days: None,
};

const FOUNDATION: LayerDef = LayerDef {
    name: "foundation",
    types: &[DocType::Product, DocType::System, DocType::Terms],
    files: &["product.md", "system.md", "terms.md"],
    max_chars: Some(30_000),
    ..BASE
};

const RULES: LayerDef = LayerDef {
    name: "rules",
    types: &[DocType::Rule],
    max_docs: Some(30),
    ..BASE
};

const REQUIREMENTS: LayerDef = LayerDef {
    name: "requirements",
    types: &[DocType::Requirements],
    ..BASE
};

const INTERFACES: LayerDef = LayerDef {
    name: "interfaces",
    types: &[DocType::Interface],
    ..BASE
};

const DECISIONS: LayerDef = LayerDef {
    name: "decisions",
    types: &[DocType::Decision],
    ..BASE
};

const PROCEDURES: LayerDef = LayerDef {
    name: "procedures",
    types: &[DocType::Procedure],
    ..BASE
};

const RESEARCH: LayerDef = LayerDef {
    name: "research",
    types: &[DocType::Research],
    moves_to: &["decisions", "requirements"],
    linked_from: &["decisions", "requirements"],
    context: ContextMode::OnRequest,
    ..BASE
};

const INTENT: LayerDef = LayerDef {
    name: "intent",
    types: &[DocType::Intent, DocType::Spec, DocType::Plan],
    moves_to: &[
        "requirements",
        "interfaces",
        "rules",
        "decisions",
        "procedures",
    ],
    keep: Keep::UntilFinished,
    context: ContextMode::OnRequest,
    max_docs: Some(30),
    max_started: Some(3),
    stale_days: Some(14),
    ..BASE
};

const INCIDENTS: LayerDef = LayerDef {
    name: "incidents",
    types: &[DocType::Incident],
    moves_to: &["intent"],
    linked_from: &[
        "rules",
        "procedures",
        "decisions",
        "requirements",
        "interfaces",
        "intent",
    ],
    context: ContextMode::OnRequest,
    ..BASE
};

const PENDING: LayerDef = LayerDef {
    name: "pending",
    types: &[DocType::Roadmap, DocType::Questions],
    files: &["roadmap.md", "questions.md"],
    moves_to: &["intent", "decisions"],
    linked_from: &["intent", "requirements", "decisions"],
    context: ContextMode::OnRequest,
    max_docs: Some(2),
    ..BASE
};

/// The default graph (design "Default graph"), in the order of its table.
const DEFAULT_GRAPH: [LayerDef; 10] = [
    FOUNDATION,
    RULES,
    REQUIREMENTS,
    INTERFACES,
    DECISIONS,
    PROCEDURES,
    RESEARCH,
    INTENT,
    INCIDENTS,
    PENDING,
];

/// The small graph (design, table "Size"), in the order of its text. It
/// differs from the default graph in 3 keys: `pending` moves only to
/// `intent`, and the edges of `intent` and `incidents` name only the layers
/// of this graph.
const SMALL_GRAPH: [LayerDef; 7] = [
    FOUNDATION,
    LayerDef {
        moves_to: &["intent"],
        ..PENDING
    },
    RULES,
    REQUIREMENTS,
    DECISIONS,
    LayerDef {
        moves_to: &["rules", "requirements", "decisions"],
        ..INTENT
    },
    INCIDENTS,
];

const SMALL_NAMES: [&str; 7] = [
    "foundation",
    "pending",
    "rules",
    "requirements",
    "decisions",
    "intent",
    "incidents",
];

/// Reads the table `[layers]`. `text` is the source of the file: it gives the
/// order of the layers.
pub fn parse_table(tbl: &Table, text: &str) -> Result<Graph, String> {
    let mut layers = Vec::new();
    for name in file_order(text, tbl) {
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err(format!(
                "`layers.{name}`: a layer name has only letters, digits, `-` and `_`"
            ));
        }
        let t = tbl[&name]
            .as_table()
            .ok_or_else(|| format!("`layers.{name}` must be a table"))?;
        layers.push(Layer::parse(&name, t)?);
    }
    Graph::new(layers)
}

/// The names of the layers in the order of the file. The key of a layer is
/// its first key in the text: the header `[layers.x]`, the first dotted key
/// `layers.x.types`, or the name in an inline table. The TOML parser gives
/// the byte offset of each key. A name without an offset comes last.
fn file_order(text: &str, layers: &Table) -> Vec<String> {
    let mut spans = Vec::new();
    if let Ok(doc) = DeTable::parse(text)
        && let Some(v) = doc.get_ref().get("layers")
        && let toml::de::DeValue::Table(t) = v.get_ref()
    {
        for (k, _) in t.iter() {
            spans.push((k.span().start, k.get_ref().to_string()));
        }
    }
    spans.sort();
    let mut order: Vec<String> = spans
        .into_iter()
        .map(|(_, n)| n)
        .filter(|n| layers.contains_key(n))
        .collect();
    for name in layers.keys() {
        if !order.contains(name) {
            order.push(name.clone());
        }
    }
    order
}

fn quote_list<S: AsRef<str>>(items: &[S]) -> String {
    let v: Vec<String> = items.iter().map(|s| quote(s.as_ref())).collect();
    format!("[{}]", v.join(", "))
}

/// Writes the table `[layers.<name>]` of a layer. Only the keys that differ
/// from the default are in the text. The text ends with a line feed.
pub fn render_table(l: &Layer) -> String {
    let mut out = format!("[layers.{}]\n", l.name);
    if !l.types.is_empty() {
        let names: Vec<&str> = l.types.iter().map(|t| t.name()).collect();
        out.push_str(&format!("types = {}\n", quote_list(&names)));
    }
    if let Some(d) = &l.dir_key {
        out.push_str(&format!("dir = {}\n", quote(d)));
    }
    for (key, list) in [
        ("files", &l.files),
        ("paths", &l.paths),
        ("moves-to", &l.moves_to),
        ("linked-from", &l.linked_from),
    ] {
        if !list.is_empty() {
            out.push_str(&format!("{key} = {}\n", quote_list(list)));
        }
    }
    if l.keep != Keep::Forever {
        out.push_str(&format!("keep = {}\n", quote(l.keep.name())));
    }
    if l.context != ContextMode::Always {
        out.push_str(&format!("context = {}\n", quote(l.context.name())));
    }
    for (key, n) in [
        ("max-docs", l.max_docs),
        ("max-chars", l.max_chars),
        ("max-started", l.max_started),
        ("stale-days", l.stale_days),
    ] {
        if let Some(n) = n {
            out.push_str(&format!("{key} = {n}\n"));
        }
    }
    out
}

/// Writes each layer of a graph as a table, in the order of the graph, with
/// one empty line between the tables.
pub fn render_graph(g: &Graph) -> String {
    let tables: Vec<String> = g.layers().iter().map(render_table).collect();
    tables.join("\n")
}

/// The insertion that adds the layer `target` to the `moves-to` list of the
/// layer `name`. It adds text only: each comment and each other byte of the
/// file stay. The TOML parser gives the byte range of each value.
///
/// - The list exists (in each form): the insertion follows its last entry,
///   or follows its `[` if the list is empty.
/// - The list does not exist and the layer is a table with a header: the
///   insertion is a new line after the header.
/// - Another form, with no list: the error asks for a hand edit.
fn edge_splice(text: &str, name: &str, target: &str, eol: &str) -> Result<Splice, String> {
    let by_hand = || {
        format!(
            "`layers.{name}` has no `moves-to` key and is not written as a table with a header: add `moves-to = [\"{target}\"]` to it by hand"
        )
    };
    let doc = DeTable::parse(text).map_err(|e| format!("spec-dna.toml does not parse: {e}"))?;
    let Some(toml::de::DeValue::Table(layers)) = doc.get_ref().get("layers").map(|v| v.get_ref())
    else {
        return Err(by_hand());
    };
    let Some((key, value)) = layers.iter().find(|(k, _)| k.get_ref().as_ref() == name) else {
        return Err(by_hand());
    };
    let toml::de::DeValue::Table(layer) = value.get_ref() else {
        return Err(by_hand());
    };
    if let Some(list) = layer.get("moves-to") {
        let toml::de::DeValue::Array(items) = list.get_ref() else {
            return Err(by_hand());
        };
        return Ok(match items.last() {
            Some(last) => Splice {
                at: last.span().end,
                text: format!(", {}", quote(target)),
            },
            None => Splice {
                at: list.span().start + 1,
                text: quote(target),
            },
        });
    }
    // A header line starts with `[`, and no other line of key spans does.
    let start = key.span().start;
    let line_start = text[..start].rfind('\n').map_or(0, |i| i + 1);
    let line = text[line_start..].trim_start();
    if !line.starts_with('[') || line.starts_with("[[") {
        return Err(by_hand());
    }
    let (at, lead) = match text[start..].find('\n') {
        Some(i) => (start + i + 1, ""),
        None => (text.len(), eol),
    };
    Ok(Splice {
        at,
        text: format!("{lead}moves-to = [{}]{eol}", quote(target)),
    })
}

/// Adds a layer to a configuration text and returns the new text. `graph` is
/// the graph that `config` read from `text`, and `declared` is true if the
/// text has a `[layers]` table. A text with no such table first gets the
/// tables of the implied graph. The new table comes last. For each name in
/// `moved_from`, the new layer joins the `moves-to` list of that layer.
///
/// Each error is a refusal: the caller writes nothing. The function reads the
/// new text again, and the graph that it gives must be the graph that this function
/// expects.
pub fn add_layer(
    text: &str,
    graph: &Graph,
    declared: bool,
    new: Layer,
    moved_from: &[String],
) -> Result<String, String> {
    if graph.index(&new.name).is_some() {
        return Err(format!(
            "layer `{}` exists; choose another name, or edit `[layers.{}]` by hand",
            new.name, new.name
        ));
    }
    let mut layers: Vec<Layer> = graph.layers().to_vec();
    for from in moved_from {
        let Some(l) = layers.iter_mut().find(|l| &l.name == from) else {
            return Err(format!(
                "`--moved-from`: layer `{from}` is not in `[layers]`"
            ));
        };
        l.moves_to.push(new.name.clone());
    }
    layers.push(new.clone());
    let expected = Graph::new(layers.clone())?;
    // Each line that this function writes ends as the lines of the file.
    let eol = line_ending(text);
    let mut out;
    if declared {
        let mut splices = Vec::new();
        for from in moved_from {
            // A layer that has the new layer already cannot happen: the name is new.
            splices.push(edge_splice(text, from, &new.name, eol)?);
        }
        // From the end of the text to its start: an offset stays valid.
        splices.sort_by_key(|s| std::cmp::Reverse(s.at));
        out = text.to_string();
        for s in splices {
            out.insert_str(s.at, &s.text);
        }
        append_tables(&mut out, &render_table(&new), eol);
    } else {
        out = text.to_string();
        append_tables(&mut out, &render_graph(&expected), eol);
    }
    let read = crate::config::Config::parse(&out).map_err(|e| {
        format!("the new spec-dna.toml would not be valid ({e}); add the layer by hand")
    })?;
    if read.layers != expected {
        return Err(
            "the new spec-dna.toml would not give the expected layers; add the layer by hand"
                .to_string(),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order_of(text: &str) -> Vec<String> {
        let table: Table = text.parse().unwrap();
        let layers = table["layers"].as_table().unwrap();
        file_order(text, layers)
    }

    // The 4 ways to write a layer. The names are not in lexicographic order,
    // so a result in the file order shows that the offsets decide.
    #[test]
    fn file_order_of_the_four_forms() {
        let header = "[layers.zeta]\ntypes = [\"rule\"]\n[layers.alpha]\ntypes = [\"decision\"]\n[layers.mid]\n";
        assert_eq!(order_of(header), ["zeta", "alpha", "mid"]);

        let dotted = "layers.zeta.types = [\"rule\"]\nlayers.alpha.types = [\"decision\"]\nlayers.mid.keep = \"forever\"\n";
        assert_eq!(order_of(dotted), ["zeta", "alpha", "mid"]);

        let inline = "layers = { zeta = { types = [\"rule\"] }, alpha = { types = [\"decision\"] }, mid = {} }\n";
        assert_eq!(order_of(inline), ["zeta", "alpha", "mid"]);

        let inline_in_header = "[layers]\nzeta = { types = [\"rule\"] }\nalpha = { types = [\"decision\"] }\nmid = {}\n";
        assert_eq!(order_of(inline_in_header), ["zeta", "alpha", "mid"]);
    }

    #[test]
    fn file_order_of_quoted_names_and_mixed_forms() {
        let quoted = "[layers.\"zeta\"]\n[layers.'alpha']\n[layers.\"mid-1\"]\n";
        assert_eq!(order_of(quoted), ["zeta", "alpha", "mid-1"]);

        // A header, then a dotted key, then an inline table in one file.
        let mixed = "[layers.zeta]\nkeep = \"forever\"\n\n[other]\nx = 1\n";
        assert_eq!(order_of(mixed), ["zeta"]);
        let mixed = "layers.zeta.keep = \"forever\"\n[layers.alpha]\nkeep = \"forever\"\n";
        assert_eq!(order_of(mixed), ["zeta", "alpha"]);

        // A layer that dotted keys define in 2 places keeps its first place.
        let split = "layers.zeta.types = [\"rule\"]\nlayers.alpha.types = [\"decision\"]\nlayers.zeta.keep = \"forever\"\n";
        assert_eq!(order_of(split), ["zeta", "alpha"]);

        // A multi-line string that looks like a layer header is no key.
        let tricky = "[layers.zeta]\nkeep = \"forever\"\n[layers.alpha]\nmax-docs = 1\npaths = [\"\"\"\n[layers.mid]\n\"\"\"]\n[layers.mid]\n";
        assert_eq!(order_of(tricky), ["zeta", "alpha", "mid"]);
    }

    #[test]
    fn file_order_with_a_sub_table_first() {
        // `[layers.b.x]` before `[layers.a]` puts `b` first. The sub-table
        // is an unknown key later, so this order never reaches a command.
        let t = "[layers.b.x]\n[layers.a]\n";
        assert_eq!(order_of(t), ["b", "a"]);
    }

    fn graph(text: &str) -> Result<Graph, String> {
        let table: Table = text.parse().unwrap();
        parse_table(table["layers"].as_table().unwrap(), text)
    }

    fn level_of(g: &Graph, name: &str) -> u32 {
        g.level(g.index(name).unwrap())
    }

    #[test]
    fn levels_follow_the_longest_path() {
        let g = graph(
            "[layers.a]\n[layers.b]\nmoves-to = [\"a\"]\n[layers.c]\nmoves-to = [\"b\", \"a\"]\n[layers.d]\nmoves-to = [\"a\"]\n[layers.lone]\n",
        )
        .unwrap();
        assert_eq!(level_of(&g, "a"), 0);
        assert_eq!(level_of(&g, "b"), 1);
        assert_eq!(level_of(&g, "c"), 2);
        assert_eq!(level_of(&g, "d"), 1);
        assert_eq!(level_of(&g, "lone"), 0);
    }

    #[test]
    fn more_stable_is_reachability_by_moves_to() {
        let g = graph(
            "[layers.a]\n[layers.b]\nmoves-to = [\"a\"]\n[layers.c]\nmoves-to = [\"b\"]\n[layers.d]\nmoves-to = [\"a\"]\n",
        )
        .unwrap();
        let i = |n: &str| g.index(n).unwrap();
        assert!(g.more_stable(i("a"), i("c")));
        assert!(g.more_stable(i("b"), i("c")));
        assert!(!g.more_stable(i("c"), i("a")));
        // No path between b and d: neither is more stable.
        assert!(!g.more_stable(i("b"), i("d")));
        assert!(!g.more_stable(i("d"), i("b")));
        assert!(!g.more_stable(i("a"), i("a")));
    }

    #[test]
    fn a_cycle_is_an_error_that_names_the_key() {
        let e = graph("[layers.a]\nmoves-to = [\"b\"]\n[layers.b]\nmoves-to = [\"c\"]\n[layers.c]\nmoves-to = [\"a\"]\n")
            .unwrap_err();
        assert!(e.contains("moves-to"), "{e}");
        assert!(e.contains("cycle"), "{e}");
        assert!(e.contains("a -> b -> c -> a"), "{e}");
        let e = graph("[layers.a]\nmoves-to = [\"a\"]\n").unwrap_err();
        assert!(e.contains("cycle") && e.contains("a -> a"), "{e}");
    }

    #[test]
    fn linked_from_adds_no_level_and_no_cycle() {
        // `linked-from` is not an edge of the graph.
        let g = graph(
            "[layers.a]\nlinked-from = [\"b\"]\n[layers.b]\nmoves-to = [\"a\"]\nlinked-from = [\"a\"]\n",
        )
        .unwrap();
        assert_eq!(level_of(&g, "a"), 0);
        assert_eq!(level_of(&g, "b"), 1);
        let e = graph("[layers.a]\nlinked-from = [\"nope\"]\n").unwrap_err();
        assert!(
            e.contains("layers.a.linked-from") && e.contains("`nope`"),
            "{e}"
        );
    }

    #[test]
    fn context_order_is_level_then_file_order() {
        let g = graph(
            "[layers.top]\nmoves-to = [\"mid\"]\n[layers.mid]\nmoves-to = [\"base\", \"other\"]\n[layers.base]\n[layers.other]\n[layers.top2]\nmoves-to = [\"mid\"]\n",
        )
        .unwrap();
        let names: Vec<&str> = g
            .context_order()
            .into_iter()
            .map(|i| g.layers()[i].name.as_str())
            .collect();
        assert_eq!(names, ["base", "other", "mid", "top", "top2"]);
    }

    #[test]
    fn context_order_of_the_default_graph_puts_foundation_before_rules() {
        let g = Graph::default_graph();
        let names: Vec<&str> = g
            .context_order()
            .into_iter()
            .map(|i| g.layers()[i].name.as_str())
            .collect();
        assert_eq!(
            names,
            [
                "foundation",
                "rules",
                "requirements",
                "interfaces",
                "decisions",
                "procedures",
                "research",
                "intent",
                "incidents",
                "pending"
            ]
        );
    }

    #[test]
    fn context_order_keeps_the_file_order_of_a_custom_layer() {
        // `custom` has level 0 and comes last in the file: after the layers of
        // level 0 that come before it, and before each layer of level 1.
        let g = graph(
            "[layers.a]\nmoves-to = [\"b\"]\n[layers.b]\n[layers.custom]\npaths = [\"x/**\"]\n[layers.c]\nmoves-to = [\"custom\"]\n",
        )
        .unwrap();
        let names: Vec<&str> = g
            .context_order()
            .into_iter()
            .map(|i| g.layers()[i].name.as_str())
            .collect();
        assert_eq!(names, ["b", "custom", "a", "c"]);
    }

    #[test]
    fn the_layer_of_a_document_is_the_first_path_match_else_its_type() {
        let g = graph(
            "[layers.first]\npaths = [\"pilot/**\"]\n[layers.second]\npaths = [\"pilot/x/**\", \"**/*.draft.md\"]\n[layers.rules]\ntypes = [\"rule\"]\n",
        )
        .unwrap();
        let name = |p: &str, t| g.layer_of(p, t).map(|i| g.layers()[i].name.as_str());
        assert_eq!(name("pilot/x/a.md", DocType::Rule), Some("first"));
        assert_eq!(name("a.draft.md", DocType::Rule), Some("second"));
        assert_eq!(name("a.md", DocType::Rule), Some("rules"));
        assert_eq!(name("a.md", DocType::Plan), None);
    }

    #[test]
    fn the_constant_graphs_match_the_design() {
        let g = Graph::default_graph();
        let levels: Vec<(&str, u32)> = g
            .layers()
            .iter()
            .enumerate()
            .map(|(i, l)| (l.name.as_str(), g.level(i)))
            .collect();
        assert_eq!(
            levels,
            [
                ("foundation", 0),
                ("rules", 0),
                ("requirements", 0),
                ("interfaces", 0),
                ("decisions", 0),
                ("procedures", 0),
                ("research", 1),
                ("intent", 1),
                ("incidents", 2),
                ("pending", 2),
            ]
        );
        let i = |n: &str| g.index(n).unwrap();
        // 15 types, each in one layer.
        for ty in DocType::ALL {
            assert!(g.layer_of_type(ty).is_some(), "{}", ty.name());
        }
        assert!(g.more_stable(i("requirements"), i("intent")));
        assert!(g.more_stable(i("rules"), i("incidents")));
        assert!(!g.more_stable(i("research"), i("intent")));
        let intent = &g.layers()[i("intent")];
        assert_eq!(intent.keep, Keep::UntilFinished);
        assert_eq!(intent.max_started, Some(3));
        assert_eq!(g.layers()[i("foundation")].max_chars, Some(30_000));
        assert_eq!(g.layers()[i("foundation")].dir(), None);
        assert_eq!(g.layers()[i("rules")].dir(), Some("rules"));

        let s = Graph::small();
        let names: Vec<&str> = s.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "foundation",
                "pending",
                "rules",
                "requirements",
                "decisions",
                "intent",
                "incidents"
            ]
        );
        let i = |n: &str| s.index(n).unwrap();
        assert_eq!(s.layers()[i("pending")].moves_to, ["intent"]);
        assert_eq!(
            s.layers()[i("intent")].moves_to,
            ["rules", "requirements", "decisions"]
        );
        assert_eq!(
            s.layers()[i("incidents")].linked_from,
            ["rules", "decisions", "requirements", "intent"]
        );
        assert_eq!(s.level(i("pending")), 2);
        assert_eq!(s.level(i("intent")), 1);
        assert!(s.layer_of_type(DocType::Research).is_none());
        assert_eq!(Graph::of_size("small"), Some(s));
        assert_eq!(Graph::of_size("default"), Some(g));
        assert_eq!(Graph::of_size("big"), None);
    }

    #[test]
    fn the_implied_graph_keeps_the_layers_of_the_types_in_use() {
        let all = Graph::implied(&[]);
        assert_eq!(all, Graph::default_graph());
        let g = Graph::implied(&[DocType::Requirements, DocType::Decision, DocType::Research]);
        let names: Vec<&str> = g.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["requirements", "decisions", "research"]);
        let r = &g.layers()[g.index("research").unwrap()];
        assert_eq!(r.moves_to, ["decisions", "requirements"]);
        let g = Graph::implied(&[DocType::Spec, DocType::Rule]);
        let names: Vec<&str> = g.layers().iter().map(|l| l.name.as_str()).collect();
        assert_eq!(names, ["rules", "intent"]);
        // The edges to the absent layers drop.
        assert_eq!(g.layers()[1].moves_to, ["rules"]);
        assert_eq!(g.level(1), 1);
    }

    #[test]
    fn each_error_names_the_key() {
        let cases: [(&str, &[&str]); 9] = [
            (
                "[layers.a]\ntypes = [\"rule\"]\n[layers.b]\ntypes = [\"rule\"]\n",
                &["layers.b.types", "`rule`", "`a`"],
            ),
            (
                "[layers.a]\nmoves-to = [\"x\"]\n",
                &["layers.a.moves-to", "`x`"],
            ),
            (
                "[layers.a]\nfiles = [\"terms.md\"]\ndir = \"d\"\n",
                &["layers.a", "files", "dir"],
            ),
            (
                "[layers.a]\ndir = \"d\"\n[layers.b]\ndir = \"d\"\n",
                &["layers.b.dir", "`a`"],
            ),
            ("[layers.a]\nfoo = 1\n", &["unknown key", "layers.a.foo"]),
            (
                "[layers.a]\nkeep = \"never\"\n",
                &["layers.a.keep", "`never`"],
            ),
            (
                "[layers.a]\ncontext = \"sometimes\"\n",
                &["layers.a.context"],
            ),
            (
                "[layers.a]\ntypes = [\"change\"]\n",
                &["layers.a.types", "`intent`"],
            ),
            ("[layers.a]\nmax-docs = -1\n", &["layers.a.max-docs"]),
        ];
        for (text, needles) in cases {
            let e = graph(text).unwrap_err();
            for n in needles {
                assert!(e.contains(n), "{text}: {e}");
            }
        }
    }

    #[test]
    fn a_dir_that_leaves_the_root_is_an_error() {
        for d in ["..", "../x", "/abs", "a//b", ".", "", "a\\\\b"] {
            let e = graph(&format!("[layers.a]\ndir = \"{d}\"\n")).unwrap_err();
            assert!(e.contains("layers.a.dir"), "{d}: {e}");
        }
        let g = graph("[layers.a]\ndir = \"x/y/\"\n").unwrap();
        assert_eq!(g.layers()[0].dir(), Some("x/y"));
    }

    /// Applies the insertion of `edge_splice` to a text.
    fn spliced(text: &str, name: &str, target: &str) -> Result<String, String> {
        let s = edge_splice(text, name, target, "\n")?;
        let mut out = text.to_string();
        out.insert_str(s.at, &s.text);
        Ok(out)
    }

    // The byte range of `moves-to` in the 4 forms, with comments and a
    // trailing comma. Each other byte of the text stays.
    #[test]
    fn edge_splice_in_the_four_forms() {
        let header = "[layers.a]\nmoves-to = [\"b\"] # edge\n[layers.b]\n";
        assert_eq!(
            spliced(header, "a", "n").unwrap(),
            "[layers.a]\nmoves-to = [\"b\", \"n\"] # edge\n[layers.b]\n"
        );
        let dotted = "layers.a.moves-to = [\"b\"]\nlayers.b.keep = \"forever\"\n";
        assert_eq!(
            spliced(dotted, "a", "n").unwrap(),
            "layers.a.moves-to = [\"b\", \"n\"]\nlayers.b.keep = \"forever\"\n"
        );
        let inline = "layers = { a = { moves-to = [\"b\"] }, b = {} }\n";
        assert_eq!(
            spliced(inline, "a", "n").unwrap(),
            "layers = { a = { moves-to = [\"b\", \"n\"] }, b = {} }\n"
        );
        let in_header = "[layers]\na = { moves-to = [\"b\"] }\nb = {}\n";
        assert_eq!(
            spliced(in_header, "a", "n").unwrap(),
            "[layers]\na = { moves-to = [\"b\", \"n\"] }\nb = {}\n"
        );
        // A multi-line list, a comment inside it and a trailing comma.
        let multi = "[layers.a]\nmoves-to = [\n  \"b\", # first\n  \"c\", # second\n]\n[layers.b]\n[layers.c]\n";
        assert_eq!(
            spliced(multi, "a", "n").unwrap(),
            "[layers.a]\nmoves-to = [\n  \"b\", # first\n  \"c\", \"n\", # second\n]\n[layers.b]\n[layers.c]\n"
        );
        // An empty list, with and without a comment inside.
        assert_eq!(
            spliced("[layers.a]\nmoves-to = []\n", "a", "n").unwrap(),
            "[layers.a]\nmoves-to = [\"n\"]\n"
        );
        assert_eq!(
            spliced("[layers.a]\nmoves-to = [ # none\n]\n", "a", "n").unwrap(),
            "[layers.a]\nmoves-to = [\"n\" # none\n]\n"
        );
    }

    #[test]
    fn edge_splice_without_a_list() {
        // A table with a header gets a new line after the header.
        assert_eq!(
            spliced("[layers.a] # c\ntypes = [\"rule\"]\n", "a", "n").unwrap(),
            "[layers.a] # c\nmoves-to = [\"n\"]\ntypes = [\"rule\"]\n"
        );
        assert_eq!(
            spliced("[layers.a]", "a", "n").unwrap(),
            "[layers.a]\nmoves-to = [\"n\"]\n"
        );
        // A quoted name, and a multi-line string that looks like a header.
        assert_eq!(
            spliced(
                "[layers.\"a\"]\npaths = [\"\"\"\n[layers.b]\n\"\"\"]\n",
                "a",
                "n"
            )
            .unwrap(),
            "[layers.\"a\"]\nmoves-to = [\"n\"]\npaths = [\"\"\"\n[layers.b]\n\"\"\"]\n"
        );
        // The dotted and the inline form give no place: the caller asks for a
        // hand edit.
        for text in [
            "layers.a.keep = \"forever\"\n",
            "layers = { a = { keep = \"forever\" } }\n",
            "[layers]\na = { keep = \"forever\" }\n",
        ] {
            let e = spliced(text, "a", "n").unwrap_err();
            assert!(e.contains("by hand"), "{text}: {e}");
        }
    }

    #[test]
    fn written_tables_read_back_as_the_same_graph() {
        for g in [Graph::small(), Graph::default_graph()] {
            let text = render_graph(&g);
            assert_eq!(graph(&text).unwrap(), g, "{text}");
        }
        // The writer escapes a name with a quote or a backslash.
        let mut l = Layer::new("a");
        l.paths = vec!["x\"y\\z/**".to_string()];
        let g = Graph::new(vec![l]).unwrap();
        assert_eq!(graph(&render_graph(&g)).unwrap(), g);
    }

    fn config_text(extra: &str) -> String {
        format!(
            "spec-dna = \"{}\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n\n{extra}",
            crate::adapters::VERSION
        )
    }

    #[test]
    fn add_layer_edits_only_what_it_adds() {
        use crate::config::Config;
        let text = config_text(
            "# the layers\n[layers.a]\ntypes = [\"rule\"]\n# edge below\nmoves-to = [\"b\"] # keep me\n\n[layers.b]\ntypes = [\"decision\"]\n",
        );
        let cfg = Config::parse(&text).unwrap();
        let mut n = Layer::new("n");
        n.types = vec![DocType::Interface];
        let out = add_layer(
            &text,
            &cfg.layers,
            true,
            n,
            &["a".to_string(), "b".to_string()],
        )
        .unwrap();
        assert!(
            out.starts_with(
                &text
                    .replace("[\"b\"] # keep me", "[\"b\", \"n\"] # keep me")
                    .replace(
                        "types = [\"decision\"]\n",
                        "moves-to = [\"n\"]\ntypes = [\"decision\"]\n"
                    )
            ),
            "{out}"
        );
        assert!(
            out.ends_with("\n\n[layers.n]\ntypes = [\"interface\"]\n"),
            "{out}"
        );
        let read = Config::parse(&out).unwrap();
        assert_eq!(read.layers.layers()[2].name, "n");
        assert_eq!(read.layers.level(0), 2);
    }

    #[test]
    fn add_layer_refuses_and_returns_an_error() {
        use crate::config::Config;
        let text = config_text(
            "[layers.a]\ntypes = [\"rule\"]\n[layers.b]\ntypes = [\"decision\"]\nmoves-to = [\"a\"]\n",
        );
        let cfg = Config::parse(&text).unwrap();
        let add = |n: Layer, from: &[&str]| {
            let from: Vec<String> = from.iter().map(|s| s.to_string()).collect();
            add_layer(&text, &cfg.layers, true, n, &from)
        };
        let named = |name: &str| Layer::new(name);
        // An existing name.
        assert!(add(named("a"), &[]).unwrap_err().contains("exists"));
        // A type that another layer has.
        let mut l = named("n");
        l.types = vec![DocType::Rule];
        assert!(add(l, &[]).unwrap_err().contains("already in layer `a`"));
        // A cycle: `a` is below `b`, so `n` moves from `a` and to `b`... and
        // `b` moves to `a`.
        let mut l = named("n");
        l.moves_to = vec!["b".to_string()];
        assert!(add(l, &["a"]).unwrap_err().contains("cycle"));
        // An unknown layer in an edge.
        let mut l = named("n");
        l.moves_to = vec!["zzz".to_string()];
        assert!(add(l, &[]).unwrap_err().contains("`zzz`"));
        assert!(add(named("n"), &["zzz"]).unwrap_err().contains("`zzz`"));
    }
}

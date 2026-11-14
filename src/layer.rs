//! The layer graph (ADR 0005, docs/design/0.6.0.md "Layer graph").
//!
//! A layer is a named set of documents. The `moves-to` edges join the layers
//! into a graph without a cycle. The level of a layer comes from the edges.
//! This module parses `[layers.<name>]`, checks the graph and answers the
//! questions that the other modules ask: the level, "more stable", the context
//! order and the layer of a document. The parser of the configuration calls it.
//! The commands read it from later steps on.

// The place rule, `layer` and `context` read most of these items in the next
// slices. Remove this attribute when they do.
#![allow(dead_code)]

use crate::config::{Fields, unknown_type};
use crate::glob;
use crate::model::DocType;
use toml::Table;
use toml::de::DeTable;

/// How long a document of a layer stays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Keep {
    /// A document stays. SD202 reports a deletion.
    Forever,
    /// A finished document is deleted. SD202 stays silent.
    UntilFinished,
}

impl Keep {
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
            let d = dir.trim_matches('/');
            if d.is_empty()
                || dir.starts_with('/')
                || d.contains('\\')
                || d.split('/').any(|s| s.is_empty() || s == "." || s == "..")
            {
                return Err(format!(
                    "`{prefix}dir` must be a plain folder inside the root, such as `requirements`"
                ));
            }
            l.dir_key = Some(d.to_string());
        }
        l.files = f.opt_strs("files")?;
        l.paths = f.opt_strs("paths")?;
        for (key, list) in [("files", &l.files), ("paths", &l.paths)] {
            if list.iter().any(|s| s.trim().is_empty()) {
                return Err(format!("`{prefix}{key}` has an empty entry"));
            }
        }
        l.moves_to = dedup(f.opt_strs("moves-to")?);
        l.linked_from = dedup(f.opt_strs("linked-from")?);
        if let Some(k) = f.opt_str("keep")? {
            l.keep = match k.as_str() {
                "forever" => Keep::Forever,
                "until-finished" => Keep::UntilFinished,
                _ => {
                    return Err(format!(
                        "`{prefix}keep` must be \"forever\" or \"until-finished\", not `{k}`"
                    ));
                }
            };
        }
        if let Some(c) = f.opt_str("context")? {
            l.context = match c.as_str() {
                "always" => ContextMode::Always,
                "on-request" => ContextMode::OnRequest,
                _ => {
                    return Err(format!(
                        "`{prefix}context` must be \"always\" or \"on-request\", not `{c}`"
                    ));
                }
            };
        }
        l.max_docs = f.opt_u32("max-docs")?;
        l.max_chars = f.opt_u32("max-chars")?;
        l.max_started = f.opt_u32("max-started")?;
        l.stale_days = f.opt_u32("stale-days")?;
        f.done()?;
        Ok(l)
    }
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

    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
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
        self.layers
            .iter()
            .position(|l| l.paths.iter().any(|g| glob::matches(g, rel_path)))
            .or_else(|| self.layer_of_type(ty))
    }

    /// The layer that has a type.
    pub fn layer_of_type(&self, ty: DocType) -> Option<usize> {
        self.layers.iter().position(|l| l.types.contains(&ty))
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
    linked_from: &["intent"],
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
}

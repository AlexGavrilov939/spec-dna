//! The command `spec-dna import` (design 0.7.0, D21 to D27; ADR 0014). The
//! module has 2 parts, as `upgrade` has.
//!
//! - The plan (`plan`) is a pure function over a tree. It reads the tree and
//!   writes nothing, so the proof can run it again on the planned tree
//!   (step 5). It calls the reader of the tool, the plan of
//!   `spec-dna.toml` (`import_config`) and the plan of the documents
//!   (`import_docs`).
//! - The command (`run`) does what the plan must not do: it reads the disk and
//!   git, it checks the tree, it proves the plan, it writes, and it prints.
//!
//! If the plan changes `spec-dna.toml`, the first run writes only that file
//! (D21). The proof covers the end state: the new configuration and the
//! documents.

use crate::adapters::VERSION;
use crate::config::{self, Config, Profile};
use crate::finish;
use crate::import_config::{self, Needs};
use crate::import_docs::{
    self, CONFIG_PATH, ConfigChange, Ctx, DocsInput, Places, Plan, Read, State, corpus_of,
    link_map, plan_docs, similarity,
};
use crate::import_openspec;
use crate::import_proof::{self, Proof};
use crate::import_speckit;
use crate::json::{self, Obj};
use crate::model::DocType;
use crate::mv;
use crate::repo::{Fatal, Repo};
use crate::upgrade_docs::{Tree, TreeData};

/// The tool that the project comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Speckit,
    Openspec,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::Speckit => "speckit",
            Source::Openspec => "openspec",
        }
    }
}

/// What the command line gives to `run`.
#[derive(Clone, Debug)]
pub struct RunOpts {
    pub source: Source,
    pub dry_run: bool,
    pub skip_open: bool,
    /// `--areas`: a path from the current folder.
    pub areas: Option<String>,
    pub keep_research: bool,
    pub json: bool,
}

/// What the plan reads besides the tree.
#[derive(Clone, Debug)]
pub struct PlanOpts {
    pub source: Source,
    pub skip_open: bool,
    pub keep_research: bool,
    pub areas: Vec<(String, String)>,
}

/// The plan, and the final configuration.
pub struct Planned {
    pub plan: Plan,
    pub fin: Config,
}

fn tree_text(tree: &Tree, path: &str) -> Result<Option<String>, Fatal> {
    if !tree.files.iter().any(|f| f == path) {
        return Ok(None);
    }
    let got = (tree.read)(&[path.to_string()])?;
    Ok(got
        .get(path)
        .map(|b| String::from_utf8_lossy(b).into_owned()))
}

fn read_source(ctx: &Ctx, source: Source) -> Result<Read, Fatal> {
    match source {
        Source::Speckit => import_speckit::read(ctx),
        Source::Openspec => import_openspec::read(ctx),
    }
}

/// The types that the targets of a read need.
fn types_of(places: &Places, read: &Read) -> Vec<DocType> {
    let mut out: Vec<DocType> = Vec::new();
    let mut add = |t: DocType| {
        if !out.contains(&t) {
            out.push(t);
        }
    };
    for t in &read.targets {
        let under = |d: &Option<String>| {
            d.as_deref()
                .is_some_and(|d| t.path.starts_with(&format!("{d}/")))
        };
        if under(&places.rules) {
            add(DocType::Rule);
        } else if under(&places.requirements) {
            add(DocType::Requirements);
        } else if under(&places.intent) {
            add(DocType::Intent);
            add(DocType::Spec);
            add(DocType::Plan);
        } else if under(&places.research) {
            add(DocType::Research);
        }
    }
    out
}

/// The plan of one run on a tree (D21 to D26). The tree has the file
/// `spec-dna.toml`.
pub fn plan(tree: &Tree, o: &PlanOpts) -> Result<Planned, Fatal> {
    let text = tree_text(tree, CONFIG_PATH)?.ok_or_else(|| {
        Fatal::Config("no spec-dna.toml in the tree: run `spec-dna init` first".into())
    })?;
    let cfg = Config::parse(&text).map_err(Fatal::Config)?;
    import_config::check_areas(&cfg, &o.areas)?;
    // A type that a layer holds and no root can hold gets a root of the type
    // `mixed` (the file `spec-dna.toml` gets it, if a target needs it).
    let mut want = vec![DocType::Rule, DocType::Requirements, DocType::Intent];
    if o.keep_research {
        want.push(DocType::Research);
    }
    let missing = import_docs::types_without_root(&cfg, &want);
    let root_path = import_docs::fallback_root(&cfg);
    let cfg = if missing.is_empty() {
        cfg
    } else {
        let mut t = text.clone();
        let eol = crate::config_text::line_ending(&t);
        crate::config_text::append_tables(
            &mut t,
            &format!("[[roots]]\npath = \"{root_path}\"\ntype = \"mixed\"\n"),
            eol,
        );
        Config::parse(&t).map_err(Fatal::Config)?
    };
    let places = Places::of(&cfg);
    if o.keep_research {
        places.research_dir()?;
    }
    let (before, _) = corpus_of(tree, &cfg)?;
    let ctx = |link: &dyn Fn(&str, &str, &str) -> String| -> Result<Read, Fatal> {
        read_source(
            &Ctx {
                tree,
                cfg: &cfg,
                places: &places,
                areas: &o.areas,
                skip_open: o.skip_open,
                keep_research: o.keep_research,
                link,
                before: &before,
            },
            o.source,
        )
    };
    // The first pass has the moves. The second pass edits the links with them.
    let first = ctx(&|t, _, _| t.to_string())?;
    let map = link_map(&first);
    let read =
        ctx(&|t, from, to| mv::rewrite_links(t, mv::parent(from), mv::parent(to), &map, &before))?;
    // The keys of `--areas` name a source folder.
    for (name, _) in &o.areas {
        let part = format!("/{name}/");
        let imported = matches!(before.resolve(name), crate::model::Res::Found(..));
        if !imported && !read.universe.iter().any(|p| p.contains(&part)) {
            return Err(Fatal::Refused(format!(
                "--areas: `{name}` is not the name of a source folder, so import changes nothing"
            )));
        }
    }
    let needs = Needs {
        legacy_key: read.fr_lines,
        areas: o.areas.iter().map(|(_, a)| a.clone()).collect(),
        types: types_of(&places, &read),
        root: (!missing.is_empty()
            && read
                .targets
                .iter()
                .any(|t| t.path.starts_with(&format!("{root_path}/"))))
        .then(|| root_path.clone()),
    };
    let root = places.requirements_root.unwrap_or(0);
    let cplan = import_config::plan(&text, &cfg, root, &needs)?;
    let fin = Config::parse(&cplan.text).map_err(|e| {
        Fatal::Config(format!(
            "the new spec-dna.toml does not parse: {e}; write the change by hand"
        ))
    })?;
    let change = ConfigChange {
        old: text,
        new: cplan.text,
        notes: cplan.notes,
    };
    let mut plan = plan_docs(
        &DocsInput {
            tree,
            fin: &fin,
            read: &read,
        },
        change,
    )?;
    plan.notes.extend(report_notes(&plan, &fin, o));
    plan.notes.extend(rename_notes(tree, &plan)?);
    Ok(Planned { plan, fin })
}

/// The notes of the report that the plan knows.
fn report_notes(plan: &Plan, fin: &Config, o: &PlanOpts) -> Vec<String> {
    let mut notes = Vec::new();
    let done = plan
        .targets
        .iter()
        .filter(|t| t.moved_from.is_some() && t.path.contains("/requirements/"))
        .count();
    if o.source == Source::Speckit && done > 1 {
        notes.push(
            "Spec Kit has no merged view of the truth: each done feature is one document, and a later feature can change a requirement of an earlier one. The documents can contradict each other, so review them".to_string(),
        );
    }
    for p in &plan.edited {
        notes.push(format!("edited the links of `{p}`"));
    }
    if fin.profile == Profile::Strict {
        notes.push(
            "with `strict`, SD608 is an error for each imported active requirement. With --skip-open the diff adds requirements and touches no intent folder, so SD917 and SD921 are errors too. Start the import with an intent whose plan lists the targets".to_string(),
        );
    }
    notes
}

/// A moved file whose edit is large: git can lose its rename (it needs a
/// similarity of one half).
fn rename_notes(tree: &Tree, plan: &Plan) -> Result<Vec<String>, Fatal> {
    let forms: Vec<String> = plan.moves.iter().map(|(f, _)| f.clone()).collect();
    if forms.is_empty() {
        return Ok(Vec::new());
    }
    let bytes = (tree.read)(&forms)?;
    let mut out = Vec::new();
    for (from, to) in &plan.moves {
        let (Some(old), Some(new)) = (bytes.get(from), plan.writes.get(to)) else {
            continue;
        };
        if similarity(&String::from_utf8_lossy(old), new) < 0.5 {
            out.push(format!(
                "the edit of `{to}` is large, so git can show `{from}` as a deleted file and `{to}` as a new file; the history stays in git, and `git log --follow` can lose it"
            ));
        }
    }
    Ok(out)
}

// --- the command ---

const RESTORE: &str = "to restore the tree, run `git reset --hard` and `git clean -fd`";

fn unclean_message(paths: &[String]) -> String {
    let shown: Vec<&str> = paths.iter().take(5).map(String::as_str).collect();
    let more = if paths.len() > shown.len() {
        format!(" and {} more", paths.len() - shown.len())
    } else {
        String::new()
    };
    format!(
        "the working tree is not clean ({} path{}: {}{more}); commit or stash the changes, then run `spec-dna import` again. import changes nothing",
        paths.len(),
        if paths.len() == 1 { "" } else { "s" },
        shown.join(", ")
    )
}

/// A symbolic link in a path, or a file that git ignores, stops the write.
fn check_paths(repo: &Repo, tree: &Tree, plan: &Plan) -> Result<(), Fatal> {
    let mut paths: std::collections::BTreeSet<&str> =
        plan.writes.keys().map(String::as_str).collect();
    for (from, to) in &plan.moves {
        paths.insert(from);
        paths.insert(to);
    }
    for p in &paths {
        if let Some(m) = finish::path_problem(&repo.root, p) {
            let cut = m.split("; `finish`").next().unwrap_or(&m).to_string();
            return Err(Fatal::Refused(format!(
                "{cut}; import does not write or move through a link. Replace the link, commit the change, then run `spec-dna import` again"
            )));
        }
    }
    for p in &paths {
        if !tree.files.iter().any(|f| f == p) && repo.git.is_ignored(p) {
            return Err(Fatal::Refused(format!(
                "git ignores {p}, so the file cannot be committed; remove the ignore rule, commit the change, then run `spec-dna import` again"
            )));
        }
    }
    Ok(())
}

/// The write (D21): each `git mv`, then each write. An error after the first
/// `git mv` is exit code 4 with the restore commands. With `config_only` the
/// write is the file `spec-dna.toml` only.
fn write_all(repo: &Repo, plan: &Plan, fin: &Config, config_only: bool) -> Result<(), Fatal> {
    let fail = |what: String| {
        if what.contains("index.lock") {
            // Another git process holds the index. It can be a second run of
            // the import, so a reset would destroy its work.
            Fatal::Io(format!(
                "{what}; another git process holds the index. Wait until it ends, look at `git status`, then run `spec-dna import` again. If no other process runs, remove the file `.git/index.lock` first"
            ))
        } else {
            Fatal::Io(format!("{what}; the write stopped half way, {RESTORE}"))
        }
    };
    if config_only {
        mv::write(&repo.config_path(), &plan.config.new)
            .map_err(|e| fail(e.message().to_string()))?;
        return repo.git.add_paths(&[CONFIG_PATH.to_string()]).map_err(fail);
    }
    for (from, to) in &plan.moves {
        if let Some(dir) = repo.root.join(to).parent() {
            std::fs::create_dir_all(dir).map_err(|e| fail(format!("{}: {e}", dir.display())))?;
        }
        repo.git.mv(from, to).map_err(fail)?;
    }
    for (path, text) in &plan.writes {
        let full = repo.root.join(path);
        if let Some(dir) = full.parent() {
            std::fs::create_dir_all(dir).map_err(|e| fail(format!("{}: {e}", dir.display())))?;
        }
        mv::write(&full, text).map_err(|e| fail(e.message().to_string()))?;
    }
    for (from, _) in &plan.moves {
        let root = fin.root_of(from).map_or("", |r| &fin.roots[r].path);
        mv::remove_empty_parents(repo, root, from);
    }
    // `git mv` staged the renames only. A plain `git commit` must hold each
    // edit too, so the import stages each file that it wrote.
    repo.git.add_paths(&plan.written_paths()).map_err(fail)
}

fn shell_word(p: &str) -> String {
    if !p.is_empty()
        && p.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._/-".contains(&b))
    {
        p.to_string()
    } else {
        format!("'{}'", p.replace('\'', "'\\''"))
    }
}

fn paths_arg(paths: &[String]) -> String {
    paths
        .iter()
        .map(|p| shell_word(p))
        .collect::<Vec<_>>()
        .join(" ")
}

/// The lines of the files: one for each source file that moves, stays or is
/// skipped, and one for each file that the plan writes.
fn file_lines(plan: &Plan, dry: bool) -> Vec<String> {
    let (mv_word, wr_word, ed_word) = if dry {
        ("would move", "would write", "would edit")
    } else {
        ("moved", "wrote", "edited")
    };
    let mut out = Vec::new();
    for s in &plan.sources {
        match s.state {
            State::Moved => out.push(format!("{mv_word} {} -> {}", s.path, s.to.join(", "))),
            State::Kept => out.push(format!("kept {}: {}", s.path, s.reason)),
            State::Skipped => out.push(format!("skipped {}: {}", s.path, s.reason)),
            State::Written if s.to == [s.path.clone()] => {
                out.push(format!("{ed_word} {}: {}", s.path, s.reason));
            }
            State::Written => {}
        }
    }
    let mut written: Vec<&crate::import_docs::Target> = plan
        .targets
        .iter()
        .filter(|t| t.moved_from.is_none() && plan.writes.contains_key(&t.path))
        .collect();
    written.sort_by(|a, b| a.path.cmp(&b.path));
    for t in written {
        out.push(format!(
            "{wr_word} {} (from {})",
            t.path,
            t.sources.join(", ")
        ));
    }
    out
}

fn proof_line(p: &Proof) -> String {
    format!(
        "proof: {} ({} requirements, {} scenarios, {} ids)",
        if p.ok() { "ok" } else { "failed" },
        p.requirements,
        p.scenarios,
        p.ids
    )
}

fn output_text(plan: &Plan, proof: &Proof, dry: bool, config_only: bool, source: Source) -> String {
    let mut out = String::new();
    let (wr_word, cfg_notes) = (
        if dry { "would write" } else { "wrote" },
        &plan.config.notes,
    );
    if config_only {
        out.push_str(&format!(
            "{wr_word} {CONFIG_PATH} ({})\n",
            cfg_notes.join("; ")
        ));
        out.push_str(&proof_line(proof));
        out.push('\n');
        out.push_str(&format!(
            "the plan needs this change of {CONFIG_PATH} first, and import writes only this file now: commit it in a pull request of its own, then run `spec-dna import {}` again\n",
            source.name()
        ));
        out.push_str(&format!("to commit: git add -A -- {CONFIG_PATH}\n"));
        return out;
    }
    for l in file_lines(plan, dry) {
        out.push_str(&l);
        out.push('\n');
    }
    for p in &plan.pairs {
        match &p.delta {
            Some(d) => out.push_str(&format!("pair {} -> {} (delta of `{d}`)\n", p.from, p.id)),
            None => out.push_str(&format!("pair {} -> {}\n", p.from, p.id)),
        }
    }
    out.push_str(&proof_line(proof));
    out.push('\n');
    out.push_str("report:\n");
    for n in &plan.notes {
        out.push_str(&format!("  note: {n}\n"));
    }
    if !proof.dropped.is_empty() {
        out.push_str(&format!(
            "  dropped {} line(s) of the sources that no target holds (a heading or a task line that the reader rewrote is no loss):\n",
            proof.dropped.len()
        ));
        for d in proof.dropped.iter().take(20) {
            out.push_str(&format!("    {d}\n"));
        }
        if proof.dropped.len() > 20 {
            out.push_str(&format!("    and {} more\n", proof.dropped.len() - 20));
        }
    }
    if proof.removed_anchors > 0 {
        out.push_str(&format!(
            "  {} anchor(s) of headings of moved documents are gone, because the heading became a list item; the ids of the requirements stay\n",
            proof.removed_anchors
        ));
    }
    for (code, n) in &proof.ignored {
        out.push_str(&format!(
            "  {code}: {n} new finding(s) in the imported documents, which the proof does not guard\n"
        ));
    }
    if !plan.outside_links.is_empty() {
        out.push_str(&format!(
            "  {} link(s) of files that are no document go to a moved file, and the import does not change them\n",
            plan.outside_links.len()
        ));
    }
    let add = plan.written_paths();
    if !add.is_empty() {
        out.push_str(&format!("to commit: git add -A -- {}\n", paths_arg(&add)));
    }
    if !plan.removes.is_empty() {
        out.push_str(&format!(
            "to remove the sources: git rm {}-- {}\n",
            if plan.removes_recursive { "-r " } else { "" },
            paths_arg(&plan.removes)
        ));
    }
    out
}

fn output_json(plan: &Plan, proof: &Proof, dry: bool, config_only: bool, source: Source) -> String {
    let sources = if config_only {
        Vec::new()
    } else {
        plan.sources
            .iter()
            .map(|s| {
                Obj::new()
                    .str("path", &s.path)
                    .str("state", s.state.name())
                    .strs("to", &s.to)
                    .str("reason", &s.reason)
                    .end()
            })
            .collect()
    };
    let pairs = if config_only {
        Vec::new()
    } else {
        plan.pairs
            .iter()
            .map(|p| {
                Obj::new()
                    .str("from", &p.from)
                    .str("id", &p.id)
                    .opt_str("delta", p.delta.as_deref())
                    .end()
            })
            .collect()
    };
    let proof_obj = Obj::new()
        .bool("ok", proof.ok())
        .num("requirements", proof.requirements)
        .num("scenarios", proof.scenarios)
        .num("ids", proof.ids)
        .strs("failures", &proof.failures)
        .end();
    let mut notes: Vec<String> = Vec::new();
    if config_only {
        notes.extend(plan.config.notes.iter().map(|n| format!("config: {n}")));
    }
    notes.extend(plan.notes.iter().cloned());
    notes.extend(proof.dropped.iter().map(|d| format!("dropped {d}")));
    let report = Obj::new()
        .raw(
            "errors",
            &json::array(
                proof
                    .ignored
                    .iter()
                    .map(|(c, n)| Obj::new().str("code", c).num("count", n).end()),
            ),
        )
        .num("outside_links", plan.outside_links.len())
        .strs("notes", &notes)
        .end();
    Obj::new()
        .str("version", VERSION)
        .str("source", source.name())
        .bool("dry_run", dry)
        .bool("config_only", config_only)
        .raw("sources", &json::array(sources))
        .raw("pairs", &json::array(pairs))
        .raw("proof", &proof_obj)
        .raw("report", &report)
        .end()
}

/// `spec-dna import`. The phases: the project, the arguments, the clean tree,
/// the plan, the proof, the write, the report. It never commits.
pub fn run(repo: &Repo, o: &RunOpts) -> Result<u8, Fatal> {
    let text = std::fs::read(repo.config_path()).map_err(|_| {
        Fatal::Config(format!(
            "no {} at {}: run `spec-dna init` first",
            config::FILE_NAME,
            crate::repo::shown(&repo.root)
        ))
    })?;
    let text = String::from_utf8(text)
        .map_err(|_| Fatal::Config(format!("{} is not UTF-8", config::FILE_NAME)))?;
    let cfg = Config::parse(&text).map_err(Fatal::Config)?;
    if cfg.pin != VERSION {
        return Err(Fatal::Config(config::pin_mismatch_message(
            &cfg.pin,
            &cfg.install_source,
            cfg.sha256.as_deref(),
        )));
    }
    let areas = o
        .areas
        .as_deref()
        .map(import_config::read_areas)
        .transpose()?
        .unwrap_or_default();
    if o.keep_research {
        Places::of(&cfg).research_dir()?;
    }
    let unclean = repo.git.unclean_paths().map_err(Fatal::Io)?;
    if !unclean.is_empty() {
        return Err(Fatal::Refused(unclean_message(&unclean)));
    }
    if repo.head_commit().is_none() {
        return Err(Fatal::Refused(
            "the repository has no commit; commit the project, then run `spec-dna import` again"
                .into(),
        ));
    }
    let data = TreeData::from_git(&repo.git, "HEAD")?;
    let popts = PlanOpts {
        source: o.source,
        skip_open: o.skip_open,
        keep_research: o.keep_research,
        areas,
    };
    data.with(|tree| {
        let Planned { plan, fin } = self::plan(tree, &popts)?;
        let proof = if plan.is_empty() {
            Proof::default()
        } else {
            import_proof::prove(tree, &plan, &fin, &|t| {
                self::plan(t, &popts).map(|p| p.plan)
            })?
        };
        let config_only = plan.config.changed();
        if !proof.ok() {
            if o.json {
                println!(
                    "{}",
                    output_json(&plan, &proof, o.dry_run, config_only, o.source)
                );
            }
            return Err(Fatal::Refused(format!(
                "the proof failed, so import wrote nothing:\n  {}",
                proof.failures.join("\n  ")
            )));
        }
        if !plan.is_empty() {
            check_paths(repo, tree, &plan)?;
        }
        if !o.dry_run && !plan.is_empty() {
            write_all(repo, &plan, &fin, config_only)?;
        }
        if o.json {
            println!(
                "{}",
                output_json(&plan, &proof, o.dry_run, config_only, o.source)
            );
        } else if plan.is_empty() {
            println!("spec-dna import: nothing to change");
        } else {
            print!(
                "{}",
                output_text(&plan, &proof, o.dry_run, config_only, o.source)
            );
        }
        Ok(0)
    })
}

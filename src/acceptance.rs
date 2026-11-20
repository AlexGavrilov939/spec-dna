//! The rules around the acceptance record that `finish` and `check` share
//! (design 0.8.0, D3 items 1, 3, 7, 8 and 9).
//!
//! - "A change that needs a record" (item 1) is one function: `needs_record`.
//!   `finish` (step 5.2), `check` and `next` call it.
//! - The archive file of a finished change: its path and its name rule
//!   (item 3).
//! - The checks that read a record: the rules of a valid record with the compare
//!   of the brief (SD928, item 8), SD929, SD930, SD931 and SD933 (item 7).
//!
//! The functions read git and the files that the caller names. They write
//! nothing. The code of the diagnostics is in `diag.rs`, and the severity is
//! the severity of the profile (the group `acceptance` of `config.rs`).

use crate::accept::{self, Composed, Texts};
use crate::change::{self, Change};
use crate::config::{self, Config, Version};
use crate::diag::Code;
use crate::git::Git;
use crate::lifecycle::Diff;
use crate::model::{Class, Corpus};
use crate::parse::{ParsedDoc, parse};
use crate::record::{self, Acceptor, Expect, Record};
use crate::repo::{Fatal, Repo};
use crate::tree;
use std::collections::HashSet;
use std::sync::Arc;

/// The folder of the archive files (design 0.8.0, D3 item 3).
pub const ARCHIVE_DIR: &str = ".spec-dna/accepted";

/// The first version whose plans need an acceptance record.
const FIRST_VERSION: Version = Version(0, 8, 0);

/// True if `path` is a file of the archive folder.
pub fn is_archive_path(path: &str) -> bool {
    path.strip_prefix(ARCHIVE_DIR)
        .is_some_and(|rest| rest.starts_with('/'))
}

/// The number of an archive file of the slug: 1 for `<slug>.toml`, `n` for
/// `<slug>-<n>.toml`. `None` for any other path.
pub fn archive_number(path: &str, slug: &str) -> Option<u32> {
    let name = path.strip_prefix(ARCHIVE_DIR)?.strip_prefix('/')?;
    let stem = name.strip_suffix(".toml")?;
    if stem == slug {
        return Some(1);
    }
    let n = stem.strip_prefix(slug)?.strip_prefix('-')?;
    let ok = !n.is_empty() && !n.starts_with('0') && n.bytes().all(|b| b.is_ascii_digit());
    ok.then(|| n.parse().ok()).flatten().filter(|n| *n >= 2)
}

/// The path of the archive file that `finish` writes for the slug:
/// `<slug>.toml`, or the first `<slug>-<n>.toml` from 2 on whose file does not
/// exist. `exists` tells if a path exists on disk.
pub fn next_archive_path(slug: &str, exists: &dyn Fn(&str) -> bool) -> String {
    let first = format!("{ARCHIVE_DIR}/{slug}.toml");
    if !exists(&first) {
        return first;
    }
    (2u32..)
        .map(|n| format!("{ARCHIVE_DIR}/{slug}-{n}.toml"))
        .find(|p| !exists(p))
        .expect("an unbounded range has a free name")
}

/// The folder and the slug of a record in the folder of a change: a path
/// `<intent folder>/<slug>/acceptance.toml`. `intent_dirs` are the folders of
/// the layer `intent`.
pub fn record_folder(path: &str, intent_dirs: &[String]) -> Option<(String, String)> {
    let folder = path.strip_suffix(&format!("/{}", record::FILE_NAME))?;
    let (dir, slug) = folder.rsplit_once('/')?;
    intent_dirs
        .iter()
        .any(|d| d == dir)
        .then(|| (folder.to_string(), slug.to_string()))
}

/// What the rule "a change that needs a record" finds.
#[derive(Clone, Debug, PartialEq)]
pub enum Need {
    /// No record is needed. The text is the note that `finish` prints when the
    /// plan is older than 0.8.0.
    No(Option<String>),
    /// A record is needed. `shallow` is true if the clone is shallow and the
    /// history has no commit that added the plan.
    Yes { shallow: bool },
}

/// The rule of D3 item 1, for a change that has a plan and is not closed. The
/// caller checks both.
///
/// A record is needed when the acceptance is on (SD927 is not off), and the plan
/// is new:
/// - a plan that the base tree does not have (a commit of `base..HEAD` added it,
///   or no commit did) is always new (Q43). So a branch cannot set the pin back
///   for one commit to skip the record;
/// - a plan that the base tree has is new if the newest commit that added it and
///   that is an ancestor of the lifecycle base pinned 0.8.0 or later in
///   `spec-dna.toml`. A pin before 0.8.0 gives no record, with a note. No such
///   commit (a shallow clone), or no pin in it, gives the stricter result.
///
/// `plan_in_base` tells if the tree of `base_rev` has the plan at `plan_path`.
pub fn needs_record(
    cfg: &Config,
    git: &Git,
    base_rev: &str,
    plan_path: &str,
    plan_in_base: bool,
) -> Result<Need, String> {
    if cfg.is_off(Code::AcceptanceMissing) {
        return Ok(Need::No(None));
    }
    if !plan_in_base {
        return Ok(Need::Yes { shallow: false });
    }
    let Some(commit) = git.added_at(base_rev, plan_path)? else {
        return Ok(Need::Yes {
            shallow: git.is_shallow(),
        });
    };
    let text = git
        .read_at(&[(commit, config::FILE_NAME.to_string())])?
        .into_iter()
        .next()
        .flatten();
    let pin = text.and_then(|b| config::scan_pin(&String::from_utf8_lossy(&b)));
    match pin {
        Some(pin) if Version::parse(&pin).is_some_and(|v| v < FIRST_VERSION) => {
            Ok(Need::No(Some(format!(
                "`{plan_path}` was added when spec-dna.toml pinned {pin}, before 0.8.0, so `finish` needs no acceptance record"
            ))))
        }
        _ => Ok(Need::Yes { shallow: false }),
    }
}

/// The message of SD927 for a missing record. `where_` names the place that
/// should have it.
pub fn missing_message(slug: &str, where_: &str, shallow: bool) -> String {
    let mut m = format!(
        "the change `{slug}` needs an acceptance record, and {where_}; give the output of `spec-dna accept {slug} --brief`, unchanged, to an agent `acceptor`, and commit the record that it writes"
    );
    if shallow {
        m.push_str(
            "; the clone is shallow, so the commit that added the plan is not in the history: fetch the full history (`fetch-depth: 0`)",
        );
    }
    m
}

/// What the rules of a valid record need to know of the plan of the change.
#[derive(Clone, Copy, Debug, Default)]
pub struct PlanFacts {
    pub has_files: bool,
    pub tasks_have_files: bool,
}

/// Reads the facts from the change and the corpus that has its plan.
pub fn plan_facts(ch: &Change, corpus: &Corpus) -> PlanFacts {
    PlanFacts {
        has_files: !ch.scope_patterns(corpus).is_empty(),
        tasks_have_files: ch
            .tasks(corpus)
            .iter()
            .any(|t| !t.duplicate && t.files.iter().any(|f| crate::change::is_scope_pattern(f))),
    }
}

/// Reads a record and checks the rules of a valid record (SD928, without the
/// compare of the brief). It returns the record, if the text reads, and one text
/// for each broken rule.
pub fn read_record(
    cfg: &Config,
    text: &str,
    slug: &str,
    facts: PlanFacts,
) -> (Option<Record>, Vec<String>) {
    let intent_dirs = crate::lifecycle::intent_dirs(cfg);
    let is_code = |p: &str| tree::is_code_path(cfg, p, &intent_dirs);
    let expect = Expect {
        slug,
        plan_has_files: facts.has_files,
        tasks_have_files: facts.tasks_have_files,
        is_code_path: &is_code,
    };
    match Record::parse(text) {
        Ok(r) => {
            let problems = record::validate(&r, &expect);
            (Some(r), problems)
        }
        Err(e) => (None, vec![e]),
    }
}

/// SD931: 2 acceptor entries have the same `session` and the same `agent`. The
/// value `unknown` and an empty value match no value.
pub fn self_accepted(rec: &Record) -> Vec<String> {
    let claim = |v: &str| !v.trim().is_empty() && v != record::UNKNOWN && v != record::PENDING;
    let mut out = Vec::new();
    for (j, b) in rec.acceptors.iter().enumerate() {
        if !claim(&b.session) || !claim(&b.agent) {
            continue;
        }
        if let Some(i) = rec.acceptors[..j]
            .iter()
            .position(|a| a.session == b.session && a.agent == b.agent)
        {
            out.push(format!(
                "acceptor {} and acceptor {} have the same `session` (`{}`) and the same `agent` (`{}`); an acceptor must be another agent than the builder and the other acceptors",
                i + 1,
                j + 1,
                b.session,
                b.agent
            ));
        }
    }
    out
}

/// SD933: the record has fewer acceptor entries than the class of the change
/// needs. The roles of `[owners]` are a later slice, so none adds to the count.
pub fn acceptors_missing(cfg: &Config, class: Class, rec: &Record) -> Option<String> {
    let needed = match class {
        Class::Routine => 1,
        Class::HigherRisk => cfg.higher_risk_acceptors as usize,
    };
    (rec.acceptors.len() < needed).then(|| {
        format!(
            "the record has {} acceptor entries, and a {} change needs {needed}; run `spec-dna accept {} [--role <name>]` for each missing acceptor",
            rec.acceptors.len(),
            class.name(),
            rec.change
        )
    })
}

/// SD930: the top `result` is `rejected`.
pub fn rejected(rec: &Record) -> Option<String> {
    (rec.result == "rejected").then(|| {
        format!(
            "the record of the change `{}` says `rejected`; fix each finding in the code, delete the record, and accept again",
            rec.change
        )
    })
}

/// SD929: the record is stale. The hash of `paths` differs from the code that
/// `git` reads (the working tree, or the index for a `staged` `Git`), or the diff
/// has a code path that `scope` matches and `paths` does not list. `after_finish`
/// chooses the fix that the message names. `None`: the record is fresh.
pub fn stale(
    cfg: &Config,
    git: &Git,
    rec: &Record,
    diff: Option<&Diff>,
    after_finish: bool,
) -> Result<Option<String>, String> {
    let entries = tree::entries_of(git, cfg)?;
    let now = record::tree_hash(&entries, &rec.paths);
    let mut why: Vec<String> = Vec::new();
    if !tree::matches(&rec.tree, &now) {
        why.push(format!(
            "the hash of the code that `paths` lists is {now}, and the record has {}",
            rec.tree
        ));
    }
    if let Some(diff) = diff {
        let intent_dirs = crate::lifecycle::intent_dirs(cfg);
        let is_code = |p: &str| tree::is_code_path(cfg, p, &intent_dirs);
        let changed: Vec<String> = diff.files.iter().map(|f| f.0.clone()).collect();
        for p in record::select_paths(&rec.scope, &changed, &is_code) {
            if !rec.paths.contains(&p) {
                why.push(format!(
                    "the diff has the code path `{p}`, which `scope` matches and `paths` does not list"
                ));
            }
        }
    }
    if why.is_empty() {
        return Ok(None);
    }
    let slug = &rec.change;
    let fix = if after_finish {
        format!(
            "revert the commit of `finish`, merge the base, run `spec-dna accept {slug}` again from a new brief, and run `spec-dna finish {slug}` again"
        )
    } else {
        format!(
            "delete the record, run `spec-dna accept {slug}` again from a new brief, and run `spec-dna finish {slug}` again"
        )
    };
    Ok(Some(format!(
        "the record of the change `{slug}` is stale: {}; {fix}",
        why.join("; ")
    )))
}

// The compare of the brief (SD928, item 8).

/// The identity of an entry: its role, session and brief.
fn triple(a: &Acceptor) -> (String, String, String) {
    (
        a.role.clone(),
        a.session.clone(),
        a.brief.to_ascii_lowercase(),
    )
}

fn triples(text: Option<&[u8]>) -> HashSet<(String, String, String)> {
    text.and_then(|b| Record::parse(&String::from_utf8_lossy(b)).ok())
        .map(|r| r.acceptors.iter().map(triple).collect())
        .unwrap_or_default()
}

/// True if `s` is 16 hexadecimal digits.
fn is_hash(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The 3 files of a change in one tree, as text. `None` for a missing file.
struct Files {
    intent: Option<String>,
    spec: Option<String>,
    plan: Option<String>,
}

fn text_of(b: Option<Vec<u8>>) -> Option<String> {
    b.map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// The `declared` items of the files of a tree, read as `finish` and `check`
/// read a change: the 3 files parse into a corpus of their own.
fn declared_of(cfg: &Config, folder: &str, files: &Files) -> accept::Declared {
    let mut docs: Vec<(String, Arc<ParsedDoc>)> = Vec::new();
    for (name, text) in [
        ("intent.md", &files.intent),
        ("spec.md", &files.spec),
        ("plan.md", &files.plan),
    ] {
        if let Some(t) = text {
            docs.push((format!("{folder}/{name}"), Arc::new(parse(t))));
        }
    }
    let corpus = Corpus::build(cfg, docs.iter().map(|d| d.0.clone()).collect(), docs);
    match change::collect(&corpus)
        .into_iter()
        .find(|c| c.dir == folder)
    {
        Some(ch) => accept::declared(cfg, &ch, &corpus),
        None => accept::Declared::default(),
    }
}

/// Where the tree of an entry comes from.
enum Source {
    Commit(String),
    /// No commit has the entry: the index with `--staged`, else the working
    /// tree.
    Head,
}

/// The problems of the compare: for each entry that the branch adds to the
/// record, the brief of the entry against the brief that the tree of its adding
/// commit gives (design 0.8.0, D3 item 8).
///
/// - An entry that the record of the base has, by its role, session and brief,
///   was compared in its own pull request, so it is not compared again.
/// - The adding commit is the oldest commit of the latest unbroken run of the
///   commits of `base..HEAD` (first parent, the oldest first) whose record holds
///   an entry with the same triple. An entry that no commit holds takes the
///   index with `--staged`, else the working tree.
/// - A shallow clone that cuts `base..HEAD` gives one problem that names
///   `fetch-depth: 0`.
pub fn brief_problems(
    cfg: &Config,
    git: &Git,
    base_rev: &str,
    folder: &str,
    rec: &Record,
) -> Result<Vec<String>, String> {
    let rec_path = format!("{folder}/{}", record::FILE_NAME);
    let slug = folder.rsplit('/').next().unwrap_or(folder);
    let base_triples = triples(
        git.read_at(&[(base_rev.to_string(), rec_path.clone())])?
            .into_iter()
            .next()
            .flatten()
            .as_deref(),
    );
    // The entries to compare: those that the base does not have, and whose
    // `brief` is a hash (the rules of a valid record report any other value).
    let entries: Vec<(usize, &Acceptor)> = rec
        .acceptors
        .iter()
        .enumerate()
        .filter(|(_, a)| is_hash(&a.brief) && !base_triples.contains(&triple(a)))
        .collect();
    if entries.is_empty() {
        return Ok(Vec::new());
    }
    if git.range_is_cut(base_rev) {
        let list: Vec<String> = entries.iter().map(|(i, _)| (i + 1).to_string()).collect();
        return Ok(vec![format!(
            "the history of `{base_rev}..HEAD` is cut (a shallow clone), so the brief of acceptor {} cannot be compared with its adding commit; fetch the full history (`fetch-depth: 0`)",
            list.join(", ")
        )]);
    }
    let commits = git.first_parent_commits(base_rev)?;
    let specs: Vec<(String, String)> = commits
        .iter()
        .map(|c| (c.clone(), rec_path.clone()))
        .collect();
    let per_commit: Vec<HashSet<(String, String, String)>> = git
        .read_at(&specs)?
        .into_iter()
        .map(|t| triples(t.as_deref()))
        .collect();
    // The files of each source, read once.
    let mut cache: Vec<(String, Files)> = Vec::new();
    let mut out: Vec<String> = Vec::new();
    for (i, a) in entries {
        let t = triple(a);
        let mut run: Option<usize> = None;
        for (ci, set) in per_commit.iter().enumerate() {
            if set.contains(&t) {
                run.get_or_insert(ci);
            } else {
                run = None;
            }
        }
        let source = match run {
            Some(ci) => Source::Commit(commits[ci].clone()),
            None => Source::Head,
        };
        let (key, place) = match &source {
            Source::Commit(c) => (
                c.clone(),
                format!("its adding commit {}", &c[..c.len().min(12)]),
            ),
            Source::Head => (
                String::new(),
                format!(
                    "the {} that holds the entry (no commit has it yet)",
                    if git.is_staged() {
                        "index"
                    } else {
                        "working tree"
                    }
                ),
            ),
        };
        if !cache.iter().any(|(k, _)| *k == key) {
            let names = ["intent.md", "spec.md", "plan.md"];
            let mut texts: Vec<Option<String>> = Vec::new();
            match &source {
                Source::Commit(c) => {
                    let specs: Vec<(String, String)> = names
                        .iter()
                        .map(|n| (c.clone(), format!("{folder}/{n}")))
                        .collect();
                    texts.extend(git.read_at(&specs)?.into_iter().map(text_of));
                }
                Source::Head => {
                    for n in names {
                        texts.push(text_of(git.head_file(&format!("{folder}/{n}"))?));
                    }
                }
            }
            let mut it = texts.into_iter();
            cache.push((
                key.clone(),
                Files {
                    intent: it.next().flatten(),
                    spec: it.next().flatten(),
                    plan: it.next().flatten(),
                },
            ));
        }
        let files = &cache.iter().find(|(k, _)| *k == key).expect("cached").1;
        let (Some(intent), Some(plan)) = (&files.intent, &files.plan) else {
            out.push(format!(
                "the brief of acceptor {} cannot be computed again: the tree of {place} has no `intent.md` or no `plan.md` in `{folder}`; remove the entry, and accept again from a new brief",
                i + 1
            ));
            continue;
        };
        let decl = declared_of(cfg, folder, files);
        let text = accept::brief_text(
            cfg,
            slug,
            &a.role,
            &Texts {
                intent,
                spec: files.spec.as_deref(),
                plan,
            },
            &Composed {
                scope: &rec.scope,
                paths: &rec.paths,
                tree: &rec.tree,
            },
            &decl,
        );
        let want = record::brief_hash(&text).unwrap_or_default();
        if !want.eq_ignore_ascii_case(&a.brief) {
            out.push(format!(
                "the brief of acceptor {} is not the brief of {place}; remove the entry, and accept again from a new brief",
                i + 1
            ));
        }
    }
    Ok(out)
}

// The bytes of an archive file (SD935).

/// The text of the folder record that `finish` moved to `archive`: the newest
/// text of `<folder>/acceptance.toml` in the commits of `base..HEAD` (first
/// parent), else its text in the base.
fn moved_record(git: &Git, base_rev: &str, folder: &str) -> Result<Option<Vec<u8>>, String> {
    let rec_path = format!("{folder}/{}", record::FILE_NAME);
    let commits = git.first_parent_commits(base_rev)?;
    let mut specs: Vec<(String, String)> = commits
        .iter()
        .map(|c| (c.clone(), rec_path.clone()))
        .collect();
    specs.push((base_rev.to_string(), rec_path));
    // The newest commit comes last, and the base is the oldest source.
    let mut texts = git.read_at(&specs)?;
    let base_text = texts.pop().flatten();
    Ok(texts.into_iter().rev().flatten().next().or(base_text))
}

/// The SD935 message if the archive file `archive`, which the diff adds with
/// the bytes `bytes`, is not the record that `finish` moves: the bytes differ
/// from the folder record, or no folder record is in the history of the branch
/// or in the base. The rule is one rule for the first commit of `finish` and
/// for a later commit that edits the file: `finish` writes the folder record,
/// byte for byte. It returns `None` in a shallow clone that cuts `base..HEAD`:
/// the record cannot be found, and SD928 reports the cut.
pub fn archive_differs(
    git: &Git,
    base_rev: &str,
    folder: &str,
    archive: &str,
    bytes: &[u8],
) -> Result<Option<String>, String> {
    if git.range_is_cut(base_rev) {
        return Ok(None);
    }
    let rec_path = format!("{folder}/{}", record::FILE_NAME);
    Ok(match moved_record(git, base_rev, folder)? {
        // CRLF and LF are one record (ADR 0019).
        Some(old) if record::to_lf(&old) == record::to_lf(bytes) => None,
        Some(_) => Some(format!(
            "the archive file `{archive}` is not the record of the change that `finish` moves: its bytes differ from `{rec_path}`; write the file again with `spec-dna finish`, or revert the edit"
        )),
        None => Some(format!(
            "the archive file `{archive}` has no `{rec_path}` in the base or in the commits of the branch, so no `finish` moved it; run `spec-dna accept` and `spec-dna finish`"
        )),
    })
}

// The facts that `finish` needs (step 5.2 and 5.5).

/// The facts of `finish` about the record. `main` computes them with git, and
/// `finish::plan` stays free of git (design 0.8.0, section 11, "The steps of
/// `finish`").
#[derive(Clone, Debug)]
pub struct FinishFacts {
    pub need: Need,
    /// The problems of the compare of the briefs (SD928).
    pub brief_problems: Vec<String>,
    /// The reason of SD929, if the record is stale.
    pub stale: Option<String>,
    /// The path of the archive file that `finish` writes.
    pub archive: String,
    /// The ignore rule of git for that path, if there is one.
    pub ignored: Option<String>,
}

/// The rule "a change that needs a record" for a change of the head tree: the
/// one function that `finish`, `next` and `check` use (design 0.8.0, D3 item 1).
/// It returns `None` when the rule does not apply: the acceptance is off, the
/// change has no plan, it is closed, or git names no lifecycle base. `base` and
/// `base_rev` are the lifecycle tree and its commit.
pub fn need_for_change(
    repo: &Repo,
    cfg: &Config,
    ch: &Change,
    head: &Corpus,
    base: Option<&Corpus>,
    base_rev: Option<&str>,
) -> Result<Option<Need>, Fatal> {
    let (Some(plan), Some(base), Some(base_rev)) = (&ch.plan, base, base_rev) else {
        return Ok(None);
    };
    if ch.intent_status() == "closed" || cfg.is_off(Code::AcceptanceMissing) {
        return Ok(None);
    }
    let plan_path = &head.docs[plan.doc as usize].path;
    needs_record(
        cfg,
        &repo.git,
        base_rev,
        plan_path,
        base.by_path.contains_key(plan_path),
    )
    .map(Some)
    .map_err(Fatal::Io)
}

/// Computes the facts for the change `ch` of the folder. It returns `None` when
/// `finish` has no step 5.2 for the change ([`need_for_change`]) and the folder
/// has no record to archive. A record in the folder is archived in each profile
/// and for each plan, also when no rule needs it.
pub fn finish_facts(
    repo: &Repo,
    cfg: &Config,
    ch: &Change,
    head: &Corpus,
    base: Option<&Corpus>,
    base_rev: Option<&str>,
    diff: Option<&Diff>,
) -> Result<Option<FinishFacts>, Fatal> {
    let need = need_for_change(repo, cfg, ch, head, base, base_rev)?;
    let record_path = format!("{}/{}", ch.dir, record::FILE_NAME);
    let present = repo.root.join(&record_path).is_file();
    if need.is_none() && !present {
        return Ok(None);
    }
    let archive = next_archive_path(&ch.slug, &|p| repo.root.join(p).exists());
    let mut f = FinishFacts {
        ignored: repo.git.ignore_rule(&archive),
        archive,
        need: need.unwrap_or(Need::No(None)),
        brief_problems: Vec::new(),
        stale: None,
    };
    let (Need::Yes { .. }, Some(base_rev)) = (&f.need, base_rev) else {
        return Ok(Some(f));
    };
    let Ok(text) = std::fs::read_to_string(repo.root.join(&record_path)) else {
        return Ok(Some(f));
    };
    let Ok(rec) = Record::parse(&text) else {
        return Ok(Some(f));
    };
    if !cfg.is_off(Code::AcceptanceInvalid) {
        f.brief_problems =
            brief_problems(cfg, &repo.git, base_rev, &ch.dir, &rec).map_err(Fatal::Io)?;
    }
    if !cfg.is_off(Code::AcceptanceStale) {
        f.stale = stale(cfg, &repo.git, &rec, diff, false).map_err(Fatal::Io)?;
    }
    Ok(Some(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(acceptors: &[(&str, &str, &str)]) -> Record {
        let mut text = String::from(
            "format = 1\nchange = \"demo\"\nscope = []\npaths = []\ntree = \"0123456789abcdef\"\nresult = \"accepted\"\nacknowledged-by = []\n",
        );
        for (role, session, agent) in acceptors {
            text.push_str(&format!(
                "\n[[acceptor]]\nrole = \"{role}\"\nbrief = \"0123456789abcdef\"\nsession = \"{session}\"\nagent = \"{agent}\"\nresult = \"accepted\"\n"
            ));
        }
        Record::parse(&text).unwrap()
    }

    #[test]
    fn an_archive_path_is_a_file_of_the_archive_folder() {
        assert!(is_archive_path(".spec-dna/accepted/demo.toml"));
        assert!(is_archive_path(".spec-dna/accepted/a/b.toml"));
        assert!(!is_archive_path(".spec-dna/accepted"));
        assert!(!is_archive_path(".spec-dna/acceptedx/demo.toml"));
        assert!(!is_archive_path("docs/.spec-dna/accepted/demo.toml"));
    }

    #[test]
    fn the_archive_number_of_a_name_is_1_for_the_slug_and_n_for_a_suffix() {
        let n = |p: &str, s: &str| archive_number(p, s);
        assert_eq!(n(".spec-dna/accepted/demo.toml", "demo"), Some(1));
        assert_eq!(n(".spec-dna/accepted/demo-2.toml", "demo"), Some(2));
        assert_eq!(n(".spec-dna/accepted/demo-13.toml", "demo"), Some(13));
        // A slug that ends in a number is its own slug, not a numbered file.
        assert_eq!(n(".spec-dna/accepted/demo-2.toml", "demo-2"), Some(1));
        assert_eq!(n(".spec-dna/accepted/demo-1.toml", "demo"), None);
        assert_eq!(n(".spec-dna/accepted/demo-02.toml", "demo"), None);
        assert_eq!(n(".spec-dna/accepted/demo-x.toml", "demo"), None);
        assert_eq!(n(".spec-dna/accepted/other.toml", "demo"), None);
        assert_eq!(n(".spec-dna/accepted/demo.md", "demo"), None);
        assert_eq!(n("docs/demo.toml", "demo"), None);
    }

    #[test]
    fn the_next_archive_path_takes_the_first_free_number() {
        let taken = |set: &'static [&'static str]| move |p: &str| set.contains(&p);
        assert_eq!(
            next_archive_path("demo", &taken(&[])),
            ".spec-dna/accepted/demo.toml"
        );
        assert_eq!(
            next_archive_path("demo", &taken(&[".spec-dna/accepted/demo.toml"])),
            ".spec-dna/accepted/demo-2.toml"
        );
        assert_eq!(
            next_archive_path(
                "demo",
                &taken(&[
                    ".spec-dna/accepted/demo.toml",
                    ".spec-dna/accepted/demo-2.toml",
                    ".spec-dna/accepted/demo-4.toml"
                ])
            ),
            ".spec-dna/accepted/demo-3.toml"
        );
    }

    #[test]
    fn a_record_in_the_folder_of_a_change_has_a_folder_and_a_slug() {
        let dirs = vec!["docs/intent".to_string()];
        let f = |p: &str| record_folder(p, &dirs);
        assert_eq!(
            f("docs/intent/demo/acceptance.toml"),
            Some(("docs/intent/demo".to_string(), "demo".to_string()))
        );
        assert_eq!(f("docs/intent/demo/other.toml"), None);
        assert_eq!(f("docs/intent/demo/deep/acceptance.toml"), None);
        assert_eq!(f("docs/other/demo/acceptance.toml"), None);
        assert_eq!(f("docs/intent/acceptance.toml"), None);
    }

    #[test]
    fn two_entries_with_a_claim_in_common_give_sd931_and_unknown_gives_none() {
        let same = rec(&[("acceptor", "s1", "a1"), ("reviewer", "s1", "a1")]);
        assert_eq!(self_accepted(&same).len(), 1);
        let other_agent = rec(&[("acceptor", "s1", "a1"), ("reviewer", "s1", "a2")]);
        assert!(self_accepted(&other_agent).is_empty());
        let other_session = rec(&[("acceptor", "s1", "a1"), ("reviewer", "s2", "a1")]);
        assert!(self_accepted(&other_session).is_empty());
        // 2 subagents of one session: no agent id.
        let unknown = rec(&[("acceptor", "s1", "unknown"), ("reviewer", "s1", "unknown")]);
        assert!(self_accepted(&unknown).is_empty());
        let both_unknown = rec(&[
            ("acceptor", "unknown", "unknown"),
            ("a", "unknown", "unknown"),
        ]);
        assert!(self_accepted(&both_unknown).is_empty());
    }

    #[test]
    fn the_count_of_acceptors_follows_the_class_and_the_key() {
        let cfg = |n: u32| {
            Config::parse(&format!(
                "spec-dna = \"0.1.0\"\n[change]\npolicy = \"three-class\"\nhigher-risk-acceptors = {n}\n[[roots]]\npath = \"docs\"\n"
            ))
            .unwrap()
        };
        let one = rec(&[("acceptor", "s1", "unknown")]);
        let two = rec(&[("acceptor", "s1", "unknown"), ("acceptor", "s2", "unknown")]);
        assert!(acceptors_missing(&cfg(3), Class::Routine, &one).is_none());
        assert!(acceptors_missing(&cfg(2), Class::HigherRisk, &one).is_some());
        assert!(acceptors_missing(&cfg(2), Class::HigherRisk, &two).is_none());
        assert!(acceptors_missing(&cfg(3), Class::HigherRisk, &two).is_some());
        assert!(acceptors_missing(&cfg(1), Class::HigherRisk, &one).is_none());
    }

    #[test]
    fn a_rejected_result_gives_sd930() {
        let mut r = rec(&[("acceptor", "s1", "unknown")]);
        assert!(rejected(&r).is_none());
        r.result = "rejected".to_string();
        assert!(rejected(&r).is_some());
    }

    #[test]
    fn the_message_of_a_missing_record_names_the_shallow_clone_only_when_it_is_one() {
        let plain = missing_message("demo", "`x` is absent", false);
        assert!(plain.contains("accept demo --brief") && !plain.contains("fetch-depth"));
        let shallow = missing_message("demo", "`x` is absent", true);
        assert!(shallow.contains("fetch-depth: 0"));
    }
}

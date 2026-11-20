//! `spec-dna accept <slug>` and `spec-dna accept <slug> --brief` (design 0.8.0,
//! D3 item 5). The first form writes the skeleton of the acceptance record in
//! the folder of the change, or adds one acceptor to the record that is there.
//! The second form writes no file: it prints the brief, one text that is a
//! function of the committed state of the repository.
//!
//! Both forms refuse (exit code 1) while the folder of the change, except
//! `acceptance.toml`, or `spec-dna.toml` has an edit that `HEAD` does not have,
//! in the index or in the working tree. They also refuse while a code path that
//! `scope` matches has such an edit, or is untracked (a deleted file is an edit).
//! The brief and its compare then read the same committed files, and the brief
//! is a function of the committed state. `finish` and `check` read the same
//! files, so they find no stale record for a file that `accept` ignored. The
//! command is for each profile: a project that never runs it sees no change.
//!
//! The command does no check of the person: an agent or a person can run it,
//! and a person can write the same file by hand (ADR 0006).

use crate::change::{self, Change};
use crate::config::Config;
use crate::evidence::{self, Evidence, Proof};
use crate::json::Obj;
use crate::lifecycle::{self, Diff};
use crate::model::{Corpus, Res, canonical};
use crate::record::{self, ProofEntry, Record};
use crate::repo::{self, Fatal, Repo, Snapshot};
use crate::tree;
use crate::{VERSION, report};

/// The options of `accept`.
pub struct Opts<'a> {
    /// `--brief`: print the brief, and write no file.
    pub brief: bool,
    /// `--role`. `None`: the role `acceptor`.
    pub role: Option<&'a str>,
    /// `--session`. `None` or empty: `unknown`.
    pub session: Option<&'a str>,
    /// `--base`.
    pub base: Option<&'a str>,
    pub json: bool,
    pub use_cache: bool,
}

/// The items that the spec and the plan declare for the acceptor.
#[derive(Default)]
pub struct Declared {
    /// The ids of the invariants of the spec (D6, slice S5).
    pub invariants: Vec<String>,
    /// The deviations of the plan: id, kind and text (D7, slice S5).
    pub deviations: Vec<(String, String, String)>,
}

/// The items of `Invariants` and `Deviations`.
///
/// HOOK of slice S5: the sections arrive with D6 and D7. Until then the lists
/// are empty, so the skeleton and the brief have no such entry.
pub fn declared(_cfg: &Config, _ch: &Change, _corpus: &Corpus) -> Declared {
    Declared::default()
}

/// The roles of `[owners]` that match a path of `paths` (D5), sorted.
///
/// HOOK of slice S6: `[owners]` is not a key of the configuration yet, so the
/// list is empty.
fn owner_roles(_cfg: &Config, _paths: &[String]) -> Vec<String> {
    Vec::new()
}

/// The text of the 3 files of a change, as a tree holds them.
pub struct Texts<'a> {
    pub intent: &'a str,
    pub spec: Option<&'a str>,
    pub plan: &'a str,
}

/// The computed fields of a record that the brief names: `scope`, `paths` and
/// `tree` (D3 item 4).
pub struct Composed<'a> {
    pub scope: &'a [String],
    pub paths: &'a [String],
    pub tree: &'a str,
}

/// The brief of a change for a role (D3 item 5): one function of the 3 files, of
/// the computed fields and of what the files declare. `accept --brief` and the
/// compare of SD928 (item 8) both call it, so the text of an entry and the
/// recomputed text are the same function.
pub fn brief_text(
    cfg: &Config,
    slug: &str,
    role: &str,
    texts: &Texts,
    fields: &Composed,
    decl: &Declared,
) -> String {
    let invariants: Vec<String> = decl.invariants.clone();
    let deviations: Vec<String> = decl
        .deviations
        .iter()
        .map(|(id, kind, text)| format!("{id} ({kind}): {text}"))
        .collect();
    let roles = owner_roles(cfg, fields.paths);
    record::brief(&record::Brief {
        slug,
        role,
        intent: texts.intent,
        spec: texts.spec,
        plan: texts.plan,
        scope: fields.scope,
        paths: fields.paths,
        tree: fields.tree,
        invariants: &invariants,
        deviations: &deviations,
        roles: &roles,
    })
}

/// The scope of a change: the patterns of the plan (the items of `Files` and
/// the `Files:` of the tasks, as SD913 reads them) and the globs of
/// `[change] shared-paths`. It is sorted, with no duplicate, so the order of
/// the plan does not change the record.
pub fn scope_of(cfg: &Config, ch: &Change, corpus: &Corpus) -> Vec<String> {
    let mut scope = ch.files(corpus);
    scope.extend(cfg.shared_paths.iter().cloned());
    scope.sort();
    scope.dedup();
    scope
}

/// Runs `accept`. Exit codes: 0 written or printed, 1 a refusal, 2 usage, 3
/// pin or configuration, 4 git or file error, 5 no folder for the slug.
pub fn run(repo: &Repo, slug: &str, o: &Opts) -> Result<u8, Fatal> {
    if !crate::finish::valid_slug(slug) {
        return Err(Fatal::Usage(format!(
            "`{slug}` is not the name of a change; give the folder name of the change, with no `/`"
        )));
    }
    let role = o.role.unwrap_or(record::DEFAULT_ROLE);
    if !record::valid_role(role) {
        return Err(Fatal::Usage(format!(
            "`--role {role}` is not a role name; use lowercase letters, digits and `-`, start with a letter, and write at most 40 characters"
        )));
    }
    let cfg = repo.load_config()?;
    if cfg.pin != VERSION {
        return Err(crate::pin_mismatch(&cfg));
    }
    let intent_dirs = lifecycle::intent_dirs(&cfg);
    if intent_dirs.is_empty() {
        return Err(Fatal::Config(
            "spec-dna.toml has no layer of the type `intent`, so there is no change to accept; see `spec-dna layer add`".into(),
        ));
    }
    let folder = intent_dirs
        .iter()
        .map(|d| format!("{d}/{slug}"))
        .find(|f| repo.root.join(f).is_dir())
        .ok_or_else(|| {
            Fatal::NotFound(format!(
                "no folder `<intent folder>/{slug}` for the change `{slug}`"
            ))
        })?;
    let dirty = refuse_uncommitted(repo, &folder)?;

    // The trees: the working tree, and the lifecycle base if git names it.
    let base_rev = lifecycle::merge_base_of(&repo.git, o.base, cfg.change_base.as_deref())
        .map_err(Fatal::Config)?;
    let head_snap = Snapshot::worktree(repo, &repo::root_paths(&cfg), true)?;
    let base_snap = base_rev
        .as_deref()
        .map(|r| Snapshot::at(repo, r))
        .transpose()?;
    let mut snaps: Vec<&Snapshot> = vec![&head_snap];
    snaps.extend(base_snap.as_ref());
    let mut loaded = repo::load(repo, &cfg, &snaps, o.use_cache, false)?;
    let base = base_snap.is_some().then(|| loaded.corpora.remove(1));
    let mut head = loaded.corpora.remove(0);
    let changes = change::collect(&head);
    let ch = changes.iter().find(|c| c.dir == folder).ok_or_else(|| {
        Fatal::NotFound(format!(
            "the folder `{folder}` has no `intent.md`, `spec.md` or `plan.md`"
        ))
    })?;
    let (Some(plan), Some(intent)) = (&ch.plan, &ch.intent) else {
        let missing = if ch.plan.is_none() {
            "plan.md"
        } else {
            "intent.md"
        };
        return Err(Fatal::Refused(format!(
            "the change `{slug}` has no `{missing}`; `accept` needs the intent and the plan of the change"
        )));
    };
    let path_of = |a: &change::Artifact| head.docs[a.doc as usize].path.clone();
    let (intent_path, plan_path) = (path_of(intent), path_of(plan));
    let spec_path = ch.spec.as_ref().map(path_of);

    // The scope, the paths and the tree (D3 item 4).
    let scope = scope_of(&cfg, ch, &head);
    let is_code = |p: &str| tree::is_code_path(&cfg, p, &intent_dirs);
    refuse_dirty_code(&dirty, &scope, &is_code)?;
    let diff = base_rev
        .as_deref()
        .map(|r| Diff::read(&repo.git, r, false))
        .transpose()
        .map_err(Fatal::Io)?;
    let mut paths = match &diff {
        Some(d) => record::select_paths(
            &scope,
            &d.files.iter().map(|f| f.0.clone()).collect::<Vec<_>>(),
            &is_code,
        ),
        None => Vec::new(),
    };
    if paths.is_empty() {
        // The code merged before this pull request: take the code paths that
        // changed since the commit that added the plan.
        if let Some(added) = repo.git.added_by(&plan_path).map_err(Fatal::Io)? {
            let since = Diff::read(&repo.git, &added, false).map_err(Fatal::Io)?;
            paths = record::select_paths(
                &scope,
                &since.files.iter().map(|f| f.0.clone()).collect::<Vec<_>>(),
                &is_code,
            );
        }
    }
    let entries = tree::entries(repo, &cfg).map_err(Fatal::Io)?;
    let tree_hash = record::tree_hash(&entries, &paths);
    let decl = declared(&cfg, ch, &head);

    if o.brief {
        let read = |p: &str| -> Result<String, Fatal> {
            let got = repo
                .git
                .read_at(&[("HEAD".to_string(), p.to_string())])
                .map_err(Fatal::Io)?;
            match got.into_iter().next().flatten() {
                Some(b) => Ok(String::from_utf8_lossy(&b).into_owned()),
                None => Err(Fatal::Refused(format!(
                    "`{p}` is not in `HEAD`; commit the work, then accept"
                ))),
            }
        };
        let intent_text = read(&intent_path)?;
        let spec_text = spec_path.as_deref().map(read).transpose()?;
        let plan_text = read(&plan_path)?;
        let text = brief_text(
            &cfg,
            slug,
            role,
            &Texts {
                intent: &intent_text,
                spec: spec_text.as_deref(),
                plan: &plan_text,
            },
            &Composed {
                scope: &scope,
                paths: &paths,
                tree: &tree_hash,
            },
            &decl,
        );
        if o.json {
            println!(
                "{}",
                Obj::new()
                    .str("slug", slug)
                    .str("role", role)
                    .str("brief", &record::brief_hash(&text).unwrap_or_default())
                    .str("text", &text)
                    .end()
            );
        } else {
            print!("{text}");
        }
        return Ok(0);
    }

    // The proof states: the reports bind to the tags as `finish` binds them.
    head.covers = evidence::load(repo, &cfg, None, true)?;
    if !cfg.evidence_reports.is_empty()
        && crate::diag::Code::REPORT_CODES
            .iter()
            .chain([&crate::diag::Code::ProofNotReliable])
            .any(|c| !cfg.is_off(*c))
    {
        let reads = report::read(repo, &cfg)?;
        evidence::bind_covers(&repo.root, &mut head.covers, &reads);
    }
    let proofs = proof_entries(&cfg, ch, &head, base.as_ref(), diff.as_ref(), &changes);

    let record_path = format!("{folder}/{}", record::FILE_NAME);
    let full = repo.root.join(&record_path);
    let session = record::session_value(o.session);
    let (text, number, created) = if full.exists() {
        let old =
            std::fs::read_to_string(&full).map_err(|e| Fatal::Io(format!("{record_path}: {e}")))?;
        let r = Record::parse(&old).map_err(|e| {
            Fatal::Refused(format!(
                "`{record_path}` is not a record that `accept` can read: {e}; fix it, or delete it and accept again"
            ))
        })?;
        if r.change != slug {
            return Err(Fatal::Refused(format!(
                "`{record_path}` is the record of the change `{}`, not of `{slug}`; delete it and accept again",
                r.change
            )));
        }
        if r.scope != scope || r.paths != paths || !tree::matches(&r.tree, &tree_hash) {
            return Err(Fatal::Refused(format!(
                "the code that `{record_path}` lists changed since the record: its `scope`, `paths` or `tree` differ from the repository now; delete the record and accept again"
            )));
        }
        let n = r.acceptors.len() + 1;
        let text = record::add_acceptor(&old, role, &session);
        // The new text must read, and it must have one more entry. A text that
        // `add_acceptor` cannot place (a `[` line in a string) is a refusal.
        if Record::parse(&text).is_err_and(|_| true)
            || Record::parse(&text).is_ok_and(|t| t.acceptors.len() != n)
        {
            return Err(Fatal::Refused(format!(
                "`accept` cannot add an `[[acceptor]]` entry to `{record_path}` and keep it a valid TOML text; add the entry by hand, or delete the record and accept again"
            )));
        }
        (text, n, false)
    } else {
        let text = record::skeleton(&record::Skeleton {
            slug,
            scope: &scope,
            paths: &paths,
            tree: &tree_hash,
            role,
            session: &session,
            proofs: &proofs,
            invariants: &decl.invariants,
            deviations: &decl.deviations,
        });
        (text, 1, true)
    };
    write_atomic(&full, &text).map_err(|e| Fatal::Io(format!("{record_path}: {e}")))?;
    if o.json {
        println!(
            "{}",
            Obj::new()
                .str("slug", slug)
                .str("path", &record_path)
                .num("acceptor", number)
                .str("role", role)
                .str("session", &session)
                .bool("created", created)
                .end()
        );
    } else {
        println!("wrote {record_path}");
        println!("acceptor: {number}");
        println!(
            "next: copy the hash of the last line of your brief into `brief` of your entry, replace each `pending` value, then commit the record alone"
        );
    }
    Ok(0)
}

/// Writes a file through a temporary file and a rename, so a reader never sees
/// a half file.
fn write_atomic(path: &std::path::Path, text: &str) -> std::io::Result<()> {
    let tmp = path.with_extension("toml.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&tmp);
    })
}

/// Refuses while the folder of the change (except the record) or
/// `spec-dna.toml` has an edit that `HEAD` does not have, in the index or in
/// the working tree. An untracked file of the folder counts as an edit. It
/// returns each path that has such an edit, or is untracked, so that the caller
/// can read the code paths among them ([`refuse_dirty_code`]).
fn refuse_uncommitted(repo: &Repo, folder: &str) -> Result<Vec<String>, Fatal> {
    if repo.head_commit().is_none() {
        return Err(Fatal::Refused(
            "the repository has no commit; commit the work, then accept".to_string(),
        ));
    }
    let prefix = format!("{folder}/");
    let record = format!("{prefix}{}", record::FILE_NAME);
    let mut all: Vec<String> = Vec::new();
    let mut dirty: Vec<String> = Vec::new();
    let worktree = repo.git.diff_paths("HEAD", true).map_err(Fatal::Io)?;
    let index = repo
        .git
        .staged()
        .diff_paths("HEAD", false)
        .map_err(Fatal::Io)?;
    for (p, _) in worktree.into_iter().chain(index) {
        let counts = (p.starts_with(&prefix) && p != record) || p == crate::config::FILE_NAME;
        if counts && !dirty.contains(&p) {
            dirty.push(p.clone());
        }
        if !all.contains(&p) {
            all.push(p);
        }
    }
    if dirty.is_empty() {
        return Ok(all);
    }
    Err(Fatal::Refused(format!(
        "{} has an edit that HEAD does not have: {}; commit the work, then accept",
        if dirty.iter().all(|p| p == crate::config::FILE_NAME) {
            "`spec-dna.toml`".to_string()
        } else {
            format!("the folder `{folder}` or `spec-dna.toml`")
        },
        dirty
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Refuses while a code path that `scope` matches is in `dirty`: it has an edit
/// that `HEAD` does not have, or it is untracked. The hash of `paths` reads
/// the files of the working tree, and the compare of `finish` and `check`
/// reads the committed files, so the 2 must not differ.
fn refuse_dirty_code(
    dirty: &[String],
    scope: &[String],
    is_code: &dyn Fn(&str) -> bool,
) -> Result<(), Fatal> {
    let found = record::select_paths(scope, dirty, is_code);
    if found.is_empty() {
        return Ok(());
    }
    Err(Fatal::Refused(format!(
        "the code {} that `scope` matches {} an edit that HEAD does not have, or {} untracked: {}; commit or remove {}, then accept",
        if found.len() == 1 { "path" } else { "paths" },
        if found.len() == 1 { "has" } else { "have" },
        if found.len() == 1 { "is" } else { "are" },
        found
            .iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", "),
        if found.len() == 1 { "it" } else { "them" },
    )))
}

/// A requirement id in a form that two spellings of one document share.
fn same_target(c: &Corpus, target: &str) -> String {
    let Some((id, key)) = target.split_once('#') else {
        return canonical(target);
    };
    match c.resolve(id) {
        Res::Found(d, None, _) => format!("{}#{key}", c.docs[d as usize].id),
        _ => format!("{}#{key}", canonical(id)),
    }
}

/// One `[[proof]]` entry for each requirement of the set of SD916: with a delta,
/// the targets of its `ADD` and `CHANGE` lines; with no delta, the requirements
/// whose hash differs from the base in a document that the scope accepts.
///
/// `finish` applies the delta after the acceptance, so a target of an `ADD`
/// line is often not in the tree yet. Its state then comes from the tags of the
/// test files that name it: a tag that a bound case proves is `passed-once` or
/// `reliable`, a tag with no proof is `tagged`, and no tag is `none`. The
/// `verified-by` and the debt of a requirement that is not in the tree cannot be
/// read, so they give no state.
fn proof_entries(
    cfg: &Config,
    ch: &Change,
    head: &Corpus,
    base: Option<&Corpus>,
    diff: Option<&Diff>,
    changes: &[Change],
) -> Vec<ProofEntry> {
    let runs = cfg.evidence_reliable_runs as usize;
    let (ev, _) = Evidence::build(head);
    // (document id, key) of each requirement of the set.
    let mut wanted: Vec<(String, String)> = Vec::new();
    let lines = ch.delta(head);
    if lines.is_empty() {
        if let Some(base) = base {
            let files = ch.files(head);
            let others_touched = changes
                .iter()
                .filter(|c| c.dir != ch.dir)
                .any(|c| diff.is_some_and(|d| d.under(&c.dir).next().is_some()));
            for r in lifecycle::requirement_changes(base, head, lifecycle::requirement_doc) {
                if r.kind != lifecycle::ReqChangeKind::Removed
                    && (!others_touched || change::in_scope(&files, &r.path))
                {
                    wanted.push((r.doc, r.key));
                }
            }
        }
    } else {
        for line in &lines {
            if let Ok(d) = &line.parsed
                && d.verb != change::Verb::Remove
            {
                wanted.push((d.id.clone(), d.key.clone()));
            }
        }
    }
    let mut out: Vec<ProofEntry> = Vec::new();
    for (id, key) in wanted {
        let target = format!("{id}#{key}");
        let found = match head.resolve(&id) {
            Res::Found(di, None, _) => head.docs[di as usize].req(&key).map(|ri| (di, ri)),
            _ => None,
        };
        let (name, state, mut cases) = match found {
            Some((di, ri)) => {
                let doc = &head.docs[di as usize];
                let mut cases: Vec<String> = Vec::new();
                for (_, p) in ev.proofs.get(&(di, ri)).into_iter().flatten() {
                    if let Proof::Passed(n, _) = p {
                        cases.push(n.clone());
                    }
                }
                (
                    format!("{}#{key}", doc.id),
                    ev.state(di, doc, ri, runs),
                    cases,
                )
            }
            None => {
                let want = same_target(head, &target);
                let mut tagged = false;
                let mut proofs: Vec<Proof> = Vec::new();
                for cv in &head.covers {
                    if !cv.partial && same_target(head, &cv.target) == want {
                        tagged = true;
                        proofs.extend(cv.proof.clone());
                    }
                }
                let cases = proofs
                    .iter()
                    .filter_map(|p| match p {
                        Proof::Passed(n, _) => Some(n.clone()),
                        _ => None,
                    })
                    .collect();
                (
                    target,
                    evidence::state_of(false, tagged, false, &proofs, runs),
                    cases,
                )
            }
        };
        cases.sort();
        cases.dedup();
        out.push(ProofEntry {
            requirement: name,
            state: state.name().to_string(),
            cases,
        });
    }
    out.sort_by(|a, b| a.requirement.cmp(&b.requirement));
    out.dedup_by(|a, b| a.requirement == b.requirement);
    out
}

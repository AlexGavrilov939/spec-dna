//! `spec-dna accept <slug>` and `spec-dna accept <slug> --brief` (design 0.8.0,
//! D3 item 5). The first form writes the skeleton of the acceptance record in
//! the folder of the change, or adds one acceptor to the record that is there.
//! The second form writes no file: it prints the brief, one text that is a
//! function of the committed state of the repository.
//!
//! Both forms refuse (exit code 1) while an edit that `HEAD` does not have is in
//! the index or in the working tree. The edit can be in the folder of the change,
//! except `acceptance.toml`, or in `spec-dna.toml`. They also refuse while a code
//! path that `scope` matches has such an edit, or is untracked (a deleted file is
//! an edit).
//! The brief and its compare then read the same committed files, and the brief
//! is a function of the committed state. `finish` and `check` read the same
//! files, so they find no stale record for a file that `accept` ignored. The
//! command works in each profile: a project that never runs it sees no change.
//!
//! The command does no check of the person: an agent or a person can run it,
//! and a person can write the same file by hand (ADR 0006).

use crate::change::{self, Change};
use crate::config::Config;
use crate::evidence::{self, Evidence, Proof};
use crate::json::Obj;
use crate::lifecycle::{self, Diff};
use crate::model::{Corpus, Res};
use crate::record::{self, ProofEntry, Record};
use crate::repo::{self, Fatal, Repo, Snapshot};
use crate::tree;
use crate::{VERSION, report};

/// The options of `accept`.
pub struct Opts<'a> {
    /// `--brief`: print the brief, and write no file.
    pub brief: bool,
    /// `--follow-up`: the follow-up of a note (D73).
    pub follow_up: bool,
    /// `--role`, once or more than once. Empty: the roles of `[owners]` that
    /// match a role path, or `acceptor` when none matches (D72 item 4).
    pub roles: &'a [String],
    /// `--session`. `None` or empty: `unknown`.
    pub session: Option<&'a str>,
    /// `--base`.
    pub base: Option<&'a str>,
    pub json: bool,
    pub use_cache: bool,
}

/// The items that the spec and the plan declare for the acceptor.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Declared {
    /// The pins of the invariants of the spec, as `<doc-id>#<key>@<hash>` (D6).
    /// With no `[change] invariants`, the list is empty.
    pub invariants: Vec<String>,
    /// The deviations of the plan: id, kind and text (D7). The text has each run
    /// of white space as one space.
    pub deviations: Vec<(String, String, String)>,
}

/// The items of `Invariants` and `Deviations` that the files of a change declare
/// (design 0.8.0, D6 and D7). The function reads only the text of the spec and
/// the plan and the key `[change] invariants`, with no git data and no other
/// document. It applies no rule of the pin. Only
/// [`crate::acceptance::declared_for`] calls it. That function applies the
/// rule, so `accept`, the brief, the compare of the brief of SD928, the answers
/// (SD938) and `finish` read the same items. An item that is not valid gives no
/// entry: SD936 and SD939 report it.
pub fn declared(cfg: &Config, ch: &Change, corpus: &Corpus) -> Declared {
    Declared {
        invariants: if cfg.change_invariants {
            ch.invariants(corpus).iter().map(|p| p.text()).collect()
        } else {
            Vec::new()
        },
        deviations: ch
            .deviations(corpus)
            .into_iter()
            .map(|d| (d.id, d.kind, d.text))
            .collect(),
    }
}

/// The roles of the entry: the roles that `--role` names, sorted with no
/// duplicate, else each role of `[owners]` that matches a role path (D72 item
/// 4), else `acceptor`. `paths` is the list that the matched roles read.
fn entry_roles(cfg: &Config, given: &[String], paths: &[String]) -> Vec<String> {
    let mut roles: Vec<String> = if given.is_empty() {
        crate::acceptance::matched_roles(cfg, paths)
    } else {
        given.to_vec()
    };
    roles.sort();
    roles.dedup();
    if roles.is_empty() {
        roles.push(record::DEFAULT_ROLE.to_string());
    }
    roles
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
///
/// `format` is the number of the format of the brief: `record::BRIEF_FORMAT` for
/// a record of format 2, and `record::BRIEF_FORMAT_1` for an entry of a record
/// of format 1. `entry_roles` are the roles of the entry, sorted.
#[allow(clippy::too_many_arguments)]
pub fn brief_text(
    cfg: &Config,
    slug: &str,
    format: u32,
    entry_roles: &[String],
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
    let roles = crate::acceptance::matched_roles(cfg, fields.paths);
    record::brief(&record::Brief {
        slug,
        format,
        entry_roles,
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

/// The scope of a change: the patterns of the plan and the globs of
/// `[change] shared-paths`. [`Change::scope_patterns`] gives the patterns: the
/// items of `Files` and the `Files:` of the tasks, as SD913 reads them, without
/// a word or a bare name. The scope is sorted, with no duplicate, so the order
/// of the plan does not change the record. The scope is a function of the plan
/// text alone: no file on the disk changes it. A `shared-paths` glob always
/// stays.
pub fn scope_of(cfg: &Config, ch: &Change, corpus: &Corpus) -> Vec<String> {
    let mut scope = ch.scope_patterns(corpus);
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
    if let Some(role) = o.roles.iter().find(|r| !record::valid_role(r)) {
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
    let mut changed: Vec<String> = diff
        .as_ref()
        .map(|d| d.files.iter().map(|f| f.0.clone()).collect())
        .unwrap_or_default();
    let mut paths = record::select_paths(&scope, &changed, &is_code);
    if paths.is_empty() {
        // The code merged before this pull request: take the code paths that
        // changed since the commit that added the plan.
        if let Some(added) = repo.git.added_by(&plan_path).map_err(Fatal::Io)? {
            let since = Diff::read(&repo.git, &added, false).map_err(Fatal::Io)?;
            changed = since.files.iter().map(|f| f.0.clone()).collect();
            paths = record::select_paths(&scope, &changed, &is_code);
        }
    }
    refuse_uncovered(ch, &head, &scope, &changed, &is_code)?;
    if o.follow_up {
        let ctx = FollowCtx {
            repo,
            cfg: &cfg,
            slug,
            folder: &folder,
            base_rev: base_rev.as_deref(),
            scope: &scope,
            changed: &changed,
            is_code: &is_code,
        };
        return run_follow_up(&ctx, o);
    }
    let entries = tree::entries(repo, &cfg).map_err(Fatal::Io)?;
    let tree_hash = record::tree_hash(&entries, &paths);
    let decl = crate::acceptance::declared_for(&cfg, &repo.git, base_rev.as_deref(), ch, &head)
        .map_err(Fatal::Io)?;
    let roles = entry_roles(&cfg, o.roles, &paths);

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
            record::BRIEF_FORMAT,
            &roles,
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
                    .strs("roles", &roles)
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
    // A link is no record, and `accept` never reads through it: the check
    // looks at the path itself, and the read has a size limit.
    let existing = match crate::git::read_limited(&full, crate::acceptance::READ_LIMIT) {
        crate::git::Limited::Absent => None,
        crate::git::Limited::Bytes(b) => Some(String::from_utf8_lossy(&b).into_owned()),
        other => {
            return Err(Fatal::Refused(crate::acceptance::unreadable_message(
                &record_path,
                &other,
            )));
        }
    };
    let (text, number, created) = if let Some(old) = existing {
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
        // A new entry has the roles of format 2, so it cannot join a record of the
        // pre-release (D72 item 3).
        if r.format != record::FORMAT {
            return Err(Fatal::Refused(format!(
                "`{record_path}` has `format = {}`, and `accept` writes `format = {}`; {}",
                r.format,
                record::FORMAT,
                record::FORMAT_1_MESSAGE
            )));
        }
        // The function that SD929, `next` and `finish` use decides first, so
        // no 2 places use 2 rules. The compare of `paths` stays: `paths` also
        // differs when a path left the diff.
        let plan = crate::acceptance::PlanScope::of(ch, &head);
        let why = crate::acceptance::stale_reasons(
            &cfg,
            &repo.git,
            &r,
            diff.as_ref(),
            &plan,
            base_rev.as_deref(),
        )
        .map_err(Fatal::Io)?;
        if !why.is_empty() || r.paths != paths {
            let why = if why.is_empty() {
                "its `scope`, `paths` or `tree` differ from the repository now".to_string()
            } else {
                why.join("; ")
            };
            return Err(Fatal::Refused(format!(
                "the code that `{record_path}` lists changed since the record: {why}; delete the record and accept again"
            )));
        }
        if !tree::matches(r.last_tree(), &tree_hash) {
            return Err(Fatal::Refused(format!(
                "the code that `{record_path}` lists changed since the record: its `tree` differs from the repository now; delete the record and accept again"
            )));
        }
        let n = r.acceptors.len() + 1;
        let text = record::add_acceptor(&old, &roles, &session);
        // The reader of a record refuses a text over the limit, so the size check
        // comes before the check of the new entry.
        if text.len() > record::MAX_BYTES {
            return Err(Fatal::Refused(oversize_message(
                &record_path,
                text.len(),
                record::text_parts(&text),
                (paths.len(), decl.invariants.len(), decl.deviations.len()),
            )));
        }
        // The new text must read, and the new entry must be the last acceptor,
        // with the printed number as its position. The entries before it stay.
        // A text that `add_acceptor` cannot place (a line of a multi-line value
        // that looks like a table header) is a refusal.
        let placed = Record::parse(&text).is_ok_and(|t| {
            t.acceptors.len() == n
                && t.acceptors[..n - 1] == r.acceptors[..]
                && t.acceptors[n - 1].roles == roles
                && t.acceptors[n - 1].session == session
        });
        if !placed {
            return Err(Fatal::Refused(format!(
                "`accept` cannot add an `[[acceptor]]` entry to `{record_path}` as entry {n}; add the entry by hand after the last `[[acceptor]]` table, or delete the record and accept again"
            )));
        }
        (text, n, false)
    } else {
        let text = record::skeleton(&record::Skeleton {
            slug,
            scope: &scope,
            paths: &paths,
            tree: &tree_hash,
            roles: &roles,
            session: &session,
            proofs: &proofs,
            invariants: &decl.invariants,
            deviations: &decl.deviations,
        });
        (text, 1, true)
    };
    // A record over the size limit is never valid, and the acceptor must not
    // shorten `paths`, the invariants or the deviations. Refuse before any write.
    if text.len() > record::MAX_BYTES {
        return Err(Fatal::Refused(oversize_message(
            &record_path,
            text.len(),
            record::text_parts(&text),
            (paths.len(), decl.invariants.len(), decl.deviations.len()),
        )));
    }
    write_atomic(&full, &text).map_err(|e| Fatal::Io(format!("{record_path}: {e}")))?;
    if o.json {
        println!(
            "{}",
            Obj::new()
                .str("slug", slug)
                .str("path", &record_path)
                .num("acceptor", number)
                .strs("roles", &roles)
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

/// What `accept --follow-up` reads from the run of `accept`.
struct FollowCtx<'a> {
    repo: &'a Repo,
    cfg: &'a Config,
    slug: &'a str,
    folder: &'a str,
    base_rev: Option<&'a str>,
    scope: &'a [String],
    changed: &'a [String],
    is_code: &'a dyn Fn(&str) -> bool,
}

/// `accept <slug> --follow-up [--brief]` (design 0.8.0, D73). The brief form prints
/// the follow-up brief, and the other form adds a `[[follow-up]]` entry to the
/// record. Both refuse (exit code 1), with a message that names a full run, when
/// the fix is not a fix that a follow-up can check.
fn run_follow_up(c: &FollowCtx, o: &Opts) -> Result<u8, Fatal> {
    use crate::acceptance::{Chain, Source};
    let full = |why: String| {
        Fatal::Refused(format!(
            "{why}; a follow-up cannot check this: delete the record, commit, and accept again with a full run (`spec-dna accept {} --brief`)",
            c.slug
        ))
    };
    let Some(base_rev) = c.base_rev else {
        return Err(Fatal::Refused(
            "no lifecycle base is known, so the fix cannot be compared with the record; give `--base`".into(),
        ));
    };
    let record_path = format!("{}/{}", c.folder, record::FILE_NAME);
    let text = match crate::git::read_limited(
        &c.repo.root.join(&record_path),
        crate::acceptance::READ_LIMIT,
    ) {
        crate::git::Limited::Bytes(b) => String::from_utf8_lossy(&b).into_owned(),
        crate::git::Limited::Absent => {
            return Err(full(format!("`{record_path}` does not exist")));
        }
        other => {
            return Err(Fatal::Refused(crate::acceptance::unreadable_message(
                &record_path,
                &other,
            )));
        }
    };
    let rec =
        Record::parse(&text).map_err(|e| full(format!("`{record_path}` does not read ({e})")))?;
    if rec.change != c.slug || rec.format != record::FORMAT {
        return Err(full(format!(
            "`{record_path}` is not a record of format {} for `{}`",
            record::FORMAT,
            c.slug
        )));
    }
    if rec.result == "rejected" {
        return Err(full(format!("`{record_path}` says `rejected`")));
    }
    // A follow-up that the agent has not finished (its `brief` is `pending`) is not
    // a link. The brief ignores it; a new entry needs it finished.
    let unfinished = rec
        .follow_ups
        .last()
        .is_some_and(|u| u.brief == record::PENDING);
    let m = rec.follow_ups.len() - usize::from(unfinished);
    if unfinished && !o.brief {
        return Err(Fatal::Refused(format!(
            "`{record_path}` has a follow-up with the value `pending`; fill it in and commit it, or delete the entry"
        )));
    }
    let open = rec.open_notes(m);
    if open.is_empty() {
        return Err(full(format!("`{record_path}` has no open note")));
    }
    // A new code path in scope: a follow-up binds the same `paths` as the record.
    for p in record::select_paths(c.scope, c.changed, c.is_code) {
        if !rec.paths.contains(&p) {
            return Err(full(format!(
                "the diff of the change has the code path `{p}`, which `scope` matches and `paths` does not list"
            )));
        }
    }
    let chain =
        Chain::load(c.cfg, &c.repo.git, base_rev, c.folder, &rec).map_err(Fatal::Refused)?;
    let positions = chain.follow_positions();
    let (last_pos, last_tree) = if m == 0 {
        (chain.record_pos(), rec.tree.clone())
    } else {
        (positions[m - 1], rec.follow_ups[m - 1].tree.clone())
    };
    let from = chain.source(last_pos);
    if from == Source::Head {
        return Err(Fatal::Refused(format!(
            "the last link of the chain is not committed; commit `{record_path}`, then run the follow-up"
        )));
    }
    let fix = chain.fix(&from, &Source::Head).map_err(Fatal::Io)?;
    if fix.paths.is_empty() {
        return Err(full(
            "the diff of the fix is empty: no code path of `paths` changed since the last link"
                .into(),
        ));
    }
    let open_paths: Vec<&str> = open
        .iter()
        .map(|n| rec.findings[n - 1].path.as_str())
        .collect();
    if !fix.paths.iter().any(|p| open_paths.contains(&p.as_str())) {
        return Err(full(
            "the diff of the fix changes no path of an open note".into(),
        ));
    }
    if let Some(p) = fix.binary.first() {
        return Err(full(format!(
            "the diff of the fix has the binary file `{p}`, and the budget measures text"
        )));
    }
    let len = chain.chain_diff_len(&Source::Head).map_err(Fatal::Io)?;
    if len > record::MAX_CHAIN_DIFF {
        return Err(full(format!(
            "the diff of the chain has {len} bytes, more than {} bytes for the whole chain",
            record::MAX_CHAIN_DIFF
        )));
    }
    let brief = chain
        .brief(m, &last_tree, &Source::Head, &fix)
        .map_err(Fatal::Io)?;
    if o.brief {
        if o.json {
            println!(
                "{}",
                Obj::new()
                    .str("slug", c.slug)
                    .str(
                        "brief",
                        &record::follow_up_brief_hash(&brief).unwrap_or_default()
                    )
                    .str("text", &brief)
                    .end()
            );
        } else {
            print!("{brief}");
        }
        return Ok(0);
    }
    let session = record::session_value(o.session);
    let new = record::add_follow_up(&text, &session, &fix.to_tree, &open);
    if new.len() > record::MAX_BYTES {
        return Err(Fatal::Refused(format!(
            "the record `{record_path}` would have {} bytes, and a record has at most {} bytes",
            new.len(),
            record::MAX_BYTES
        )));
    }
    write_atomic(&c.repo.root.join(&record_path), &new)
        .map_err(|e| Fatal::Io(format!("{record_path}: {e}")))?;
    if o.json {
        println!(
            "{}",
            Obj::new()
                .str("slug", c.slug)
                .str("path", &record_path)
                .num("follow_up", m + 1)
                .str("session", &session)
                .end()
        );
    } else {
        println!("wrote {record_path}");
        println!("follow-up: {}", m + 1);
        println!(
            "next: copy the hash of the last line of the follow-up brief into `brief` of the entry, keep in `closes` each note that the diff fixes, write `result` and `text`, then commit the record alone"
        );
    }
    Ok(0)
}

/// The refusal for a record over [`record::MAX_BYTES`]. It names the largest part
/// of the text that `accept` would write: the line `paths`, the invariants, the
/// deviations, or all other tables, with its size, and the fix for that part.
/// `counts` is the number of paths, of invariants and of deviations.
fn oversize_message(
    record_path: &str,
    bytes: usize,
    parts: record::TextParts,
    counts: (usize, usize, usize),
) -> String {
    let head = format!(
        "the record `{record_path}` would have {bytes} bytes, and a record has at most {} bytes",
        record::MAX_BYTES
    );
    let all = [
        ("paths", parts.paths),
        ("invariants", parts.invariants),
        ("deviations", parts.deviations),
        ("other", parts.other),
    ];
    let (name, size) = all.into_iter().max_by_key(|p| p.1).unwrap_or(("paths", 0));
    let fix = match name {
        "invariants" => format!(
            "the cause is the invariants: `Invariants` of the spec lists {} items, and the answers are in the entries. List fewer rules, shorten the answers, or split the change into smaller changes, each with its own folder",
            counts.1
        ),
        "deviations" => format!(
            "the cause is the deviations: `Deviations` of the plan lists {} items. Shorten the text of each item, list fewer items, or split the change into smaller changes, each with its own folder",
            counts.2
        ),
        "other" => "the cause is the other tables: the entries of the acceptors, the attempts, the findings and the proof entries. Shorten the text of the attempts and the findings, or split the change into smaller changes, each with its own folder".to_string(),
        _ => format!(
            "the cause is too many paths: `paths` lists {} code paths. Split the change into smaller changes, each with its own folder, and accept each one",
            counts.0
        ),
    };
    let named = match name {
        "other" => "the other tables".to_string(),
        n => n.to_string(),
    };
    format!("{head}; the largest part is {named} ({size} bytes); {fix}")
}

/// The words that tell the user how to write a `Files` pattern that the scope of
/// a record cannot hold ([`uncovered_path`]).
pub fn scope_way(p: &str) -> String {
    if p.trim().contains(char::is_whitespace) {
        let glob = p.split_whitespace().collect::<Vec<_>>().join("*");
        format!(
            "a path of `Files` has no space; write the glob `{glob}` instead (a `*` matches the space), or add the path to `[change] shared-paths`"
        )
    } else {
        "write the path with a `/` or a `.` in `Files` of the plan (the form `./Makefile` works), or add it to `[change] shared-paths`".to_string()
    }
}

/// The cause of a pattern that the scope cannot hold.
pub fn scope_cause(p: &str) -> &'static str {
    if p.trim().contains(char::is_whitespace) {
        "has a space"
    } else {
        "has no `/`, no `.` and no `*`"
    }
}

/// The patterns of `files` (every code span of `Files`, [`Change::files`]) that
/// the scope of a record cannot hold: a word, a bare name such as `Makefile`,
/// or a path with a space ([`change::is_scope_pattern`]).
pub fn dropped_patterns(files: &[String]) -> Vec<String> {
    files
        .iter()
        .filter(|p| !change::is_scope_pattern(p))
        .cloned()
        .collect()
}

/// The first changed code path that `Files` covers only through a pattern that
/// the scope of a record cannot hold, and that `scope` does not cover. It
/// returns the path and that pattern. This is the one rule of the refusal of
/// `accept` and of SD929, so a second place cannot let such a path pass.
pub fn uncovered_path(
    files: &[String],
    scope: &[String],
    changed: &[String],
    is_code: &dyn Fn(&str) -> bool,
) -> Option<(String, String)> {
    let dropped = dropped_patterns(files);
    let p = changed
        .iter()
        .find(|p| is_code(p) && change::in_scope(&dropped, p) && !change::in_scope(scope, p))?;
    // The dropped pattern that covers the file names the cause.
    let pattern = dropped
        .iter()
        .find(|d| change::in_scope(std::slice::from_ref(d), p))
        .cloned()
        .unwrap_or_default();
    Some((p.clone(), pattern))
}

/// Refuses when the plan lists a pattern that the scope of a record cannot
/// hold, and the record would then leave a changed code file out. The patterns
/// are the words and the bare names of `Files` (`Makefile`): the scope is a
/// function of the plan text, and it keeps only a pattern with a `/`, a `.` or
/// a `*` ([`change::is_scope_pattern`]). The acceptor cannot fix such a record:
/// it has an empty `scope` or a code file that no pattern covers.
fn refuse_uncovered(
    ch: &Change,
    corpus: &Corpus,
    scope: &[String],
    changed: &[String],
    is_code: &dyn Fn(&str) -> bool,
) -> Result<(), Fatal> {
    let files = ch.files(corpus);
    let dropped = dropped_patterns(&files);
    if dropped.is_empty() {
        return Ok(());
    }
    if scope.is_empty() {
        return Err(Fatal::Refused(format!(
            "the scope of the record is empty: the patterns of `Files` of the plan cannot be in the record ({}); {}",
            dropped
                .iter()
                .map(|p| format!("`{p}` {}", scope_cause(p)))
                .collect::<Vec<_>>()
                .join(", "),
            scope_way(&dropped[0])
        )));
    }
    if let Some((p, pattern)) = uncovered_path(&files, scope, changed, is_code) {
        return Err(Fatal::Refused(format!(
            "the changed file `{p}` is in `Files` of the plan only as `{pattern}`, which {}, so the scope of the record does not cover it; {}",
            scope_cause(&pattern),
            scope_way(&pattern)
        )));
    }
    Ok(())
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
/// returns each path that has such an edit, or is untracked. The caller reads
/// the code paths among them ([`refuse_dirty_code`]).
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
                let want = change::same_target(head, &target);
                let mut tagged = false;
                let mut proofs: Vec<Proof> = Vec::new();
                for cv in &head.covers {
                    if !cv.partial && change::same_target(head, &cv.target) == want {
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

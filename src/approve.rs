//! `spec-dna approve <slug> intent|spec|plan|acceptance` (owner review of 0.7.0,
//! item 2, and design 0.8.0, D3 item 10). It is a command for a person. It sets
//! the `status` line of one artifact of a change and commits that file alone, so
//! the commit is an approval commit of the branch rule (ADR 0012, D11): the file
//! is the same as in HEAD except the `status` line. With `acceptance`, it adds the
//! e-mail of its author to `acknowledged-by` of the acceptance record and commits
//! that file alone (`approve_acceptance`).
//!
//! The command makes no check of the person: an agent can run it too. The
//! hook `protect-approvals` of the harness denies a call of an agent, and the
//! code host stays the control (ADR 0018).

use crate::change::{self, Artifact, Change};
use crate::config::{Approval, Config};
use crate::json::Obj;
use crate::lifecycle;
use crate::model::Class;
use crate::repo::{self, Fatal, Repo, Snapshot};
use crate::{VERSION, frontmatter};

/// The artifacts that a person approves, with their approval value.
const ARTIFACTS: [(&str, &str); 3] = [
    ("intent", "accepted"),
    ("spec", "approved"),
    ("plan", "approved"),
];

/// The approval value of an artifact name, or `None` for another name.
fn approval_value(artifact: &str) -> Option<&'static str> {
    ARTIFACTS.iter().find(|a| a.0 == artifact).map(|a| a.1)
}

/// Runs `approve`. Exit codes: 0 committed or already approved, 1 a refusal,
/// 2 usage, 3 configuration, 4 git or file error, 5 no folder or no file.
pub fn run(repo: &Repo, slug: &str, artifact: &str, json: bool) -> Result<u8, Fatal> {
    if artifact == "acceptance" {
        return approve_acceptance(repo, slug, json);
    }
    let Some(approved) = approval_value(artifact) else {
        return Err(Fatal::Usage(format!(
            "`{artifact}` is not an artifact; use intent, spec, plan or acceptance"
        )));
    };
    if !crate::finish::valid_slug(slug) {
        return Err(Fatal::Usage(format!(
            "`{slug}` is not the name of a change; give the folder name of the change, with no `/`"
        )));
    }
    let cfg = repo.load_config()?;
    if cfg.pin != VERSION {
        return Err(crate::pin_mismatch(&cfg));
    }
    if lifecycle::intent_dirs(&cfg).is_empty() {
        return Err(Fatal::Config(
            "spec-dna.toml has no layer of the type `intent`, so there is no change to approve; see `spec-dna layer add`".into(),
        ));
    }
    let change = find_change(repo, &cfg, slug)?;
    let ty = match artifact {
        "intent" => change.intent.as_ref(),
        "spec" => change.spec.as_ref(),
        _ => change.plan.as_ref(),
    };
    let (path, status) = match ty {
        Some(a) => (a.path.clone(), a.status.clone()),
        None => {
            return Err(Fatal::NotFound(format!(
                "the change `{slug}` has no `{artifact}.md` in `{}`",
                change.dir
            )));
        }
    };
    // Step 3: the file is tracked and has the text of HEAD.
    let clean = repo.git.is_clean_in_head(&path).map_err(Fatal::Io)?;
    if !clean {
        return Err(Fatal::Refused(format!(
            "commit the text of `{path}` first; an approval commit changes the `status` line alone"
        )));
    }
    let author = repo.git.author_email().map_err(Fatal::Io)?;
    // Step 4: the status.
    if status == approved {
        let line = format!("`{path}` is already `{status}`");
        if json {
            println!(
                "{}",
                result_json(slug, artifact, &path, approved, None, &author)
            );
        } else {
            println!("{line}");
        }
        return Ok(0);
    }
    if status != "draft" {
        return Err(Fatal::Refused(format!(
            "`{path}` has the status `{}`; only a `draft` artifact moves to `{approved}`",
            if status.is_empty() { "(none)" } else { &status }
        )));
    }
    // Step 5: the class.
    if artifact == "intent" && change.class_unset {
        return Err(Fatal::Refused(format!(
            "`{path}` has no valid `class` (routine or higher-risk); set the class in a commit of its own first"
        )));
    }
    // Step 6: the earlier artifact.
    check_earlier(&change, artifact)?;
    // Step 7: the author counts.
    if cfg.change_approval == Approval::Branch
        && !cfg
            .change_approvers
            .iter()
            .any(|a| a.eq_ignore_ascii_case(&author))
    {
        return Err(Fatal::Refused(format!(
            "`{author}` is not in `[change] approvers`, so the commit gives no approval; set `user.email` or `GIT_AUTHOR_EMAIL` to an approver"
        )));
    }
    // Step 8: the new text. Only the value of the `status` line changes.
    let full = repo.root.join(&path);
    let old = std::fs::read_to_string(&full).map_err(|e| Fatal::Io(format!("{path}: {e}")))?;
    let new = frontmatter::set_field(&old, "status", approved).ok_or_else(|| {
        Fatal::Refused(format!(
            "`{path}` has no `status` line in its front matter that the command can edit"
        ))
    })?;
    std::fs::write(&full, &new).map_err(|e| Fatal::Io(format!("{path}: {e}")))?;
    // Step 9 and 10: the commit. A failure puts the old text back.
    let message = format!("docs: approve the {artifact} of {slug}");
    let commit = match repo.git.commit_only(&message, &path) {
        Ok(c) => c,
        Err(e) => {
            let _ = std::fs::write(&full, &old);
            return Err(Fatal::Io(format!("git commit failed: {e}")));
        }
    };
    if json {
        println!(
            "{}",
            result_json(slug, artifact, &path, approved, Some(&commit), &author)
        );
    } else {
        println!(
            "approved {path} in {} (author {author})",
            commit.get(..12).unwrap_or(&commit)
        );
    }
    Ok(0)
}

/// `approve <slug> acceptance` (design 0.8.0, D3 item 10): with
/// `[change] approval = "branch"`, adds the e-mail of the author to
/// `acknowledged-by` of the record and commits the record alone. An approver
/// acknowledges the text of the record at that commit. An entry counts only when
/// its approval commit changes nothing else and no later commit changes the record
/// (SD932, SD934). The walk of the commits finds the entries that no longer count: the
/// command removes them in a commit of its own first, so the approver acknowledges
/// the new text. With the author already counted, no commit is made.
fn approve_acceptance(repo: &Repo, slug: &str, json: bool) -> Result<u8, Fatal> {
    if !crate::finish::valid_slug(slug) {
        return Err(Fatal::Usage(format!(
            "`{slug}` is not the name of a change; give the folder name of the change, with no `/`"
        )));
    }
    let cfg = repo.load_config()?;
    if cfg.pin != VERSION {
        return Err(crate::pin_mismatch(&cfg));
    }
    if lifecycle::intent_dirs(&cfg).is_empty() {
        return Err(Fatal::Config(
            "spec-dna.toml has no layer of the type `intent`, so there is no change to approve; see `spec-dna layer add`".into(),
        ));
    }
    // The mode is the stricter value of the head and the base tree (D70 item 6), the
    // value that SD934 reads: a branch cannot lower `approval` to skip the entry.
    let base_rev = lifecycle::merge_base_of(&repo.git, None, cfg.change_base.as_deref())
        .map_err(Fatal::Config)?;
    let nc = match base_rev.as_deref() {
        Some(r) => crate::acceptance::need_config(&cfg, &repo.git, r).map_err(Fatal::Io)?,
        None => crate::acceptance::need_config_of_head(&cfg),
    };
    if !nc.approval_branch(&cfg) {
        return Err(Fatal::Refused(
            "`approve <slug> acceptance` is a command of `[change] approval = \"branch\"`: with `base`, the merge of the pull request acknowledges the record".into(),
        ));
    }
    let change = find_change(repo, &cfg, slug)?;
    let path = format!("{}/{}", change.dir, crate::record::FILE_NAME);
    let full = repo.root.join(&path);
    if !full.is_file() {
        return Err(Fatal::NotFound(format!(
            "the change `{slug}` has no `{}` in `{}`",
            crate::record::FILE_NAME,
            change.dir
        )));
    }
    if !repo.git.is_clean_in_head(&path).map_err(Fatal::Io)? {
        return Err(Fatal::Refused(format!(
            "commit the text of `{path}` first; an approval commit changes the `acknowledged-by` line alone"
        )));
    }
    let author = repo.git.author_email().map_err(Fatal::Io)?;
    if !nc
        .approvers(&cfg)
        .iter()
        .any(|a| a.eq_ignore_ascii_case(&author))
    {
        return Err(Fatal::Refused(format!(
            "`{author}` is not in `[change] approvers`, so the commit gives no approval; set `user.email` or `GIT_AUTHOR_EMAIL` to an approver"
        )));
    }
    let old = std::fs::read_to_string(&full).map_err(|e| Fatal::Io(format!("{path}: {e}")))?;
    if let Err(e) = crate::record::Record::parse(&old) {
        return Err(Fatal::Refused(format!(
            "`{path}` is not a record that `approve` can read: {e}"
        )));
    }
    let listed = crate::record::ack_entries(&old);
    // The entries that count, from the walk of the commits. With no base, the
    // walk cannot run, and each entry stays as it is.
    let counted: Option<Vec<String>> = match base_rev {
        Some(b) => {
            let walk = crate::approval::ack_walk(&repo.git, &b, &path).map_err(Fatal::Io)?;
            // A walk that cannot run knows no entry that counts. The command makes no
            // commit then: the entry would not count, and SD932 would report it.
            if let Some(f) = &walk.stop {
                return Err(Fatal::Refused(format!(
                    "the walk of the commits cannot run, so the command cannot tell which entries of `{path}` count ({}); fetch the full history (`fetch-depth: 0`), then run it again",
                    f.sentence()
                )));
            }
            Some(walk.counted)
        }
        None => None,
    };
    let is_author = |e: &String| e.eq_ignore_ascii_case(&author);
    let stale: Vec<String> = match &counted {
        Some(c) => listed
            .iter()
            .filter(|e| !c.contains(&e.to_lowercase()))
            .cloned()
            .collect(),
        None => Vec::new(),
    };
    let author_counts = listed.iter().any(is_author) && !stale.iter().any(is_author);
    if author_counts && stale.is_empty() {
        let line = format!("`{path}` is already acknowledged by `{author}`");
        if json {
            println!(
                "{}",
                result_json(slug, "acceptance", &path, "acknowledged", None, &author)
            );
        } else {
            println!("{line}");
        }
        return Ok(0);
    }
    let mut text = old.clone();
    let mut commit: Option<String> = None;
    // A failure puts back the text of the last commit, so a failure of the second
    // commit keeps the first.
    let commit_file = |text: &str, before: &str, message: &str| -> Result<String, Fatal> {
        std::fs::write(&full, text).map_err(|e| Fatal::Io(format!("{path}: {e}")))?;
        repo.git.commit_only(message, &path).map_err(|e| {
            let _ = std::fs::write(&full, before);
            Fatal::Io(format!("git commit failed: {e}"))
        })
    };
    // Step 1: remove each entry that no longer counts, in a commit of its own.
    if !stale.is_empty() {
        let kept: Vec<String> = listed
            .iter()
            .filter(|e| !stale.contains(e))
            .cloned()
            .collect();
        let before = std::mem::take(&mut text);
        text = crate::record::set_acknowledged_by(&before, &kept).ok_or_else(|| {
            Fatal::Refused(format!(
                "`{path}` has no `acknowledged-by` list on one line that the command can edit"
            ))
        })?;
        commit = Some(commit_file(
            &text,
            &before,
            &format!("docs: clear the stale acknowledgements of the record of {slug}"),
        )?);
    }
    // Step 2: add the author, in the approval commit.
    if !author_counts {
        let mut list: Vec<String> = crate::record::ack_entries(&text);
        list.push(author.clone());
        let before = std::mem::take(&mut text);
        text = crate::record::set_acknowledged_by(&before, &list).ok_or_else(|| {
            Fatal::Refused(format!(
                "`{path}` has no `acknowledged-by` list on one line that the command can edit"
            ))
        })?;
        commit = Some(commit_file(
            &text,
            &before,
            &format!("docs: acknowledge the acceptance record of {slug}"),
        )?);
    }
    let commit = commit.expect("a commit was made");
    if json {
        println!(
            "{}",
            result_json(
                slug,
                "acceptance",
                &path,
                "acknowledged",
                Some(&commit),
                &author
            )
        );
    } else {
        println!(
            "acknowledged {path} in {} (author {author})",
            commit.get(..12).unwrap_or(&commit)
        );
    }
    Ok(0)
}

/// The JSON of a result.
fn result_json(
    slug: &str,
    artifact: &str,
    path: &str,
    status: &str,
    commit: Option<&str>,
    author: &str,
) -> String {
    Obj::new()
        .str("slug", slug)
        .str("artifact", artifact)
        .str("path", path)
        .str("status", status)
        .opt_str("commit", commit)
        .str("author", author)
        .end()
}

/// The change `slug` in the working tree.
fn find_change(repo: &Repo, cfg: &Config, slug: &str) -> Result<FoundChange, Fatal> {
    let snap = Snapshot::worktree(repo, &repo::root_paths(cfg), true)?;
    let mut loaded = repo::load(repo, cfg, &[&snap], true, false)?;
    let corpus = loaded.corpora.remove(0);
    let ch: Change = change::collect(&corpus)
        .into_iter()
        .find(|c| c.slug == slug)
        .ok_or_else(|| {
            Fatal::NotFound(format!(
                "no folder `<intent folder>/{slug}` for the change `{slug}`"
            ))
        })?;
    let located = |a: &Option<Artifact>| {
        a.as_ref().map(|a| ArtifactPath {
            path: corpus.docs[a.doc as usize].path.clone(),
            status: a.status.clone(),
        })
    };
    Ok(FoundChange {
        dir: ch.dir.clone(),
        class: ch.class,
        class_unset: ch.class_unset,
        intent: located(&ch.intent),
        spec: located(&ch.spec),
        plan: located(&ch.plan),
    })
}

/// An artifact of a change with its path from the repository root.
struct ArtifactPath {
    path: String,
    status: String,
}

/// The data of a change that `approve` reads.
struct FoundChange {
    dir: String,
    class: Class,
    class_unset: bool,
    intent: Option<ArtifactPath>,
    spec: Option<ArtifactPath>,
    plan: Option<ArtifactPath>,
}

/// Step 6: the earlier artifact must have its approval value. A spec needs an
/// `accepted` intent. A plan needs an `approved` spec for a higher-risk change,
/// and an `accepted` intent for a routine change.
fn check_earlier(ch: &FoundChange, artifact: &str) -> Result<(), Fatal> {
    let need = |a: &Option<ArtifactPath>, name: &str, value: &str| -> Result<(), Fatal> {
        match a {
            None => Err(Fatal::Refused(format!(
                "the change has no {name}; the {artifact} needs an {value} {name} first"
            ))),
            Some(a) if a.status != value => Err(Fatal::Refused(format!(
                "approve the {name} first: `{}` has the status `{}`, and the {artifact} needs `{value}`",
                a.path,
                if a.status.is_empty() {
                    "(none)"
                } else {
                    &a.status
                }
            ))),
            Some(_) => Ok(()),
        }
    };
    match artifact {
        "spec" => need(&ch.intent, "intent", "accepted"),
        "plan" if ch.class == Class::HigherRisk => need(&ch.spec, "spec", "approved"),
        "plan" => need(&ch.intent, "intent", "accepted"),
        _ => Ok(()),
    }
}

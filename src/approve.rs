//! `spec-dna approve <slug> intent|spec|plan` (owner review of 0.7.0, item 2).
//! It is a command for a person. It sets the `status` line of one artifact of
//! a change and commits that file alone, so the commit is an approval commit
//! of the branch rule (ADR 0012, D11): the file is the same as in HEAD except
//! the `status` line.
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
    let Some(approved) = approval_value(artifact) else {
        return Err(Fatal::Usage(format!(
            "`{artifact}` is not an artifact; use intent, spec or plan"
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

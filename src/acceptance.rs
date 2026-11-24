//! The rules around the acceptance record that `finish` and `check` share
//! (design 0.8.0, D3 items 1, 3, 7, 8 and 9, and D70).
//!
//! - The rule "a change that needs a record" (D70): [`need_config`] reads the
//!   configuration, [`record_class`] gives the class of the change for the
//!   acceptance, and [`decide`] joins them with the rule of the plan. `finish`
//!   (step 5.2), `check` and `next` call these functions.
//! - The archive file of a finished change: its path and its name rule
//!   (item 3).
//! - The checks that read a record: the rules of a valid record with the compare
//!   of the brief (SD928, item 8), SD929, SD930, SD931 and SD933 (item 7).
//!
//! The functions read git and the files that the caller names. They write
//! nothing. The code of the diagnostics is in `diag.rs`. The severity is the
//! severity of the profile (the group `acceptance` of `config.rs`).

use crate::accept::{self, Composed, Texts};
use crate::approval::Branch;
use crate::change::{self, Change};
use crate::config::{self, AcceptanceFor, Config, Version};
use crate::diag::{Code, Diag};
use crate::git::{Git, Limited, read_limited};
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

/// The codes of the record that `check` reads in the diff and `finish` reads in
/// step 5.2. The configuration of the need decides which of them are on
/// ([`NeedConfig::gate`]).
pub const CODES: [Code; 11] = [
    Code::AcceptanceMissing,
    Code::AcceptanceInvalid,
    Code::AcceptanceStale,
    Code::AcceptanceRejected,
    Code::SelfAccepted,
    Code::RecordCommitInvalid,
    Code::AcceptorMissing,
    Code::AckMissing,
    Code::RecordEdited,
    Code::RecordIncomplete,
    Code::AcceptanceRerun,
];

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

/// The change that an archive file belongs to: the field `change` inside the
/// file. The name of the file does not decide it: `fix-2.toml` is the archive of
/// the change `fix-2`, and also the file 2 of the change `fix`. Only a file that
/// does not read as a record has no field. Then the name without `.toml` is
/// the change (SD928 reports the file that does not read).
pub fn archive_owner(path: &str, text: &str) -> String {
    record::Record::parse(text)
        .map(|r| r.change)
        .unwrap_or_else(|_| {
            path.rsplit('/')
                .next()
                .unwrap_or(path)
                .trim_end_matches(".toml")
                .to_string()
        })
}

/// The result of the read of one file: its bytes, or `None` if it has none.
type Read = Result<Option<Vec<u8>>, String>;

/// The archive file of the change `slug` among the archive files that a diff
/// adds: its path and its bytes. A file is the archive of the change if its
/// field `change` is the slug ([`archive_owner`]). The number in its name only
/// orders the files of one change (the newest wins). `read` gives the
/// bytes of a path, or `None` if there is no file.
pub fn archive_of(
    slug: &str,
    added: &[&str],
    read: &mut dyn FnMut(&str) -> Read,
) -> Result<Option<(String, Vec<u8>)>, String> {
    let mut best: Option<(u32, String, Vec<u8>)> = None;
    // Only a file of the archive folder is an archive file: the filter of the
    // path that `finish` writes. A file elsewhere is never the archive.
    for p in added.iter().filter(|p| is_archive_path(p)) {
        // The number orders the files of one change. A file whose name has no
        // number of this slug comes first (a file that a person named).
        let n = archive_number(p, slug).unwrap_or(0);
        if best.as_ref().is_some_and(|(m, _, _)| *m >= n) {
            continue;
        }
        let Some(bytes) = read(p)? else { continue };
        if archive_owner(p, &String::from_utf8_lossy(&bytes)) == slug {
            best = Some((n, (*p).to_string(), bytes));
        }
    }
    Ok(best.map(|(_, p, b)| (p, b)))
}

/// The biggest file that a reader of a record or of an archive file reads. A
/// record is small. A caller reads with this limit (`Git::head_file_checked`):
/// the caller does not read a link or a big file in the diff.
pub const READ_LIMIT: usize = 256 * 1024;

/// The biggest change file (`intent.md`, `spec.md`, `plan.md`) that the compare
/// of the brief reads from the working tree.
const DOC_READ_LIMIT: usize = 4 * 1024 * 1024;

/// The SD928 text for a record path whose file is not read: a link or another
/// file that is not a regular file, or a file of more than [`READ_LIMIT`] bytes.
pub fn unreadable_message(path: &str, why: &Limited) -> String {
    let what = match why {
        Limited::TooBig => format!("a file of more than {READ_LIMIT} bytes"),
        _ => "not a regular file (a link, or a folder)".to_string(),
    };
    format!(
        "`{path}` is {what}, so it is not read as a record; replace it with the record that `spec-dna accept` writes (a regular file of at most {READ_LIMIT} bytes)"
    )
}

/// The path of a file that reads as the record of the change `slug`. The path is
/// among the paths that a diff adds and is outside the archive folder. The file
/// is the archive file that a commit moved out of `.spec-dna/accepted/`. A record
/// in a folder of a change (`is_folder_record`) is not an archive file. `read`
/// gives the bytes of a path.
pub fn moved_archive(
    slug: &str,
    added: &[&str],
    is_folder_record: &dyn Fn(&str) -> bool,
    read: &mut dyn FnMut(&str) -> Read,
) -> Result<Option<String>, String> {
    for p in added {
        if is_archive_path(p) || is_folder_record(p) {
            continue;
        }
        let Some(bytes) = read(p)? else { continue };
        if bytes.len() > READ_LIMIT {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        if record::Record::parse(&text).is_ok_and(|r| r.change == slug) {
            return Ok(Some((*p).to_string()));
        }
    }
    Ok(None)
}

/// The path of the archive file that `finish` writes for the slug:
/// `<slug>.toml`, or the first `<slug>-<n>.toml` with `n` of 2 or more whose
/// file does not exist. `exists` tells if a path exists on disk.
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
    /// history has no commit that added the plan. `branch_cut` is true if a
    /// shallow clone cuts the branch, so the class of the change reads as
    /// `higher-risk` (D70 item 2).
    Yes { shallow: bool, branch_cut: bool },
}

/// The plan part of the rule of D3 item 1 and D70 item 5, for a change that has a
/// plan and is not closed. The caller checks both. [`decide`] calls it when a
/// reason for a record exists.
///
/// The plan is new when:
/// - the plan is not in the tree of the base (a commit of the branch added it, or
///   no commit did). The plan is then always new (Q43). So a branch cannot set the
///   pin back for one commit to skip the record;
/// - a plan that the base tree has is new if the newest commit that added it,
///   and that is an ancestor of the lifecycle base, pinned 0.8.0 or later in
///   `spec-dna.toml`. A pin before 0.8.0 gives no record, with a note. If no
///   such commit exists (a shallow clone), or if the commit has no pin, the
///   result is the stricter one.
///
/// `plan_in_base` tells if the tree of `base_rev` has the plan at `plan_path`.
fn plan_is_new(
    git: &Git,
    base_rev: &str,
    plan_path: &str,
    plan_in_base: bool,
) -> Result<Need, String> {
    if !plan_in_base {
        return Ok(Need::Yes {
            shallow: false,
            branch_cut: false,
        });
    }
    match pin_when_added(git, base_rev, plan_path)? {
        None => Ok(Need::Yes {
            shallow: git.is_shallow(),
            branch_cut: false,
        }),
        Some(pin) if pin.as_deref().is_some_and(is_before_0_8) => Ok(Need::No(Some(format!(
            "`{plan_path}` was added when spec-dna.toml pinned {}, before 0.8.0, so `finish` needs no acceptance record; `finish` does not compare the brief of a record in the folder, and `check` does",
            pin.unwrap_or_default()
        )))),
        Some(_) => Ok(Need::Yes {
            shallow: false,
            branch_cut: false,
        }),
    }
}

// The configuration of the need (D70 item 6).

/// The configuration of the lifecycle base tree, as the need reads it.
#[derive(Clone, Debug)]
enum BaseConfig {
    /// The base tree has no `spec-dna.toml` (a pull request that adopts
    /// spec-dna). It has nothing to lower, so the head decides.
    Absent,
    Parsed(Box<Config>),
    /// The base tree has a `spec-dna.toml` that does not parse. The need then
    /// reads the strictest values.
    Unreadable,
}

/// The values that the need, [`record_class`], SD933, `next` and `finish` read
/// from the configuration. A branch can edit `spec-dna.toml`, so each value is the
/// stricter of the head and the configuration in the tree of the lifecycle base
/// (D70 item 6): the acceptance is on when SD927 is on in one of them,
/// `acceptance-for` is the stricter value, `high-risk-paths` and
/// `high-risk-layers` are the union, and `higher-risk-acceptors` is the larger
/// value. The commits between the base and the head add nothing, because they
/// cannot lower the base.
#[derive(Clone, Debug)]
pub struct NeedConfig {
    /// True if SD927 is on in the head or in the base.
    pub on: bool,
    /// The stricter `[change] acceptance-for`, each with the default of its profile.
    pub acceptance_for: AcceptanceFor,
    /// The union of the globs of `[change] high-risk-paths`.
    pub high_risk_paths: Vec<String>,
    /// The union of the layers of `[change] high-risk-layers`.
    pub high_risk_layers: Vec<String>,
    /// The larger value of `[change] higher-risk-acceptors`.
    pub higher_risk_acceptors: u32,
    /// The configuration whose severities decide the codes of the group
    /// `acceptance`: the head, or the base when the head turns off a code of the
    /// group that the base has on. So a branch cannot turn the group off with its own `profile`, `[rules]`
    /// or `[change]`.
    gate: Config,
    base: BaseConfig,
}

impl NeedConfig {
    /// The configuration that decides the codes of the group `acceptance`.
    pub fn gate(&self) -> &Config {
        &self.gate
    }

    /// True if the head or the base tree has `[change] approval = "branch"`. SD941
    /// is used only then (D74 item 7), and a branch cannot turn it off with its own
    /// `spec-dna.toml`.
    pub fn approval_branch(&self, head: &Config) -> bool {
        head.change_approval == config::Approval::Branch
            || matches!(&self.base, BaseConfig::Parsed(b) if b.change_approval == config::Approval::Branch)
    }

    /// True if the head or the base table of `[owners]` has a role.
    pub fn has_owners(&self, head: &Config) -> bool {
        !head.owners.is_empty()
            || matches!(&self.base, BaseConfig::Parsed(b) if !b.owners.is_empty())
    }

    /// The approvers of the head and of the base tree. A branch cannot drop an
    /// approver, as it cannot lower `approval`.
    pub fn approvers(&self, head: &Config) -> Vec<String> {
        let mut out = head.change_approvers.clone();
        if let BaseConfig::Parsed(b) = &self.base {
            for a in &b.change_approvers {
                if !out.iter().any(|o| o.eq_ignore_ascii_case(a)) {
                    out.push(a.clone());
                }
            }
        }
        out
    }

    /// True if the base tree has `approval = "branch"` and the head has not: the
    /// branch lowered the mode, and the stricter value stays.
    pub fn approval_lowered(&self, head: &Config) -> bool {
        head.change_approval != config::Approval::Branch && self.approval_branch(head)
    }

    /// The roles of `[owners]` that match one of `role_paths`, in the union of the
    /// head and the base table (D77). See [`matched_roles`].
    pub fn matched_roles(&self, head: &Config, role_paths: &[String]) -> Vec<String> {
        let mut roles = matched_roles(head, role_paths);
        if let BaseConfig::Parsed(b) = &self.base {
            roles.extend(matched_roles(b, role_paths));
        }
        roles.sort();
        roles.dedup();
        roles
    }
}

/// The union of 2 lists, in the order of the first list and then the new items.
fn union(a: &[String], b: &[String]) -> Vec<String> {
    let mut out = a.to_vec();
    out.extend(b.iter().filter(|x| !a.contains(x)).cloned());
    out
}

/// The configuration that gives the strictest values, for a base configuration
/// that does not parse: the profile `strict`, no rule and the policy
/// `three-class`.
fn strictest(head: &Config) -> Config {
    let mut c = head.clone();
    c.profile = config::Profile::Strict;
    c.rules.clear();
    for r in &mut c.roots {
        r.rules.clear();
        r.profile = None;
    }
    c.change_policy = config::ChangePolicy::ThreeClass;
    c.change_acceptance_for = Some(AcceptanceFor::All);
    c
}

/// Reads the configuration of the need ([`NeedConfig`]): the head and the tree
/// of `base_rev`. A base with no `spec-dna.toml` leaves the head alone. A base
/// whose file does not parse gives the strictest values.
pub fn need_config(head: &Config, git: &Git, base_rev: &str) -> Result<NeedConfig, String> {
    let text = git
        .read_at(&[(base_rev.to_string(), config::FILE_NAME.to_string())])?
        .into_iter()
        .next()
        .flatten();
    let base = match text {
        None => BaseConfig::Absent,
        Some(b) => match Config::parse(&String::from_utf8_lossy(&b)) {
            Ok(c) => BaseConfig::Parsed(Box::new(c)),
            Err(_) => BaseConfig::Unreadable,
        },
    };
    let mut nc = need_config_of_head(head);
    // True if the head turns off a code of the group `acceptance` that `other` has
    // on. The head is then the weaker configuration for that code.
    let weaker = |other: &Config| {
        crate::diag::Code::ALL
            .iter()
            .any(|&c| config::is_acceptance_code(c) && head.is_off(c) && !other.is_off(c))
    };
    match &base {
        BaseConfig::Absent => {}
        BaseConfig::Parsed(b) => {
            if !nc.on && !b.is_off(Code::AcceptanceMissing) {
                nc.on = true;
            }
            if weaker(b) {
                nc.gate = (**b).clone();
            }
            nc.acceptance_for = nc.acceptance_for.max(b.acceptance_for());
            nc.high_risk_paths = union(&head.high_risk_paths, &b.high_risk_paths);
            nc.high_risk_layers = union(&head.high_risk_layers, &b.high_risk_layers);
            nc.higher_risk_acceptors = nc.higher_risk_acceptors.max(b.higher_risk_acceptors);
        }
        BaseConfig::Unreadable => {
            nc.on = true;
            let strict = strictest(head);
            if weaker(&strict) {
                nc.gate = strict;
            }
            nc.acceptance_for = AcceptanceFor::All;
        }
    }
    nc.base = base;
    Ok(nc)
}

/// True if the acceptance is on by the need rule: SD927 is on in the head or in
/// the tree of `base_rev` ([`NeedConfig::on`]). With no base, the head alone. A
/// task that names `spec-dna accept <slug>` closes with `finish` only then (D75
/// item 6), so a branch cannot change this gate with its own `spec-dna.toml`.
pub fn need_on(head: &Config, git: &Git, base_rev: Option<&str>) -> Result<bool, String> {
    Ok(match base_rev {
        Some(r) => need_config(head, git, r)?.on,
        None => need_config_of_head(head).on,
    })
}

/// The configuration of the need when git names no base: the head alone.
pub fn need_config_of_head(head: &Config) -> NeedConfig {
    NeedConfig {
        on: !head.is_off(Code::AcceptanceMissing),
        acceptance_for: head.acceptance_for(),
        high_risk_paths: head.high_risk_paths.clone(),
        high_risk_layers: head.high_risk_layers.clone(),
        higher_risk_acceptors: head.higher_risk_acceptors,
        gate: head.clone(),
        base: BaseConfig::Absent,
    }
}

// The roles of `[owners]` (D70 item 7 and D77).

/// The roles of `[owners]` that match one of `role_paths`, sorted. The need,
/// SD933 and the brief all read the roles through this one function. A person
/// entry of the table is no role ([`crate::owners`]).
pub fn matched_roles(cfg: &Config, role_paths: &[String]) -> Vec<String> {
    cfg.owners.matched(role_paths)
}

/// The code paths that a role of `[owners]` reads for one change (D70 item 7):
///
/// - each code path of the diff that `scope` of the change matches;
/// - each code path of the diff that no scope of a change of the diff matches
///   (`other_scopes` are the scopes of the other changes of the diff). A path that
///   `Files` leaves out thus still needs its role;
/// - when the diff has no code path in `scope`, the code paths in `scope` that
///   changed since the commit that added the plan (`since_plan`).
///
/// The result is sorted, with no duplicate. [`change_roles`] passes it to
/// [`NeedConfig::matched_roles`].
pub fn role_paths(
    diff_code: &[String],
    scope: &[String],
    other_scopes: &[&[String]],
    since_plan: &[String],
) -> Vec<String> {
    let mut out: Vec<String> = diff_code
        .iter()
        .filter(|p| {
            change::in_scope(scope, p) || !other_scopes.iter().any(|s| change::in_scope(s, p))
        })
        .cloned()
        .collect();
    if !diff_code.iter().any(|p| change::in_scope(scope, p)) {
        out.extend(
            since_plan
                .iter()
                .filter(|p| change::in_scope(scope, p))
                .cloned(),
        );
    }
    out.sort();
    out.dedup();
    out
}

/// What the roles of one change read.
pub struct RoleInput<'a> {
    pub cfg: &'a Config,
    pub git: &'a Git,
    /// The diff of the head against the lifecycle base.
    pub diff: &'a Diff,
    /// The scope of the change ([`accept::scope_of`]).
    pub scope: &'a [String],
    /// The scopes of the other changes that the diff touches.
    pub other_scopes: &'a [Vec<String>],
    /// The path of the plan, for the paths since the commit that added it.
    pub plan_path: &'a str,
}

/// The roles of `[owners]` that one change needs (D70 item 7 and D77): the roles
/// that match a role path ([`role_paths`]), in the union of the head table and
/// the base table ([`NeedConfig::matched_roles`]). The need, SD933, `next`, `finish`
/// and `accept` all call this one function. With no table in the head and in the
/// base, it reads no git data and gives no role.
pub fn change_roles(nc: &NeedConfig, i: &RoleInput) -> Result<Vec<String>, String> {
    if !nc.has_owners(i.cfg) {
        return Ok(Vec::new());
    }
    let dirs = crate::lifecycle::intent_dirs(i.cfg);
    let is_code = |p: &str| tree::is_code_path(i.cfg, p, &dirs);
    let diff_code: Vec<String> = i
        .diff
        .files
        .iter()
        .map(|f| f.0.clone())
        .filter(|p| is_code(p))
        .collect();
    let others: Vec<&[String]> = i.other_scopes.iter().map(Vec::as_slice).collect();
    // The paths since the commit that added the plan count only when the diff has
    // no code path in `scope` (a pull request with only `finish`). The diff starts
    // at the first parent of that commit, as the fallback of `accept` does, so a
    // squash commit that holds the plan and the code gives its code. A root commit
    // has no parent: the diff then starts at the commit itself.
    let mut since: Vec<String> = Vec::new();
    if !diff_code.iter().any(|p| change::in_scope(i.scope, p))
        && let Some(added) = i.git.added_by(i.plan_path)?
    {
        let from = i.git.commit(&format!("{added}^")).unwrap_or(added);
        since = Diff::read(i.git, &from, false)?
            .files
            .into_iter()
            .map(|f| f.0)
            .filter(|p| is_code(p))
            .collect();
    }
    Ok(nc.matched_roles(i.cfg, &role_paths(&diff_code, i.scope, &others, &since)))
}

// The class of the change for the acceptance (D70 items 2 to 4).

/// What [`record_class`] reads.
pub struct ClassInput<'a> {
    pub nc: &'a NeedConfig,
    pub git: &'a Git,
    pub base_rev: &'a str,
    /// The folder of the change, from the repository root.
    pub folder: &'a str,
    /// The change in the head tree. `None` when the diff deletes the folder
    /// (D70 item 4).
    pub head: Option<&'a Change>,
    /// True if the minimum class of SD917 is `higher-risk`
    /// ([`crate::lifecycle::minimum_is_higher_risk`]). It runs last, because it is
    /// the most costly part.
    pub minimum: &'a dyn Fn() -> bool,
}

/// The class of a change for the acceptance, and a flag for a cut history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordClass {
    pub class: Class,
    /// True if a shallow clone cuts the branch, so the class is `higher-risk` as
    /// the stricter result, and SD927 names `fetch-depth: 0`.
    pub shallow: bool,
}

/// True if the text of an `intent.md` has `class: higher-risk`.
fn text_is_higher_risk(bytes: &[u8]) -> bool {
    parse(&String::from_utf8_lossy(bytes))
        .front_matter
        .and_then(|f| match f.get("class") {
            Some((crate::frontmatter::Value::Str(v), _)) => Some(v.trim() == "higher-risk"),
            _ => None,
        })
        .unwrap_or(false)
}

/// True if the text of an `intent.md` has no valid `class`: it is absent, empty
/// or not `routine` or `higher-risk` (`class_unset` of the head intent).
fn text_class_is_unset(bytes: &[u8]) -> bool {
    parse(&String::from_utf8_lossy(bytes))
        .front_matter
        .and_then(|f| match f.get("class") {
            Some((crate::frontmatter::Value::Str(v), _)) => {
                Some(crate::model::Class::from_name(v.trim()).is_none())
            }
            _ => None,
        })
        .unwrap_or(true)
}

/// The class of the change for the acceptance (D70 items 2 to 4). It is
/// `higher-risk` when one of these is true. Else it is `routine`.
///
/// - The intent of the head has `class: higher-risk`, or its `class` is absent,
///   empty or not valid.
/// - The intent in the tree of the lifecycle base has `class: higher-risk`.
/// - A commit of the branch has an intent with `class: higher-risk`
///   ([`Git::first_parent_commits`]).
/// - A shallow clone cuts the branch.
/// - The minimum class of SD917 is `higher-risk`.
///
/// Thus a branch cannot lower or remove the class to skip the record. Where the
/// diff deletes the folder, the head has no intent, and the function reads the
/// other points. The head is read first and the walk of the commits second, so a
/// change that the head already marks costs no git read. The `high-risk-paths` and
/// `high-risk-layers` are those of the need ([`NeedConfig`]).
pub fn record_class(i: &ClassInput) -> Result<RecordClass, String> {
    let higher = |shallow| {
        Ok(RecordClass {
            class: Class::HigherRisk,
            shallow,
        })
    };
    if i.head
        .is_some_and(|c| c.intent.is_some() && (c.class == Class::HigherRisk || c.class_unset))
    {
        return higher(false);
    }
    let path = format!("{}/intent.md", i.folder);
    // The base tree and each commit of the branch, in one batch.
    let commits = if i.git.range_is_cut(i.base_rev) {
        return higher(true);
    } else {
        i.git.first_parent_commits(i.base_rev)?
    };
    let mut specs: Vec<(String, String)> = vec![(i.base_rev.to_string(), path.clone())];
    specs.extend(commits.iter().map(|c| (c.clone(), path.clone())));
    let texts = i.git.read_at(&specs)?;
    if texts.iter().flatten().any(|b| text_is_higher_risk(b)) {
        return higher(false);
    }
    // The diff deletes the folder (D70 item 4): the newest intent of the change
    // takes the place of the intent of the head. It is the intent at the last
    // commit of the branch that has the folder, else in the base tree. An absent,
    // empty or invalid class there is `higher-risk`, as for the head. The ratchet
    // above counts only an explicit `higher-risk`.
    if i.head.is_none()
        && texts
            .iter()
            .rev()
            .flatten()
            .next()
            .is_some_and(|b| text_class_is_unset(b))
    {
        return higher(false);
    }
    if (i.minimum)() {
        return higher(false);
    }
    Ok(RecordClass {
        class: Class::Routine,
        shallow: false,
    })
}

// The need (D70 item 1).

/// What the rule decides for one change: the need, and the class of the change
/// for the acceptance. `finish`, `next` and `check` read the class for SD933 and
/// for SD620 (`reliable-for`), also where no record is needed.
#[derive(Clone, Debug, PartialEq)]
pub struct Decision {
    pub need: Need,
    pub class: Class,
    /// The roles of `[owners]` that the change needs ([`change_roles`]).
    pub roles: Vec<String>,
}

/// The rule "a change that needs a record" (D70 item 1), for a change that has a
/// plan and is not closed. The caller checks both. A record is needed when the
/// acceptance is on, the plan is new ([`plan_is_new`]) and one reason holds:
/// `acceptance-for` is `all`; it is `higher-risk` and the class is `higher-risk`;
/// or a role of `[owners]` matches (`roles` is the result of
/// [`NeedConfig::matched_roles`]). A matched role needs a record also with
/// `acceptance-for = "none"`.
pub fn decide(
    i: &ClassInput,
    plan_path: &str,
    plan_in_base: bool,
    roles: &[String],
) -> Result<Decision, String> {
    let rc = record_class(i)?;
    let class = rc.class;
    let nc = i.nc;
    if !nc.on {
        return Ok(Decision {
            need: Need::No(None),
            class,
            roles: roles.to_vec(),
        });
    }
    let reason = !roles.is_empty()
        || match nc.acceptance_for {
            AcceptanceFor::All => true,
            AcceptanceFor::HigherRisk => class == Class::HigherRisk,
            AcceptanceFor::None => false,
        };
    if !reason {
        return Ok(Decision {
            need: Need::No(None),
            class,
            roles: roles.to_vec(),
        });
    }
    let need = match plan_is_new(i.git, i.base_rev, plan_path, plan_in_base)? {
        Need::Yes { shallow, .. } => Need::Yes {
            shallow,
            branch_cut: rc.shallow,
        },
        other => other,
    };
    Ok(Decision {
        need,
        class,
        roles: roles.to_vec(),
    })
}

/// True if a pin is a version before 0.8.0.
fn is_before_0_8(pin: &str) -> bool {
    Version::parse(pin).is_some_and(|v| v < FIRST_VERSION)
}

/// The pin of `spec-dna.toml` in the newest commit that added `path` and that is
/// an ancestor of `base_rev`. The outer `None` means no such commit (a shallow
/// clone). The inner `None` means that the commit has no pin. The rule of D3
/// item 1 for a plan and the rule of D6 for a spec both read it here.
pub fn pin_when_added(
    git: &Git,
    base_rev: &str,
    path: &str,
) -> Result<Option<Option<String>>, String> {
    let Some(commit) = git.added_at(base_rev, path)? else {
        return Ok(None);
    };
    let text = git
        .read_at(&[(commit, config::FILE_NAME.to_string())])?
        .into_iter()
        .next()
        .flatten();
    Ok(Some(text.and_then(|b| {
        config::scan_pin(&String::from_utf8_lossy(&b))
    })))
}

/// True if the section `Invariants` of a spec, or the section `Deviations` of a
/// plan, at `path` is read as 0.8.0 reads it. A file that a commit added when
/// `spec-dna.toml` pinned a version before 0.8.0 keeps the meaning of 0.7.1: the
/// section is plain text (design 0.8.0, D6 and D7). This is the one rule for
/// `check`, `finish`, `next`, the skeleton and the brief.
///
/// - The pin is the pin of the commit that added the file. For a file that the
///   tree of the base has, it is the newest such commit in the history of the
///   base. For any other file (a file of the branch, or any file with no base),
///   it is the newest such commit in the history of `HEAD`. So a plan that the
///   branch adds under a pin of 0.7.1 is plain text too: the owner rule is that
///   a 0.7.1 project gets no new error without a key. (The acceptance for a new
///   plan is the exception of Q43. It has its own rule, [`needs_record`].)
/// - A file with no commit found (a shallow clone, or a file that is not
///   committed) and a commit with no pin get the rules of 0.8.0, the stricter
///   result. A repository with no commit has none to read.
///
/// The reader of the pin is [`added_before`]. `skip_early_plans` (SD923 and
/// SD924, the version 0.7.0) calls it too, so the 2 rules read one commit.
pub fn section_is_read(git: &Git, base_rev: Option<&str>, path: &str) -> Result<bool, String> {
    if git.commit("HEAD").is_none() {
        return Ok(true);
    }
    let rev = match base_rev {
        Some(rev)
            if git
                .read_at(&[(rev.to_string(), path.to_string())])?
                .into_iter()
                .next()
                .flatten()
                .is_some() =>
        {
            rev
        }
        _ => "HEAD",
    };
    Ok(added_before(git, rev, path, FIRST_VERSION)?.is_none())
}

/// The pin of `spec-dna.toml` in the newest commit of the history of `rev` that
/// added `path`, if that pin is a version before `first`. `None` if the pin is
/// `first` or later, if the commit has no pin, or if no commit is found (a
/// shallow clone, a file that is not committed): each of these is the stricter
/// result. It is the one reader of the pin of an adding commit for
/// `section_is_read` (the version 0.8.0) and `skip_early_plans` (the version
/// 0.7.0).
pub fn added_before(
    git: &Git,
    rev: &str,
    path: &str,
    first: Version,
) -> Result<Option<String>, String> {
    Ok(pin_when_added(git, rev, path)?
        .flatten()
        .filter(|p| Version::parse(p).is_some_and(|v| v < first)))
}

/// The message of SD927 for a missing record. `where_` names the place that
/// should have it.
pub fn missing_message(
    slug: &str,
    where_: &str,
    roles: &[String],
    shallow: bool,
    branch_cut: bool,
) -> String {
    let mut m = format!(
        "the change `{slug}` needs an acceptance record, and {where_}; start an agent `acceptor` with the prompt that `spec-dna next` prints, `cd '<root>' && spec-dna accept {slug} --brief`, and commit the record that it writes"
    );
    if !roles.is_empty() {
        m.push_str(&format!(
            "; the table `[owners]` needs the {} {}",
            if roles.len() == 1 { "role" } else { "roles" },
            roles
                .iter()
                .map(|r| format!("`{r}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if shallow {
        m.push_str(
            "; the clone is shallow, so the commit that added the plan is not in the history: fetch the full history (`fetch-depth: 0`)",
        );
    }
    if branch_cut {
        m.push_str(
            "; the clone is shallow and cuts the branch, so the class of the change cannot be read and counts as `higher-risk`: fetch the full history (`fetch-depth: 0`)",
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

/// Where a record is: the folder of a change, or an archive file. A record of
/// format 1 in a folder is not valid, and in an archive file no rule of format 2
/// applies to it (D72 item 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    Folder,
    Archive,
}

/// Reads a record and checks the rules of a valid record (SD928, without the
/// compare of the brief). It returns the record, if the text reads, and one text
/// for each broken rule.
pub fn read_record(
    cfg: &Config,
    text: &str,
    slug: &str,
    facts: PlanFacts,
    place: Place,
) -> (Option<Record>, Vec<String>) {
    let intent_dirs = crate::lifecycle::intent_dirs(cfg);
    let is_code = |p: &str| tree::is_code_path(cfg, p, &intent_dirs);
    let expect = Expect {
        archive: place == Place::Archive,
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
/// needs (D70 item 3): 1 for a `routine` change, and `higher_risk` (the value of
/// `[change] higher-risk-acceptors` of [`NeedConfig`]) for a `higher-risk` change.
/// A role of `[owners]` that matches a role path (`roles`) needs an entry that
/// names it in `roles`, and one entry can name more than one role (D77). The
/// count and the roles are 2 rules, and one entry can count for both.
pub fn acceptors_missing(
    class: Class,
    higher_risk: u32,
    roles: &[String],
    rec: &Record,
) -> Option<String> {
    let needed = match class {
        Class::Routine => 1,
        Class::HigherRisk => higher_risk as usize,
    };
    let mut why: Vec<String> = Vec::new();
    if rec.acceptors.len() < needed {
        why.push(format!(
            "the record has {} acceptor entries, and a {} change needs {needed}",
            rec.acceptors.len(),
            class.name()
        ));
    }
    let lacking: Vec<&String> = roles
        .iter()
        .filter(|r| !rec.acceptors.iter().any(|a| a.roles.contains(r)))
        .collect();
    if !lacking.is_empty() {
        why.push(format!(
            "no acceptor entry has the role {} of `[owners]` in `roles`",
            lacking
                .iter()
                .map(|r| format!("`{r}`"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    (!why.is_empty()).then(|| {
        format!(
            "{}; run `spec-dna accept {} [--role <name>]` for each missing acceptor",
            why.join("; "),
            rec.change
        )
    })
}

// The acknowledgement with `branch` (design 0.8.0, D3 item 10): SD932 and SD934.

/// What the walk of the commits finds about the acknowledgement of one record.
#[derive(Clone, Debug, Default)]
pub struct AckFacts {
    /// True with `[change] approval = "branch"`: the acknowledgement is an entry of
    /// `acknowledged-by`. With `base`, the merge acknowledges.
    pub branch: bool,
    /// The messages of SD932: a commit that changes the record and another path,
    /// an entry that does not count, or a walk that cannot run.
    pub invalid: Vec<String>,
    /// The message of SD934: the record has a deviation and no entry of an approver
    /// counts.
    pub missing: Option<String>,
}

/// SD932 and SD934 for the record of the change in `folder`, with
/// `[change] approval = "branch"` only (D74 item 7 names the same limit for SD941).
/// `entries` are the entries of `acknowledged-by` of the record at the head (the
/// folder record, or the archive file after `finish`), and `deviations` is the number
/// of its `[[deviation]]` entries. The walk reads the commits of the folder record:
/// the commit of `finish` moves the same bytes to the archive and keeps the entries.
/// The result is empty with `base`, and when git names no base.
pub fn ack_facts(
    cfg: &Config,
    nc: &NeedConfig,
    git: &Git,
    base_rev: &str,
    (folder, slug): (&str, &str),
    (entries, deviations): (&[String], usize),
) -> Result<AckFacts, String> {
    let mut facts = AckFacts::default();
    if !nc.approval_branch(cfg) {
        return Ok(facts);
    }
    facts.branch = true;
    let path = format!("{folder}/{}", record::FILE_NAME);
    let walk = crate::approval::ack_walk(git, base_rev, &path)?;
    if let Some(f) = &walk.stop {
        facts.invalid.push(format!(
            "the walk of the commits of the branch cannot run, so no commit of the record counts ({}); the record needs `acknowledged-by` that an approval commit added",
            f.sentence()
        ));
    }
    for (commit, other) in &walk.mixed {
        // A commit of the walk is in the current life of the record (D80 item 1), and
        // history stays as it is: the fix is a new life. The staged changes are no
        // commit yet, so they only need to change the record alone.
        let (who, fix) = if commit.is_empty() {
            (
                "the staged changes change".to_string(),
                format!("unstage `{other}`, and commit the record alone"),
            )
        } else {
            (
                format!("the commit {} changes", &commit[..commit.len().min(12)]),
                format!(
                    "delete the record in a commit of its own, then accept again (`spec-dna accept {slug}`)"
                ),
            )
        };
        facts.invalid.push(format!(
            "{who} `{path}` and `{other}`; a commit that adds or edits the record changes the record alone: {fix}"
        ));
    }
    let head: Vec<String> = entries.iter().map(|e| e.to_lowercase()).collect();
    if walk.stop.is_none() {
        for (entry, low) in entries.iter().zip(&head) {
            if !walk.counted.contains(low) {
                facts.invalid.push(format!(
                    "the entry `{entry}` of `acknowledged-by` does not count: an approval commit of its author must add it and change nothing else, and no later commit may change the record; the record changed after the entry, or the commit is not an approval commit. Ask an approver to run `spec-dna approve {slug} acceptance` again"
                ));
            }
        }
    }
    let approvers = nc.approvers(cfg);
    let approved = approvers
        .iter()
        .any(|a| head.contains(&a.to_lowercase()) && walk.counted.contains(&a.to_lowercase()));
    if deviations > 0 && !approved {
        facts.missing = Some(format!(
            "the record of the change `{slug}` has {deviations} deviation{} and no acknowledgement of an approver ({}); an approver runs `spec-dna approve {slug} acceptance`, and the approval commit changes `acknowledged-by` alone",
            if deviations == 1 { "" } else { "s" },
            approvers.join(", ")
        ));
        if nc.approval_lowered(cfg)
            && let Some(m) = &mut facts.missing
        {
            m.push_str("; the base has `approval = \"branch\"`, and a branch cannot lower it");
        }
    }
    Ok(facts)
}

/// SD938: the declared invariants and deviations that the record does not
/// answer (design 0.8.0, D6 and D7). An invariant needs an `[[invariant]]` entry
/// with the same pin: the same `<doc-id>#<key>@<hash>`. An answer for another
/// hash of the rule is no answer. A deviation needs a `[[deviation]]` entry with
/// the same id, kind and text, with each run of white space read as one space.
/// An entry that no plan item names gives no finding. SD928 reports an id that 2
/// entries share ([`record::validate`]), so the first entry of an id is the
/// entry. `check`, `finish` and `next` all call this one function. The messages
/// name the entry to add.
pub fn answers_missing(rec: &Record, decl: &accept::Declared) -> Vec<String> {
    let mut out = Vec::new();
    let answered: Vec<String> = rec
        .invariants
        .iter()
        .filter_map(|v| change::parse_pin(&v.id).ok())
        .map(|p| p.text())
        .collect();
    for pin in &decl.invariants {
        if !answered.contains(pin) {
            out.push(format!(
                "the record has no answer for the invariant `{pin}`; an acceptor adds an entry `[[invariant]]` with `id = \"{pin}\"`, `answer` (`held`, `not-affected` or `violated`) and, for `not-affected` and `violated`, a `reason`, and commits the record alone"
            ));
        }
    }
    let same = |t: &str| t.split_whitespace().collect::<Vec<_>>().join(" ");
    for (id, kind, text) in &decl.deviations {
        match rec.deviations.iter().find(|d| d.id == *id) {
            None => out.push(format!(
                "the record has no entry for the deviation `{id}` of the plan; an acceptor adds an entry `[[deviation]]` with `id = \"{id}\"`, `kind = \"{kind}\"`, `text = {}` and `verdict` (`accepted` or `rejected`), and commits the record alone",
                record::toml_string(text)
            )),
            Some(d) if same(&d.text) != *text => out.push(format!(
                "the text of the entry `[[deviation]]` `{id}` is not the text of the plan item (`{text}`); copy the text of the plan item into the entry again, and commit the record alone"
            )),
            Some(d) if d.kind != *kind => out.push(format!(
                "the kind `{}` of the entry `[[deviation]]` `{id}` is not the kind `{kind}` of the plan item; copy the kind of the plan item into the entry, and commit the record alone",
                d.kind
            )),
            Some(_) => {}
        }
    }
    out
}

/// The paths among `paths` whose sections are not read as 0.8.0 reads them
/// ([`section_is_read`]). `check` ([`crate::lifecycle::skip_early_sections`]),
/// `finish` and `next` ([`sections_of`]) all call this one function. It makes
/// one lookup for each distinct path.
pub fn sections_skipped(
    git: &Git,
    base_rev: Option<&str>,
    paths: &[&str],
) -> Result<Vec<String>, String> {
    let mut seen: Vec<&str> = Vec::new();
    let mut skipped: Vec<String> = Vec::new();
    for p in paths {
        if seen.contains(p) {
            continue;
        }
        seen.push(p);
        if !section_is_read(git, base_rev, p)? {
            skipped.push((*p).to_string());
        }
    }
    Ok(skipped)
}

/// What step 5.2 and `next` read besides the record: the findings of the
/// sections of the spec and the plan (SD936, SD937, SD939 and SD940), and the
/// items that the record must answer (SD938).
#[derive(Clone, Debug, Default)]
pub struct Sections {
    /// The findings of [`crate::lifecycle::invariant_diags`] and
    /// [`crate::lifecycle::deviation_diags`] that are on, with the rule of the
    /// pin applied ([`section_is_read`]).
    pub diags: Vec<Diag>,
    pub declared: accept::Declared,
}

/// The messages of the findings of one code, for a stop: the place, then the text.
fn section_messages(sec: &Sections, code: Code) -> Vec<String> {
    sec.diags
        .iter()
        .filter(|d| d.code == code)
        .map(|d| format!("`{}` line {}: {}", d.path, d.line, d.message))
        .collect()
}

/// The first finding of a section of the spec or the plan that stops the
/// acceptor before a record exists (D88): SD936, SD937, SD939 or SD940, in the
/// order of [`first_stop`], for a code that is on. SD938 needs a record, so it
/// is not here. `next` and `accept` call this function with the [`Sections`]
/// that `finish` reads, so the 3 commands stop on the same finding.
pub fn section_stop<'a>(cfg: &Config, sec: &'a Sections) -> Option<&'a Diag> {
    [
        Code::InvariantInvalid,
        Code::InvariantStale,
        Code::DeviationInvalid,
        Code::DeviationUnresolved,
    ]
    .into_iter()
    .filter(|c| !cfg.is_off(*c))
    .find_map(|c| sec.diags.iter().find(|d| d.code == c))
}

/// The items that the spec and the plan of the change `ch` declare, with the
/// rule of the pin applied ([`section_is_read`]): a spec or a plan of 0.7.1
/// declares nothing. `accept` (the skeleton and the brief), the compare of the
/// brief (SD928), SD938 and `finish` all call this one function, so they read
/// the same items.
pub fn declared_for(
    cfg: &Config,
    git: &Git,
    base_rev: Option<&str>,
    ch: &Change,
    corpus: &Corpus,
) -> Result<accept::Declared, String> {
    let read = |a: &Option<change::Artifact>| -> Result<bool, String> {
        match a {
            Some(a) => section_is_read(git, base_rev, &corpus.docs[a.doc as usize].path),
            None => Ok(true),
        }
    };
    let mut d = accept::declared(cfg, ch, corpus);
    if !read(&ch.spec)? {
        d.invariants.clear();
    }
    if !read(&ch.plan)? {
        d.deviations.clear();
    }
    Ok(d)
}

/// Reads [`Sections`] for the change `ch` of the head tree. `base_rev` is the
/// lifecycle base. `finish` (step 5.2) and `next` call this one function, so
/// they read the same items as `check` does.
pub fn sections_of(
    cfg: &Config,
    git: &Git,
    base_rev: Option<&str>,
    ch: &Change,
    head: &Corpus,
) -> Result<Sections, String> {
    let mut diags = crate::lifecycle::invariant_diags(cfg, head, ch);
    diags.extend(crate::lifecycle::deviation_diags(cfg, head, ch));
    let mut diags = crate::check::apply_severity(cfg, diags);
    let paths: Vec<&str> = diags.iter().map(|d| d.path.as_str()).collect();
    let skipped = sections_skipped(git, base_rev, &paths)?;
    diags.retain(|d| !skipped.contains(&d.path));
    Ok(Sections {
        diags,
        declared: declared_for(cfg, git, base_rev, ch, head)?,
    })
}

/// The first check of the record that stops `finish` step 5.2, and its messages.
/// `finish` and `next` use this one function, so a check that is off stops
/// neither. The order is the order of the codes: SD928 (`problems`: the rules of
/// a valid record, and the compare of the briefs), SD929 (`stale`), SD930,
/// SD931, SD932 and SD933, then SD936, SD937, SD938 (`answers_missing`), SD939 and
/// SD940, then SD934 (part 5.3 of step 5: a stop prints `step 5`). A record that does not read (`rec` is `None`) stops with SD928 only;
/// with SD928 off it passes, as no other check can read it.
pub fn first_stop(
    cfg: &Config,
    (class, higher_risk, roles): (Class, u32, &[String]),
    rec: Option<&Record>,
    problems: Vec<String>,
    stale: Option<String>,
    sec: &Sections,
    (rerun, ack): (Option<&str>, &AckFacts),
) -> Option<(Code, Vec<String>)> {
    let on = |c: Code| !cfg.is_off(c);
    if on(Code::AcceptanceInvalid) && !problems.is_empty() {
        return Some((Code::AcceptanceInvalid, problems));
    }
    let rec = rec?;
    if on(Code::AcceptanceStale)
        && let Some(why) = stale
    {
        return Some((Code::AcceptanceStale, vec![why]));
    }
    if on(Code::AcceptanceRejected)
        && let Some(why) = rejected(rec)
    {
        return Some((Code::AcceptanceRejected, vec![why]));
    }
    if on(Code::SelfAccepted) {
        let found = self_accepted(rec);
        if !found.is_empty() {
            return Some((Code::SelfAccepted, found));
        }
    }
    if on(Code::RecordCommitInvalid) && !ack.invalid.is_empty() {
        return Some((Code::RecordCommitInvalid, ack.invalid.clone()));
    }
    if on(Code::AcceptorMissing)
        && let Some(why) = acceptors_missing(class, higher_risk, roles, rec)
    {
        return Some((Code::AcceptorMissing, vec![why]));
    }
    // The stop rule (D74): the record comes after 2 rejected records.
    if on(Code::AcceptanceRerun)
        && let Some(why) = rerun
    {
        return Some((Code::AcceptanceRerun, vec![why.to_string()]));
    }
    for code in [Code::InvariantInvalid, Code::InvariantStale] {
        let found = section_messages(sec, code);
        if on(code) && !found.is_empty() {
            return Some((code, found));
        }
    }
    if on(Code::RecordIncomplete) {
        let found = answers_missing(rec, &sec.declared);
        if !found.is_empty() {
            return Some((Code::RecordIncomplete, found));
        }
    }
    for code in [Code::DeviationInvalid, Code::DeviationUnresolved] {
        let found = section_messages(sec, code);
        if on(code) && !found.is_empty() {
            return Some((code, found));
        }
    }
    // Part 5.3: the acknowledgement of the deviations of the record.
    if on(Code::AckMissing)
        && let Some(why) = &ack.missing
    {
        return Some((Code::AckMissing, vec![why.clone()]));
    }
    None
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

/// The start of the fix in a SD929 message after `finish` (an archive file).
pub const STALE_AFTER_FINISH: &str = "`git revert --no-commit` the commit of `finish`";

/// The start of the fix in a SD929 message before `finish` (a folder record).
pub const STALE_BEFORE_FINISH: &str = "delete the record";

/// The plan of a change, as the stale rule reads it: the patterns that its scope
/// has, and every code span of `Files`. Each reader of the rule builds it from
/// the plan that it reads: the plan of the head or, for a folder that the diff
/// deletes, the newest plan of the branch.
#[derive(Clone, Debug, Default)]
pub struct PlanScope {
    /// The patterns of `Files` that a record can hold ([`Change::scope_patterns`]),
    /// without the globs of `[change] shared-paths`. Sorted, with no duplicate.
    pub patterns: Vec<String>,
    /// Every code span of `Files` ([`Change::files`]): the view of SD913.
    pub files: Vec<String>,
}

impl PlanScope {
    pub fn of(ch: &Change, corpus: &Corpus) -> PlanScope {
        let mut patterns = ch.scope_patterns(corpus);
        patterns.sort();
        patterns.dedup();
        PlanScope {
            patterns,
            files: ch.files(corpus),
        }
    }
}

/// The words of a SD929 message for a plan that has a pattern that the record
/// has not. [`stale_cause`] reads them.
const SCOPE_MARK: &str = "the scope of the plan has the pattern";

/// The words of a SD929 message for a record that has a pattern that neither the
/// plan nor `[change] shared-paths` has. [`stale_cause`] reads them.
const RECORD_ONLY_MARK: &str = "neither the plan nor `[change] shared-paths` has it";

/// The words of the SD929 message for a changed path that `Files` covers only
/// through a pattern that a record cannot hold. [`stale_cause`] reads them.
const UNCOVERED_MARK: &str = "of the plan only as";

/// The words of the SD929 message for a changed path that only a glob of
/// `[change] shared-paths` covers, a glob that the base configuration has not.
/// [`stale_cause`] reads them.
const SHARED_ADDED_MARK: &str = "shared-paths changed after the acceptance";

/// The words of a SD929 message for code that changed after the acceptance.
const CODE_MARKS: [&str; 2] = ["the hash of the code", "the diff has the code path"];

/// What changed after the acceptance, for the text of `next` and `finish`. It
/// reads a SD929 message of [`stale`]. It gives one phrase for each kind of
/// reason in the message: the scope of the plan, a pattern that the record has
/// and no source has, a name that `Files` has and a record cannot hold, a new
/// glob of `shared-paths`, and the code. The text says what changed, so the user
/// fixes the right thing.
pub fn stale_cause(message: &str) -> String {
    let mut phrases: Vec<&str> = Vec::new();
    if message.contains(SCOPE_MARK) {
        phrases.push("the scope of the plan changed after the acceptance");
    }
    if message.contains(RECORD_ONLY_MARK) {
        phrases
            .push("the record has a pattern that neither the plan nor `[change] shared-paths` has");
    }
    if message.contains(UNCOVERED_MARK) {
        phrases.push("a changed path is in `Files` only as a bare name or a path with a space");
    }
    if message.contains(SHARED_ADDED_MARK) {
        phrases.push("shared-paths changed after the acceptance");
    }
    if CODE_MARKS.iter().any(|m| message.contains(m)) {
        phrases.push("the code changed after the acceptance");
    }
    if phrases.is_empty() {
        phrases.push("the code changed after the acceptance");
    }
    phrases.join(" and ")
}

/// The globs of `[change] shared-paths` in the configuration of `base_rev`, or
/// `None` if the base has no configuration that reads.
pub fn base_shared_paths(git: &Git, base_rev: &str) -> Result<Option<Vec<String>>, String> {
    let text = git
        .read_at(&[(base_rev.to_string(), config::FILE_NAME.to_string())])?
        .into_iter()
        .next()
        .flatten();
    Ok(text
        .and_then(|b| Config::parse(&String::from_utf8_lossy(&b)).ok())
        .map(|c| c.shared_paths))
}

/// Why a record is stale: one text for each reason, empty if the record is
/// fresh. One function decides it for `accept` (the refusal of a second
/// acceptor), SD929 in `check`, `next` and `finish` step 5.2, so no 2 places use
/// 2 rules. A record is stale if:
///
/// - the plan has a pattern that `scope` of the record has not, or the record has
///   a pattern that the plan has not. Both sides omit the globs of
///   `[change] shared-paths` of the head configuration: the record does not mark
///   which globs came from that key, and a change of the key after the
///   acceptance must not make a record stale (design 0.8.0, D3 item 4). So a
///   glob that a later commit removes from the key reads as a glob that the plan
///   lacks, and makes the record stale: write the record again.
/// - a changed code path is in `Files` of the plan only as a pattern that a
///   record cannot hold (a bare name, or a path with a space), and neither
///   `scope` nor `[change] shared-paths` covers it
///   ([`crate::accept::uncovered_path`]),
/// - the hash of `paths` differs from the code that `git` reads (the working
///   tree, or the index for a `staged` `Git`), or
/// - the diff has a code path that `scope` matches and `paths` does not list.
pub fn stale_reasons(
    cfg: &Config,
    git: &Git,
    rec: &Record,
    diff: Option<&Diff>,
    plan: &PlanScope,
    base_rev: Option<&str>,
) -> Result<Vec<String>, String> {
    let mut why: Vec<String> = Vec::new();
    let quote = |v: Vec<&String>| {
        v.iter()
            .map(|p| format!("`{p}`"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let shared = &cfg.shared_paths;
    let plan_side: Vec<&String> = plan
        .patterns
        .iter()
        .filter(|p| !shared.contains(p))
        .collect();
    let rec_side: Vec<&String> = rec.scope.iter().filter(|p| !shared.contains(p)).collect();
    let only_plan: Vec<&String> = plan_side
        .iter()
        .filter(|p| !rec_side.contains(p))
        .copied()
        .collect();
    let only_record: Vec<&String> = rec_side
        .iter()
        .filter(|p| !plan_side.contains(p))
        .copied()
        .collect();
    if !only_plan.is_empty() {
        why.push(format!(
            "{SCOPE_MARK} {}, and `scope` of the record has not",
            quote(only_plan)
        ));
    }
    if !only_record.is_empty() {
        why.push(format!(
            "`scope` of the record has the pattern {}, and {RECORD_ONLY_MARK} (the plan dropped it, or the key lost it)",
            quote(only_record)
        ));
    }
    let entries = tree::entries_of(git, cfg)?;
    let now = record::tree_hash(&entries, &rec.paths);
    // The record is fresh against the tree of the last link of its chain (D73 item 6).
    if !tree::matches(rec.last_tree(), &now) {
        why.push(format!(
            "{} that `paths` lists is {now}, and the record has {}",
            CODE_MARKS[0],
            rec.last_tree()
        ));
    }
    if let Some(diff) = diff {
        let intent_dirs = crate::lifecycle::intent_dirs(cfg);
        let is_code = |p: &str| tree::is_code_path(cfg, p, &intent_dirs);
        let changed: Vec<String> = diff.files.iter().map(|f| f.0.clone()).collect();
        for p in record::select_paths(&rec.scope, &changed, &is_code) {
            if !rec.paths.contains(&p) {
                why.push(format!(
                    "{} `{p}`, which `scope` matches and `paths` does not list",
                    CODE_MARKS[1]
                ));
            }
        }
        // `scope` and the globs of `shared-paths` are what the record covers.
        let mut covered = rec.scope.clone();
        covered.extend(shared.iter().cloned());
        if let Some((p, pattern)) =
            crate::accept::uncovered_path(&plan.files, &covered, &changed, &is_code)
        {
            why.push(format!(
                "the changed code path `{p}` is in `Files` {UNCOVERED_MARK} `{pattern}`, which {}, so `scope` of the record does not cover it; {}",
                crate::accept::scope_cause(&pattern),
                crate::accept::scope_way(&pattern)
            ));
        }
        // A glob of `shared-paths` that the base configuration has not is new
        // since the acceptance. A changed code path that only such a glob covers
        // has no acceptance. A glob that the base has is as old as the
        // record: a base commit that adds one changes no record.
        if let Some(base_shared) = base_rev
            .map(|r| base_shared_paths(git, r))
            .transpose()?
            .flatten()
        {
            let added: Vec<String> = shared
                .iter()
                .filter(|g| !base_shared.contains(g))
                .cloned()
                .collect();
            for p in changed.iter().filter(|p| is_code(p)) {
                if change::in_scope(&rec.scope, p) || change::in_scope(&base_shared, p) {
                    continue;
                }
                if let Some(g) = added
                    .iter()
                    .find(|g| change::in_scope(std::slice::from_ref(g), p))
                {
                    why.push(format!(
                        "the changed code path `{p}` is covered only by the glob `{g}` of `[change] shared-paths`, which the base configuration has not: {SHARED_ADDED_MARK}; accept again, or revert the change of `shared-paths`"
                    ));
                }
            }
        }
    }
    Ok(why)
}

/// SD929: the record is stale ([`stale_reasons`]). `after_finish` chooses the fix
/// that the message names. `None`: the record is fresh.
pub fn stale(
    cfg: &Config,
    git: &Git,
    rec: &Record,
    diff: Option<&Diff>,
    plan: &PlanScope,
    base_rev: Option<&str>,
    after_finish: bool,
) -> Result<Option<String>, String> {
    let why = stale_reasons(cfg, git, rec, diff, plan, base_rev)?;
    if why.is_empty() {
        return Ok(None);
    }
    let slug = &rec.change;
    // `Diag::fix` reads the mark to choose the fix of the same place.
    let fix = if after_finish {
        format!(
            "{STALE_AFTER_FINISH}, delete `acceptance.toml` of the folder of the change, commit, merge the base, run `spec-dna accept {slug}` again from a new brief, and run `spec-dna finish {slug}` again"
        )
    } else {
        format!(
            "{STALE_BEFORE_FINISH}, run `spec-dna accept {slug}` again from a new brief, and run `spec-dna finish {slug}` again"
        )
    };
    Ok(Some(format!(
        "the record of the change `{slug}` is stale: {}; {fix}",
        why.join("; ")
    )))
}

// The compare of the brief (SD928, item 8).

/// The identity of an entry: its roles (joined with `,`), session and brief.
fn triple(a: &Acceptor) -> (String, String, String) {
    (
        a.roles.join(","),
        a.session.clone(),
        a.brief.to_ascii_lowercase(),
    )
}

/// True if `s` is 16 hexadecimal digits.
fn is_hash(s: &str) -> bool {
    s.len() == 16 && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The 3 files of a change in one tree, as text. `None` for a missing file. With
/// them, the configuration of the same tree: the brief reads the key `[change]
/// invariants`, so the compare of an entry needs the key that its adding commit
/// had. `None` means that the tree has no configuration that reads. Then the
/// head configuration replaces it.
struct Files {
    intent: Option<String>,
    spec: Option<String>,
    plan: Option<String>,
    cfg: Option<Config>,
}

fn text_of(b: Option<Vec<u8>>) -> Option<String> {
    b.map(|b| String::from_utf8_lossy(&b).into_owned())
}

/// The `declared` items of the files of a tree, read as `finish` and `check`
/// read a change: the 3 files parse into a corpus of their own. The rule of the
/// pin reads the history of the base (`base_rev`), as for the entry itself.
fn declared_of(
    cfg: &Config,
    git: &Git,
    base_rev: &str,
    folder: &str,
    files: &Files,
) -> Result<accept::Declared, String> {
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
        Some(ch) => declared_for(cfg, git, Some(base_rev), &ch, &corpus),
        None => Ok(accept::Declared::default()),
    }
}

/// Where the tree of an entry comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    Commit(String),
    /// No commit has the entry: the index with `--staged`, else the working
    /// tree.
    Head,
}

/// The 3 files of a change and the configuration of one source (a commit, or the
/// head tree).
fn read_files(git: &Git, folder: &str, source: &Source) -> Result<Files, String> {
    let names = ["intent.md", "spec.md", "plan.md"];
    let mut texts: Vec<Option<String>> = Vec::new();
    match source {
        Source::Commit(c) => {
            let mut specs: Vec<(String, String)> = names
                .iter()
                .map(|n| (c.clone(), format!("{folder}/{n}")))
                .collect();
            specs.push((c.clone(), config::FILE_NAME.to_string()));
            texts.extend(git.read_at(&specs)?.into_iter().map(text_of));
        }
        Source::Head => {
            for n in names {
                // A working tree with `core.autocrlf` has CRLF, and `accept --brief`
                // reads the files of `HEAD` with LF (ADR 0019).
                let read = git.head_file_limited(&format!("{folder}/{n}"), DOC_READ_LIMIT)?;
                texts.push(
                    text_of(read).map(|t| {
                        String::from_utf8_lossy(&record::to_lf(t.as_bytes())).into_owned()
                    }),
                );
            }
        }
    }
    let mut it = texts.into_iter();
    Ok(Files {
        intent: it.next().flatten(),
        spec: it.next().flatten(),
        plan: it.next().flatten(),
        cfg: it.next().flatten().and_then(|t| Config::parse(&t).ok()),
    })
}

/// The identity of a follow-up: its session and brief.
fn follow_identity(u: &record::FollowUp) -> (String, String) {
    (u.session.clone(), u.brief.to_ascii_lowercase())
}

/// The problems of the compare: for each entry that the branch adds to the
/// record, the brief of the entry against the brief that the tree of its adding
/// commit gives (design 0.8.0, D3 item 8).
///
/// - The compare skips an entry that the record of the base has (same role,
///   session and brief). The pull request of that entry compared it.
/// - The adding commit is the oldest commit of the latest unbroken run of the
///   commits of the branch whose record holds an entry with the same triple.
///   `Git::first_parent_commits` gives the commits: `<base>..<tip>`, first
///   parent, the oldest first. On a test merge the tip is the second parent of
///   `HEAD`. An entry that no commit holds takes the index with `--staged`, else
///   the working tree.
/// - A shallow clone that cuts the commits of the branch gives one problem that
///   names `fetch-depth: 0`.
pub fn brief_problems(
    cfg: &Config,
    git: &Git,
    base_rev: &str,
    folder: &str,
    rec: &Record,
) -> Result<Vec<String>, String> {
    let rec_path = format!("{folder}/{}", record::FILE_NAME);
    let slug = folder.rsplit('/').next().unwrap_or(folder);
    let base_raw = git
        .read_at(&[(base_rev.to_string(), rec_path.clone())])?
        .into_iter()
        .next()
        .flatten();
    let base_rec = base_raw
        .as_ref()
        .and_then(|b| Record::parse(&String::from_utf8_lossy(b)).ok());
    let base_triples: HashSet<(String, String, String)> = base_rec
        .as_ref()
        .map(|r| r.acceptors.iter().map(triple).collect())
        .unwrap_or_default();
    // The entries to compare: those that the base does not have, and whose
    // `brief` is a hash (the rules of a valid record report any other value).
    let entries: Vec<(usize, &Acceptor)> = rec
        .acceptors
        .iter()
        .enumerate()
        .filter(|(_, a)| is_hash(&a.brief) && !base_triples.contains(&triple(a)))
        .collect();
    // The follow-ups that the base does not have (D73 item 5).
    let base_follow: HashSet<(String, String)> = base_rec
        .as_ref()
        .map(|r| r.follow_ups.iter().map(follow_identity).collect())
        .unwrap_or_default();
    let follow: Vec<usize> = rec
        .follow_ups
        .iter()
        .enumerate()
        .filter(|(_, u)| is_hash(&u.brief) && !base_follow.contains(&follow_identity(u)))
        .map(|(j, _)| j)
        .collect();
    // The rule of D74 item 2 reads the history for a record that says `accepted`.
    let rule2 = rec.result == "accepted";
    if entries.is_empty() && follow.is_empty() && !rule2 {
        return Ok(Vec::new());
    }
    if git.range_is_cut(base_rev) {
        if entries.is_empty() && follow.is_empty() {
            return Ok(Vec::new());
        }
        let mut list: Vec<String> = entries
            .iter()
            .map(|(i, _)| format!("acceptor {}", i + 1))
            .collect();
        list.extend(follow.iter().map(|j| format!("follow-up {}", j + 1)));
        return Ok(vec![format!(
            "the history of the branch (`{base_rev}` to its tip) is cut (a shallow clone), so the brief of {} cannot be compared with its adding commit; fetch the full history (`fetch-depth: 0`)",
            list.join(", ")
        )]);
    }
    let commits = git.first_parent_commits(base_rev)?;
    let specs: Vec<(String, String)> = commits
        .iter()
        .map(|c| (c.clone(), rec_path.clone()))
        .collect();
    let raw = git.read_at(&specs)?;
    let mut states: Vec<State> = vec![state_of(base_raw.as_deref())];
    states.extend(raw.iter().map(|b| state_of(b.as_deref())));
    let per_rec: Vec<Option<Record>> = raw
        .into_iter()
        .map(|t| t.and_then(|b| Record::parse(&String::from_utf8_lossy(&b)).ok()))
        .collect();
    let per_commit: Vec<HashSet<(String, String, String)>> = per_rec
        .iter()
        .map(|r| {
            r.as_ref()
                .map(|r| r.acceptors.iter().map(triple).collect())
                .unwrap_or_default()
        })
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
            cache.push((key.clone(), read_files(git, folder, &source)?));
        }
        let files = &cache.iter().find(|(k, _)| *k == key).expect("cached").1;
        let (Some(intent), Some(plan)) = (&files.intent, &files.plan) else {
            out.push(format!(
                "the brief of acceptor {} cannot be computed again: the tree of {place} has no `intent.md` or no `plan.md` in `{folder}`; remove the entry, and accept again from a new brief",
                i + 1
            ));
            continue;
        };
        let at = files.cfg.as_ref().unwrap_or(cfg);
        let decl = declared_of(at, git, base_rev, folder, files)?;
        // An entry of a record of format 1 has the brief of format 1.
        let format = if rec.format == record::FORMAT_1 {
            record::BRIEF_FORMAT_1
        } else {
            record::BRIEF_FORMAT
        };
        let text = accept::brief_text(
            at,
            slug,
            format,
            &a.roles,
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
    // No edit clears a rejection (D74 item 2).
    if rule2 {
        let runs = Runs {
            base_rev: base_rev.to_string(),
            commits: commits.clone(),
            lives: lives(&states, head_state(Some(rec))),
        };
        if let Some(at) = runs.head_life_rejected_at() {
            out.push(rejection_not_cleared(&at));
        }
    }
    if !rec.follow_ups.is_empty() {
        let chain = Chain {
            cfg,
            git,
            base_rev,
            folder,
            rec,
            commits,
            per_rec,
            base_rec,
        };
        out.extend(chain.problems(&follow)?);
    }
    Ok(out)
}

// The count of the rejected records (design 0.8.0, D74).

/// What one commit (or the head tree) has of the record: the file, and its top
/// result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct State {
    pub exists: bool,
    /// True if the file reads and its top `result` is `rejected`.
    pub rejected: bool,
}

/// The state of a file that git gave, or `None` for an absent file.
pub fn state_of(bytes: Option<&[u8]>) -> State {
    match bytes {
        None => State {
            exists: false,
            rejected: false,
        },
        Some(b) => State {
            exists: true,
            rejected: says_rejected(&String::from_utf8_lossy(b)),
        },
    }
}

/// True if the top-level `result` of a record text is `rejected`. The count of the
/// stop rule reads it leniently: a record that the strict reader refuses (an unknown
/// key, a syntax error) still counts, or an agent could keep it from the count. The
/// text is read as generic TOML first. If even that fails, a scan of the lines before
/// the first table reads `result = "rejected"`, which is the strictest honest reading.
/// SD928 keeps the strict reader for the rules of a valid record.
fn says_rejected(text: &str) -> bool {
    if let Ok(t) = text.parse::<toml::Table>() {
        return t.get("result").and_then(toml::Value::as_str) == Some("rejected");
    }
    for line in text.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            break;
        }
        if let Some(v) = l.strip_prefix("result").map(str::trim_start)
            && let Some(v) = v.strip_prefix('=')
        {
            let v = v.split('#').next().unwrap_or("").trim();
            if v == "\"rejected\"" || v == "'rejected'" {
                return true;
            }
        }
    }
    false
}

/// The state of the record of the head, if the head has one.
pub fn head_state(rec: Option<&Record>) -> Option<State> {
    rec.map(|r| State {
        exists: true,
        rejected: r.result == "rejected",
    })
}

/// One life of a record: an unbroken run of commits of the branch that hold the
/// file `<folder>/acceptance.toml` (D74 item 1). A commit with no such file ends
/// the life, and the next record starts a new one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Life {
    pub first: Pos,
    /// The first commit where the record says `rejected`. A life with `Some` counts
    /// as one rejected record, whatever number of commits or entries it has.
    pub rejected_at: Option<Pos>,
}

/// The lives of a record. `states` are the state in the base tree, then the state
/// at each commit of the branch, the oldest first. `head` is the record of the head
/// tree: it continues the life of the last commit if that commit has the file, else
/// it starts a life. A record that the base holds starts a life that counts once,
/// also when the branch keeps it.
pub fn lives(states: &[State], head: Option<State>) -> Vec<Life> {
    let mut out: Vec<Life> = Vec::new();
    let mut open = false;
    let all = states
        .iter()
        .enumerate()
        .map(|(i, s)| {
            (
                if i == 0 {
                    Pos::Base
                } else {
                    Pos::Commit(i - 1)
                },
                *s,
            )
        })
        .chain(head.map(|h| (Pos::Head, h)));
    for (pos, s) in all {
        if !s.exists {
            open = false;
            continue;
        }
        if !open {
            out.push(Life {
                first: pos,
                rejected_at: None,
            });
            open = true;
        }
        let life = out.last_mut().expect("a life is open");
        if s.rejected && life.rejected_at.is_none() {
            life.rejected_at = Some(pos);
        }
    }
    out
}

/// The lives of the record of a change, with the commits that name them.
pub struct Runs {
    base_rev: String,
    commits: Vec<String>,
    lives: Vec<Life>,
}

impl Runs {
    /// Reads the record at the base and at each commit of the branch, in one batch.
    /// `head` is the record of the head. `None` if a shallow clone cuts the branch:
    /// the count is not known then.
    pub fn load(
        git: &Git,
        base_rev: &str,
        folder: &str,
        head: Option<State>,
    ) -> Result<Option<Runs>, String> {
        if git.range_is_cut(base_rev) {
            return Ok(None);
        }
        let rec_path = format!("{folder}/{}", record::FILE_NAME);
        let commits = git.first_parent_commits(base_rev)?;
        let mut specs: Vec<(String, String)> = vec![(base_rev.to_string(), rec_path.clone())];
        specs.extend(commits.iter().map(|c| (c.clone(), rec_path.clone())));
        let states: Vec<State> = git
            .read_at(&specs)?
            .iter()
            .map(|b| state_of(b.as_deref()))
            .collect();
        Ok(Some(Runs {
            base_rev: base_rev.to_string(),
            lives: lives(&states, head),
            commits,
        }))
    }

    /// The number of records (lives).
    pub fn total(&self) -> usize {
        self.lives.len()
    }

    /// The number of rejected records.
    pub fn rejected_total(&self) -> usize {
        self.lives
            .iter()
            .filter(|l| l.rejected_at.is_some())
            .count()
    }

    /// The first 7 characters of the commit of a place.
    fn id(&self, p: Pos) -> String {
        let full = match p {
            Pos::Base => self.base_rev.as_str(),
            Pos::Commit(i) => self.commits[i].as_str(),
            Pos::Head => return "HEAD".to_string(),
        };
        full[..full.len().min(7)].to_string()
    }

    /// The rejected records that count for the stop: the lives that say `rejected`
    /// after the newest approval commit of `plan.md` (D74 item 4). With no
    /// approval in the branch, or with `base`, each rejected life counts. Each item
    /// is the number of the life and the commit where it said `rejected`.
    pub fn rejected_since(&self, approval: Option<(&Branch, &str)>) -> Vec<(usize, String)> {
        let last = approval.and_then(|(b, path)| b.last_approval(path).map(|a| (b, a)));
        self.lives
            .iter()
            .enumerate()
            .filter_map(|(i, l)| l.rejected_at.map(|p| (i, p)))
            .filter(|(_, p)| match (last, p) {
                (None, _) => true,
                (Some(_), Pos::Base) => false,
                (Some(_), Pos::Head) => true,
                (Some((b, a)), Pos::Commit(i)) => {
                    b.position(&self.commits[*i]).is_none_or(|x| x > a)
                }
            })
            .map(|(i, p)| (i, self.id(p)))
            .collect()
    }

    /// The index of the last life, which holds the record of the head, or the last
    /// record of the branch.
    pub fn last_life(&self) -> Option<usize> {
        self.lives.len().checked_sub(1)
    }

    /// The commit where the life that holds the head said `rejected`, if it said so
    /// in a commit before the head tree.
    pub fn head_life_rejected_at(&self) -> Option<String> {
        match self.lives.last()?.rejected_at? {
            Pos::Head => None,
            p => Some(self.id(p)),
        }
    }
}

/// The steps to the word of the owner with `approval = "branch"` (D80 item 3). The
/// message of SD941 and the item of `next` use this text, so they say the same.
fn owner_word_steps(slug: &str) -> String {
    format!(
        "commit `status: draft` of plan.md on its own, skipping the commit hook for that commit only and naming the skip in the body of the commit (the draft commit gives SD401 and SD910 reason 9), then the approver runs `spec-dna approve {slug} plan`"
    )
}

/// The text of SD941 (D74 item 6): the record comes after 2 rejected records, and
/// no approval commit of `plan.md` comes after the second. `since` is
/// [`Runs::rejected_since`]; the record of the head is the last life, which the 2
/// do not include. `None` when the rule does not hold.
pub fn rerun_message(slug: &str, runs: &Runs, since: &[(usize, String)]) -> Option<String> {
    let last = runs.last_life()?;
    let earlier: Vec<&str> = since
        .iter()
        .filter(|(i, _)| *i != last)
        .map(|(_, id)| id.as_str())
        .collect();
    (earlier.len() >= 2).then(|| {
        format!(
            "the record of the change `{slug}` comes after {} rejected records ({}), and no approval commit of `plan.md` comes after the second; a third run needs the word of the owner: {} (both codes are gone)",
            earlier.len(),
            earlier.join(", "),
            owner_word_steps(slug)
        )
    })
}

/// The text of SD941 for a shallow clone that cuts the branch: the count is not
/// known, which is the stricter result (D74 item 3).
pub fn rerun_cut_message(slug: &str, base_rev: &str) -> String {
    format!(
        "the history of the branch (`{base_rev}` to its tip) is cut (a shallow clone), so the rejected records of the change `{slug}` cannot be counted; fetch the full history (`fetch-depth: 0`)"
    )
}

/// The text of the item of `next` that waits for the owner (D74 item 5). `since`
/// is [`Runs::rejected_since`]. With `branch`, it adds the way to the word of the
/// owner.
pub fn wait_text(slug: &str, since: &[(usize, String)], branch: bool) -> String {
    let ids: Vec<&str> = since.iter().map(|(_, id)| id.as_str()).collect();
    let mut t = format!(
        "wait for the owner: {} acceptance records of {slug} say `rejected` ({}); a third run needs the word of the owner",
        ids.len(),
        ids.join(", ")
    );
    if branch {
        t.push_str(&format!(": {}", owner_word_steps(slug)));
    }
    t
}

/// The problem of D74 item 2: no edit clears a rejection. The record of the head
/// says `accepted`, and an earlier commit of its life said `rejected`.
pub fn rejection_not_cleared(rejected_at: &str) -> String {
    format!(
        "the record said `rejected` at {rejected_at}; a later edit cannot accept it: fix the code, delete the record, commit, and accept again"
    )
}

/// What the stop rule finds for one change (D74): the numbers for the line
/// `Acceptance runs:`, the rejected records that count for the stop, and the text
/// of SD941.
#[derive(Clone, Debug, Default)]
pub struct StopFacts {
    /// The number of records of the branch, and how many say `rejected`.
    pub total: usize,
    pub rejected_total: usize,
    /// The rejected records after the newest approval of the plan ([`Runs::rejected_since`]).
    pub since: Vec<(usize, String)>,
    /// SD941 (`branch` only): the text, when the record needs the word of the owner.
    pub rerun: Option<String>,
    /// True if the approval of the branch is the rule in force.
    pub branch: bool,
}

/// Reads the stop facts of the change in `folder`. `head` is the record of the head
/// (`None` where the diff deletes the folder). `trees` are the base tree and the
/// head tree for the walk of the approvals (D74 item 4), which the rule reads only
/// when 2 records say `rejected` and the mode is `branch`. A shallow clone that cuts
/// the branch gives no count, and with `branch` it gives SD941 with the fix
/// `fetch-depth: 0`.
pub fn stop_facts(
    cfg: &Config,
    nc: &NeedConfig,
    git: &Git,
    base_rev: &str,
    folder: &str,
    head: Option<State>,
    trees: (&Corpus, &Corpus),
) -> Result<StopFacts, String> {
    let slug = folder.rsplit('/').next().unwrap_or(folder);
    let branch = nc.approval_branch(cfg);
    let Some(runs) = Runs::load(git, base_rev, folder, head)? else {
        return Ok(StopFacts {
            rerun: branch.then(|| rerun_cut_message(slug, base_rev)),
            branch,
            ..StopFacts::default()
        });
    };
    let plan = format!("{folder}/plan.md");
    let walk = (branch && runs.rejected_total() >= 2)
        .then(|| Branch::read(cfg, git, base_rev, trees.0, trees.1));
    let since = runs.rejected_since(walk.as_ref().map(|b| (b, plan.as_str())));
    let rerun = if branch {
        rerun_message(slug, &runs, &since)
    } else {
        None
    };
    Ok(StopFacts {
        total: runs.total(),
        rejected_total: runs.rejected_total(),
        since,
        rerun,
        branch,
    })
}

// The chain of the record and its follow-ups (design 0.8.0, D73).

/// Where a link of the chain has its adding commit (D73 item 3). The order is the
/// order of the history: the base, a commit of the branch (by its index in the
/// list of the commits), then the head tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Pos {
    Base,
    Commit(usize),
    Head,
}

/// The oldest commit of the latest unbroken run of commits for which `has` is true,
/// as in D3 item 8.
fn run_start(has: &[bool]) -> Option<usize> {
    let mut run: Option<usize> = None;
    for (i, h) in has.iter().enumerate() {
        if *h {
            run.get_or_insert(i);
        } else {
            run = None;
        }
    }
    run
}

/// The diff of a fix, as the follow-up brief and the checks read it.
#[derive(Clone, Debug, Default)]
pub struct Fix {
    /// The hash of `paths` in the tree that the fix binds.
    pub to_tree: String,
    /// The paths of `paths` whose entry (mode and object id) differs.
    pub paths: Vec<String>,
    /// The part `=== fix ===`: one line for each path.
    pub lines: Vec<String>,
    /// The text of `git diff` with the fixed options.
    pub diff: String,
    /// The paths of the fix that are binary files.
    pub binary: Vec<String>,
}

/// The record, the commits of the branch and the records at each of them: what the
/// chain of the follow-ups reads. `brief_problems` builds it from the reads that it
/// makes for the entries, and `accept --follow-up` loads it ([`Chain::load`]).
pub struct Chain<'a> {
    pub cfg: &'a Config,
    pub git: &'a Git,
    pub base_rev: &'a str,
    pub folder: &'a str,
    pub rec: &'a Record,
    commits: Vec<String>,
    per_rec: Vec<Option<Record>>,
    base_rec: Option<Record>,
}

impl<'a> Chain<'a> {
    /// Reads the commits of the branch and the record at the base and at each of
    /// them. A shallow clone that cuts the branch gives an error that names
    /// `fetch-depth: 0`.
    pub fn load(
        cfg: &'a Config,
        git: &'a Git,
        base_rev: &'a str,
        folder: &'a str,
        rec: &'a Record,
    ) -> Result<Chain<'a>, String> {
        if git.range_is_cut(base_rev) {
            return Err(format!(
                "the history of the branch (`{base_rev}` to its tip) is cut (a shallow clone), so the commits of the follow-ups cannot be read; fetch the full history (`fetch-depth: 0`)"
            ));
        }
        let rec_path = format!("{folder}/{}", record::FILE_NAME);
        let commits = git.first_parent_commits(base_rev)?;
        let mut specs: Vec<(String, String)> = vec![(base_rev.to_string(), rec_path.clone())];
        specs.extend(commits.iter().map(|c| (c.clone(), rec_path.clone())));
        let mut all = git
            .read_at(&specs)?
            .into_iter()
            .map(|t| t.and_then(|b| Record::parse(&String::from_utf8_lossy(&b)).ok()));
        let base_rec = all.next().flatten();
        Ok(Chain {
            cfg,
            git,
            base_rev,
            folder,
            rec,
            commits,
            per_rec: all.collect(),
            base_rec,
        })
    }

    fn short(c: &str) -> &str {
        &c[..c.len().min(12)]
    }

    /// The position of the adding commit of the record (the oldest adding commit of
    /// its entries `[[acceptor]]`).
    pub fn record_pos(&self) -> Pos {
        let base: HashSet<(String, String, String)> = self
            .base_rec
            .as_ref()
            .map(|r| r.acceptors.iter().map(triple).collect())
            .unwrap_or_default();
        self.rec
            .acceptors
            .iter()
            .map(|a| {
                let t = triple(a);
                if base.contains(&t) {
                    return Pos::Base;
                }
                let has: Vec<bool> = self
                    .per_rec
                    .iter()
                    .map(|r| {
                        r.as_ref()
                            .is_some_and(|r| r.acceptors.iter().any(|x| triple(x) == t))
                    })
                    .collect();
                run_start(&has).map_or(Pos::Head, Pos::Commit)
            })
            .min()
            .unwrap_or(Pos::Head)
    }

    /// The position of the adding commit of each follow-up. A follow-up that the
    /// record of the base has takes the base (D73 item 3).
    pub fn follow_positions(&self) -> Vec<Pos> {
        let base: HashSet<(String, String)> = self
            .base_rec
            .as_ref()
            .map(|r| r.follow_ups.iter().map(follow_identity).collect())
            .unwrap_or_default();
        self.rec
            .follow_ups
            .iter()
            .map(|u| {
                let id = follow_identity(u);
                if base.contains(&id) {
                    return Pos::Base;
                }
                let has: Vec<bool> = self
                    .per_rec
                    .iter()
                    .map(|r| {
                        r.as_ref()
                            .is_some_and(|r| r.follow_ups.iter().any(|x| follow_identity(x) == id))
                    })
                    .collect();
                run_start(&has).map_or(Pos::Head, Pos::Commit)
            })
            .collect()
    }

    pub fn source(&self, p: Pos) -> Source {
        match p {
            Pos::Base => Source::Commit(self.base_rev.to_string()),
            Pos::Commit(i) => Source::Commit(self.commits[i].clone()),
            Pos::Head => Source::Head,
        }
    }

    fn place(&self, s: &Source) -> String {
        match s {
            Source::Commit(c) => format!("its adding commit {}", Self::short(c)),
            Source::Head => format!(
                "the {} that holds the entry (no commit has it yet)",
                if self.git.is_staged() {
                    "index"
                } else {
                    "working tree"
                }
            ),
        }
    }

    /// The code entries of `paths` in a source: the tree of a commit, or the head
    /// tree (the index with `--staged`, else the working tree).
    fn entries(&self, s: &Source) -> Result<Vec<tree::Entry>, String> {
        match s {
            Source::Commit(c) => self.git.tree_entries_at(c, &self.rec.paths),
            Source::Head => tree::entries_of(self.git, self.cfg),
        }
    }

    /// The hash of `paths` in a source.
    pub fn tree_in(&self, s: &Source) -> Result<String, String> {
        Ok(record::tree_hash(&self.entries(s)?, &self.rec.paths))
    }

    /// The diff of the fix between 2 sources (D73 item 4).
    pub fn fix(&self, from: &Source, to: &Source) -> Result<Fix, String> {
        let (a, b) = (self.entries(from)?, self.entries(to)?);
        let find = |v: &[tree::Entry], p: &str| {
            v.iter()
                .find(|e| e.2 == p.as_bytes())
                .map(|(m, o, _)| (m.clone(), o.clone()))
        };
        let mut fix = Fix {
            to_tree: record::tree_hash(&b, &self.rec.paths),
            ..Fix::default()
        };
        for p in &self.rec.paths {
            let (x, y) = (find(&a, p), find(&b, p));
            if x == y {
                continue;
            }
            let side = |e: Option<(String, String)>| {
                e.map_or("absent absent".to_string(), |(m, o)| format!("{m} {o}"))
            };
            fix.lines.push(format!("{p} {} {}", side(x), side(y)));
            fix.paths.push(p.clone());
        }
        if let Source::Commit(c) = from {
            let to_rev = match to {
                Source::Commit(t) => Some(t.as_str()),
                Source::Head => None,
            };
            fix.diff =
                String::from_utf8_lossy(&self.git.follow_diff(c, to_rev, &fix.paths)?).into_owned();
            fix.binary = self.git.follow_binary_paths(c, to_rev, &fix.paths)?;
        }
        Ok(fix)
    }

    /// The number of bytes of the diff of the whole chain: `paths` from the commit
    /// of the record to `to`. Zero when the record has no commit yet.
    pub fn chain_diff_len(&self, to: &Source) -> Result<usize, String> {
        let Source::Commit(c0) = self.source(self.record_pos()) else {
            return Ok(0);
        };
        let to_rev = match to {
            Source::Commit(t) => Some(t.as_str()),
            Source::Head => None,
        };
        Ok(self.git.follow_diff(&c0, to_rev, &self.rec.paths)?.len())
    }

    /// The follow-up brief for the follow-up after the first `before` follow-ups.
    /// `from_tree` is the tree of the last link, `to` the source that the follow-up
    /// binds, and `fix` the diff of the fix. The files and the keys are those of
    /// `to`, as the compare of SD928 reads them.
    pub fn brief(
        &self,
        before: usize,
        from_tree: &str,
        to: &Source,
        fix: &Fix,
    ) -> Result<String, String> {
        let files = read_files(self.git, self.folder, to)?;
        let Some(plan) = &files.plan else {
            return Err(format!(
                "the tree of {} has no `plan.md` in `{}`",
                self.place(to),
                self.folder
            ));
        };
        let at = files.cfg.as_ref().unwrap_or(self.cfg);
        let decl = declared_of(at, self.git, self.base_rev, self.folder, &files)?;
        let deviations: Vec<String> = decl
            .deviations
            .iter()
            .map(|(id, kind, text)| format!("{id} ({kind}): {text}"))
            .collect();
        let delta = files
            .spec
            .as_deref()
            .map(|s| record::section_text(s, "Delta"))
            .unwrap_or_default();
        let proof = record::section_text(plan, "Proof");
        let notes: Vec<record::BriefNote> = self
            .rec
            .open_notes(before)
            .into_iter()
            .map(|n| {
                let f = &self.rec.findings[n - 1];
                record::BriefNote {
                    number: n,
                    path: &f.path,
                    line: f.line,
                    text: &f.text,
                }
            })
            .collect();
        let slug = self.folder.rsplit('/').next().unwrap_or(self.folder);
        Ok(record::follow_up_brief(&record::FollowUpBrief {
            slug,
            delta: &delta,
            invariants: &decl.invariants,
            deviations: &deviations,
            proof: &proof,
            notes: &notes,
            from_tree,
            to_tree: &fix.to_tree,
            fix: &fix.lines,
            diff: &fix.diff,
        }))
    }

    /// The problems of the follow-ups that the branch adds, by the rules of D73
    /// item 5. `follow` lists their indexes. A follow-up that the base has was
    /// compared in its own pull request, but its `closes` count for the rule of the
    /// note that 2 follow-ups close.
    pub fn problems(&self, follow: &[usize]) -> Result<Vec<String>, String> {
        let rec = self.rec;
        let record_pos = self.record_pos();
        let positions = self.follow_positions();
        let mut closed: Vec<i64> = Vec::new();
        let mut out: Vec<String> = Vec::new();
        for (j, u) in rec.follow_ups.iter().enumerate() {
            let before = closed.clone();
            closed.extend(u.closes.iter().copied());
            if !follow.contains(&j) {
                continue;
            }
            let at = format!("follow-up {}", j + 1);
            // `closes`.
            if u.closes.is_empty() {
                out.push(format!(
                    "{at}: `closes` is empty; it names each note that the fix closes, at least one"
                ));
            }
            let mut seen: Vec<i64> = Vec::new();
            for n in &u.closes {
                match usize::try_from(*n).ok().and_then(|i| rec.findings.get(i.wrapping_sub(1))) {
                    None => out.push(format!("{at}: `closes` names the finding {n}, and the record has no such finding")),
                    Some(f) if f.severity != record::NOTE => out.push(format!(
                        "{at}: `closes` names the finding {n}, which is not a note (a follow-up closes a note, never a blocker)"
                    )),
                    Some(_) => {}
                }
                if before.contains(n) || seen.contains(n) {
                    out.push(format!(
                        "{at}: `closes` names the note {n} more than once in the chain"
                    ));
                }
                seen.push(*n);
            }
            // The 2 links and the fix.
            let (prev_pos, prev_tree, prev_name) = if j == 0 {
                (record_pos, rec.tree.as_str(), "the record".to_string())
            } else {
                (
                    positions[j - 1],
                    rec.follow_ups[j - 1].tree.as_str(),
                    format!("follow-up {}", j),
                )
            };
            let (prev, cur) = (self.source(prev_pos), self.source(positions[j]));
            let prev_now = self.tree_in(&prev)?;
            if !tree::matches(prev_tree, &prev_now) {
                out.push(format!(
                    "{at}: {prev_name} has `tree` {prev_tree}, and the code of its commit has the hash {prev_now}"
                ));
            }
            let fix = self.fix(&prev, &cur)?;
            if !tree::matches(&u.tree, &fix.to_tree) {
                out.push(format!(
                    "{at}: `tree` is {}, and the code of {} has the hash {}",
                    u.tree,
                    self.place(&cur),
                    fix.to_tree
                ));
            }
            if fix.paths.is_empty() {
                out.push(format!(
                    "{at}: the diff of the fix is empty: the code of `paths` is the same in the commit of {prev_name} and in its own commit"
                ));
            } else if !u
                .closes
                .iter()
                .filter_map(|n| {
                    usize::try_from(*n)
                        .ok()
                        .and_then(|i| rec.findings.get(i.wrapping_sub(1)))
                })
                .any(|f| fix.paths.contains(&f.path))
            {
                out.push(format!(
                    "{at}: the diff of the fix changes no path of a note that `closes` names"
                ));
            }
            for p in &fix.binary {
                out.push(format!(
                    "{at}: the diff of the fix has the binary file `{p}`, and the budget measures text; a full run follows"
                ));
            }
            let len = self.chain_diff_len(&cur)?;
            if len > record::MAX_CHAIN_DIFF {
                out.push(format!(
                    "{at}: the diff of the chain has {len} bytes, more than {} bytes for the whole chain; a full run follows",
                    record::MAX_CHAIN_DIFF
                ));
            }
            // The brief of the tree of its commit.
            let want = record::follow_up_brief_hash(&self.brief(j, prev_tree, &cur, &fix)?)
                .unwrap_or_default();
            if !want.eq_ignore_ascii_case(&u.brief) {
                out.push(format!(
                    "the brief of follow-up {} is not the follow-up brief of {}; remove the entry, and run `spec-dna accept {} --follow-up` again from a new brief",
                    j + 1,
                    self.place(&cur),
                    self.folder.rsplit('/').next().unwrap_or(self.folder)
                ));
            }
        }
        Ok(out)
    }
}

// The bytes of an archive file (SD935).

/// The text of the folder record that `finish` moved to `archive`: the newest
/// text of `<folder>/acceptance.toml` in the commits of the branch (see `Git::first_parent_commits`; first
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

/// The words of the SD935 message for a cut history. `Diag::fix` reads them, so
/// the finding for a cut history gets its own fix text.
pub const CUT_MARK: &str = "is cut (a shallow clone)";

/// The SD935 message if the archive file `archive`, which the diff adds with
/// the bytes `bytes`, is not the record that `finish` moves: the bytes differ
/// from the folder record, or no folder record is in the history of the branch
/// or in the base. One rule applies to the first commit of `finish` and to a
/// later commit that edits the file: `finish` writes the folder record, byte
/// for byte. In a shallow clone that cuts the commits of the branch, the
/// record cannot be found: the message then names the cause and `fetch-depth: 0`.
pub fn archive_differs(
    git: &Git,
    base_rev: &str,
    folder: &str,
    archive: &str,
    bytes: &[u8],
) -> Result<Option<String>, String> {
    let rec_path = format!("{folder}/{}", record::FILE_NAME);
    // A cut history (a shallow clone) hides the folder record. SD928 compares
    // only a record in a folder, and after `finish` there is none, so SD935 itself
    // names the cause. A repository that is not shallow is never cut.
    if git.range_is_cut(base_rev) {
        return Ok(Some(format!(
            "the history of the branch (`{base_rev}` to its tip) {CUT_MARK}, so the bytes of the archive file `{archive}` cannot be compared with `{rec_path}`; fetch the full history (`fetch-depth: 0`)"
        )));
    }
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
    /// The file `<folder>/acceptance.toml` of the working tree. The read has a
    /// size limit and never follows a link.
    pub record: RecordFile,
    pub need: Need,
    /// The class of the change for the acceptance ([`record_class`]). `finish`
    /// reads it for SD933 and for SD620.
    pub class: Class,
    /// The roles of `[owners]` that the change needs, for SD933 and SD927.
    pub roles: Vec<String>,
    /// The value of `[change] higher-risk-acceptors` that SD933 reads: the larger
    /// of the head and the base ([`NeedConfig`]).
    pub acceptors: u32,
    /// The configuration that decides the codes of the group `acceptance`
    /// ([`NeedConfig::gate`]).
    pub gate: Config,
    /// The records of the branch and the rejected ones, for the line `Acceptance
    /// runs:` (D74 item 7). `Some` when the branch has 2 or more.
    pub runs: Option<(usize, usize)>,
    /// The text of SD941, if the record needs the word of the owner.
    pub rerun: Option<String>,
    /// SD932 and SD934, with `[change] approval = \"branch\"` ([`ack_facts`]).
    pub ack: AckFacts,
    /// The problems of the compare of the briefs (SD928).
    pub brief_problems: Vec<String>,
    /// The reason of SD929, if the record is stale.
    pub stale: Option<String>,
    /// The findings of the sections, and the items that the record answers.
    pub sections: Sections,
    /// The path of the archive file that `finish` writes.
    pub archive: String,
    /// The ignore rule of git for that path, if there is one.
    pub ignored: Option<String>,
}

/// The record file of a change folder in the working tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecordFile {
    /// The folder has no record.
    Absent,
    /// The text of the record.
    Text(String),
    /// A link, another file that is not a regular file, or a file over
    /// [`READ_LIMIT`]: the message for SD928.
    Unreadable(String),
}

/// The rule "a change that needs a record" for a change of the head tree: the
/// one function that `finish` and `next` use (design 0.8.0, D3 item 1 and D70).
/// It returns `None` when the rule does not apply: the change has no plan, it is
/// closed, or git names no lifecycle base. Else it returns the need and the class
/// for the acceptance, also when no record is needed, because SD620 reads the
/// class. `nc` is [`need_config`] of the run. `base` and `base_rev` are the
/// lifecycle tree and its commit. `diff` is the diff of the head against the base.
#[allow(clippy::too_many_arguments)]
pub fn need_for_change(
    repo: &Repo,
    cfg: &Config,
    nc: &NeedConfig,
    ch: &Change,
    head: &Corpus,
    base: Option<&Corpus>,
    base_rev: Option<&str>,
    diff: Option<&Diff>,
) -> Result<Option<Decision>, Fatal> {
    let (Some(plan), Some(base), Some(base_rev)) = (&ch.plan, base, base_rev) else {
        return Ok(None);
    };
    if ch.intent_status() == "closed" {
        return Ok(None);
    }
    let plan_path = &head.docs[plan.doc as usize].path;
    let minimum = || {
        let (head_layers, base_layers) = (
            crate::check::doc_layers(cfg, head),
            crate::check::doc_layers(cfg, base),
        );
        let none = Diff::default();
        let diff = diff.unwrap_or(&none);
        let facts = crate::lifecycle::MinimumFacts {
            config: cfg,
            high_risk_layers: &nc.high_risk_layers,
            high_risk_paths: &nc.high_risk_paths,
            head,
            base,
            head_layers: &head_layers,
            base_layers: &base_layers,
            diff,
        };
        // No other change touches the diff: each path of the diff is a path of
        // this change.
        let sole = !change::collect(head)
            .iter()
            .filter(|c| c.dir != ch.dir)
            .any(|c| diff.under(&c.dir).next().is_some());
        crate::lifecycle::minimum_is_higher_risk(&facts, ch, head, false, sole)
    };
    // The roles of `[owners]` that match a role path of the change (D70 item 7).
    let roles = if nc.has_owners(cfg) {
        let none = Diff::default();
        let diff = diff.unwrap_or(&none);
        let scope = accept::scope_of(cfg, ch, head);
        let other_scopes: Vec<Vec<String>> = change::collect(head)
            .iter()
            .filter(|c| c.dir != ch.dir && diff.under(&c.dir).next().is_some())
            .map(|c| accept::scope_of(cfg, c, head))
            .collect();
        change_roles(
            nc,
            &RoleInput {
                cfg,
                git: &repo.git,
                diff,
                scope: &scope,
                other_scopes: &other_scopes,
                plan_path,
            },
        )
        .map_err(Fatal::Io)?
    } else {
        Vec::new()
    };
    decide(
        &ClassInput {
            nc,
            git: &repo.git,
            base_rev,
            folder: &ch.dir,
            head: Some(ch),
            minimum: &minimum,
        },
        plan_path,
        base.by_path.contains_key(plan_path),
        &roles,
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
    // The configuration of the need: the head and the tree of the base. With no
    // base, the head alone.
    let nc = match base_rev {
        Some(r) => need_config(cfg, &repo.git, r).map_err(Fatal::Io)?,
        None => need_config_of_head(cfg),
    };
    let decision = need_for_change(repo, cfg, &nc, ch, head, base, base_rev, diff)?;
    let record_path = format!("{}/{}", ch.dir, record::FILE_NAME);
    let record = match read_limited(&repo.root.join(&record_path), READ_LIMIT) {
        Limited::Absent => RecordFile::Absent,
        Limited::Bytes(b) => RecordFile::Text(String::from_utf8_lossy(&b).into_owned()),
        other => RecordFile::Unreadable(unreadable_message(&record_path, &other)),
    };
    if decision.is_none() && record == RecordFile::Absent {
        return Ok(None);
    }
    let (need, class, roles) = match decision {
        Some(d) => (d.need, d.class, d.roles),
        None => (Need::No(None), ch.class, Vec::new()),
    };
    let archive = next_archive_path(&ch.slug, &|p| repo.root.join(p).exists());
    let gate = nc.gate().clone();
    let mut f = FinishFacts {
        record,
        ignored: repo.git.ignore_rule(&archive),
        archive,
        need,
        class,
        roles,
        acceptors: nc.higher_risk_acceptors,
        gate,
        runs: None,
        rerun: None,
        ack: AckFacts::default(),
        brief_problems: Vec::new(),
        stale: None,
        sections: Sections::default(),
    };
    let gate = &f.gate.clone();
    // The count of D74. It needs the 2 trees for the walk of the approvals.
    if let (Some(base_rev), Some(base)) = (base_rev, base) {
        let head_rec = match &f.record {
            RecordFile::Text(t) => Some(state_of(Some(t.as_bytes()))),
            _ => None,
        };
        let sf = stop_facts(
            cfg,
            &nc,
            &repo.git,
            base_rev,
            &ch.dir,
            head_rec,
            (base, head),
        )
        .map_err(Fatal::Io)?;
        f.runs = (sf.total >= 2).then_some((sf.total, sf.rejected_total));
        f.rerun = sf.rerun;
    }
    if matches!(f.need, Need::Yes { .. }) {
        f.sections = sections_of(gate, &repo.git, base_rev, ch, head).map_err(Fatal::Io)?;
    }
    let (Need::Yes { .. }, Some(base_rev)) = (&f.need, base_rev) else {
        return Ok(Some(f));
    };
    let RecordFile::Text(text) = &f.record else {
        return Ok(Some(f));
    };
    let Ok(rec) = Record::parse(text) else {
        return Ok(Some(f));
    };
    if !gate.is_off(Code::AcceptanceInvalid) {
        f.brief_problems =
            brief_problems(cfg, &repo.git, base_rev, &ch.dir, &rec).map_err(Fatal::Io)?;
    }
    if !gate.is_off(Code::RecordCommitInvalid) || !gate.is_off(Code::AckMissing) {
        f.ack = ack_facts(
            cfg,
            &nc,
            &repo.git,
            base_rev,
            (&ch.dir, &ch.slug),
            (&rec.acknowledged_by, rec.deviations.len()),
        )
        .map_err(Fatal::Io)?;
    }
    if !gate.is_off(Code::AcceptanceStale) {
        let plan = PlanScope::of(ch, head);
        f.stale =
            stale(cfg, &repo.git, &rec, diff, &plan, Some(base_rev), false).map_err(Fatal::Io)?;
    }
    Ok(Some(f))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(acceptors: &[(&str, &str, &str)]) -> Record {
        let mut text = String::from(
            "format = 2\nchange = \"demo\"\nscope = []\npaths = []\ntree = \"0123456789abcdef\"\nresult = \"accepted\"\nacknowledged-by = []\n",
        );
        for (role, session, agent) in acceptors {
            text.push_str(&format!(
                "\n[[acceptor]]\nroles = [\"{role}\"]\nbrief = \"0123456789abcdef\"\nsession = \"{session}\"\nagent = \"{agent}\"\nresult = \"accepted\"\n"
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

    /// The archive file of a change among 2 changes whose names overlap.
    #[test]
    fn the_archive_of_a_change_is_the_file_that_names_it_in_the_field_change() {
        let file = |change: &str| {
            format!(
                "format = 1\nchange = \"{change}\"\nscope = []\npaths = []\ntree = \"0123456789abcdef\"\nresult = \"accepted\"\nacknowledged-by = []\n"
            )
            .into_bytes()
        };
        let a = ".spec-dna/accepted/fix.toml";
        let b = ".spec-dna/accepted/fix-2.toml";
        let c = ".spec-dna/accepted/fix-3.toml";
        // `fix-2.toml` is the archive of the change `fix-2`, not the file 2 of `fix`.
        let find = |slug: &str, added: &[&str], files: &[(&str, &str)]| {
            let mut read =
                |p: &str| Ok(files.iter().find(|(f, _)| *f == p).map(|(_, ch)| file(ch)));
            archive_of(slug, added, &mut read).unwrap().map(|(p, _)| p)
        };
        let both = [(a, "fix"), (b, "fix-2")];
        for order in [[a, b], [b, a]] {
            assert_eq!(find("fix", &order, &both).as_deref(), Some(a));
            assert_eq!(find("fix-2", &order, &both).as_deref(), Some(b));
        }
        // `fix` deleted with no `finish`: the file of `fix-2` hides nothing.
        assert_eq!(find("fix", &[b], &[(b, "fix-2")]), None);
        assert_eq!(find("fix-2", &[b], &[(b, "fix-2")]).as_deref(), Some(b));
        // The file 2 of the change `fix`, and the archive of `fix-2`, in one diff.
        let three = [(a, "fix"), (b, "fix"), (c, "fix-2")];
        assert_eq!(find("fix", &[a, b, c], &three).as_deref(), Some(b));
        // A file of `fix-2` named `fix-3`: the field decides.
        assert_eq!(find("fix-2", &[a, b, c], &three).as_deref(), Some(c));
        assert_eq!(find("fix-2", &[a, b], &[(a, "fix"), (b, "fix")]), None);
        // A file that does not read is owned by its name.
        let mut bad = |_: &str| Ok(Some(b"not a record = = =".to_vec()));
        assert_eq!(
            archive_of("fix-2", &[b], &mut bad)
                .unwrap()
                .map(|x| x.0)
                .as_deref(),
            Some(b)
        );
        let mut bad = |_: &str| Ok(Some(b"not a record = = =".to_vec()));
        assert_eq!(archive_of("fix", &[b], &mut bad).unwrap(), None);
    }

    /// A path outside the archive folder is never the archive of a change, also
    /// when it reads as the record of the change; `moved_archive` finds it.
    #[test]
    fn a_path_outside_the_archive_folder_is_no_archive_and_a_record_there_is_a_moved_one() {
        let record = b"format = 1\nchange = \"fix\"\nscope = []\npaths = []\ntree = \"0123456789abcdef\"\nresult = \"accepted\"\nacknowledged-by = []\n".to_vec();
        for p in ["notes/fix.toml", "keep/fix.txt", "src/fix"] {
            let mut read = |_: &str| Ok(Some(record.clone()));
            assert_eq!(archive_of("fix", &[p], &mut read).unwrap(), None, "{p}");
        }
        let no_folder = |_: &str| false;
        let mut read = |p: &str| {
            Ok(match p {
                "keep/fix.txt" | "docs/fix/acceptance.toml" => Some(record.clone()),
                "src/fix" => Some(b"not a record".to_vec()),
                _ => None,
            })
        };
        let added = ["src/fix", "keep/fix.txt"];
        assert_eq!(
            moved_archive("fix", &added, &no_folder, &mut read)
                .unwrap()
                .as_deref(),
            Some("keep/fix.txt")
        );
        // Another change, a non-record file, and a path of the archive folder.
        assert_eq!(
            moved_archive("other", &added, &no_folder, &mut read).unwrap(),
            None
        );
        assert_eq!(
            moved_archive("fix", &["src/fix"], &no_folder, &mut read).unwrap(),
            None
        );
        let inside = [".spec-dna/accepted/fix.toml"];
        assert_eq!(
            moved_archive("fix", &inside, &no_folder, &mut read).unwrap(),
            None
        );
        // A record in the folder of a change is no moved archive file.
        let folder = |p: &str| p.ends_with("/acceptance.toml");
        assert_eq!(
            moved_archive("fix", &["docs/fix/acceptance.toml"], &folder, &mut read).unwrap(),
            None
        );
    }

    /// The text of `next` and `finish` follows the reason of the SD929 message.
    #[test]
    fn the_cause_of_a_stale_record_follows_the_reason_of_the_message() {
        let scope = format!("{SCOPE_MARK} `src/x.rs`, and `scope` of the record has not");
        let uncovered = format!(
            "the changed code path `Makefile` is in `Files` {UNCOVERED_MARK} `Makefile`, which has no `/`"
        );
        let code = format!(
            "{} that `paths` lists is a, and the record has b",
            CODE_MARKS[0]
        );
        let diff = format!("{} `src/y.rs`, which `scope` matches", CODE_MARKS[1]);
        assert!(stale_cause(&scope).starts_with("the scope of the plan changed"));
        assert!(
            stale_cause(&uncovered).starts_with("a changed path is in `Files` only as a bare name")
        );
        assert!(!stale_cause(&uncovered).contains("scope of the plan"));
        assert!(stale_cause(&code).starts_with("the code changed"));
        assert!(stale_cause(&diff).starts_with("the code changed"));
        let both = format!("{scope}; {code}");
        assert_eq!(
            stale_cause(&both),
            "the scope of the plan changed after the acceptance and the code changed after the acceptance"
        );
        let shared = format!(
            "the changed code path `lib/a.py` is covered only by the glob `lib/**`: {SHARED_ADDED_MARK}; accept again"
        );
        assert_eq!(
            stale_cause(&shared),
            "shared-paths changed after the acceptance"
        );
        let lost =
            format!("`scope` of the record has the pattern `lib/**`, and {RECORD_ONLY_MARK}");
        assert!(stale_cause(&lost).contains("`[change] shared-paths`"));
        assert!(!stale_cause(&lost).contains("scope of the plan"));
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

    fn st(exists: bool, rejected: bool) -> State {
        State { exists, rejected }
    }

    /// D74 item 1: a life is an unbroken run of commits that hold the file.
    #[test]
    fn a_life_is_an_unbroken_run_of_commits_that_hold_the_record() {
        let (none, ok, bad) = (st(false, false), st(true, false), st(true, true));
        // The base, then 5 commits: a record that says `rejected` in 2 commits, a
        // commit with no file, and a second record.
        let l = lives(&[none, bad, bad, none, ok, ok], None);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].first, Pos::Commit(0));
        assert_eq!(l[0].rejected_at, Some(Pos::Commit(0)));
        assert_eq!(l[1].first, Pos::Commit(3));
        assert_eq!(l[1].rejected_at, None);
        // A record that says `ok` and then `rejected` is one life that counts once.
        let l = lives(&[none, ok, bad, bad], None);
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].rejected_at, Some(Pos::Commit(1)));
        // The base holds a record, and the branch keeps it: one life.
        let l = lives(&[bad, bad, bad], None);
        assert_eq!(
            (l.len(), l[0].first, l[0].rejected_at),
            (1, Pos::Base, Some(Pos::Base))
        );
        // The head continues the life of the last commit, or starts a life.
        assert_eq!(lives(&[none, ok], Some(bad)).len(), 1);
        assert_eq!(lives(&[none, ok, none], Some(ok)).len(), 2);
        assert_eq!(lives(&[none, none], None).len(), 0);
        assert_eq!(
            lives(&[none, bad, none], None)[0].rejected_at,
            Some(Pos::Commit(0))
        );
    }

    /// The count reads `result` leniently: a record that the strict reader refuses
    /// still counts as rejected.
    #[test]
    fn the_count_reads_the_result_of_a_record_that_does_not_parse() {
        let head = "format = 2\nchange = \"demo\"\nresult = \"rejected\"  # why\n";
        assert!(says_rejected(head));
        // An unknown key: valid TOML, so the generic reader gives `result`.
        assert!(says_rejected(&format!("{head}\n[[attempt]]\nmood = 5\n")));
        // A syntax error after the top-level keys: the line scan.
        assert!(says_rejected(&format!("{head}\n[[finding]\nx = 1\n")));
        assert!(says_rejected("result = 'rejected'\n[[a]\n"));
        assert!(!says_rejected("result = \"accepted\"\n[[finding]\n"));
        // A `result` below a table is no top-level key.
        assert!(!says_rejected(
            "format = 2\n[[finding]\nresult = \"rejected\"\n"
        ));
        assert!(!says_rejected("not toml = = ="));
    }

    #[test]
    fn the_head_state_reads_the_result_of_the_record() {
        let r = rec(&[("acceptor", "s1", "unknown")]);
        assert_eq!(head_state(Some(&r)), Some(st(true, false)));
        let mut r2 = r.clone();
        r2.result = "rejected".to_string();
        assert_eq!(head_state(Some(&r2)), Some(st(true, true)));
        assert_eq!(head_state(None), None);
        assert_eq!(state_of(None), st(false, false));
        assert_eq!(state_of(Some(b"not toml = =")), st(true, false));
    }

    #[test]
    fn the_texts_of_the_stop_name_the_records_and_the_way_to_the_owner() {
        let since = vec![(0, "3f2a9c1".to_string()), (1, "9c1b7e0".to_string())];
        assert_eq!(
            wait_text("demo", &since, false),
            "wait for the owner: 2 acceptance records of demo say `rejected` (3f2a9c1, 9c1b7e0); a third run needs the word of the owner"
        );
        let branch = wait_text("demo", &since, true);
        assert!(
            branch.contains("the word of the owner: commit `status: draft` of plan.md on its own, skipping the commit hook for that commit only and naming the skip in the body of the commit (the draft commit gives SD401 and SD910 reason 9), then the approver runs `spec-dna approve demo plan`"),
            "{branch}"
        );
        assert!(branch.ends_with("`spec-dna approve demo plan`"), "{branch}");
        let runs = Runs {
            base_rev: "b".repeat(40),
            commits: vec![],
            lives: vec![
                Life {
                    first: Pos::Commit(0),
                    rejected_at: Some(Pos::Commit(0)),
                },
                Life {
                    first: Pos::Commit(1),
                    rejected_at: Some(Pos::Commit(1)),
                },
                Life {
                    first: Pos::Commit(2),
                    rejected_at: None,
                },
            ],
        };
        // The gate counts the 2 rejected records before the last life.
        let m = rerun_message(
            "demo",
            &runs,
            &[(0, "aaaaaaa".into()), (1, "bbbbbbb".into())],
        )
        .unwrap();
        assert!(
            m.contains("after 2 rejected records (aaaaaaa, bbbbbbb)"),
            "{m}"
        );
        assert!(m.contains("spec-dna approve demo plan"), "{m}");
        // The last life itself is no earlier record.
        assert!(
            rerun_message(
                "demo",
                &runs,
                &[(1, "bbbbbbb".into()), (2, "ccccccc".into())]
            )
            .is_none()
        );
        assert!(
            rejection_not_cleared("3f2a9c1").starts_with(
                "the record said `rejected` at 3f2a9c1; a later edit cannot accept it"
            )
        );
        assert!(rerun_cut_message("demo", "abc").contains("fetch-depth: 0"));
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
        let one = rec(&[("acceptor", "s1", "unknown")]);
        let two = rec(&[("acceptor", "s1", "unknown"), ("acceptor", "s2", "unknown")]);
        assert!(acceptors_missing(Class::Routine, 3, &[], &one).is_none());
        assert!(acceptors_missing(Class::HigherRisk, 2, &[], &one).is_some());
        assert!(acceptors_missing(Class::HigherRisk, 2, &[], &two).is_none());
        assert!(acceptors_missing(Class::HigherRisk, 3, &[], &two).is_some());
        assert!(acceptors_missing(Class::HigherRisk, 1, &[], &one).is_none());
    }

    /// D70 item 7: the role paths of one change.
    #[test]
    fn the_role_paths_of_a_change_follow_item_7() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        let mine = s(&["src/a/**"]);
        let other = s(&["src/b/**"]);
        // A path in my scope, and a path that no scope matches (`Files` left it
        // out); a path of another change is not mine.
        let diff = s(&["src/a/x.rs", "src/b/y.rs", "src/c/z.rs"]);
        assert_eq!(
            role_paths(&diff, &mine, &[other.as_slice()], &[]),
            s(&["src/a/x.rs", "src/c/z.rs"])
        );
        // With no other change, each path of the diff is a role path.
        assert_eq!(role_paths(&diff, &mine, &[], &[]), diff);
        // No code path in my scope: the paths in scope since the plan.
        let since = s(&["src/a/old.rs", "src/b/old.rs"]);
        let only_other = s(&["src/b/y.rs"]);
        assert_eq!(
            role_paths(&only_other, &mine, &[other.as_slice()], &since),
            s(&["src/a/old.rs"])
        );
        // With a path in scope in the diff, `since_plan` adds nothing.
        assert_eq!(
            role_paths(&s(&["src/a/x.rs"]), &mine, &[], &since),
            s(&["src/a/x.rs"])
        );
        // Sorted, with no duplicate.
        assert_eq!(
            role_paths(
                &s(&["src/a/z.rs", "src/a/b.rs", "src/a/z.rs"]),
                &mine,
                &[],
                &[]
            ),
            s(&["src/a/b.rs", "src/a/z.rs"])
        );
    }

    #[test]
    fn no_table_gives_no_role_and_a_table_gives_the_matched_roles() {
        let cfg = Config::parse(
            "spec-dna = \"0.1.0\"\n[change]\npolicy = \"three-class\"\n[[roots]]\npath = \"docs\"\n",
        )
        .unwrap();
        assert!(matched_roles(&cfg, &["src/a.rs".to_string()]).is_empty());
        let nc = need_config_of_head(&cfg);
        assert!(!nc.has_owners(&cfg));
        assert!(nc.matched_roles(&cfg, &["src/a.rs".to_string()]).is_empty());
        let cfg = Config::parse(
            "spec-dna = \"0.1.0\"\n[change]\npolicy = \"three-class\"\n[owners]\n\"src/**\" = [\"b-role\", \"a-role\"]\n[[roots]]\npath = \"docs\"\n",
        )
        .unwrap();
        let nc = need_config_of_head(&cfg);
        assert!(nc.has_owners(&cfg));
        assert_eq!(
            nc.matched_roles(&cfg, &["src/a.rs".to_string()]),
            ["a-role", "b-role"]
        );
    }

    #[test]
    fn sd933_needs_each_role_in_the_union_of_the_roles_of_the_entries() {
        let one = rec(&[("a-role", "s1", "unknown")]);
        let roles = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert!(acceptors_missing(Class::Routine, 1, &roles(&["a-role"]), &one).is_none());
        let m = acceptors_missing(Class::Routine, 1, &roles(&["a-role", "b-role"]), &one).unwrap();
        assert!(m.contains("`b-role`") && !m.contains("`a-role`"), "{m}");
        // The role rule and the count are 2 rules: one entry counts for both.
        assert!(acceptors_missing(Class::HigherRisk, 2, &roles(&["a-role"]), &one).is_some());
    }

    fn standard(change: &str, rules: &str) -> Config {
        Config::parse(&format!(
            "spec-dna = \"0.1.0\"\nprofile = \"standard\"\n[change]\npolicy = \"three-class\"\n{change}\n[rules]\n{rules}\n[[roots]]\npath = \"docs\"\n"
        ))
        .unwrap()
    }

    #[test]
    fn the_need_config_of_the_head_alone_reads_the_head() {
        let c = standard(
            "acceptance-for = \"all\"\nhigh-risk-paths = [\"src/db/**\"]\nhigher-risk-acceptors = 2",
            "",
        );
        let nc = need_config_of_head(&c);
        assert!(nc.on);
        assert_eq!(nc.acceptance_for, AcceptanceFor::All);
        assert_eq!(nc.high_risk_paths, ["src/db/**"]);
        assert_eq!(nc.higher_risk_acceptors, 2);
        let off = standard("", "acceptance = \"off\"");
        assert!(!need_config_of_head(&off).on);
    }

    #[test]
    fn the_union_keeps_the_order_and_adds_each_new_item_once() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<String>>();
        assert_eq!(
            union(&s(&["a", "b"]), &s(&["b", "c", "c"])),
            s(&["a", "b", "c", "c"])
        );
        assert_eq!(union(&s(&[]), &s(&["x"])), s(&["x"]));
    }

    #[test]
    fn the_strictest_configuration_has_the_acceptance_on_for_all() {
        let off = standard("acceptance-for = \"none\"", "acceptance = \"off\"");
        assert!(off.is_off(Code::AcceptanceMissing));
        let s = strictest(&off);
        assert!(!s.is_off(Code::AcceptanceMissing));
        assert_eq!(s.acceptance_for(), AcceptanceFor::All);
    }

    #[test]
    fn a_text_with_class_higher_risk_is_higher_risk_and_no_other_text_is() {
        let t = |class: &str| format!("---\nstatus: accepted\n{class}---\n# T\n");
        assert!(text_is_higher_risk(t("class: higher-risk\n").as_bytes()));
        assert!(!text_is_higher_risk(t("class: routine\n").as_bytes()));
        assert!(!text_is_higher_risk(t("").as_bytes()));
        assert!(!text_is_higher_risk(b"# No front matter\n"));
    }

    #[test]
    fn a_rejected_result_gives_sd930() {
        let mut r = rec(&[("acceptor", "s1", "unknown")]);
        assert!(rejected(&r).is_none());
        r.result = "rejected".to_string();
        assert!(rejected(&r).is_some());
    }

    #[test]
    fn the_message_of_a_missing_record_names_the_prompt_line_not_a_paste_of_the_brief() {
        let m = missing_message("demo", "`x` is absent", &[], false, false);
        assert!(
            m.contains("`cd '<root>' && spec-dna accept demo --brief`"),
            "{m}"
        );
        assert!(
            m.contains("`spec-dna next`") && m.contains("agent `acceptor`"),
            "{m}"
        );
        assert!(
            !m.contains("unchanged") && !m.contains("give the output"),
            "{m}"
        );
    }

    #[test]
    fn the_message_of_a_missing_record_names_the_shallow_clone_only_when_it_is_one() {
        let plain = missing_message("demo", "`x` is absent", &[], false, false);
        assert!(plain.contains("accept demo --brief") && !plain.contains("fetch-depth"));
        let shallow = missing_message("demo", "`x` is absent", &[], true, false);
        assert!(shallow.contains("fetch-depth: 0"));
        assert!(!shallow.contains("cuts the branch"));
        let cut = missing_message("demo", "`x` is absent", &[], false, true);
        assert!(cut.contains("cuts the branch") && cut.contains("fetch-depth: 0"));
        assert!(!cut.contains("added the plan"));
    }
}

//! The harness and adapter part of the plan of `upgrade` (design 0.6.0, D147
//! and D148; ADR 0008, "Rename, `upgrade` and `check`"). The functions are pure.
//! They read a tree and write nothing. S10 adds the command that calls them.
//!
//! - `unmarked_roles` is step 4 of the plan. It names the roles whose agent
//!   file has no marker, so pass 2 of the configuration turns them off.
//! - `plan_files` is step 7. It gives the adapters and the harness files of a
//!   final configuration, as `init` writes them.

// `upgrade` (step 5, slice 10) calls this module. Until then only the tests do.
#![allow(dead_code)]

use crate::adapters;
use crate::config::Config;
use crate::harness::{self, Level, Mode, Model, Role};
use crate::init::{self, Adapter, HOOK_FILES};
use crate::repo::Fatal;
use crate::upgrade_docs::Tree;
use crate::upgrade_old::OldConfig;
use std::collections::{BTreeMap, BTreeSet, HashMap};

/// The adapter and harness files of a plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct FilesPlan {
    /// The adapters that `init` writes for the project.
    pub adapters: Vec<Adapter>,
    /// The final path of each file to write, with its new text.
    pub writes: BTreeMap<String, String>,
    /// The files to remove, in path order: the unedited files of an ended role.
    pub removes: Vec<String>,
    /// A short description of each write and remove, for the output.
    pub notes: BTreeMap<String, String>,
    /// The lines of the report that start with `kept`: a file that `upgrade`
    /// leaves, and why.
    pub kept: Vec<String>,
    /// The roles of 0.5 that ended, each with its model: (role, model).
    pub ended_roles: Vec<(String, String)>,
    /// The warnings of the configuration of the harness.
    pub warnings: Vec<String>,
    /// The bytes that the harness adds to each request of the main session.
    pub request_bytes: usize,
}

/// A file that `upgrade` must read as text. Another file that is not UTF-8,
/// such as a skill of the project, is not read, and no plan writes it.
fn must_be_text(path: &str) -> bool {
    harness::is_output_path(path) || adapters::PATHS.contains(&path) || HOOK_FILES.contains(&path)
}

/// The text of each file of the tree that the adapters and the harness read.
/// A path that the plan can write and that is not UTF-8 stops the plan: the
/// plan would replace it with no way to read it (D210).
fn read_texts(tree: &Tree) -> Result<HashMap<String, String>, Fatal> {
    let wanted: Vec<String> = tree
        .files
        .iter()
        .filter(|p| {
            harness::wanted(p)
                || adapters::PATHS.contains(&p.as_str())
                || HOOK_FILES.contains(&p.as_str())
        })
        .cloned()
        .collect();
    let mut out = HashMap::new();
    for (path, bytes) in (tree.read)(&wanted)? {
        match String::from_utf8(bytes) {
            Ok(t) => {
                out.insert(path, t);
            }
            Err(_) if must_be_text(&path) => {
                return Err(Fatal::Config(format!(
                    "{path}: the file is not UTF-8, and upgrade cannot read it"
                )));
            }
            Err(_) => {}
        }
    }
    Ok(out)
}

/// The files that the harness reads, in path order: (path, text).
fn harness_files(texts: &HashMap<String, String>) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = texts
        .iter()
        .filter(|(p, _)| harness::wanted(p))
        .map(|(p, t)| (p.clone(), t.clone()))
        .collect();
    out.sort();
    out
}

/// The roles of the level whose agent file has no marker: (role, path).
fn conflicts(existing: &[(String, String)], level: Level) -> Vec<(Role, String)> {
    harness::name_conflicts(existing)
        .into_iter()
        .filter(|(role, _)| level.includes(*role))
        .collect()
}

/// Step 4: the keys of the roles of the level whose agent file has no marker.
/// Pass 2 of the configuration turns them off (D147), so `upgrade` never
/// replaces such a file.
pub fn unmarked_roles(tree: &Tree, old: &OldConfig) -> Result<BTreeSet<String>, Fatal> {
    let Some(h) = &old.harness else {
        return Ok(BTreeSet::new());
    };
    let level = Level::from_name(&h.level).unwrap_or(Level::None);
    let existing = harness_files(&read_texts(tree)?);
    Ok(conflicts(&existing, level)
        .into_iter()
        .map(|(role, _)| role.key().to_string())
        .collect())
}

/// True if the configuration generates the agent file of the role.
fn generates(h: &harness::Harness, role: Role) -> bool {
    h.on(role)
        && match role {
            Role::Advisor => h.cfg(role).agent != Model::None,
            _ => true,
        }
}

/// A conflict that `init` reports as a usage error stops `upgrade` with exit
/// code 1: the plan is valid, a file of the project is in the way, and nothing
/// changed (D136).
fn refused(e: Fatal) -> Fatal {
    match e {
        Fatal::Usage(m) => Fatal::Refused(m),
        other => other,
    }
}

/// The model in the front matter of an agent file.
fn model_of(text: &str) -> Option<String> {
    let mut lines = text.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return None;
    }
    lines
        .take_while(|l| l.trim_end() != "---")
        .find_map(|l| l.strip_prefix("model:"))
        .map(|m| m.trim().trim_matches(['"', '\'']).to_string())
}

/// Step 7: the adapters and the harness files of the final configuration, as
/// `init` writes them (D148). `from_old` is true if the source is a 0.5 file:
/// then the plan runs as `init --reconfigure` and it removes an unedited file
/// that the configuration does not generate, such as `checker.md`. A source of
/// 0.6.0 runs as plain `init`, and it removes nothing (D147). The check for a
/// file that git ignores is a step of the command (`init::refuse_ignored`).
pub fn plan_files(tree: &Tree, fin: &Config, from_old: bool) -> Result<FilesPlan, Fatal> {
    let texts = read_texts(tree)?;
    let src = |p: &str| texts.get(p).cloned();
    let (chosen, mut changes) = init::adapters_of(&src, fin).map_err(refused)?;
    let mut out = FilesPlan {
        adapters: chosen,
        ..FilesPlan::default()
    };
    if let Some(h) = fin.harness.as_ref().filter(|h| h.level != Level::None) {
        let existing = harness_files(&texts);
        let mode = if from_old {
            Mode::Reconfigure
        } else {
            Mode::Upgrade
        };
        for (role, path) in conflicts(&existing, h.level) {
            if !generates(h, role) {
                out.kept.push(format!(
                    "kept {path}: no marker, so spec-dna did not generate it; the role `{}` writes no file",
                    role.key()
                ));
            }
        }
        if from_old {
            for path in harness::OLD_AGENT_PATHS {
                if texts.get(path).is_some_and(|t| !harness::has_marker(t)) {
                    out.kept.push(format!(
                        "kept {path}: no marker, so spec-dna did not generate it; upgrade leaves it"
                    ));
                }
            }
            if let Some(t) = texts
                .get(".claude/agents/spec-writer.md")
                .filter(|t| harness::has_marker(t))
            {
                out.ended_roles.push((
                    "spec-writer".to_string(),
                    model_of(t).unwrap_or_else(|| "unknown".to_string()),
                ));
            }
        }
        let run =
            init::harness_run(existing, h, Some(h), &[], mode, &mut changes).map_err(refused)?;
        out.kept.extend(run.plan.kept.iter().cloned());
        out.warnings = run.plan.warnings.clone();
        out.request_bytes = run.plan.request_bytes;
    }
    for c in changes {
        if c.remove {
            out.removes.push(c.path.clone());
        } else {
            out.writes.insert(c.path.clone(), c.text);
        }
        out.notes.insert(c.path, c.note);
    }
    out.removes.sort();
    out.kept.sort();
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters;
    use crate::diag::Code;
    use crate::harness::{self, Level, Role};
    use crate::upgrade_config;
    use crate::upgrade_docs::{Basis, TreeData, classify, tests::opts};
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};

    fn fixtures() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/upgrade")
    }

    fn walk(dir: &Path, rel: &str, out: &mut HashMap<String, Vec<u8>>) {
        for e in std::fs::read_dir(dir).unwrap() {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().to_string();
            let path = if rel.is_empty() {
                name
            } else {
                format!("{rel}/{name}")
            };
            if e.file_type().unwrap().is_dir() {
                walk(&e.path(), &path, out);
            } else {
                out.insert(path, std::fs::read(e.path()).unwrap());
            }
        }
    }

    /// `v05/` with the overlays on it, in order. A `spec-dna.toml.append` goes
    /// to the end of the configuration (the helper `Repo::from_fixture`).
    fn blobs_of(overlays: &[&str]) -> HashMap<String, Vec<u8>> {
        let mut blobs = HashMap::new();
        walk(&fixtures().join("v05"), "", &mut blobs);
        for o in overlays {
            let mut more = HashMap::new();
            walk(&fixtures().join(o), "", &mut more);
            if let Some(a) = more.remove("spec-dna.toml.append") {
                blobs.get_mut("spec-dna.toml").unwrap().extend(a);
            }
            blobs.extend(more);
        }
        blobs
    }

    fn text(blobs: &HashMap<String, Vec<u8>>, path: &str) -> String {
        String::from_utf8(blobs[path].clone()).unwrap()
    }

    /// The final configuration of a 0.5 tree, as `upgrade` gives it. The facts
    /// have the roles of `unmarked_roles`.
    fn final_config(tree: &TreeData, toml: &str) -> (OldConfig, Config) {
        let old = OldConfig::read(toml).unwrap();
        let p1 = upgrade_config::pass1(toml, &old, &opts()).unwrap();
        let cfg1 = Config::parse(&p1.text).unwrap();
        let classified = tree
            .with(|t| {
                classify(
                    t,
                    &Basis {
                        cfg: &cfg1,
                        old: Some(&old),
                    },
                )
            })
            .unwrap();
        let mut facts = classified.facts(&cfg1);
        facts.unmarked_roles = tree.with(|t| unmarked_roles(t, &old)).unwrap();
        let plan = upgrade_config::plan(toml, &old, &facts, &opts()).unwrap();
        (old, Config::parse(&plan.text).unwrap())
    }

    struct Case {
        blobs: HashMap<String, Vec<u8>>,
        tree: TreeData,
        old: OldConfig,
        fin: Config,
    }

    fn case(overlays: &[&str]) -> Case {
        with_files(overlays, &[])
    }

    /// The same, with more files in the tree (a path and its text).
    fn with_files(overlays: &[&str], more: &[(&str, &str)]) -> Case {
        let mut blobs = blobs_of(overlays);
        for (p, t) in more {
            blobs.insert(p.to_string(), t.as_bytes().to_vec());
        }
        let tree = TreeData::from_map(blobs.clone());
        let toml = text(&blobs, "spec-dna.toml");
        let (old, fin) = final_config(&tree, &toml);
        Case {
            blobs,
            tree,
            old,
            fin,
        }
    }

    fn plan_of(c: &Case) -> FilesPlan {
        c.tree.with(|t| plan_files(t, &c.fin, true)).unwrap()
    }

    fn paths(m: &BTreeMap<String, String>) -> Vec<&str> {
        m.keys().map(String::as_str).collect()
    }

    /// The tree after the plan: the blobs with each write and each remove.
    fn applied(c: &Case, p: &FilesPlan) -> HashMap<String, Vec<u8>> {
        let mut blobs = c.blobs.clone();
        for (path, t) in &p.writes {
            blobs.insert(path.clone(), t.clone().into_bytes());
        }
        for path in &p.removes {
            assert!(blobs.remove(path).is_some(), "{path} is not in the tree");
        }
        blobs
    }

    /// The findings of `harness::check` and `adapters::check` on a tree.
    fn findings(blobs: &HashMap<String, Vec<u8>>, fin: &Config) -> Vec<(Code, String)> {
        let files: Vec<(String, String)> = blobs
            .iter()
            .filter(|(p, _)| harness::wanted(p))
            .map(|(p, b)| (p.clone(), String::from_utf8(b.clone()).unwrap()))
            .collect();
        let mut out: Vec<(Code, String)> = Vec::new();
        if let Some(h) = &fin.harness {
            out.extend(
                harness::check(h, &files)
                    .into_iter()
                    .map(|d| (d.code, d.path)),
            );
        }
        let adapter_files: Vec<(&str, String)> = adapters::PATHS
            .iter()
            .filter_map(|p| {
                blobs
                    .get(*p)
                    .map(|b| (*p, String::from_utf8(b.clone()).unwrap()))
            })
            .collect();
        let types = adapters::project_types(&fin.layers, &fin.types);
        out.extend(
            adapters::check(&adapter_files, &types)
                .into_iter()
                .map(|d| (d.code, d.path)),
        );
        out.sort_by(|a, b| a.1.cmp(&b.1));
        out
    }

    const OWN_AGENT: &str = "---\nname: verifier\ndescription: My own.\n---\nMine.\n";

    #[test]
    fn lean_checker_goes_and_verifier_comes() {
        let c = case(&["harness-lean"]);
        let p = plan_of(&c);
        assert_eq!(p.removes, [".claude/agents/checker.md"]);
        assert!(p.writes.contains_key(".claude/agents/verifier.md"));
        let v = &p.writes[".claude/agents/verifier.md"];
        assert!(v.starts_with("---\nname: verifier\n"), "{v}");
        assert!(!c.blobs.contains_key(".claude/agents/verifier.md"));
        assert!(p.kept.is_empty(), "{:?}", p.kept);
        assert!(p.ended_roles.is_empty());
    }

    #[test]
    fn the_files_of_a_lean_upgrade_are_these() {
        let c = case(&["harness-lean"]);
        let p = plan_of(&c);
        // The adapters of 0.6 (both instruction files, the skill), `verifier`
        // and the team page, which names it. The marker of each other agent
        // file has the version, so `Explore` and `fallback-advisor` change.
        // `.claude/settings.json` has the hooks of 0.5 already.
        assert_eq!(
            paths(&p.writes),
            [
                ".claude/agents/Explore.md",
                ".claude/agents/fallback-advisor.md",
                ".claude/agents/model-team.md",
                ".claude/agents/verifier.md",
                ".claude/skills/spec-dna/SKILL.md",
                "AGENTS.md",
                "CLAUDE.md",
            ]
        );
        assert_eq!(p.removes, [".claude/agents/checker.md"]);
        assert!(p.request_bytes > 0);
    }

    #[test]
    fn a_full_upgrade_removes_checker_and_spec_writer_and_writes_no_simplifier() {
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        assert_eq!(
            p.removes,
            [".claude/agents/checker.md", ".claude/agents/spec-writer.md"]
        );
        assert!(p.writes.contains_key(".claude/agents/verifier.md"));
        // The code simplifier is off (D139), and the new parts are off.
        assert!(!p.writes.contains_key(".claude/agents/code-simplifier.md"));
        assert!(
            !p.writes.keys().any(|k| k.contains("spec-dna-")),
            "{:?}",
            paths(&p.writes)
        );
        assert!(!p.writes.contains_key("REVIEW.md"));
        let settings = p.writes.get(".claude/settings.json");
        assert!(
            settings.is_none_or(|s| !s.contains("protect-tests")),
            "{settings:?}"
        );
        assert!(!c.fin.harness.as_ref().unwrap().on(Role::CodeSimplifier));
    }

    #[test]
    fn the_files_of_a_full_upgrade_are_these() {
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        // As lean, and `implementer`, whose marker has the version. No
        // `code-simplifier.md`: the role is off.
        assert_eq!(
            paths(&p.writes),
            [
                ".claude/agents/Explore.md",
                ".claude/agents/fallback-advisor.md",
                ".claude/agents/implementer.md",
                ".claude/agents/model-team.md",
                ".claude/agents/verifier.md",
                ".claude/skills/spec-dna/SKILL.md",
                "AGENTS.md",
                "CLAUDE.md",
            ]
        );
    }

    #[test]
    fn an_unedited_agent_file_changes_only_its_marker() {
        // The marker has the version, so the file is written again. The text
        // above the marker is the text of 0.5.
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        let above = |t: &str| t.trim_end().rsplit_once('\n').unwrap().0.to_string();
        for name in ["Explore", "fallback-advisor", "implementer"] {
            let path = format!(".claude/agents/{name}.md");
            let old = text(&c.blobs, &path);
            assert_ne!(old, p.writes[&path], "{name}");
            assert_eq!(above(&old), above(&p.writes[&path]), "{name}");
        }
    }

    #[test]
    fn the_report_names_the_model_of_the_ended_role() {
        let c = case(&["harness-full"]);
        let p = plan_of(&c);
        let file = text(&c.blobs, ".claude/agents/spec-writer.md");
        let model = file
            .lines()
            .find_map(|l| l.strip_prefix("model: "))
            .unwrap()
            .to_string();
        assert_eq!(model, "sonnet");
        assert_eq!(p.ended_roles, [("spec-writer".to_string(), model)]);
        // A lean project has no such role.
        assert!(plan_of(&case(&["harness-lean"])).ended_roles.is_empty());
    }

    #[test]
    fn the_planned_tree_has_no_finding_of_the_harness_or_the_adapters() {
        for overlays in [&["harness-lean"][..], &["harness-full"][..]] {
            let c = case(overlays);
            let p = plan_of(&c);
            let found = findings(&applied(&c, &p), &c.fin);
            assert!(found.is_empty(), "{overlays:?}: {found:?}");
        }
    }

    #[test]
    fn an_edited_checker_stays_and_gives_sd501_only() {
        let c = case(&["harness-lean", "harness-edited"]);
        let p = plan_of(&c);
        assert!(p.removes.is_empty(), "{:?}", p.removes);
        assert!(p.writes.contains_key(".claude/agents/verifier.md"));
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/checker.md") && k.contains("SD501")),
            "{:?}",
            p.kept
        );
        let found = findings(&applied(&c, &p), &c.fin);
        assert_eq!(
            found,
            [(
                Code::AdapterModified,
                ".claude/agents/checker.md".to_string()
            )]
        );
    }

    #[test]
    fn a_verifier_file_with_no_marker_is_never_replaced() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/agents/verifier.md", OWN_AGENT)],
        );
        let roles = c.tree.with(|t| unmarked_roles(t, &c.old)).unwrap();
        assert_eq!(roles, BTreeSet::from(["verifier".to_string()]));
        let h = c.fin.harness.as_ref().unwrap();
        assert!(!h.on(Role::Verifier));
        let p = plan_of(&c);
        assert!(!p.writes.contains_key(".claude/agents/verifier.md"));
        assert!(
            !p.removes
                .contains(&".claude/agents/verifier.md".to_string())
        );
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/verifier.md")
                    && k.contains("no marker")
                    && k.contains("`verifier`")),
            "{:?}",
            p.kept
        );
        let after = applied(&c, &p);
        assert_eq!(text(&after, ".claude/agents/verifier.md"), OWN_AGENT);
        // The unedited generated `checker.md` still goes: spec-dna wrote it.
        assert_eq!(p.removes, [".claude/agents/checker.md"]);
        assert_eq!(findings(&after, &c.fin), []);
    }

    #[test]
    fn a_file_of_the_project_under_a_role_name_turns_off_only_roles_of_the_level() {
        // At level lean, `implementer` is not a role of the project.
        let own = |name: &str| format!("---\nname: {name}\ndescription: My own.\n---\nMine.\n");
        let c = with_files(
            &["harness-lean"],
            &[(".claude/agents/implementer.md", &own("implementer"))],
        );
        let roles = c.tree.with(|t| unmarked_roles(t, &c.old)).unwrap();
        assert!(roles.is_empty(), "{roles:?}");
        let p = plan_of(&c);
        assert!(p.kept.is_empty(), "{:?}", p.kept);
        // At level full, the advisor, the researcher and the implementer are.
        let c = with_files(
            &["harness-full"],
            &[
                (".claude/agents/implementer.md", &own("implementer")),
                (
                    ".claude/agents/fallback-advisor.md",
                    &own("fallback-advisor"),
                ),
            ],
        );
        let roles = c.tree.with(|t| unmarked_roles(t, &c.old)).unwrap();
        assert_eq!(
            roles,
            BTreeSet::from(["advisor".to_string(), "implementer".to_string()])
        );
        let p = plan_of(&c);
        assert_eq!(p.kept.len(), 2, "{:?}", p.kept);
        let h = c.fin.harness.as_ref().unwrap();
        assert_eq!(h.cfg(Role::Advisor).agent, harness::Model::None);
        assert!(!h.on(Role::Implementer));
        assert!(!p.writes.contains_key(".claude/agents/implementer.md"));
        assert!(!p.writes.contains_key(".claude/agents/fallback-advisor.md"));
        assert_eq!(findings(&applied(&c, &p), &c.fin), []);
    }

    #[test]
    fn an_old_agent_file_with_no_marker_stays_and_the_report_names_it() {
        let c = with_files(
            &["harness-lean"],
            &[(
                ".claude/agents/checker.md",
                "---\nname: checker\n---\nMine.\n",
            )],
        );
        let p = plan_of(&c);
        assert!(p.removes.is_empty(), "{:?}", p.removes);
        assert!(!p.writes.contains_key(".claude/agents/checker.md"));
        assert!(
            p.kept
                .iter()
                .any(|k| k.starts_with("kept .claude/agents/checker.md") && k.contains("no marker")),
            "{:?}",
            p.kept
        );
    }

    #[test]
    fn a_project_with_no_harness_gets_the_adapters_only() {
        let c = case(&[]);
        let p = plan_of(&c);
        assert_eq!(
            paths(&p.writes),
            [".claude/skills/spec-dna/SKILL.md", "AGENTS.md", "CLAUDE.md"]
        );
        assert!(p.removes.is_empty());
        assert_eq!(p.request_bytes, 0);
        let line = adapters::line();
        assert!(p.writes["AGENTS.md"].contains(&line));
        assert!(p.writes["CLAUDE.md"].contains(&line));
        let skill = &p.writes[".claude/skills/spec-dna/SKILL.md"];
        let types = adapters::project_types(&c.fin.layers, &c.fin.types);
        assert_eq!(*skill, adapters::skill_file(&types));
        assert_eq!(findings(&applied(&c, &p), &c.fin), []);
    }

    #[test]
    fn the_skill_lists_the_types_of_the_new_layers() {
        let c = case(&["harness-lean"]);
        let p = plan_of(&c);
        let skill = &p.writes[".claude/skills/spec-dna/SKILL.md"];
        for t in ["intent", "research", "requirements"] {
            assert!(skill.contains(t), "{t}");
        }
        assert!(!skill.contains("Types of this project: spec, "), "{skill}");
    }

    #[test]
    fn a_hook_of_0_4_1_gets_the_staged_flag() {
        let hook = "repos:\n  - repo: local\n    hooks:\n      - id: spec-dna\n        entry: spec-dna check\n";
        let c = with_files(&["harness-lean"], &[(".pre-commit-config.yaml", hook)]);
        let p = plan_of(&c);
        assert!(
            p.writes[".pre-commit-config.yaml"].contains("entry: spec-dna check --staged"),
            "{:?}",
            p.writes.get(".pre-commit-config.yaml")
        );
    }

    #[test]
    fn a_second_plan_on_the_result_changes_nothing() {
        for overlays in [&[][..], &["harness-lean"][..], &["harness-full"][..]] {
            let c = case(overlays);
            let p = plan_of(&c);
            let tree = TreeData::from_map(applied(&c, &p));
            // The source is now a 0.6 file: the final configuration.
            let again = tree.with(|t| plan_files(t, &c.fin, false)).unwrap();
            assert!(
                again.writes.is_empty(),
                "{overlays:?}: {:?}",
                paths(&again.writes)
            );
            assert!(again.removes.is_empty());
            assert!(again.kept.is_empty(), "{overlays:?}: {:?}", again.kept);
        }
    }

    #[test]
    fn a_source_of_0_6_removes_nothing() {
        let c = case(&["harness-lean"]);
        let from_new = c.tree.with(|t| plan_files(t, &c.fin, false)).unwrap();
        assert!(from_new.removes.is_empty());
        assert!(from_new.writes.contains_key(".claude/agents/verifier.md"));
    }

    #[test]
    fn a_source_of_0_6_refuses_a_file_that_the_role_would_replace() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/agents/verifier.md", OWN_AGENT)],
        );
        // The configuration of the 0.5 run has the role off. A 0.6 file with
        // the role on is a conflict, and the plan stops.
        let mut on = c.fin.harness.clone().unwrap();
        on.cfg_mut(Role::Verifier).model = harness::Model::Id("opus".into());
        let mut fin = c.fin.clone();
        fin.harness = Some(on);
        let e = c.tree.with(|t| plan_files(t, &fin, false)).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)), "{e:?}");
        assert!(e.message().contains("verifier.md"), "{}", e.message());
    }

    #[test]
    fn a_skill_of_the_project_at_the_adapter_path_stops_the_plan() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/skills/spec-dna/SKILL.md", "My own skill.\n")],
        );
        let e = c.tree.with(|t| plan_files(t, &c.fin, true)).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)), "{e:?}");
        assert!(e.message().contains("SKILL.md"), "{}", e.message());
    }

    #[test]
    fn a_settings_file_that_is_not_json_stops_the_plan() {
        let c = with_files(
            &["harness-lean"],
            &[(".claude/settings.json", "{ not json")],
        );
        let e = c.tree.with(|t| plan_files(t, &c.fin, true)).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)), "{e:?}");
        assert!(e.message().contains("settings.json"), "{}", e.message());
    }

    #[test]
    fn a_file_that_is_not_text_stops_the_plan_with_its_path() {
        let mut c = case(&["harness-lean"]);
        c.blobs.insert("AGENTS.md".into(), vec![0xff, 0xfe, 0x41]);
        let tree = TreeData::from_map(c.blobs.clone());
        let e = tree.with(|t| plan_files(t, &c.fin, true)).unwrap_err();
        assert!(matches!(e, Fatal::Config(_)), "{e:?}");
        assert!(e.message().starts_with("AGENTS.md"), "{}", e.message());
    }

    #[test]
    fn level_none_writes_no_harness_file_and_keeps_the_old_ones() {
        let c = with_files(&["harness-lean"], &[]);
        let mut fin = c.fin.clone();
        fin.harness = Some(harness::Harness::new(Level::None));
        let p = c.tree.with(|t| plan_files(t, &fin, true)).unwrap();
        assert!(p.removes.is_empty());
        assert!(
            !p.writes.keys().any(|k| k.starts_with(".claude/agents/")),
            "{:?}",
            paths(&p.writes)
        );
        assert!(!p.writes.contains_key(".claude/settings.json"));
        assert!(p.writes.contains_key("AGENTS.md"));
    }

    #[test]
    fn the_adapters_follow_the_files_of_the_tree() {
        // Both instruction files exist: all 3 adapters.
        let c = case(&[]);
        assert_eq!(
            plan_of(&c).adapters,
            [Adapter::AgentsMd, Adapter::ClaudeMd, Adapter::Skill]
        );
        // `CLAUDE.md` only: `AGENTS.md` is no target, and `upgrade` creates none.
        let mut blobs = c.blobs.clone();
        blobs.remove("AGENTS.md");
        let tree = TreeData::from_map(blobs);
        let p = tree.with(|t| plan_files(t, &c.fin, true)).unwrap();
        assert_eq!(p.adapters, [Adapter::ClaudeMd, Adapter::Skill]);
        assert!(!p.writes.contains_key("AGENTS.md"));
        // No instruction file: all 3 adapters, and both files are new.
        let mut blobs = c.blobs.clone();
        blobs.remove("AGENTS.md");
        blobs.remove("CLAUDE.md");
        let tree = TreeData::from_map(blobs);
        let p = tree.with(|t| plan_files(t, &c.fin, true)).unwrap();
        assert_eq!(
            p.adapters,
            [Adapter::AgentsMd, Adapter::ClaudeMd, Adapter::Skill]
        );
        assert!(p.writes.contains_key("AGENTS.md") && p.writes.contains_key("CLAUDE.md"));
    }
}

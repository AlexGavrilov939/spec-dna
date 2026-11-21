//! The table `[owners]` (design 0.8.0, D5, D70 item 7 and D77): a path glob and
//! the agent roles that must review the change when a role path matches it.
//!
//! - An entry is a role name (`[a-z][a-z0-9-]*`, at most 40 characters). A role
//!   is a word that the acceptance record repeats in `roles` of an entry.
//! - An entry that names a person (`@<login>` or an e-mail) is no role in 0.8.0.
//!   The table keeps it out of the roles and gives one warning. The persons of
//!   `[owners]` come in 0.8.1. The exit code does not change.
//! - Another entry, a bad glob or a value of the wrong type is a configuration
//!   error (exit code 3).
//!
//! The glob form is the form of `[change] high-risk-paths`, with the match of
//! `glob.rs`. The result of a match is the union of the roles of each glob that
//! matches: no order and no last match exist.

use crate::glob;
use crate::record;
use std::collections::BTreeSet;
use std::sync::Mutex;
use toml::Value;

/// The table `[owners]`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Owners {
    /// A glob and its roles, in the order of the file. The roles are agent
    /// roles only: a person entry is not here.
    entries: Vec<(String, Vec<String>)>,
    /// The warnings of the table: one for each entry that names a person.
    warnings: Vec<String>,
}

/// The form of a role, for a message.
const ROLE_FORM: &str =
    "a role name is [a-z][a-z0-9-]* with at most 40 characters, such as `security-reviewer`";

impl Owners {
    /// Reads the value of the key `owners`. The error is the text that
    /// `Config::parse` returns (exit code 3).
    pub fn parse(v: &Value) -> Result<Owners, String> {
        let tbl = v
            .as_table()
            .ok_or("`owners` must be a table: a glob and a list of role names")?;
        let mut out = Owners::default();
        for (glob, list) in tbl {
            if let Some(why) = glob_problem(glob) {
                return Err(format!(
                    "`owners`: the glob `{glob}` {why}; write a glob from the repository root, such as `src/auth/**`"
                ));
            }
            let items = list.as_array().ok_or_else(|| {
                format!("`owners.\"{glob}\"` must be a list of role names; {ROLE_FORM}")
            })?;
            let mut roles: Vec<String> = Vec::new();
            for item in items {
                let entry = item.as_str().ok_or_else(|| {
                    format!("`owners.\"{glob}\"` must be a list of strings; {ROLE_FORM}")
                })?;
                if record::valid_role(entry) {
                    if !roles.iter().any(|r| r == entry) {
                        roles.push(entry.to_string());
                    }
                } else if names_a_person(entry) {
                    out.warnings.push(format!(
                        "[owners] \"{glob}\": \"{entry}\" names a person; 0.8.0 checks agent roles only, so this entry needs nothing; the persons of [owners] come in 0.8.1"
                    ));
                } else {
                    return Err(format!(
                        "`owners.\"{glob}\"`: `{entry}` is not a role name; {ROLE_FORM}"
                    ));
                }
            }
            out.entries.push((trimmed(glob).to_string(), roles));
        }
        Ok(out)
    }

    /// True if the table has no role. A person entry is no role.
    pub fn is_empty(&self) -> bool {
        self.entries.iter().all(|(_, roles)| roles.is_empty())
    }

    /// The warnings of the table, for a command that reads the configuration.
    pub fn warnings(&self) -> &[String] {
        &self.warnings
    }

    /// The roles of each glob that matches one of `paths`, sorted with no
    /// duplicate (D5 item 2: the union).
    pub fn matched(&self, paths: &[String]) -> Vec<String> {
        let mut out: BTreeSet<String> = BTreeSet::new();
        for (g, roles) in &self.entries {
            if roles.is_empty() {
                continue;
            }
            if paths
                .iter()
                .any(|p| glob::matches(g, p.trim_start_matches("./")))
            {
                out.extend(roles.iter().cloned());
            }
        }
        out.into_iter().collect()
    }
}

/// The glob without white space at the ends and without a leading `./`, as
/// `[change] high-risk-paths` stores it.
fn trimmed(g: &str) -> &str {
    let mut v = g.trim();
    while let Some(rest) = v.strip_prefix("./") {
        v = rest;
    }
    v
}

/// The reason that a glob is no glob from the repository root, or `None`. The
/// rules are the rules of `[change] high-risk-paths`.
fn glob_problem(g: &str) -> Option<&'static str> {
    let v = trimmed(g);
    if v.trim_matches('/').is_empty() {
        return Some("is empty");
    }
    if v.starts_with('/') || v.contains('\\') {
        return Some("is not a relative path with `/` separators");
    }
    if v.split('/').any(|s| s == "." || s == "..") {
        return Some("has a `.` or `..` segment");
    }
    None
}

/// True if `entry` names a person: `@<login>`, or an e-mail (0.8.1, D5 of the
/// note). A bare `@` and a text with a space are no person.
fn names_a_person(entry: &str) -> bool {
    if entry.chars().any(char::is_whitespace) {
        return false;
    }
    match entry.strip_prefix('@') {
        Some(login) => {
            login.starts_with(|c: char| c.is_ascii_alphanumeric())
                && login
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        }
        None => match entry.split_once('@') {
            Some((local, domain)) => {
                !local.is_empty() && !domain.is_empty() && !domain.contains('@')
            }
            None => false,
        },
    }
}

/// The warnings that a process printed. A command reads the configuration more
/// than once, and each warning is printed once.
static PRINTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

/// Prints each warning of `owners` on the standard error, once for the process.
pub fn print_warnings(owners: &Owners) {
    let Ok(mut seen) = PRINTED.lock() else {
        return;
    };
    for w in owners.warnings() {
        if !seen.contains(w) {
            eprintln!("warning: {w}");
            seen.push(w.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn read(text: &str) -> Result<Owners, String> {
        let t: toml::Table = text.parse().unwrap();
        Owners::parse(t.get("owners").unwrap())
    }

    #[test]
    fn a_role_matches_a_glob_and_the_result_is_the_sorted_union() {
        let o = read(
            "[owners]\n\"src/auth/**\" = [\"security-reviewer\"]\n\"src/**\" = [\"db-reviewer\", \"security-reviewer\"]\n",
        )
        .unwrap();
        let p = |s: &str| vec![s.to_string()];
        assert_eq!(
            o.matched(&p("src/auth/a.rs")),
            ["db-reviewer", "security-reviewer"]
        );
        assert_eq!(
            o.matched(&p("src/x.rs")),
            ["db-reviewer", "security-reviewer"]
        );
        assert!(o.matched(&p("docs/a.md")).is_empty());
        assert!(o.matched(&[]).is_empty());
    }

    #[test]
    fn a_glob_with_a_space_or_a_leading_dot_slash_is_stored_trimmed() {
        let o = read("[owners]\n\" ./src/** \" = [\"security-reviewer\"]\n").unwrap();
        assert_eq!(o.matched(&["src/a.rs".to_string()]), ["security-reviewer"]);
    }

    #[test]
    fn a_person_is_a_warning_and_no_role() {
        let o = read("[owners]\n\"src/**\" = [\"@lead\", \"lead@example.test\"]\n").unwrap();
        assert!(o.is_empty());
        assert_eq!(o.warnings().len(), 2);
        assert!(o.matched(&["src/a.rs".to_string()]).is_empty());
    }

    #[test]
    fn the_forms_of_a_person_and_of_a_bad_entry() {
        for ok in ["@lead", "@a-b_c.d", "lead@example.test"] {
            assert!(names_a_person(ok), "{ok}");
        }
        for bad in [
            "@",
            "@-x",
            "a b",
            "Security Reviewer",
            "x@",
            "@@a",
            "a@b@c",
            "role",
        ] {
            assert!(!names_a_person(bad), "{bad}");
        }
        assert!(read("[owners]\n\"a/**\" = [\"Security Reviewer\"]\n").is_err());
        assert!(read("[owners]\n\"a/**\" = [\"@\"]\n").is_err());
        assert!(read("[owners]\n\"a/**\" = \"role\"\n").is_err());
        assert!(read("[owners]\n\"/a/**\" = [\"role\"]\n").is_err());
        assert!(read("[owners]\n\"a/../b\" = [\"role\"]\n").is_err());
    }
}

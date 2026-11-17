//! The plan of `spec-dna.toml` for `import` (design 0.7.0, D21; decision I16).
//! The import needs 3 kinds of change in the file: the legacy key `FR-{n}` for
//! the requirements of a done feature, the areas of `--areas`, and a type that
//! the key `types` lacks. If the plan has one, the first run of the command
//! writes only this file (SD503 rejects it together with documents).
//!
//! The plan is a pure function over the text of the file. Each edit is a
//! splice (`config_text`), so each comment and each other byte stays.

use crate::config::{self, Config};
use crate::config_text::{at, insert_keys, k, set_value};
use crate::model::DocType;
use crate::repo::Fatal;
use crate::upgrade_docs::valid_area;
use toml::Value;

/// The legacy key pattern of a requirement line of Spec Kit.
pub const LEGACY_KEY: &str = "FR-{n}";

/// What the plan of the documents needs from the file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Needs {
    /// The root that holds the requirements has no pattern that matches
    /// `FR-001`, and a done feature has `FR-nnn` lines.
    pub legacy_key: bool,
    /// The areas of `--areas` that `areas` lacks, in the order of the file of
    /// `--areas`.
    pub areas: Vec<String>,
    /// The types that the plan writes. The plan adds the ones that a non-empty
    /// `types` list lacks.
    pub types: Vec<DocType>,
}

/// The new text of the file, and a note for each change (for the report).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ConfigPlan {
    pub text: String,
    pub notes: Vec<String>,
}

impl ConfigPlan {
    #[cfg(test)]
    pub fn changed(&self) -> bool {
        !self.notes.is_empty()
    }
}

/// Reads `--areas`: the table `[areas]`, whose keys are the names of the source
/// folders and whose values are area folders. The result is in the order of
/// the file. An error is exit code 3.
pub fn read_areas(path: &str) -> Result<Vec<(String, String)>, Fatal> {
    let bad = |m: String| Fatal::Config(format!("--areas {path}: {m}"));
    let text = std::fs::read_to_string(path).map_err(|e| bad(format!("cannot read it ({e})")))?;
    let table: toml::Table = text
        .parse()
        .map_err(|e: toml::de::Error| bad(e.to_string().trim().to_string()))?;
    let Some(areas) = table.get("areas").and_then(Value::as_table) else {
        return Err(bad(
            "the table `[areas]` is missing; each key is the name of a source folder and each value is an area"
                .into(),
        ));
    };
    areas
        .iter()
        .map(|(name, v)| match v.as_str() {
            Some(area) => Ok((name.clone(), area.to_string())),
            None => Err(bad(format!("the area of `{name}` is not a string"))),
        })
        .collect()
}

/// Checks the areas of `--areas` against the project: each value is one folder
/// name, and no 2 names differ only in case, also against `areas` of the file
/// (D30). A problem is a stop (exit code 1): the parse of the new file would
/// be exit code 3.
pub fn check_areas(cfg: &Config, areas: &[(String, String)]) -> Result<(), Fatal> {
    for (name, area) in areas {
        if !valid_area(area) {
            return Err(Fatal::Refused(format!(
                "--areas: the area `{area}` of `{name}` is not one folder name (letters, digits, `-` and `_`), so import changes nothing"
            )));
        }
    }
    let names = cfg
        .areas
        .iter()
        .map(String::as_str)
        .chain(areas.iter().map(|(_, a)| a.as_str()));
    if let Some((a, b)) = config::area_case_pair(names) {
        return Err(Fatal::Refused(format!(
            "--areas: the areas `{a}` and `{b}` differ only in case, and a file system that ignores case gives one folder for both; use one of them, so import changes nothing"
        )));
    }
    Ok(())
}

fn strings(items: &[String]) -> Value {
    Value::Array(items.iter().map(|s| Value::String(s.clone())).collect())
}

/// The plan of the file for the needs of the import. `root` is the index of
/// the root that holds the `requirements` documents.
pub fn plan(text: &str, cfg: &Config, root: usize, needs: &Needs) -> Result<ConfigPlan, Fatal> {
    let mut out = text.to_string();
    let mut notes = Vec::new();
    let r = &cfg.roots[root];
    if needs.legacy_key && !r.legacy_keys.iter().any(|p| p.is_match("FR-001")) {
        let mut list: Vec<String> = r.legacy_keys.iter().map(|p| p.source.clone()).collect();
        list.push(LEGACY_KEY.to_string());
        let path = [k("roots"), at(root)];
        out = if r.legacy_keys.is_empty() {
            insert_keys(&out, &path, &[("legacy-keys", strings(&list))])?
        } else {
            set_value(
                &out,
                &[k("roots"), at(root), k("legacy-keys")],
                &strings(&list),
            )?
        };
        notes.push(format!(
            "legacy key `{LEGACY_KEY}` in the root `{}`",
            r.path
        ));
    }
    let new_areas: Vec<&String> = needs
        .areas
        .iter()
        .enumerate()
        .filter(|(i, a)| !cfg.areas.contains(*a) && !needs.areas[..*i].contains(*a))
        .map(|(_, a)| a)
        .collect();
    if !new_areas.is_empty() {
        let mut list = cfg.areas.clone();
        list.extend(new_areas.iter().map(|a| (*a).clone()));
        out = if cfg.areas.is_empty() {
            insert_keys(&out, &[], &[("areas", strings(&list))])?
        } else {
            set_value(&out, &[k("areas")], &strings(&list))?
        };
        let shown: Vec<String> = new_areas.iter().map(|a| format!("`{a}`")).collect();
        notes.push(format!("areas {}", shown.join(", ")));
    }
    if !cfg.types.is_empty() {
        let missing: Vec<DocType> = needs
            .types
            .iter()
            .enumerate()
            .filter(|(i, t)| !cfg.types.contains(*t) && !needs.types[..*i].contains(*t))
            .map(|(_, t)| *t)
            .collect();
        if !missing.is_empty() {
            let mut list: Vec<String> = cfg.types.iter().map(|t| t.name().to_string()).collect();
            list.extend(missing.iter().map(|t| t.name().to_string()));
            out = set_value(&out, &[k("types")], &strings(&list))?;
            for t in missing {
                notes.push(format!("type `{}`", t.name()));
            }
        }
    }
    Ok(ConfigPlan { text: out, notes })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg_text(extra_top: &str, root_extra: &str) -> String {
        format!(
            "spec-dna = \"0.6.1\"\n{extra_top}\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n{root_extra}"
        )
    }

    fn cfg(text: &str) -> Config {
        Config::parse(text).unwrap()
    }

    fn keys_needed() -> Needs {
        Needs {
            legacy_key: true,
            ..Needs::default()
        }
    }

    #[test]
    fn the_legacy_key_is_added_to_a_root_that_has_none() {
        let text = cfg_text("", "");
        let p = plan(&text, &cfg(&text), 0, &keys_needed()).unwrap();
        let c = cfg(&p.text);
        assert!(c.roots[0].legacy_keys[0].is_match("FR-001"), "{}", p.text);
        assert!(p.text.contains("legacy-keys = [\"FR-{n}\"]"), "{}", p.text);
        assert!(p.changed());
        assert_eq!(p.notes, ["legacy key `FR-{n}` in the root `docs`"]);
    }

    #[test]
    fn the_legacy_key_is_added_to_the_list_that_a_root_has() {
        let text = cfg_text("", "legacy-keys = [\"R-{n}\"]  # old keys\n");
        let p = plan(&text, &cfg(&text), 0, &keys_needed()).unwrap();
        let c = cfg(&p.text);
        assert_eq!(c.roots[0].legacy_keys.len(), 2, "{}", p.text);
        assert!(p.text.contains("# old keys"), "{}", p.text);
    }

    #[test]
    fn a_pattern_that_matches_fr_keys_needs_no_change() {
        let text = cfg_text("", "legacy-keys = [\"FR-{n}*\"]\n");
        let p = plan(&text, &cfg(&text), 0, &keys_needed()).unwrap();
        assert_eq!(p.text, text);
        assert!(!p.changed());
    }

    #[test]
    fn areas_are_added_after_the_areas_of_the_file_and_never_twice() {
        let text = cfg_text("", "");
        let needs = Needs {
            areas: vec!["media".into(), "sharing".into()],
            ..Needs::default()
        };
        let p = plan(&text, &cfg(&text), 0, &needs).unwrap();
        assert_eq!(cfg(&p.text).areas, ["media", "sharing"], "{}", p.text);
        assert_eq!(p.notes, ["areas `media`, `sharing`"]);
        // The key is before the first table, as the top level.
        assert!(p.text.find("areas =").unwrap() < p.text.find("[layers").unwrap());

        let text = cfg_text("areas = [\"sharing\"]  # the areas\n", "");
        let p = plan(&text, &cfg(&text), 0, &needs).unwrap();
        assert_eq!(cfg(&p.text).areas, ["sharing", "media"], "{}", p.text);
        assert!(p.text.contains("# the areas"), "{}", p.text);
        assert_eq!(p.notes, ["areas `media`"]);
    }

    #[test]
    fn a_type_that_a_list_lacks_is_added_and_no_list_gets_no_change() {
        let text = cfg_text("types = [\"requirements\", \"intent\"]\n", "");
        let needs = Needs {
            types: vec![DocType::Requirements, DocType::Rule],
            ..Needs::default()
        };
        let p = plan(&text, &cfg(&text), 0, &needs).unwrap();
        let names: Vec<&str> = cfg(&p.text).types.iter().map(|t| t.name()).collect();
        assert_eq!(names, ["requirements", "intent", "rule"], "{}", p.text);
        assert_eq!(p.notes, ["type `rule`"]);

        let text = cfg_text("", "");
        let p = plan(&text, &cfg(&text), 0, &needs).unwrap();
        assert_eq!(p.text, text);
        assert!(!p.changed());
    }

    #[test]
    fn the_file_of_areas_has_the_table_and_string_values() {
        let dir = std::env::temp_dir().join(format!("import-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, text: &str| {
            let p = dir.join(name);
            std::fs::write(&p, text).unwrap();
            p.to_string_lossy().into_owned()
        };
        let ok = read_areas(&file(
            "ok.toml",
            "[areas]\n\"001-a\" = \"media\"\nb = \"x\"\n",
        ))
        .unwrap();
        assert_eq!(
            ok,
            [
                ("001-a".to_string(), "media".to_string()),
                ("b".to_string(), "x".to_string())
            ]
        );
        for (name, text, part) in [
            ("none.toml", "x = 1\n", "the table `[areas]` is missing"),
            ("num.toml", "[areas]\na = 3\n", "is not a string"),
            ("bad.toml", "[areas\n", "--areas"),
        ] {
            let e = read_areas(&file(name, text)).unwrap_err();
            assert!(matches!(e, Fatal::Config(_)), "{name}");
            assert!(e.message().contains(part), "{name}: {}", e.message());
        }
        let e = read_areas(&dir.join("missing.toml").to_string_lossy()).unwrap_err();
        assert!(matches!(e, Fatal::Config(_)));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn a_bad_area_stops_with_exit_code_1() {
        let text = cfg_text("areas = [\"media\"]\n", "");
        let c = cfg(&text);
        let pair = |a: &str, b: &str| {
            vec![
                ("001-a".to_string(), a.to_string()),
                ("002-b".to_string(), b.to_string()),
            ]
        };
        assert!(check_areas(&c, &pair("media", "sharing")).is_ok());
        assert!(check_areas(&c, &pair("media", "media")).is_ok());
        let e = check_areas(&c, &pair("Sharing", "sharing")).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        assert!(
            e.message().contains("`Sharing`") && e.message().contains("`sharing`"),
            "{}",
            e.message()
        );
        assert!(e.message().contains("only in case"), "{}", e.message());
        // Against the areas of the file.
        let e = check_areas(&c, &pair("Media", "x")).unwrap_err();
        assert!(matches!(e, Fatal::Refused(_)));
        assert!(e.message().contains("`media`"), "{}", e.message());
        // A value that is not one folder name.
        for bad in ["a/b", "", "a b", ".."] {
            let e = check_areas(&c, &pair(bad, "x")).unwrap_err();
            assert!(matches!(e, Fatal::Refused(_)), "{bad}");
            assert!(e.message().contains("`001-a`"), "{}", e.message());
        }
    }
}

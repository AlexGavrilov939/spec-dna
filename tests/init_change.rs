//! Integration tests of the change policy and the gate in `spec-dna init`
//! (questions 4 and 5), the table `[change]` that they write, and the
//! `CODEOWNERS` line. The helpers are small copies of the ones in
//! tests/init_graph.rs. The neutral domains are irrigation and library.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const BIN: &str = env!("CARGO_BIN_EXE_spec-dna");
const VERSION: &str = env!("CARGO_PKG_VERSION");
/// The e-mail that `Repo::empty` sets as `user.email`.
const OWNER: &str = "owner@example.test";
static COUNTER: AtomicUsize = AtomicUsize::new(0);

struct Repo {
    dir: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

fn git(dir: &Path, args: &[&str]) {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "commit.gpgsign=false",
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.org",
        ])
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

impl Repo {
    fn empty() -> Repo {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("spec-dna-change-{}-{n}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        git(&dir, &["init", "-q", "-b", "main"]);
        // The e-mail of the project, for the key `approvers`. The tests that
        // need no e-mail use `without_email`.
        git(&dir, &["config", "user.email", OWNER]);
        Repo { dir }
    }

    /// A repository where git has no `user.email`. `run` also hides the
    /// global git configuration of the machine.
    fn without_email() -> Repo {
        let r = Repo::empty();
        git(&r.dir, &["config", "--unset", "user.email"]);
        r
    }

    fn write(&self, path: &str, text: &str) {
        let p = self.dir.join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, text).unwrap();
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap()
    }

    fn exists(&self, path: &str) -> bool {
        self.dir.join(path).exists()
    }

    fn commit(&self, msg: &str) {
        git(&self.dir, &["add", "-A"]);
        git(&self.dir, &["commit", "-q", "--allow-empty", "-m", msg]);
    }

    fn run(&self, args: &[&str]) -> (i32, String, String) {
        let out = Command::new(BIN)
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", &self.dir)
            .output()
            .expect("binary runs");
        (
            out.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// Runs `init` with an answers file. The harness is off unless the
    /// answers say otherwise, so that the output is short.
    fn init_with(&self, answers: &str, args: &[&str]) -> (i32, String, String) {
        let file = PathBuf::from(format!("{}-answers.toml", self.dir.display()));
        let text = if answers.contains("harness") {
            answers.to_string()
        } else {
            format!("harness = \"none\"\n{answers}")
        };
        fs::write(&file, text).unwrap();
        let mut a = vec!["init", "--answers", file.to_str().unwrap()];
        a.extend_from_slice(args);
        let out = self.run(&a);
        let _ = fs::remove_file(&file);
        out
    }

    /// The table `[change]` of `spec-dna.toml`, or `None`.
    fn change(&self) -> Option<toml::Table> {
        let t: toml::Table = self.read("spec-dna.toml").parse().unwrap();
        t.get("change").map(|v| v.as_table().unwrap().clone())
    }
}

/// The text value of a key of `[change]`.
fn text(t: &toml::Table, key: &str) -> String {
    t.get(key)
        .and_then(toml::Value::as_str)
        .unwrap_or_else(|| panic!("no string `{key}` in {t:?}"))
        .to_string()
}

/// The strings of an array key of `[change]`.
fn list(t: &toml::Table, key: &str) -> Vec<String> {
    t.get(key)
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("no array `{key}` in {t:?}"))
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect()
}

const CODEOWNERS_LINE: &str = "docs/intent/*/spec.md @OWNER";

#[test]
fn a_new_project_gets_three_class_block_and_the_seven_keys() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().expect("a new project has a [change] table");
    assert_eq!(text(&t, "policy"), "three-class");
    assert_eq!(text(&t, "gate"), "block");
    // The small graph has no `interfaces` layer, so the list has no such name.
    assert_eq!(list(&t, "high-risk-layers"), ["rules"]);
    assert!(list(&t, "high-risk-paths").is_empty());
    assert_eq!(
        t.get("max-fanout").and_then(toml::Value::as_integer),
        Some(8)
    );
    let mut keys: Vec<&str> = t.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "approval",
            "approvers",
            "gate",
            "high-risk-layers",
            "high-risk-paths",
            "max-fanout",
            "policy"
        ]
    );
    // The preview says how the answers 4 and 5 were applied.
    let (_, out, _) = r.run(&["init", "--reconfigure", "--dry-run", "--non-interactive"]);
    assert!(
        out.contains("\n  4. change policy: three-class (default) -> [change] policy, "),
        "{out}"
    );
    assert!(
        out.contains("\n  5. change gate: block (default) -> [change] gate\n"),
        "{out}"
    );
}

#[test]
fn check_after_init_exits_0_with_the_new_defaults() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    // A new repository has no base commit yet.
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt spec-dna");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("0 errors"), "{out}");
    let (code, out, err) = r.run(&["check", "--base", "HEAD"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn a_new_change_table_gives_no_sd502_and_no_sd503() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt spec-dna");
    // A change of the policy is a change of `spec-dna.toml` and nothing else.
    let (code, out, err) = r.init_with(
        "harness = \"lean\"\nchange-policy = \"two-class\"\ngate = \"warn\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    let changed = Command::new("git")
        .arg("-C")
        .arg(&r.dir)
        .args(["status", "--porcelain"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&changed.stdout).trim(),
        "M spec-dna.toml"
    );
    r.commit("choose the two-class policy");
    for args in [
        vec!["check", "--base", "HEAD~1"],
        vec!["check", "--all"],
        vec!["check", "--staged"],
    ] {
        let (code, out, err) = r.run(&args);
        assert_eq!(code, 0, "{args:?}: {out}{err}");
        assert!(!out.contains("SD502") && !out.contains("SD503"), "{out}");
    }
    // The adapters do not depend on the policy: the same run changes no file
    // but `spec-dna.toml`.
    let (code, out, err) = r.run(&["init"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("wrote "), "{out}");
}

#[test]
fn the_default_graph_keeps_both_high_risk_layers() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("graph = \"default\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(list(&t, "high-risk-layers"), ["interfaces", "rules"]);
    // A smaller graph at the next reconfigure drops the layer that it lacks.
    let (code, out, err) = r.init_with("graph = \"small\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(list(&t, "high-risk-layers"), ["rules"]);
    r.commit("reconfigure");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn two_class_and_the_gate_are_written_as_answered() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with(
        "change-policy = \"two-class\"\ngate = \"warn\"\n",
        &["--dry-run"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains(
            "\n  4. change policy: two-class (changed from three-class) -> [change] policy"
        ),
        "{out}"
    );
    assert!(
        out.contains("\n  5. change gate: warn (changed from block) -> [change] gate"),
        "{out}"
    );
    assert!(!r.exists("spec-dna.toml"));
    let (code, out, err) = r.init_with("change-policy = \"two-class\"\ngate = \"warn\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(text(&t, "policy"), "two-class");
    assert_eq!(text(&t, "gate"), "warn");
    // The line is also a hint for `two-class`: it has a spec for a
    // higher-risk change.
    assert!(out.contains(CODEOWNERS_LINE), "{out}");
}

#[test]
fn the_policy_none_writes_the_table_and_gives_no_codeowners_line() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("change-policy = \"none\"\n", &["--json", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains("\"codeowners\":null"), "{out}");
    assert!(!out.contains("CODEOWNERS"), "{out}");
    let (code, out, err) = r.init_with("change-policy = \"none\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("CODEOWNERS"), "{out}");
    let t = r.change().expect("a new project has the table");
    assert_eq!(text(&t, "policy"), "none");
    // The gate is not asked, and it keeps its default.
    assert_eq!(text(&t, "gate"), "block");
    r.commit("adopt spec-dna");
    let (code, out, err) = r.run(&["check"]);
    assert_eq!(code, 0, "{out}{err}");
}

#[test]
fn the_codeowners_line_is_in_the_preview_and_in_json_and_the_file_is_not_written() {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    let note = out
        .lines()
        .find(|l| l.starts_with("note: CODEOWNERS"))
        .unwrap_or_else(|| panic!("no CODEOWNERS note:\n{out}"));
    assert!(note.contains(&format!("`{CODEOWNERS_LINE}`")), "{note}");
    assert!(
        note.contains("a higher-risk plan has the same path as a routine plan")
            || note.contains("A higher-risk plan has the same path as a routine plan"),
        "{note}"
    );
    assert!(note.contains("`init` does not write that file"), "{note}");
    let (code, json, err) = r.run(&["init", "--non-interactive", "--dry-run", "--json"]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(
        json.contains(&format!("\"codeowners\":\"{CODEOWNERS_LINE}\"")),
        "{json}"
    );
    // A write creates no CODEOWNERS file, and the note shows after the write.
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(out.contains(&format!("`{CODEOWNERS_LINE}`")), "{out}");
    for p in ["CODEOWNERS", ".github/CODEOWNERS", "docs/CODEOWNERS"] {
        assert!(!r.exists(p), "init wrote {p}");
    }
    // The path follows the root of the project.
    let r = Repo::empty();
    let (code, json, err) = r.run(&[
        "init",
        "--non-interactive",
        "--dry-run",
        "--json",
        "--root",
        "specs",
    ]);
    assert_eq!(code, 0, "{json}{err}");
    assert!(
        json.contains("\"codeowners\":\"specs/intent/*/spec.md @OWNER\""),
        "{json}"
    );
}

/// A configuration whose layers have no `intent` layer.
fn no_intent_config() -> String {
    format!(
        "spec-dna = \"{VERSION}\"\nprofile = \"standard\"\n\n[layers.rules]\ntypes = [\"rule\"]\n\n[layers.requirements]\ntypes = [\"requirements\"]\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\n"
    )
}

#[test]
fn custom_layers_with_no_intent_layer_give_the_policy_none_and_a_note() {
    let r = Repo::empty();
    r.write("spec-dna.toml", &no_intent_config());
    r.write(
        "docs/rules/water.md",
        "---\nstatus: active\n---\n# Water\n\n## Rules\n\n- **metric-units**: The controller SHALL store volumes in liters.\n",
    );
    r.commit("start");
    let (code, out, err) = r.init_with(
        "change-policy = \"three-class\"\n",
        &["--reconfigure", "--dry-run"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("note: the policy `three-class` needs an `intent` layer"),
        "{out}"
    );
    assert!(
        out.contains("\n  4. change policy: none (changed from three-class)")
            || out.contains("\n  4. change policy: none (default)"),
        "{out}"
    );
    assert!(!out.contains("CODEOWNERS"), "{out}");
    let (code, out, err) = r.init_with("change-policy = \"three-class\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().expect("the file says `none`");
    assert_eq!(text(&t, "policy"), "none");
    r.commit("reconfigure");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    // The graph `small` has the layer, so the same answer is valid there.
    let (code, out, err) = r.init_with(
        "graph = \"small\"\nchange-policy = \"three-class\"\n",
        &["--reconfigure"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(text(&r.change().unwrap(), "policy"), "three-class");
}

#[test]
fn reconfigure_keeps_the_current_change_values() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with(
        "graph = \"default\"\nchange-policy = \"two-class\"\ngate = \"off\"\n",
        &[],
    );
    assert_eq!(code, 0, "{out}{err}");
    // The project edits the table by hand.
    let cfg = r
        .read("spec-dna.toml")
        .replace("high-risk-paths = []", "high-risk-paths = [\"src/api/**\"]")
        .replace("max-fanout = 8", "max-fanout = 5")
        .replace(
            "high-risk-layers = [\"interfaces\", \"rules\"]",
            "high-risk-layers = [\"rules\"]",
        );
    r.write("spec-dna.toml", &cfg);
    r.commit("adopt and edit");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\n  4. change policy: two-class (default)")
            && out.contains("\n  5. change gate: off (default)"),
        "{out}"
    );
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(text(&t, "policy"), "two-class");
    assert_eq!(text(&t, "gate"), "off");
    assert_eq!(list(&t, "high-risk-layers"), ["rules"]);
    assert_eq!(list(&t, "high-risk-paths"), ["src/api/**"]);
    assert_eq!(
        t.get("max-fanout").and_then(toml::Value::as_integer),
        Some(5)
    );
    r.commit("reconfigure");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn reconfigure_keeps_the_change_base() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("graph = \"default\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    // The project adds the key by hand. A new `init` never proposes it.
    let cfg = r
        .read("spec-dna.toml")
        .replace("\nmax-fanout = 8", "\nbase = \"trunk\"\nmax-fanout = 8");
    assert!(cfg.contains("base = \"trunk\""), "{cfg}");
    r.write("spec-dna.toml", &cfg);
    r.commit("adopt and edit");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(text(&r.change().unwrap(), "base"), "trunk");
    r.commit("reconfigure");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
    // A project with no key still gets none.
    let r = Repo::empty();
    let (code, out, err) = r.init_with("graph = \"default\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!r.change().unwrap().contains_key("base"));
}

#[test]
fn a_table_with_no_gate_keeps_the_severity_of_the_profile() {
    // Without `gate`, the profile sets SD910: a warning in `lite`, an error in
    // `standard`. A default answer to question 5 writes the gate with the same
    // severity.
    for (profile, gate) in [("lite", "warn"), ("standard", "block"), ("strict", "block")] {
        let r = Repo::empty();
        let (code, out, err) = r.init_with(&format!("profile = \"{profile}\"\n"), &[]);
        assert_eq!(code, 0, "{out}{err}");
        let cfg = r.read("spec-dna.toml");
        let start = cfg.find("\n[change]").unwrap();
        let end = cfg.find("\n[harness]").unwrap();
        r.write(
            "spec-dna.toml",
            &format!(
                "{}\n[change]\npolicy = \"three-class\"\n{}",
                &cfg[..start],
                &cfg[end..]
            ),
        );
        r.commit("a table with the key policy only");
        let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let t = r.change().unwrap();
        assert_eq!(text(&t, "policy"), "three-class", "{profile}");
        assert_eq!(text(&t, "gate"), gate, "{profile}");
    }
}

#[test]
fn reconfigure_of_a_file_with_no_change_table_stays_with_none() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("", &[]);
    assert_eq!(code, 0, "{out}{err}");
    // A file of an older version has no table.
    let cfg = r.read("spec-dna.toml");
    let start = cfg.find("\n[change]").unwrap();
    let end = cfg.find("\n[harness]").unwrap();
    r.write(
        "spec-dna.toml",
        &format!("{}{}", &cfg[..start], &cfg[end..]),
    );
    assert!(r.change().is_none());
    r.commit("an old file");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\n  4. change policy: none (default, writes nothing)"),
        "{out}"
    );
    assert!(!out.contains("CODEOWNERS"), "{out}");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.change().is_none(), "an upgrade adds no table");
    // An answer for a policy writes the table with the 5 keys.
    let (code, out, err) = r.init_with("change-policy = \"three-class\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(text(&t, "policy"), "three-class");
    assert_eq!(text(&t, "gate"), "block");
    assert_eq!(t.len(), 5);
    // The new table is a change of its own: no SD503, and no SD502.
    r.commit("choose a policy");
    let (code, out, err) = r.run(&["check", "--base", "HEAD~1"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("SD502") && !out.contains("SD503"), "{out}");
}

#[test]
fn a_bad_policy_or_gate_is_a_usage_error_and_writes_nothing() {
    for (answers, want) in [
        ("change-policy = \"strict\"\n", "change policy `strict`"),
        ("gate = \"stop\"\n", "gate `stop`"),
        ("gate = 1\n", "`gate` must be a string"),
    ] {
        let r = Repo::empty();
        let (code, out, err) = r.init_with(answers, &[]);
        assert_eq!(code, 2, "{answers}: {out}{err}");
        assert!(err.contains(want), "{answers}: {err}");
        assert!(!r.exists("spec-dna.toml"), "{answers}");
    }
}

/// Runs `spec-dna init` on a pseudo-terminal, writes the answers, and returns
/// the exit code and the output.
#[cfg(unix)]
fn on_terminal(dir: &Path, args: &[&str], answers: &[&str]) -> (i32, String) {
    use std::io::{Read, Write};
    use std::os::fd::{FromRawFd, OwnedFd};
    use std::process::Stdio;
    let (mut master, mut slave) = (0, 0);
    // SAFETY: openpty writes two new file descriptors. The null pointers ask
    // for no name and the default terminal settings.
    let rc = unsafe {
        libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, 0, "openpty");
    // SAFETY: the descriptors are new and owned here only.
    let (master, slave) = unsafe {
        (
            std::fs::File::from_raw_fd(master),
            OwnedFd::from_raw_fd(slave),
        )
    };
    let mut child = {
        let mut cmd = Command::new(BIN);
        cmd.args(args)
            .current_dir(dir)
            .stdin(Stdio::from(slave.try_clone().unwrap()))
            .stdout(Stdio::from(slave.try_clone().unwrap()))
            .stderr(Stdio::from(slave));
        cmd.spawn().expect("binary runs")
    };
    let mut reader = master.try_clone().unwrap();
    let out = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 4096];
        // The read ends with EOF or EIO when the child closes the terminal.
        while let Ok(n) = reader.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&chunk[..n]);
        }
        buf
    });
    let mut writer = master;
    for line in answers {
        std::thread::sleep(std::time::Duration::from_millis(30));
        writer.write_all(format!("{line}\n").as_bytes()).unwrap();
    }
    let start = std::time::Instant::now();
    let code = loop {
        if let Some(s) = child.try_wait().unwrap() {
            break s.code().unwrap_or(-1);
        }
        if start.elapsed() > std::time::Duration::from_secs(30) {
            let _ = child.kill();
            panic!("init waited for more input");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    };
    drop(writer);
    let text = String::from_utf8_lossy(&out.join().unwrap()).replace("\r\n", "\n");
    (code, text)
}

#[cfg(unix)]
#[test]
fn question_5_is_asked_unless_the_policy_is_none() {
    // Questions 1 to 3 keep their defaults. The policy `none` ends the change
    // questions, so the 10 answers after it belong to the questions 6 to 14
    // (question 12 has 2 prompts).
    let r = Repo::empty();
    let none = [
        "", "", "", "none", "", "", "", "", "", "", "", "", "", "", "y",
    ];
    let (code, out) = on_terminal(&r.dir, &["init"], &none);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("4. Change policy (three-class, two-class, none) [three-class]: "),
        "{out}"
    );
    assert!(!out.contains("5. Gate"), "{out}");
    assert!(out.contains("13. Models"), "{out}");
    assert_eq!(text(&r.change().unwrap(), "policy"), "none");
    assert!(out.contains("14. Evals"), "{out}");
    // The other policies ask question 5: 15 answers and the confirmation.
    let r = Repo::empty();
    let two = [
        "",
        "",
        "",
        "two-class",
        "warn",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "",
        "y",
    ];
    let (code, out) = on_terminal(&r.dir, &["init"], &two);
    assert_eq!(code, 0, "{out}");
    assert!(
        out.contains("5. Gate (block, warn, off) [block]: "),
        "{out}"
    );
    let t = r.change().unwrap();
    assert_eq!(text(&t, "policy"), "two-class");
    assert_eq!(text(&t, "gate"), "warn");
}

#[test]
fn a_small_graph_reconfigured_to_the_default_graph_gets_both_high_risk_layers() {
    // The small graph has no `interfaces` layer, so `init` wrote `["rules"]`.
    // That is the default of the small graph, so the new graph gets its own
    // default (D17).
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(list(&r.change().unwrap(), "high-risk-layers"), ["rules"]);
    r.commit("adopt spec-dna");
    let (code, out, err) = r.init_with("graph = \"default\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(list(&t, "high-risk-layers"), ["interfaces", "rules"]);
    r.commit("reconfigure");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn a_high_risk_layers_value_that_is_not_the_default_stays_in_a_reconfigure() {
    // An empty list is a choice of the project, and it stays.
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r
        .read("spec-dna.toml")
        .replace("high-risk-layers = [\"rules\"]", "high-risk-layers = []");
    r.write("spec-dna.toml", &cfg);
    r.commit("adopt and edit");
    let (code, out, err) = r.init_with("graph = \"default\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(list(&r.change().unwrap(), "high-risk-layers").is_empty());
    // A value that names a layer of the new graph, with no default, stays too.
    let cfg = r.read("spec-dna.toml").replace(
        "high-risk-layers = []",
        "high-risk-layers = [\"interfaces\"]",
    );
    r.write("spec-dna.toml", &cfg);
    r.commit("edit again");
    let (code, out, err) = r.init_with("graph = \"default\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(
        list(&r.change().unwrap(), "high-risk-layers"),
        ["interfaces"]
    );
}

/// A committed project of the small graph whose `spec-dna.toml` has `types`.
fn project_with_types(types: &str) -> Repo {
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let cfg = r.read("spec-dna.toml");
    let at = cfg.find("profile = ").unwrap();
    r.write(
        "spec-dna.toml",
        &format!("{}types = {types}\n{}", &cfg[..at], &cfg[at..]),
    );
    r.commit("adopt spec-dna");
    r
}

#[test]
fn reconfigure_warns_when_types_lacks_a_type_of_the_policy() {
    let r = project_with_types("[\"requirements\", \"intent\"]");
    let want = "warning: the policy three-class needs the types intent, spec and plan; add spec and plan to types in spec-dna.toml";
    // The preview and the write both print the warning, once.
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.matches(want).count(), 1, "{out}");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(out.matches(want).count(), 1, "{out}");
    // `init` does not edit the key: no question asks for it.
    let cfg = r.read("spec-dna.toml");
    assert!(
        cfg.contains("types = [\"requirements\", \"intent\"]\n"),
        "{cfg}"
    );
    // `--json` has the warning in the field `warnings`.
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive", "--json"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(
        out.contains("\"warnings\":[\"the policy three-class needs"),
        "{out}"
    );
}

#[test]
fn reconfigure_names_each_missing_type_and_warns_only_when_one_is_missing() {
    for (types, add) in [
        ("[\"requirements\"]", "intent, spec and plan"),
        ("[\"requirements\", \"intent\", \"spec\"]", "plan"),
    ] {
        let r = project_with_types(types);
        let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
        assert_eq!(code, 0, "{out}{err}");
        assert!(
            out.contains(&format!("; add {add} to types in spec-dna.toml")),
            "{types}: {out}"
        );
    }
    // No warning: the 3 types are there, the policy is `none`, or the file
    // has no key `types` (each type is valid).
    let r = project_with_types("[\"requirements\", \"intent\", \"spec\", \"plan\"]");
    let (_, out, _) = r.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
    assert!(!out.contains("warning:"), "{out}");
    let r = project_with_types("[\"requirements\", \"intent\"]");
    let (code, out, err) = r.init_with(
        "change-policy = \"none\"\n",
        &["--reconfigure", "--dry-run"],
    );
    assert_eq!(code, 0, "{out}{err}");
    assert!(!out.contains("warning:"), "{out}");
    let r = Repo::empty();
    let (code, out, err) = r.run(&["init", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    r.commit("adopt spec-dna");
    let (_, out, _) = r.run(&["init", "--reconfigure", "--non-interactive", "--dry-run"]);
    assert!(!out.contains("warning:"), "{out}");
}

// The keys `approval` and `approvers` (design 0.7.0, D15). With a `user.email`,
// `lite` and `standard` write `approval = "branch"` and the e-mail as the one
// approver. `strict` writes `base`. With no `user.email`, `init` writes `base`
// and no approvers, and it prints a note.

#[test]
fn lite_and_standard_write_branch_and_the_git_e_mail() {
    for profile in ["lite", "standard"] {
        let r = Repo::empty();
        let (code, out, err) = r.init_with(&format!("profile = \"{profile}\"\n"), &[]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let t = r.change().unwrap();
        assert_eq!(text(&t, "approval"), "branch", "{profile}");
        assert_eq!(list(&t, "approvers"), [OWNER], "{profile}");
        r.commit("init");
        let (code, out, _) = r.run(&["check", "--all"]);
        assert_eq!(code, 0, "{profile}: {out}");
    }
}

#[test]
fn strict_writes_base_and_no_approvers() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("profile = \"strict\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(text(&t, "approval"), "base");
    assert!(!t.contains_key("approvers"), "{t:?}");
    r.commit("init");
    let (code, out, _) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}");
}

#[test]
fn with_no_user_e_mail_init_writes_base_and_prints_a_note() {
    for profile in ["lite", "standard", "strict"] {
        let r = Repo::without_email();
        let (code, out, err) = r.init_with(&format!("profile = \"{profile}\"\n"), &[]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
        let t = r.change().unwrap();
        assert_eq!(text(&t, "approval"), "base", "{profile}");
        assert!(!t.contains_key("approvers"), "{profile}: {t:?}");
        if profile != "strict" {
            // The note names git and the 2 keys to set.
            let all = format!("{out}{err}");
            assert!(
                all.contains("note: ")
                    && all.contains("user.email")
                    && all.contains("[change] approval")
                    && all.contains("[change] approvers"),
                "{profile}: {all}"
            );
        }
        r.commit("init");
        let (code, out, err) = r.run(&["check", "--all"]);
        assert_eq!(code, 0, "{profile}: {out}{err}");
    }
}

#[test]
fn the_policy_none_writes_no_approval_keys_when_it_writes_no_table() {
    // A project with no `[change]` table keeps none, so it gets no approval key.
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        &format!("spec-dna = \"{VERSION}\"\n\n[[roots]]\npath = \"docs\"\n"),
    );
    r.write("docs/terms.md", "# Terms\n");
    r.commit("start");
    let (code, out, err) = r.init_with("graph = \"small\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert!(r.change().is_none(), "{}", r.read("spec-dna.toml"));
}

#[test]
fn reconfigure_keeps_the_approval_keys_of_the_file() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("graph = \"default\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    // The project sets `base` and 2 e-mails by hand. `init` never overwrites it.
    let cfg = r
        .read("spec-dna.toml")
        .replace("approval = \"branch\"", "approval = \"base\"")
        .replace(
            &format!("approvers = [\"{OWNER}\"]"),
            "approvers = [\"a@example.test\", \"b@example.test\"]",
        );
    assert!(cfg.contains("approval = \"base\""), "{cfg}");
    r.write("spec-dna.toml", &cfg);
    r.commit("adopt and edit");
    let (code, out, err) = r.run(&["init", "--reconfigure", "--non-interactive"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(text(&t, "approval"), "base");
    assert_eq!(list(&t, "approvers"), ["a@example.test", "b@example.test"]);
}

#[test]
fn reconfigure_adds_the_keys_to_a_file_of_0_6() {
    // A table of 0.6 has no approval key. `--reconfigure` writes the keys of
    // the profile (D41).
    let r = Repo::empty();
    r.write(
        "spec-dna.toml",
        &format!(
            "spec-dna = \"{VERSION}\"\nprofile = \"standard\"\n\n[[roots]]\npath = \"docs\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\n"
        ),
    );
    r.write("docs/terms.md", "# Terms\n");
    r.commit("start");
    let (code, out, err) = r.init_with("graph = \"default\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    let t = r.change().unwrap();
    assert_eq!(text(&t, "approval"), "branch");
    assert_eq!(list(&t, "approvers"), [OWNER]);
}

#[test]
fn a_switch_to_strict_writes_base() {
    let r = Repo::empty();
    let (code, out, err) = r.init_with("profile = \"standard\"\n", &[]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(text(&r.change().unwrap(), "approval"), "branch");
    r.commit("init");
    let (code, out, err) = r.init_with("profile = \"strict\"\n", &["--reconfigure"]);
    assert_eq!(code, 0, "{out}{err}");
    assert_eq!(text(&r.change().unwrap(), "approval"), "base");
    r.commit("strict");
    let (code, out, err) = r.run(&["check", "--all"]);
    assert_eq!(code, 0, "{out}{err}");
}

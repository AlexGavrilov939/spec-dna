//! The release check (ADR 0031): `scripts/verify-release.sh` verifies the 9
//! assets of a release with `gh attestation verify`. A stub `gh` is first on
//! `PATH`, so no test uses the network. The stub records its arguments. The
//! asset names come from the targets of `src/release.rs`. The fixture folder
//! comes from `scripts/release-assets.sh`, so the sums are real.
#![cfg(unix)]

mod common;

#[allow(dead_code)]
#[path = "../src/release.rs"]
mod release;

use common::{Repo, path_with, stub_bin};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const TAG: &str = "v9.9.9";
const VERSION: &str = "9.9.9";
const REPO: &str = "example-owner/example-repo";

/// The stub `gh`. It records the arguments of each call in `$STUB_LOG`.
/// `$STUB_MODE`: `silent` (exit 0, no output: the defect of 0.7.1), `zero`
/// (print 0), `one` (print 1), `fail-one` (exit 1 for the asset that ends with
/// `$STUB_FAIL`, print 1 for the others) or `download-fail` (`release download`
/// exits 1). `release download` copies `$STUB_ASSETS` in the other modes.
const STUB_GH: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> "$STUB_LOG"
case "$1 $2" in
    "release download")
        if [ "$STUB_MODE" = download-fail ]; then
            echo "release not found: $3" >&2
            exit 1
        fi
        dir=
        while [ $# -gt 0 ]; do
            [ "$1" = "--dir" ] && dir=$2
            shift
        done
        cp "$STUB_ASSETS"/* "$dir"
        exit 0
        ;;
esac
case $STUB_MODE in
    silent) exit 0 ;;
    zero) echo 0; exit 0 ;;
    one) echo 1; exit 0 ;;
    fail-one)
        case $3 in
            *"$STUB_FAIL") echo "no attestation found for $3" >&2; exit 1 ;;
        esac
        echo 1
        exit 0
        ;;
esac
exit 3
"#;

struct Fixture {
    root: Repo,
}

impl Fixture {
    /// A folder `assets` with the 9 assets of `TAG`, and a stub `gh`.
    fn new() -> Fixture {
        let root = Repo::empty();
        let artifacts = root.dir.join("artifacts");
        for target in release::TARGETS {
            let dir = artifacts.join(format!("spec-dna-{target}"));
            fs::create_dir_all(&dir).unwrap();
            let ext = if target.contains("-windows-") {
                ".exe"
            } else {
                ""
            };
            fs::write(
                dir.join(format!("spec-dna{ext}")),
                format!("binary {target}\n"),
            )
            .unwrap();
        }
        let made = Command::new(script("release-assets.sh"))
            .arg(&artifacts)
            .arg(root.dir.join("assets"))
            .arg(VERSION)
            .output()
            .unwrap();
        assert!(made.status.success(), "release-assets.sh failed");
        stub_bin(&root.dir.join("bin"), "gh", STUB_GH);
        Fixture { root }
    }

    fn assets(&self) -> PathBuf {
        self.root.dir.join("assets")
    }

    fn log(&self) -> PathBuf {
        self.root.dir.join("gh.log")
    }

    /// The lines of the argument log of the stub: 1 line for each `gh` call.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.log())
            .unwrap_or_default()
            .lines()
            .map(String::from)
            .collect()
    }

    /// Runs the script with `args` and the stub in the mode `mode`.
    fn run(&self, mode: &str, fail: &str, args: &[&str]) -> Output {
        Command::new(script("verify-release.sh"))
            .args(args)
            .env("PATH", path_with(&self.root.dir.join("bin")))
            .env("GH_REPO", REPO)
            .env("STUB_MODE", mode)
            .env("STUB_FAIL", fail)
            .env("STUB_LOG", self.log())
            .env("STUB_ASSETS", self.assets())
            .output()
            .unwrap()
    }

    /// Runs a copy of the script in a folder of its own, with `cross` as its
    /// `cross-build.sh` (or none). The folder of the assets has only the
    /// files `keep`, so it matches the short list of targets that a broken
    /// `cross-build.sh` gives. A script that does not stop then reports a pass.
    fn run_with_cross(&self, cross: Option<&str>, keep: &[&str]) -> Output {
        let small = self.root.dir.join("small");
        fs::create_dir_all(&small).unwrap();
        for name in keep {
            fs::copy(self.assets().join(name), small.join(name)).unwrap();
        }
        let scripts = self.root.dir.join("scripts");
        fs::create_dir_all(&scripts).unwrap();
        fs::copy(
            script("verify-release.sh"),
            scripts.join("verify-release.sh"),
        )
        .unwrap();
        if let Some(body) = cross {
            stub_bin(&scripts, "cross-build.sh", body);
        }
        Command::new(scripts.join("verify-release.sh"))
            .args([TAG, small.to_str().unwrap()])
            .env("PATH", path_with(&self.root.dir.join("bin")))
            .env("GH_REPO", REPO)
            .env("STUB_MODE", "one")
            .env("STUB_FAIL", "")
            .env("STUB_LOG", self.log())
            .env("STUB_ASSETS", self.assets())
            .output()
            .unwrap()
    }

    fn verify(&self, mode: &str) -> Output {
        self.verify_with(mode, "")
    }

    fn verify_with(&self, mode: &str, fail: &str) -> Output {
        let dir = self.assets();
        self.run(mode, fail, &[TAG, dir.to_str().unwrap()])
    }
}

fn script(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("scripts")
        .join(name)
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn all_assets() -> Vec<String> {
    let mut names = release::assets(VERSION);
    names.push("SHA256SUMS".into());
    names.push("spec-dna-pin.toml".into());
    names
}

#[test]
fn the_targets_of_the_script_equal_the_targets_of_the_binary() {
    let out = Command::new(script("cross-build.sh"))
        .args(["--list", "--all"])
        .output()
        .unwrap();
    let list = text(&out.stdout);
    assert_eq!(
        list.lines().collect::<Vec<_>>(),
        release::TARGETS.to_vec(),
        "the 2 lists of targets differ"
    );
}

/// The defect of 0.7.1: `gh` exits 0 and prints nothing. That is no proof.
#[test]
fn a_gh_that_exits_0_with_no_output_fails_and_names_each_asset() {
    let f = Fixture::new();
    let out = f.verify("silent");
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let (stdout, stderr) = (text(&out.stdout), text(&out.stderr));
    for name in all_assets() {
        assert!(stderr.contains(&name), "no failure for {name}: {stderr}");
    }
    assert!(!stdout.contains("verified"), "{stdout}");
    assert!(!stdout.contains("assets verified for"), "{stdout}");
    assert_eq!(f.calls().len(), 9, "the script checks every asset");
}

#[test]
fn a_gh_that_prints_1_gives_exit_0_and_9_verified_lines() {
    let f = Fixture::new();
    let out = f.verify("one");
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    let stdout = text(&out.stdout);
    for name in all_assets() {
        assert!(
            stdout.contains(&format!("verified {name}: 1 attestation(s)\n")),
            "no line for {name}: {stdout}"
        );
    }
    assert_eq!(stdout.matches("\nverified ").count() + 1, 9, "{stdout}");
    assert!(
        stdout.ends_with(&format!(
            "verify-release: 9 of 9 assets verified for {TAG}\n"
        )),
        "{stdout}"
    );

    // Each call has the policy flags.
    let calls = f.calls();
    assert_eq!(calls.len(), 9, "{calls:?}");
    for name in all_assets() {
        let call = calls
            .iter()
            .find(|c| {
                c.starts_with(&format!(
                    "attestation verify {}/{name} ",
                    f.assets().display()
                ))
            })
            .unwrap_or_else(|| panic!("no call for {name}: {calls:?}"));
        assert!(call.contains(&format!("--repo {REPO} ")), "{call}");
        assert!(
            call.contains(&format!(
                "--signer-workflow {REPO}/.github/workflows/release.yml "
            )),
            "{call}"
        );
        assert!(
            call.contains(&format!("--source-ref refs/tags/{TAG} ")),
            "{call}"
        );
        assert!(call.contains("--deny-self-hosted-runners"), "{call}");
        assert!(call.ends_with("--format json --jq length"), "{call}");
    }
}

#[test]
fn a_gh_that_prints_0_fails_and_names_each_asset() {
    let f = Fixture::new();
    let out = f.verify("zero");
    assert_eq!(out.status.code(), Some(1));
    let stderr = text(&out.stderr);
    for name in all_assets() {
        assert!(stderr.contains(&name), "no failure for {name}: {stderr}");
    }
    assert!(!text(&out.stdout).contains("verified"));
}

#[test]
fn a_gh_that_exits_1_for_one_asset_names_it_and_checks_the_others() {
    let f = Fixture::new();
    let bad = format!("spec-dna-{VERSION}-x86_64-unknown-linux-musl");
    let out = f.verify_with("fail-one", &bad);
    assert_eq!(out.status.code(), Some(1));
    let (stdout, stderr) = (text(&out.stdout), text(&out.stderr));
    assert!(stderr.contains(&bad), "{stderr}");
    assert!(stderr.contains("exit code 1"), "{stderr}");
    assert!(stderr.contains("8 of 9"), "{stderr}");
    assert!(!stdout.contains(&format!("verified {}", bad)), "{stdout}");
    assert_eq!(stdout.matches("verified ").count(), 8, "{stdout}");
    assert!(!stdout.contains("assets verified for"), "{stdout}");
    assert_eq!(f.calls().len(), 9, "the script checks the next assets");
}

#[test]
fn an_absent_asset_fails_before_gh_runs() {
    let f = Fixture::new();
    let name = format!("spec-dna-{VERSION}-aarch64-apple-darwin");
    fs::remove_file(f.assets().join(&name)).unwrap();
    let out = f.verify("one");
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains(&name), "{}", text(&out.stderr));
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn an_extra_file_fails_before_gh_runs() {
    let f = Fixture::new();
    fs::write(f.assets().join("notes.md"), "x\n").unwrap();
    let out = f.verify("one");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("notes.md"),
        "{}",
        text(&out.stderr)
    );
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn a_hidden_file_or_a_folder_is_an_extra_file() {
    for extra in [".hidden", "sub"] {
        let f = Fixture::new();
        let path = f.assets().join(extra);
        if extra == "sub" {
            fs::create_dir(&path).unwrap();
        } else {
            fs::write(&path, "x\n").unwrap();
        }
        let out = f.verify("one");
        assert_eq!(out.status.code(), Some(1), "{extra}");
        assert!(text(&out.stderr).contains(extra), "{}", text(&out.stderr));
        assert!(f.calls().is_empty(), "{extra}");
    }
}

#[test]
fn a_wrong_sum_fails_before_gh_runs() {
    let f = Fixture::new();
    let name = format!("spec-dna-{VERSION}-x86_64-pc-windows-gnu.exe");
    fs::write(f.assets().join(&name), "changed\n").unwrap();
    let out = f.verify("one");
    assert_eq!(out.status.code(), Some(1));
    assert!(text(&out.stderr).contains(&name), "{}", text(&out.stderr));
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn a_binary_with_no_line_in_the_sums_fails_before_gh_runs() {
    let f = Fixture::new();
    let sums = f.assets().join("SHA256SUMS");
    let kept: Vec<String> = fs::read_to_string(&sums)
        .unwrap()
        .lines()
        .filter(|l| !l.contains("aarch64-apple-darwin"))
        .map(String::from)
        .collect();
    fs::write(&sums, kept.join("\n") + "\n").unwrap();
    let out = f.verify("one");
    assert_eq!(out.status.code(), Some(1));
    assert!(
        text(&out.stderr).contains("aarch64-apple-darwin"),
        "{}",
        text(&out.stderr)
    );
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn a_wrong_use_gives_exit_2() {
    let f = Fixture::new();
    let dir = f.assets();
    for args in [
        vec![],
        vec![TAG],
        vec!["--download"],
        vec!["--download", TAG, "extra"],
        vec![TAG, dir.to_str().unwrap(), "extra"],
        vec!["1.2.3", dir.to_str().unwrap()],
        vec![TAG, "/no/such/folder"],
        vec!["--unknown", TAG],
    ] {
        let out = f.run("one", "", &args);
        assert_eq!(
            out.status.code(),
            Some(2),
            "{args:?}: {}",
            text(&out.stderr)
        );
    }
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn download_fetches_the_assets_and_verifies_them() {
    let f = Fixture::new();
    let out = f.run("one", "", &["--download", TAG]);
    assert_eq!(out.status.code(), Some(0), "{}", text(&out.stderr));
    assert!(
        text(&out.stdout).ends_with(&format!(
            "verify-release: 9 of 9 assets verified for {TAG}\n"
        )),
        "{}",
        text(&out.stdout)
    );
    let calls = f.calls();
    assert_eq!(calls.len(), 10, "{calls:?}");
    assert!(
        calls[0].starts_with(&format!("release download {TAG} --repo {REPO} --dir ")),
        "{}",
        calls[0]
    );
}

#[test]
fn a_failed_download_gives_exit_1_and_names_the_tag() {
    let f = Fixture::new();
    let out = f.run("download-fail", "", &["--download", TAG]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    let stderr = text(&out.stderr);
    assert!(
        stderr.contains(&format!("cannot download the assets of {TAG}")),
        "{stderr}"
    );
    assert!(stderr.contains("release not found"), "{stderr}");
    assert!(!text(&out.stdout).contains("verified"));
    assert_eq!(f.calls().len(), 1, "{:?}", f.calls());
}

/// The list of targets comes from `cross-build.sh`. If it fails, the script
/// must not check a shorter list and report a pass.
#[test]
fn a_cross_build_that_exits_1_with_no_target_fails_before_gh_runs() {
    let f = Fixture::new();
    let out = f.run_with_cross(
        Some("#!/bin/sh\nexit 1\n"),
        &["SHA256SUMS", "spec-dna-pin.toml"],
    );
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    assert!(!text(&out.stdout).contains("verified"));
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn a_cross_build_that_prints_1_target_then_exits_1_fails_before_gh_runs() {
    let f = Fixture::new();
    let out = f.run_with_cross(
        Some("#!/bin/sh\necho aarch64-apple-darwin\nexit 1\n"),
        &[
            "SHA256SUMS",
            "spec-dna-pin.toml",
            "spec-dna-9.9.9-aarch64-apple-darwin",
        ],
    );
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    assert!(!text(&out.stdout).contains("verified"));
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn a_cross_build_that_prints_no_target_fails_before_gh_runs() {
    let f = Fixture::new();
    let out = f.run_with_cross(
        Some("#!/bin/sh\nexit 0\n"),
        &["SHA256SUMS", "spec-dna-pin.toml"],
    );
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    assert!(!text(&out.stdout).contains("verified"));
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

#[test]
fn a_missing_cross_build_fails_before_gh_runs() {
    let f = Fixture::new();
    let out = f.run_with_cross(None, &["SHA256SUMS", "spec-dna-pin.toml"]);
    assert_eq!(out.status.code(), Some(1), "{}", text(&out.stderr));
    assert!(!text(&out.stdout).contains("verified"));
    assert!(f.calls().is_empty(), "{:?}", f.calls());
}

//! The release targets and the release asset names (ADR 0003, amendment
//! "phase 2"). `scripts/cross-build.sh --list --all` gives the same targets.

/// The 7 release targets, in the order of `scripts/cross-build.sh`.
pub const TARGETS: [&str; 7] = [
    "aarch64-apple-darwin",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-unknown-linux-musl",
    "x86_64-pc-windows-gnu",
    "aarch64-pc-windows-gnullvm",
];

/// The release target of this binary. It is `None` on any other platform.
pub const TARGET: Option<&str> = if cfg!(all(target_arch = "aarch64", target_os = "macos")) {
    Some("aarch64-apple-darwin")
} else if cfg!(all(target_arch = "x86_64", target_os = "macos")) {
    Some("x86_64-apple-darwin")
} else if cfg!(all(
    target_arch = "x86_64",
    target_os = "linux",
    target_env = "gnu"
)) {
    Some("x86_64-unknown-linux-gnu")
} else if cfg!(all(
    target_arch = "aarch64",
    target_os = "linux",
    target_env = "gnu"
)) {
    Some("aarch64-unknown-linux-gnu")
} else if cfg!(all(
    target_arch = "x86_64",
    target_os = "linux",
    target_env = "musl"
)) {
    Some("x86_64-unknown-linux-musl")
} else if cfg!(all(
    target_arch = "x86_64",
    target_os = "windows",
    target_env = "gnu",
    target_abi = ""
)) {
    Some("x86_64-pc-windows-gnu")
} else if cfg!(all(
    target_arch = "aarch64",
    target_os = "windows",
    target_env = "gnu",
    target_abi = "llvm"
)) {
    Some("aarch64-pc-windows-gnullvm")
} else {
    None
};

/// The name of the release asset of a target: `spec-dna-<version>-<target>`,
/// with `.exe` for a Windows target.
pub fn asset_name(version: &str, target: &str) -> String {
    let ext = if target.contains("-windows-") {
        ".exe"
    } else {
        ""
    };
    format!("spec-dna-{version}-{target}{ext}")
}

/// The names of the 7 assets of a version.
pub fn assets(version: &str) -> Vec<String> {
    TARGETS.iter().map(|t| asset_name(version, t)).collect()
}

/// The name of the asset of this binary, or `None` if its platform is not a
/// release target.
pub fn own_asset(version: &str) -> Option<String> {
    TARGET.map(|t| asset_name(version, t))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_targets_equal_the_list_of_the_build_script() {
        let script = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/cross-build.sh");
        let out = std::process::Command::new("bash")
            .args([script, "--list", "--all"])
            .output()
            .expect("bash runs the build script");
        assert!(out.status.success());
        let listed: Vec<&str> = std::str::from_utf8(&out.stdout).unwrap().lines().collect();
        assert_eq!(listed, TARGETS);
    }

    #[test]
    fn the_target_of_the_binary_is_a_release_target() {
        if let Some(t) = TARGET {
            assert!(TARGETS.contains(&t));
        }
    }

    #[test]
    fn only_the_windows_assets_end_in_exe() {
        let names = assets("1.2.3");
        assert_eq!(names.len(), 7);
        for n in &names {
            assert!(n.starts_with("spec-dna-1.2.3-"));
            assert_eq!(n.ends_with(".exe"), n.contains("-windows-"), "{n}");
        }
        assert_eq!(
            names[5],
            "spec-dna-1.2.3-x86_64-pc-windows-gnu.exe".to_string()
        );
        assert_eq!(names[0], "spec-dna-1.2.3-aarch64-apple-darwin".to_string());
    }
}

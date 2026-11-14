//! `cargo install --git <url> --tag <tag>` without a package name fails if the
//! repository has more than one package with a binary. Cargo searches the
//! whole checkout, except the directories whose name starts with a dot and
//! the `target` directories. This test keeps the helper packages (the
//! benchmarks and the fixtures) free of binaries.

use std::fs;
use std::path::{Path, PathBuf};

fn manifests(dir: &Path, out: &mut Vec<PathBuf>) {
    for e in fs::read_dir(dir).unwrap() {
        let e = e.unwrap();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') || name == "target" || !e.file_type().unwrap().is_dir() {
            if name == "Cargo.toml" {
                out.push(e.path());
            }
            continue;
        }
        manifests(&e.path(), out);
    }
}

#[test]
fn only_the_root_package_has_a_binary() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut found = Vec::new();
    manifests(root, &mut found);
    let nested: Vec<&PathBuf> = found.iter().filter(|p| p.parent() != Some(root)).collect();
    assert!(
        !nested.is_empty(),
        "the helper packages are gone: update this test"
    );
    for m in nested {
        let dir = m.parent().unwrap();
        let text = fs::read_to_string(m).unwrap();
        let shown = m.strip_prefix(root).unwrap().display();
        assert!(
            !text.contains("[[bin]]"),
            "{shown}: a helper package must not declare a binary (use [[example]])"
        );
        let implicit = dir.join("src/main.rs").exists() || dir.join("src/bin").exists();
        assert!(
            !implicit || text.contains("autobins = false"),
            "{shown}: has src/main.rs or src/bin; set `autobins = false` and use [[example]]"
        );
    }
}

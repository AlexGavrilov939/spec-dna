---
status: approved
---
# Readme and examples

## Files

- `README.md` The new front page.
- `examples/**` The 3 example scripts, their pages and the shared functions.
- `tests/examples.rs` The test that runs the examples and compares the pages with the output.
- `docs/troubleshooting.md` The new page of frequent stops.
- `docs/assets/logo.svg` The logo, light variant.
- `docs/assets/logo-dark.svg` The logo, dark variant.
- `.github/social-preview.png` The social preview image for the GitHub settings.
- `docs/README.md` The map gets the examples and the troubleshooting page.
- `docs/quickstart.md` The install commands of 0.6.1. The commands make `~/.local/bin` before `install`.
- `docs/guides/existing-repo.md` A sentence about the proposed roots of `init`.
- `CONTRIBUTING.md` The layout table gets `examples/`.

## Order

- [x] t1: Write the 3 examples and their pages. Files: `examples/**`. Verify: each `run.sh` exits with code 0 twice with the same output.
- [x] t2: Add the test of the examples and the README. Files: `tests/examples.rs`. Verify: `cargo test --test examples`.
- [x] t3: Write the README, the troubleshooting page and the logo. Files: `README.md`, `docs/troubleshooting.md`, `docs/assets/logo.svg`, `docs/assets/logo-dark.svg`. Verify: the test of t2 and a render at 1280 and 390 pixels in light and dark.
- [x] t4: Make the social preview image. Files: `.github/social-preview.png`. Verify: the image is 1280 by 640 pixels and less than 1 MB.
- [x] t5: Update the map, the quickstart, the guide and the layout table. Files: `docs/README.md`, `docs/quickstart.md`, `docs/guides/existing-repo.md`, `CONTRIBUTING.md`. Verify: `spec-dna check --base origin/master` and `pre-commit run --all-files`.

## Risks

- A page can show output that the binary does not give. The test of t2 compares each `console` block with the real output, after it masks the version and the commit ids.
- The install commands name v0.6.1, which has no tag before the release. Merge this change after the tag `v0.6.1`.
- The 0.6.1 branch changes the version from 0.6.0 to 0.6.1 in `README.md` and `docs/quickstart.md`. A trial merge with its head `f5de7ca` merges `docs/quickstart.md` with no conflict and gives a conflict in `README.md`. To resolve it, keep the `README.md` of this change: it replaces the old text and names 0.6.1.

## Proof

- `cargo test --test examples` passes. It runs the 3 scripts and checks `README.md` and the 3 example pages.
- `spec-dna check --base origin/master` gives no new error.
- `pre-commit run --all-files` passes.

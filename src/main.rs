//! spec-dna keeps engineering specifications small, linked and
//! machine-checked. It serves AI coding agents the minimum sufficient
//! context. See README.md for commands, exit codes and diagnostics.

mod adapters;
mod check;
mod cite;
mod config;
mod context;
mod diag;
mod drift;
mod frontmatter;
mod git;
mod glob;
mod hash;
mod index;
mod json;
mod lint;
mod model;
mod mv;
mod parse;
mod rank;
mod repo;
mod split;
mod transcript;

use crate::adapters::VERSION;
use crate::diag::Severity;
use crate::json::Obj;
use crate::repo::{Fatal, Repo, Snapshot};
use std::process::ExitCode;

const USAGE: &str = "usage: spec-dna <command> [options]

Commands:
    init [--root <dir>]
        Writes spec-dna.toml (if it does not exist) and the agent adapters.
    check [--base <rev>] [--all] [--lint]
        Reports the diagnostics that are new against the merge base of <rev>
        and HEAD (default: HEAD). With --all, it reports every diagnostic.
        With --base, it reads tracked files only, as CI does. It prints the
        errors first, then the drift warnings, then the other warnings. The
        prose lint warnings (SD801-SD805) are one count for each code;
        --lint lists each of them. --json lists all diagnostics.
    context <target>... [--path <file>]... [--budget <chars>] [--query <text>]
            [--include-inactive]
        Assembles the sections that an agent needs for the targets. With
        --query, the sections that match the words of the text come first.
        A superseded, retired or rejected document is only listed, unless
        --include-inactive is set or a target names it.
    show <id>[#<anchor>]
        Prints a document, a section or a requirement.
    mv <from> <to>
    mv <doc-id>#<key or section>... <spec-id>
        Moves a document. Or moves requirements and whole sections to an
        existing or a new spec, with an alias for every moved key of HEAD. A
        preservation check runs first; if it fails, nothing changes.
    transcript <file or dir>... [--expect <id>]... [--format claude-code]
        Reports token usage, tool calls and document reads per session and
        episode of agent transcripts. It prints counts and ids only.

Options:
    --json        Prints machine-readable output
    --no-cache    Does not read or write the index
    -h, --help    Prints this help
    -V, --version Prints the version

Exit codes: 0 ok, 1 check found new errors, 2 usage, 3 configuration,
4 git or I/O, 5 not found.";

struct Args {
    command: String,
    positional: Vec<String>,
    base: Option<String>,
    all: bool,
    json: bool,
    no_cache: bool,
    paths: Vec<String>,
    budget: Option<u32>,
    root: Option<String>,
    query: Option<String>,
    include_inactive: bool,
    expect: Vec<String>,
    format: Option<String>,
    lint: bool,
}

fn parse_args(raw: Vec<String>) -> Result<Args, Fatal> {
    let mut a = Args {
        command: String::new(),
        positional: Vec::new(),
        base: None,
        all: false,
        json: false,
        no_cache: false,
        paths: Vec::new(),
        budget: None,
        root: None,
        query: None,
        include_inactive: false,
        expect: Vec::new(),
        format: None,
        lint: false,
    };
    let mut it = raw.into_iter();
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if f.starts_with("--") => (f.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| -> Result<String, Fatal> {
            inline
                .clone()
                .or_else(|| it.next())
                .ok_or_else(|| Fatal::Usage(format!("{name} needs a value")))
        };
        match flag.as_str() {
            "-h" | "--help" => a.command = "help".into(),
            "-V" | "--version" => a.command = "version".into(),
            "--json" => a.json = true,
            "--all" => a.all = true,
            "--lint" => a.lint = true,
            "--no-cache" => a.no_cache = true,
            "--include-inactive" => a.include_inactive = true,
            "--base" => a.base = Some(value("--base")?),
            "--path" => a.paths.push(value("--path")?),
            "--root" => a.root = Some(value("--root")?),
            "--query" => a.query = Some(value("--query")?),
            "--expect" => a.expect.push(value("--expect")?),
            "--format" => a.format = Some(value("--format")?),
            "--budget" => {
                let v = value("--budget")?;
                a.budget = Some(
                    v.parse()
                        .map_err(|_| Fatal::Usage(format!("--budget: `{v}` is not a number")))?,
                );
            }
            s if s.starts_with('-') && s.len() > 1 => {
                return Err(Fatal::Usage(format!("unknown option `{s}`")));
            }
            _ if a.command.is_empty() => a.command = arg,
            _ => a.positional.push(arg),
        }
    }
    Ok(a)
}

fn main() -> ExitCode {
    let args = match parse_args(std::env::args().skip(1).collect()) {
        Ok(a) => a,
        Err(e) => return fail(&e),
    };
    match run(args) {
        Ok(code) => ExitCode::from(code),
        Err(e) => fail(&e),
    }
}

fn fail(e: &Fatal) -> ExitCode {
    eprintln!("spec-dna: {}", e.message());
    if matches!(e, Fatal::Usage(_)) {
        eprintln!("Run `spec-dna --help` for usage.");
    }
    ExitCode::from(e.exit_code() as u8)
}

fn run(a: Args) -> Result<u8, Fatal> {
    let cwd = std::env::current_dir().map_err(|e| Fatal::Io(e.to_string()))?;
    match a.command.as_str() {
        "" | "help" => {
            println!("{USAGE}");
            Ok(0)
        }
        "version" => {
            println!("spec-dna {VERSION}");
            Ok(0)
        }
        "init" => init(&Repo::discover(&cwd)?, &a),
        "check" => check(&Repo::discover(&cwd)?, &a),
        "context" => context(&Repo::discover(&cwd)?, &a),
        "show" => show(&Repo::discover(&cwd)?, &a),
        "mv" => mv(&Repo::discover(&cwd)?, &a),
        "transcript" => transcript(&cwd, &a),
        other => Err(Fatal::Usage(format!("unknown command `{other}`"))),
    }
}

fn expect_args(a: &Args, min: usize, max: usize) -> Result<(), Fatal> {
    let n = a.positional.len();
    if n < min || n > max {
        return Err(Fatal::Usage(format!(
            "`{}` takes {min}{} arguments, got {n}",
            a.command,
            if max > min { "+" } else { "" }
        )));
    }
    Ok(())
}

fn warn_pin(cfg: &config::Config) {
    if cfg.pin != VERSION {
        eprintln!(
            "spec-dna: warning: spec-dna.toml pins {}, this binary is {VERSION}",
            cfg.pin
        );
    }
}

const DEFAULT_CONFIG: &str = "# spec-dna configuration. Reference: https://github.com/AlexGavrilov939/spec-dna/blob/master/docs/config.md
spec-dna = \"{version}\"
# The git source of spec-dna for `cargo install --git`, for example a local or
# private clone in phase 1. The message of a version mismatch uses it.
# install-source = \"https://github.com/AlexGavrilov939/spec-dna\"

# Each root is a directory of documents. The id of a document is its path
# under the root, without the extension.
[[roots]]
path = \"{root}\"
type = \"mixed\"
default-type = \"spec\"
status = \"draft\"
";

/// The error for a binary whose version differs from the pin (ADR 0003).
fn pin_mismatch(cfg: &config::Config) -> Fatal {
    let (pin, source) = (&cfg.pin, &cfg.install_source);
    Fatal::Config(format!(
        "spec-dna.toml pins version {pin}, but this binary is {VERSION}.\n  To use {pin}, install it:\n    cargo install --locked --force --git {source} --tag v{pin} spec-dna\n  To move the project to {VERSION}, set `spec-dna = \"{VERSION}\"` in spec-dna.toml and run `spec-dna init`."
    ))
}

fn init(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    let mut written = Vec::new();
    let cfg_path = repo.config_path();
    if cfg_path.exists() {
        let cfg = repo.load_config()?;
        if cfg.pin != VERSION {
            return Err(pin_mismatch(&cfg));
        }
    } else {
        let root = a.root.as_deref().unwrap_or("docs").trim_matches('/');
        let text = DEFAULT_CONFIG
            .replace("{version}", VERSION)
            .replace("{root}", if root.is_empty() { "." } else { root });
        mv::write(&cfg_path, &text)?;
        written.push(config::FILE_NAME.to_string());
    }
    // One line in each root instruction file that an agent reads (ADR 0003,
    // amendment of 2026-11-08). AGENTS.md gets it if the file exists, or if
    // there is no instruction file. CLAUDE.md gets it, unless it imports an
    // AGENTS.md that has the line.
    // An older version wrote a `claude` block that imported AGENTS.md. If
    // AGENTS.md has text of its own, the import stays as a plain line.
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).unwrap_or_default();
    let exists = |p: &str| repo.root.join(p).exists();
    let agents_target = exists(adapters::AGENTS) || !exists(adapters::CLAUDE);
    let claude = adapters::migrate_claude(
        &read(adapters::CLAUDE),
        adapters::has_own_text(&read(adapters::AGENTS)),
    );
    let imports = adapters::imports_agents(&claude);
    let claude_target = !(agents_target && imports);
    for (path, target) in [
        (adapters::AGENTS, agents_target),
        (adapters::CLAUDE, claude_target),
    ] {
        let old = read(path);
        let current = if path == adapters::CLAUDE {
            claude.clone()
        } else {
            old.clone()
        };
        // A file that the agent reads through an import keeps no second copy.
        let new = if target {
            adapters::upsert_line(&current)
        } else {
            adapters::remove_line(&current)
        };
        if new != old {
            mv::write(&repo.root.join(path), &new)?;
            written.push(path.to_string());
        }
    }
    let skill_path = repo.root.join(adapters::SKILL);
    let skill = read(adapters::SKILL);
    if !skill.is_empty() && !skill.contains("<!-- spec-dna:begin skill ") {
        return Err(Fatal::Usage(format!(
            "{} exists and was not generated by spec-dna; move it first",
            adapters::SKILL
        )));
    }
    if skill != adapters::skill_file() {
        std::fs::create_dir_all(skill_path.parent().unwrap())
            .map_err(|e| Fatal::Io(e.to_string()))?;
        mv::write(&skill_path, &adapters::skill_file())?;
        written.push(adapters::SKILL.to_string());
    }
    // The bytes that spec-dna adds to the prompt prefix of each request.
    // Claude Code reads one copy of the line (in CLAUDE.md, or in AGENTS.md
    // through an import) and the skill descriptions. Other agents read
    // AGENTS.md.
    let line = adapters::line().len() + 1;
    let claude_line = line;
    let skill_bytes = adapters::skill_prefix_bytes();
    let agents_bytes = if agents_target { line } else { 0 };
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .strs("written", &written)
                .raw(
                    "prefix_bytes",
                    &Obj::new()
                        .num("claude_code", claude_line + skill_bytes)
                        .num("agents_md", agents_bytes)
                        .end()
                )
                .end()
        );
    } else {
        if written.is_empty() {
            println!("spec-dna init: everything is up to date");
        }
        for w in &written {
            println!("wrote {w}");
        }
        println!(
            "per-request prefix: {} bytes for Claude Code (line {claude_line}, skill description {skill_bytes}), {agents_bytes} bytes for AGENTS.md",
            claude_line + skill_bytes
        );
        if !written.is_empty() {
            println!("to commit: git add {}", written.join(" "));
        }
    }
    Ok(0)
}

fn adapter_texts(repo: &Repo, snap: &Snapshot) -> Result<Vec<(&'static str, String)>, Fatal> {
    let mut out = Vec::new();
    if snap.worktree {
        for p in adapters::PATHS {
            if snap.blob_of(p).is_some()
                && let Ok(t) = std::fs::read_to_string(repo.root.join(p))
            {
                out.push((p, t));
            }
        }
    } else {
        let ids: Vec<(&'static str, String)> = adapters::PATHS
            .iter()
            .filter_map(|p| {
                snap.blob_of(p)
                    .and_then(|e| e.blob.clone())
                    .map(|b| (*p, b))
            })
            .collect();
        let blobs = repo
            .git
            .read_blobs(&ids.iter().map(|i| i.1.clone()).collect::<Vec<_>>())
            .map_err(Fatal::Io)?;
        for (p, b) in ids {
            if let Some(bytes) = blobs.get(&b) {
                out.push((p, String::from_utf8_lossy(bytes).into_owned()));
            }
        }
    }
    Ok(out)
}

fn check(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    let cfg = repo.load_config()?;
    if cfg.pin != VERSION {
        return Err(pin_mismatch(&cfg));
    }
    // The base is the merge base of --base and HEAD, or HEAD. If there is no
    // commit, the base is None. `--all` uses the same base: it changes only
    // which diagnostics `check` prints and which errors fail it.
    let mut t = std::time::Instant::now();
    let base_commit = match &a.base {
        Some(rev) => {
            let c = repo.git.commit(rev).ok_or_else(|| {
                Fatal::Config(format!(
                    "base `{rev}` is not a commit here; fetch it first (a depth-1 fetch of the base commit is enough)"
                ))
            })?;
            Some(match repo.head_commit() {
                Some(h) => repo.git.merge_base(&c, &h).unwrap_or(c),
                None => c,
            })
        }
        None => repo.head_commit(),
    };
    repo::trace("resolve base", &mut t);
    // List the working tree and the base tree in parallel.
    let roots = repo::root_paths(&cfg);
    let (head_snap, base_snap) = std::thread::scope(|s| {
        let base = s.spawn(|| match &base_commit {
            Some(c) => Snapshot::at(repo, c),
            None => Ok(Snapshot::empty()),
        });
        // A gate against `--base` reads tracked files only, as CI does
        // (ADR 0001, amendments).
        let head = Snapshot::worktree(repo, &roots, a.base.is_none());
        (
            head,
            base.join()
                .unwrap_or_else(|_| Err(Fatal::Io("listing the base tree failed".into()))),
        )
    });
    let (head_snap, base_snap) = (head_snap?, base_snap?);
    let use_base = base_commit.is_some();
    let snaps: Vec<&Snapshot> = if use_base {
        vec![&head_snap, &base_snap]
    } else {
        vec![&head_snap]
    };
    repo::trace("git base tree", &mut t);
    let loaded = repo::load(repo, &cfg, &snaps, !a.no_cache, true)?;
    t = std::time::Instant::now();
    let head = &loaded.corpora[0];
    let base = loaded.corpora.get(1);
    let mut outcome = check::gate(&cfg, base, head);
    repo::trace("diagnose", &mut t);
    if let Some(base) = base {
        let drift = drift::check(
            repo,
            &cfg,
            (base, &base_snap),
            (head, &head_snap),
            a.base.is_none(),
        )?;
        outcome.new.extend(std::iter::repeat_n(true, drift.len()));
        outcome.head.extend(drift);
        repo::trace("drift", &mut t);
    }
    let head_ad = adapters::check(&adapter_texts(repo, &head_snap)?);
    let base_ad: Vec<_> = if use_base {
        adapters::check(&adapter_texts(repo, &base_snap)?)
    } else {
        Vec::new()
    };
    for d in head_ad {
        let is_new = !base_ad.iter().any(|b| b.fingerprint() == d.fingerprint());
        outcome.head.push(d);
        outcome.new.push(is_new);
    }

    let count = |sev: Severity, only_new: bool| {
        outcome
            .head
            .iter()
            .zip(&outcome.new)
            .filter(|(d, n)| d.severity == sev && (!only_new || **n))
            .count()
    };
    let (errors, warnings) = (
        count(Severity::Error, false),
        count(Severity::Warning, false),
    );
    let (new_errors, new_warnings) = (count(Severity::Error, true), count(Severity::Warning, true));
    let failed = if a.all { errors > 0 } else { new_errors > 0 };
    let mut order: Vec<usize> = (0..outcome.head.len()).collect();
    order.sort_by(|&x, &y| outcome.head[x].cmp(&outcome.head[y]));
    if a.json {
        let diags = order
            .iter()
            .filter(|&&i| a.all || outcome.new[i])
            .map(|&i| outcome.head[i].json(outcome.new[i]));
        let summary = Obj::new()
            .num("documents", head.docs.len())
            .num("errors", errors)
            .num("warnings", warnings)
            .num("new_errors", new_errors)
            .num("new_warnings", new_warnings)
            .num(
                "requirements",
                head.docs.iter().map(|d| d.reqs.len()).sum::<usize>(),
            )
            .num("parsed", loaded.parsed)
            .num("cached", loaded.cached)
            .end();
        let docs = head.docs.iter().map(|d| {
            Obj::new()
                .str("id", &d.id)
                .str("path", &d.path)
                .str("type", d.ty.name())
                .opt_str("status", d.status.as_deref())
                .num("chars", d.parsed.body_chars)
                .num("requirements", d.reqs.len())
                .end()
        });
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .str("mode", if a.all { "all" } else { "base" })
                .opt_str("base", base_commit.as_deref())
                .bool("ok", !failed)
                .raw("summary", &summary)
                .raw("diagnostics", &json::array(diags))
                .raw("documents", &json::array(docs))
                .end()
        );
    } else {
        let shown: Vec<&diag::Diag> = order
            .iter()
            .filter(|&&i| a.all || outcome.new[i])
            .map(|&i| &outcome.head[i])
            .collect();
        for line in human_lines(&shown, a.lint) {
            println!("{line}");
        }
        let n = |count: usize, what: &str| {
            format!("{count} {what}{}", if count == 1 { "" } else { "s" })
        };
        let total = format!("{} and {}", n(errors, "error"), n(warnings, "warning"));
        let counts = match (&base_commit, a.all) {
            (_, true) => total,
            (Some(c), false) => format!(
                "{} and {} against {} ({total} in total)",
                n(new_errors, "new error"),
                n(new_warnings, "new warning"),
                &c[..c.len().min(12)]
            ),
            (None, false) => format!("{total} (no base commit, so all are new)"),
        };
        println!("spec-dna check: {} documents; {counts}", head.docs.len());
    }
    Ok(if failed { 1 } else { 0 })
}

/// The lines of the human output of `check`: the errors, the drift
/// warnings (SD606, SD607), the other warnings, and then the prose lint
/// warnings. Without `lint`, the prose lint warnings are one line for each
/// code, with the count and the documents.
fn human_lines(diags: &[&diag::Diag], lint: bool) -> Vec<String> {
    use diag::{Code, Severity};
    let rank = |d: &diag::Diag| match (d.severity, d.code) {
        (Severity::Error, _) => 0,
        (_, Code::CitedPathRemoved | Code::CitedSymbolRemoved) => 1,
        (_, c) if c.is_prose_lint() => 3,
        _ => 2,
    };
    let mut sorted: Vec<&diag::Diag> = diags.to_vec();
    sorted.sort_by_key(|d| rank(d));
    let mut out = Vec::new();
    let mut counts: Vec<(Code, usize, Vec<&str>)> = Vec::new();
    for d in sorted {
        if lint || rank(d) < 3 {
            out.push(d.text());
            continue;
        }
        match counts.iter_mut().find(|c| c.0 == d.code) {
            Some(c) => {
                c.1 += 1;
                if !c.2.contains(&d.path.as_str()) {
                    c.2.push(&d.path);
                }
            }
            None => counts.push((d.code, 1, vec![&d.path])),
        }
    }
    counts.sort_by_key(|c| c.0);
    for (code, n, paths) in counts {
        let place = match paths.as_slice() {
            [one] => one.to_string(),
            many => format!("{} documents", many.len()),
        };
        out.push(format!(
            "warning {} {}: {n} in {place} (`check --lint` lists them)",
            code.code(),
            code.name()
        ));
    }
    out
}

fn load_head(repo: &Repo, a: &Args) -> Result<(config::Config, model::Corpus), Fatal> {
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    let snap = Snapshot::worktree(repo, &repo::root_paths(&cfg), true)?;
    let mut loaded = repo::load(repo, &cfg, &[&snap], !a.no_cache, false)?;
    Ok((cfg, loaded.corpora.remove(0)))
}

/// The corpus of the HEAD commit, or `None` if there is no commit.
fn load_base(repo: &Repo, cfg: &config::Config, a: &Args) -> Result<Option<model::Corpus>, Fatal> {
    let Some(commit) = repo.head_commit() else {
        return Ok(None);
    };
    let snap = Snapshot::at(repo, &commit)?;
    let mut loaded = repo::load(repo, cfg, &[&snap], !a.no_cache, false)?;
    Ok(Some(loaded.corpora.remove(0)))
}

fn context(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    if a.positional.is_empty() && a.paths.is_empty() {
        return Err(Fatal::Usage(
            "`context` needs a target id, path, or --path".into(),
        ));
    }
    let (cfg, c) = load_head(repo, a)?;
    let mut targets = Vec::new();
    let mut paths = a.paths.clone();
    for t in &a.positional {
        match context::resolve_target(&c, t) {
            Some(r) => targets.push(r),
            // A tracked file, or a file of the working tree that git does
            // not track yet: an agent asks for context before it adds a file.
            None if c.files.contains(t.trim_start_matches("./"))
                || (!t.starts_with('/')
                    && !t.split('/').any(|s| s == "..")
                    && repo.root.join(t).is_file()) =>
            {
                paths.push(t.trim_start_matches("./").to_string())
            }
            None => {
                return Err(Fatal::NotFound(format!(
                    "`{t}` names no document, section or tracked file"
                )));
            }
        }
    }
    let query = a.query.as_deref().map(|q| context::Query {
        root: &repo.root,
        terms: rank::terms(q),
    });
    let paths: Vec<String> = paths
        .iter()
        .map(|p| p.trim_start_matches("./").to_string())
        .collect();
    let cited = if paths.is_empty() {
        Vec::new()
    } else {
        cite::citing(&c, &cite::Citations::build(&c), &repo.root, &paths)
    };
    let asm = context::assemble(
        &c,
        &targets,
        &paths,
        &cited,
        a.budget.unwrap_or(cfg.context_budget),
        query.as_ref(),
        a.include_inactive,
    );
    print!("{}", context::render(&c, &repo.root, &asm, a.json));
    Ok(0)
}

fn show(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 1, 1)?;
    let (_cfg, c) = load_head(repo, a)?;
    let t = &a.positional[0];
    let (d, frag) = context::resolve_target(&c, t).ok_or_else(|| {
        Fatal::NotFound(format!("`{t}` names no document, section or requirement"))
    })?;
    print!(
        "{}",
        context::show(&c, &repo.root, d, frag.as_deref(), a.json)
    );
    Ok(0)
}

fn mv(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 2, usize::MAX)?;
    let (cfg, c) = load_head(repo, a)?;
    // A move of requirements records an alias only for an id of the base
    // tree (HEAD).
    let base = if a.positional.iter().any(|p| p.contains('#')) {
        load_base(repo, &cfg, a)?
    } else {
        None
    };
    let m = mv::run(repo, &cfg, &c, base.as_ref(), &a.positional)?;
    if a.json {
        let mut o = Obj::new()
            .str("kind", m.kind)
            .str("from", &m.from)
            .str("to", &m.to)
            .str("alias", &m.alias)
            .strs("files", &m.files);
        if let Some(s) = &m.split {
            let (hashes, ids, decisions) = s.preserved;
            o = o
                .str("path", &s.to_path)
                .bool("created", s.created)
                .num("requirements", s.moved.len())
                .num("sections", s.sections)
                .raw(
                    "preserved",
                    &Obj::new()
                        .num("hashes", hashes)
                        .num("ids", ids)
                        .num("decisions", decisions)
                        .end(),
                );
        }
        println!("{}", o.end());
        return Ok(0);
    }
    match &m.split {
        Some(s) if m.kind == "split" => {
            let (hashes, ids, decisions) = s.preserved;
            let fresh = s.moved.len() - s.aliases.len();
            println!(
                "moved {} requirements{} to {} ({}{}); {}",
                s.moved.len(),
                if s.sections > 0 {
                    format!(" in {} sections", s.sections)
                } else {
                    String::new()
                },
                m.to,
                s.to_path,
                if s.created { ", new document" } else { "" },
                match fresh {
                    0 => "every old id stays as an alias".to_string(),
                    _ if fresh == s.moved.len() => {
                        "no alias, because the keys are new in this change".to_string()
                    }
                    _ => format!(
                        "{} old ids stay as aliases; {fresh} keys are new in this change and get no alias",
                        s.aliases.len()
                    ),
                }
            );
            println!(
                "preservation check: {hashes} requirement hashes unchanged, {ids} ids resolve, {decisions} accepted decisions unchanged"
            );
        }
        _ if m.alias.is_empty() => println!(
            "moved {} {} -> {} (no alias: the key is new in this change)",
            m.kind, m.from, m.to
        ),
        _ => println!(
            "moved {} {} -> {} (alias kept: {})",
            m.kind, m.from, m.to, m.alias
        ),
    }
    if let Some(s) = m.split.as_ref().filter(|s| s.created) {
        println!("new file: {} (not yet added to git)", s.to_path);
    }
    Ok(0)
}

fn transcript(cwd: &std::path::Path, a: &Args) -> Result<u8, Fatal> {
    if a.positional.is_empty() {
        return Err(Fatal::Usage(
            "`transcript` needs a transcript file or directory".into(),
        ));
    }
    let files = transcript::collect_files(&a.positional).map_err(Fatal::Usage)?;
    // Without a configured repository, the report has no document ids.
    let loaded = match Repo::discover(cwd) {
        Ok(repo) => match load_head(&repo, a) {
            Ok((_, c)) => Some((repo.root, c)),
            Err(Fatal::Config(_)) => None,
            Err(e) => return Err(e),
        },
        Err(Fatal::Config(_)) => None,
        Err(e) => return Err(e),
    };
    let expect: Vec<String> = a
        .expect
        .iter()
        .map(|x| match &loaded {
            Some((_, c)) => match c.resolve(x) {
                model::Res::Found(d, _, _) => c.docs[d as usize].id.clone(),
                _ => x.clone(),
            },
            None => x.clone(),
        })
        .collect();
    let mut analysis = transcript::Analysis::new(
        loaded.as_ref().map(|l| &l.1),
        loaded.as_ref().map(|l| l.0.as_path()),
    );
    transcript::read_files(&files, a.format.as_deref(), &mut analysis).map_err(Fatal::Usage)?;
    print!("{}", analysis.report(&expect, a.json));
    Ok(0)
}

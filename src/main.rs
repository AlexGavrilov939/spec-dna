//! spec-dna keeps engineering specifications small, linked and
//! machine-checked. It serves AI coding agents the minimum sufficient
//! context. See README.md for commands, exit codes and diagnostics.

mod adapters;
mod check;
mod config;
mod context;
mod diag;
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

use crate::adapters::VERSION;
use crate::diag::Severity;
use crate::json::Obj;
use crate::repo::{Fatal, Repo, Snapshot};
use std::process::ExitCode;

const USAGE: &str = "usage: spec-dna <command> [options]

Commands:
    init [--root <dir>]
        Writes spec-dna.toml (if it does not exist) and the agent adapters.
    check [--base <rev>] [--all]
        Reports the diagnostics that are new against the merge base of <rev>
        and HEAD (default: HEAD). With --all, it reports every diagnostic.
    context <target>... [--path <file>]... [--budget <chars>] [--query <text>]
        Assembles the sections that an agent needs for the targets. With
        --query, the sections that match the words of the text come first.
    show <id>[#<anchor>]
        Prints a document, a section or a requirement.
    mv <from> <to>
        Moves a document, or moves <doc-id>#<key> to another document.

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
            "--no-cache" => a.no_cache = true,
            "--base" => a.base = Some(value("--base")?),
            "--path" => a.paths.push(value("--path")?),
            "--root" => a.root = Some(value("--root")?),
            "--query" => a.query = Some(value("--query")?),
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

# Each root is a directory of documents. The id of a document is its path
# under the root, without the extension.
[[roots]]
path = \"{root}\"
type = \"mixed\"
default-type = \"spec\"
status = \"draft\"
";

fn init(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    let mut written = Vec::new();
    let cfg_path = repo.config_path();
    if cfg_path.exists() {
        repo.load_config()?;
    } else {
        let root = a.root.as_deref().unwrap_or("docs").trim_matches('/');
        let text = DEFAULT_CONFIG
            .replace("{version}", VERSION)
            .replace("{root}", if root.is_empty() { "." } else { root });
        mv::write(&cfg_path, &text)?;
        written.push(config::FILE_NAME.to_string());
    }
    let read = |p: &str| std::fs::read_to_string(repo.root.join(p)).unwrap_or_default();
    let agents = read(adapters::AGENTS);
    let new_agents = adapters::upsert(&agents, "agents");
    if new_agents != agents {
        mv::write(&repo.root.join(adapters::AGENTS), &new_agents)?;
        written.push(adapters::AGENTS.to_string());
    }
    let claude = read(adapters::CLAUDE);
    if !adapters::imports_agents(&claude) {
        let new_claude = adapters::upsert(&claude, "claude");
        if new_claude != claude {
            mv::write(&repo.root.join(adapters::CLAUDE), &new_claude)?;
            written.push(adapters::CLAUDE.to_string());
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
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .strs("written", &written)
                .end()
        );
    } else if written.is_empty() {
        println!("spec-dna init: everything is up to date");
    } else {
        for w in &written {
            println!("wrote {w}");
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
        return Err(Fatal::Config(format!(
            "spec-dna.toml pins version {pin}, but this binary is {VERSION}; install the pinned version:\n  cargo install --locked --git https://github.com/AlexGavrilov939/spec-dna --tag v{pin} spec-dna",
            pin = cfg.pin
        )));
    }
    // The base is the merge base of --base and HEAD, or HEAD. If there is no
    // commit, the base is None.
    let mut t = std::time::Instant::now();
    let base_commit = if a.all {
        None
    } else {
        match &a.base {
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
        }
    };
    repo::trace("resolve base", &mut t);
    // List the working tree and the base tree in parallel.
    let roots = repo::root_paths(&cfg);
    let (head_snap, base_snap) = std::thread::scope(|s| {
        let base = s.spawn(|| match &base_commit {
            Some(c) => Snapshot::at(repo, c),
            None => Ok(Snapshot::empty()),
        });
        let head = Snapshot::worktree(repo, &roots);
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
            .filter(|(d, n)| d.code.severity() == sev && (!only_new || **n))
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
            .num("parsed", loaded.parsed)
            .num("cached", loaded.cached)
            .end();
        println!(
            "{}",
            Obj::new()
                .str("version", VERSION)
                .str("mode", if a.all { "all" } else { "base" })
                .opt_str("base", base_commit.as_deref())
                .bool("ok", !failed)
                .raw("summary", &summary)
                .raw("diagnostics", &json::array(diags))
                .end()
        );
    } else {
        for &i in &order {
            if a.all || outcome.new[i] {
                println!("{}", outcome.head[i].text());
            }
        }
        let base_text = match (&base_commit, a.all) {
            (_, true) => "all diagnostics".to_string(),
            (Some(c), false) => format!("new against {}", &c[..c.len().min(12)]),
            (None, false) => "no base commit, all are new".to_string(),
        };
        println!(
            "spec-dna check: {} documents; {new_errors} errors and {new_warnings} warnings {base_text} ({errors} errors and {warnings} warnings in total)",
            head.docs.len()
        );
    }
    Ok(if failed { 1 } else { 0 })
}

fn load_head(repo: &Repo, a: &Args) -> Result<(config::Config, model::Corpus), Fatal> {
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    let snap = Snapshot::worktree(repo, &repo::root_paths(&cfg))?;
    let mut loaded = repo::load(repo, &cfg, &[&snap], !a.no_cache, false)?;
    Ok((cfg, loaded.corpora.remove(0)))
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
            None if c.files.contains(t.trim_start_matches("./")) => {
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
    let asm = context::assemble(
        &c,
        &targets,
        &paths,
        a.budget.unwrap_or(cfg.context_budget),
        query.as_ref(),
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
    expect_args(a, 2, 2)?;
    let (cfg, c) = load_head(repo, a)?;
    let m = mv::run(repo, &cfg, &c, &a.positional[0], &a.positional[1])?;
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("kind", m.kind)
                .str("from", &m.from)
                .str("to", &m.to)
                .str("alias", &m.alias)
                .strs("files", &m.files)
                .end()
        );
    } else {
        println!(
            "moved {} {} -> {} (alias kept: {})",
            m.kind, m.from, m.to, m.alias
        );
    }
    Ok(0)
}

//! spec-dna keeps engineering specifications small, linked and
//! machine-checked. It gives AI coding agents the minimum sufficient context.
//! See README.md for commands, exit codes and diagnostics.

// These macros replace `print!`, `println!` and `eprintln!` in all
// modules that follow. They ignore a closed pipe (`src/out.rs`).
macro_rules! print {
    ($($arg:tt)*) => { $crate::out::stdout(format_args!($($arg)*)) };
}
macro_rules! println {
    () => { $crate::out::stdout(format_args!("\n")) };
    ($($arg:tt)*) => { $crate::out::stdout(format_args!("{}\n", format_args!($($arg)*))) };
}
macro_rules! eprintln {
    () => { $crate::out::stderr(format_args!("\n")) };
    ($($arg:tt)*) => { $crate::out::stderr(format_args!("{}\n", format_args!($($arg)*))) };
}

mod adapters;
mod check;
mod cite;
mod config;
mod context;
mod detect;
mod diag;
mod drift;
mod evidence;
mod frontmatter;
mod git;
mod glob;
mod harness;
mod hash;
mod hook;
mod index;
mod init;
mod json;
mod lint;
mod model;
mod mv;
mod out;
mod parse;
mod rank;
mod registry;
mod release;
mod repo;
mod split;
mod templates;
mod transcript;

use crate::adapters::VERSION;
use crate::diag::Severity;
use crate::json::Obj;
use crate::repo::{Fatal, Repo, Snapshot};
use std::process::ExitCode;

/// The help of each command: the name, the usage lines and the description.
/// `spec-dna <command> --help` prints one entry, so that an agent reads only
/// the help that it asks for.
const COMMANDS: &[(&str, &str, &str)] = &[
    (
        "init",
        "init [--non-interactive | --answers <file>] [--reconfigure] [--dry-run] [--root <dir>]",
        "Detects the repository, asks at most 10 questions with the detected
defaults, shows spec-dna.toml and the files to change, and writes them
after a confirmation. Without a terminal, it uses the defaults. If
spec-dna.toml exists, it updates the agent adapters, and the harness
files if the file has a [harness] table, unless --reconfigure is set.
With --reconfigure, it asks the questions again.",
    ),
    (
        "check",
        "check [--base <rev>] [--all] [--lint] [--staged]",
        "Reports the diagnostics that are new against the merge base of <rev>
and HEAD (default: HEAD). With --all, it reports every diagnostic. With
--base, it reads tracked files only, as CI does. It prints the errors
first, then the drift warnings, then the other warnings. The prose lint
warnings (SD801-SD805) are one count for each code; --lint lists each of
them. Another warning code with more than 10 diagnostics, for example
SD608 on a mature corpus, is one count too. --lint also runs the prose lint
in a profile that has it off, as a report: its warnings never fail the
gate. Without --all, --lint lists the new findings only. --staged checks the
git index (the tree that the next commit records) in place of the working
tree. Untracked and unstaged files do not count. The hook that `init`
writes uses it. --json lists all diagnostics.",
    ),
    (
        "context",
        "context <target>... [--path <file>]... [--budget <chars>] [--query <text>] [--include-inactive]",
        "Assembles the sections that an agent needs for the targets. With
--query, the sections that match the words of the text come first. A
superseded, retired or rejected document is only listed, unless
--include-inactive is set or a target names it.",
    ),
    (
        "show",
        "show <id>[#<anchor>]\nshow --conventions <type>",
        "Prints a document, a section or a requirement, with the evidence of
each requirement. With --conventions, it prints the sections that state
the project conventions for documents of the type (`conventions` in
spec-dna.toml).",
    ),
    (
        "new",
        "new <type> <id> [--root <dir>]",
        "Writes a new document from the template of its type.",
    ),
    (
        "mv",
        "mv <from> <to> [--stub]\nmv <doc-id>#<key or section>... <doc-id or path> [--type <type>]",
        "Moves a document; --stub leaves a stub at the old path for tools that
read by path. Or moves requirements and whole sections of a document of
any type to an existing or a new document of any type. Every moved key and
section of HEAD keeps its old id as an alias, and the companion rows of the
moved requirements move with them. Each moved section leaves its heading
with a one-line pointer. A decision keeps its frozen text: only its amendment
and history sections move, into a reference document. A preservation
check runs first; if it fails, nothing changes.",
    ),
    (
        "history",
        "history <doc-id> [<reference doc id or path>]",
        "Moves the history sections of a document (`history-sections`, and the
amendment sections of a decision) into a reference document, with a
pointer in place of each section and a backlink.",
    ),
    (
        "index",
        "index <dir or file> [--replace]",
        "Writes a generated index of the documents under a directory (default
file: <dir>/README.md). --replace replaces a hand-kept file. `check`
reports a stale index (SD610).",
    ),
    (
        "transcript",
        "transcript <file or dir>... [--expect <id>]... [--format claude-code]",
        "Reports token usage, tool calls and document reads per session and
episode of agent transcripts. It prints counts and ids only.",
    ),
    (
        "hook",
        "hook context-meter [--levels <a,b>] [--subagent <n>]\nhook fanout-cap [--cap <n>]",
        "Runs a hook of a coding agent. It reads one JSON object from stdin,
prints JSON only when the agent must see a message, and exits with 0.
It reads no configuration.",
    ),
];

const OPTIONS: &str = "Options:
    --json        Prints machine-readable output
    --no-cache    Does not read or write the index
    -h, --help    Prints this help; `spec-dna <command> --help` prints the help of one command
    -V, --version Prints the version

Exit codes: 0 ok, 1 check found new errors, 2 usage, 3 configuration,
4 git or I/O, 5 not found.";

fn indent(text: &str, by: &str) -> String {
    text.lines()
        .map(|l| format!("{by}{l}"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The help of all commands.
fn usage() -> String {
    let mut out = String::from("usage: spec-dna <command> [options]\n\nCommands:\n");
    for (_, synopsis, text) in COMMANDS {
        out.push_str(&format!(
            "{}\n{}\n",
            indent(synopsis, "    "),
            indent(text, "        ")
        ));
    }
    out.push('\n');
    out.push_str(OPTIONS);
    out
}

/// The help of one command, or `None` for an unknown command.
fn command_usage(name: &str) -> Option<String> {
    let (_, synopsis, text) = COMMANDS.iter().find(|c| c.0 == name)?;
    let lines: Vec<String> = synopsis
        .lines()
        .enumerate()
        .map(|(i, l)| format!("{} spec-dna {l}", if i == 0 { "usage:" } else { "      " }))
        .collect();
    Some(format!(
        "{}\n\n{text}\n\nAll commands accept --json and --no-cache.",
        lines.join("\n")
    ))
}

struct Args {
    command: String,
    help: bool,
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
    staged: bool,
    non_interactive: bool,
    answers: Option<String>,
    reconfigure: bool,
    dry_run: bool,
    ty: Option<String>,
    stub: bool,
    replace: bool,
    conventions: Option<String>,
}

fn parse_args(raw: Vec<String>) -> Result<Args, Fatal> {
    let mut a = Args {
        command: String::new(),
        help: false,
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
        staged: false,
        non_interactive: false,
        answers: None,
        reconfigure: false,
        dry_run: false,
        ty: None,
        stub: false,
        replace: false,
        conventions: None,
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
            "-h" | "--help" => a.help = true,
            "-V" | "--version" => a.command = "version".into(),
            "--json" => a.json = true,
            "--all" => a.all = true,
            "--lint" => a.lint = true,
            "--staged" => a.staged = true,
            "--no-cache" => a.no_cache = true,
            "--include-inactive" => a.include_inactive = true,
            "--non-interactive" => a.non_interactive = true,
            "--reconfigure" => a.reconfigure = true,
            "--dry-run" => a.dry_run = true,
            "--stub" => a.stub = true,
            "--replace" => a.replace = true,
            "--type" => a.ty = Some(value("--type")?),
            "--conventions" => a.conventions = Some(value("--conventions")?),
            "--answers" => a.answers = Some(value("--answers")?),
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
    let argv: Vec<String> = std::env::args().skip(1).collect();
    // A hook has its own flags and always exits with 0.
    if argv.first().map(String::as_str) == Some("hook") {
        // Without a hook name, or with a help flag, print the help.
        if matches!(
            argv.get(1).map(String::as_str),
            None | Some("--help" | "-h")
        ) {
            println!("{}", command_usage("hook").unwrap_or_default());
            return ExitCode::SUCCESS;
        }
        hook::run(&argv[1..]);
        return ExitCode::SUCCESS;
    }
    let args = match parse_args(argv) {
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
    if a.help || a.command == "help" {
        let name = if a.command == "help" {
            a.positional.first().map(String::as_str).unwrap_or("")
        } else {
            a.command.as_str()
        };
        if name.is_empty() || name == "version" || name == "help" {
            println!("{}", usage());
            return Ok(0);
        }
        return match command_usage(name) {
            Some(text) => {
                println!("{text}");
                Ok(0)
            }
            None => Err(Fatal::Usage(format!("unknown command `{name}`"))),
        };
    }
    match a.command.as_str() {
        "" => {
            println!("{}", usage());
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
        "new" => new_doc(&Repo::discover(&cwd)?, &a),
        "mv" => mv(&Repo::discover(&cwd)?, &a),
        "history" => history(&Repo::discover(&cwd)?, &a),
        "index" => index(&Repo::discover(&cwd)?, &a),
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

/// The error for a binary whose version differs from the pin (ADR 0003).
fn pin_mismatch(cfg: &config::Config) -> Fatal {
    Fatal::Config(config::pin_mismatch_message(
        &cfg.pin,
        &cfg.install_source,
        cfg.sha256.as_deref(),
    ))
}

fn init(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    if a.non_interactive && a.answers.is_some() {
        return Err(Fatal::Usage(
            "use --non-interactive or --answers, not both".into(),
        ));
    }
    init::run(
        repo,
        &init::Opts {
            root: a.root.clone(),
            non_interactive: a.non_interactive,
            answers: a.answers.clone(),
            reconfigure: a.reconfigure,
            dry_run: a.dry_run,
            json: a.json,
        },
    )
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

/// Reads the harness files of a snapshot: (path, text). The working tree reads
/// the files from disk, because the snapshot lists untracked files in the
/// document roots only. A gate against `--base` reads tracked files only. The
/// git index (`--staged`) and a commit read the blobs of the snapshot, so an
/// edit that is not staged does not count.
fn harness_texts(
    repo: &Repo,
    snap: &Snapshot,
    untracked: bool,
) -> Result<Vec<(String, String)>, Fatal> {
    let mut paths: Vec<String> = snap
        .files
        .iter()
        .filter(|e| harness::wanted(&e.path))
        .map(|e| e.path.clone())
        .collect();
    let mut out = Vec::new();
    if snap.worktree {
        if untracked {
            let st = repo
                .git
                .status(
                    &[
                        format!(":(literal){}", harness::CLAUDE),
                        ":(literal).claude".to_string(),
                    ],
                    true,
                )
                .map_err(Fatal::Io)?;
            paths.extend(st.untracked.into_iter().filter(|p| harness::wanted(p)));
            paths.sort();
            paths.dedup();
        }
        for p in paths {
            if let Ok(t) = std::fs::read_to_string(repo.root.join(&p)) {
                out.push((p, t));
            }
        }
        return Ok(out);
    }
    let ids: Vec<(String, String)> = paths
        .into_iter()
        .filter_map(|p| {
            snap.blob_of(&p)
                .and_then(|e| e.blob.clone())
                .map(|b| (p, b))
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
    Ok(out)
}

/// Reads the `[harness]` configuration of the base snapshot. Returns `None`
/// if the base has no `spec-dna.toml`, no `[harness]` table, or a
/// configuration that does not parse.
fn base_harness(repo: &Repo, snap: &Snapshot) -> Result<Option<harness::Harness>, Fatal> {
    let Some(blob) = snap.blob_of(config::FILE_NAME).and_then(|e| e.blob.clone()) else {
        return Ok(None);
    };
    let blobs = repo
        .git
        .read_blobs(std::slice::from_ref(&blob))
        .map_err(Fatal::Io)?;
    Ok(blobs
        .get(&blob)
        .and_then(|b| config::Config::parse(&String::from_utf8_lossy(b)).ok())
        .and_then(|c| c.harness))
}

fn check(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    // With `--staged`, the head tree is the git index. Each read of the head
    // tree then goes to the index, and no read lists untracked files.
    let staged_repo;
    let repo = if a.staged {
        staged_repo = repo.staged();
        &staged_repo
    } else {
        repo
    };
    let untracked = a.base.is_none() && !a.staged;
    let mut cfg = if a.staged {
        repo.load_staged_config()?
    } else {
        repo.load_config()?
    };
    // `--lint` runs the prose lint in every profile. The lint gives warnings,
    // so it never fails the gate.
    cfg.force_lint = a.lint;
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
        let head = if a.staged {
            Snapshot::index(repo)
        } else {
            Snapshot::worktree(repo, &roots, untracked)
        };
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
    let mut loaded = repo::load(repo, &cfg, &snaps, !a.no_cache, true)?;
    // The `covers:` tags of the test files, for the evidence checks.
    if !(cfg.is_off(diag::Code::CoversUnresolved) && cfg.is_off(diag::Code::RequirementUnverified))
    {
        // The working tree and the base, in parallel. The snapshots do not
        // tell whether a test file outside the document roots changed, so the
        // command reads both trees.
        let (head_covers, base_covers) = std::thread::scope(|s| {
            let base = s.spawn(|| match &base_commit {
                Some(c) => evidence::load(repo, &cfg, Some(c), false).map(Some),
                None => Ok(None),
            });
            let head = evidence::load(repo, &cfg, None, untracked);
            (
                head,
                base.join()
                    .unwrap_or_else(|_| Err(Fatal::Io("reading the base tests failed".into()))),
            )
        });
        loaded.corpora[0].covers = head_covers?;
        if let (Some(b), Some(covers)) = (loaded.corpora.get_mut(1), base_covers?) {
            b.covers = covers;
        }
        repo::trace("evidence", &mut t);
    }
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
            untracked,
        )?;
        outcome.new.extend(std::iter::repeat_n(true, drift.len()));
        outcome.head.extend(drift);
        repo::trace("drift", &mut t);
    }
    let head_ad = check::apply_severity(
        &cfg,
        adapters::check(&adapter_texts(repo, &head_snap)?, &cfg.types),
    );
    let base_ad: Vec<_> = if use_base {
        check::apply_severity(
            &cfg,
            adapters::check(&adapter_texts(repo, &base_snap)?, &cfg.types),
        )
    } else {
        Vec::new()
    };
    for d in head_ad {
        let is_new = !base_ad.iter().any(|b| b.fingerprint() == d.fingerprint());
        outcome.head.push(d);
        outcome.new.push(is_new);
    }
    if let Some(h) = cfg
        .harness
        .as_ref()
        .filter(|h| h.level != harness::Level::None)
    {
        let head_files = harness_texts(repo, &head_snap, untracked)?;
        let head_hd = check::apply_severity(&cfg, harness::check(h, &head_files));
        // A base without any harness file has nothing to compare: the harness
        // is new in this change, so each finding of the head is new.
        let base_hd: Vec<_> = if use_base {
            let base_files = harness_texts(repo, &base_snap, false)?;
            // The base findings use the configuration of the base, so a
            // change of `[harness]` makes new drift. A base with no
            // `[harness]`, or with a configuration that does not parse, has
            // no findings.
            match base_harness(repo, &base_snap)? {
                Some(bh) if harness::is_installed(&base_files) => {
                    check::apply_severity(&cfg, harness::check(&bh, &base_files))
                }
                _ => Vec::new(),
            }
        } else {
            Vec::new()
        };
        for d in head_hd {
            let is_new = !base_hd.iter().any(|b| b.fingerprint() == d.fingerprint());
            outcome.head.push(d);
            outcome.new.push(is_new);
        }
    }
    if let Some(c) = &base_commit
        && let Some(d) = config_not_alone(repo, &cfg, c, untracked)?
    {
        outcome.head.push(d);
        outcome.new.push(true);
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

/// SD503: a change that edits `spec-dna.toml` together with documents or
/// code. A configuration change must be a change of its own. The files that
/// `init` writes with it (the adapters, the harness files and files named
/// `spec-dna*`) do not
/// count, and a new `spec-dna.toml` (the adoption) is not checked.
fn config_not_alone(
    repo: &Repo,
    cfg: &config::Config,
    base: &str,
    untracked: bool,
) -> Result<Option<diag::Diag>, Fatal> {
    let changed = repo.git.changed_since(base, untracked).map_err(Fatal::Io)?;
    let Some((_, added)) = changed.iter().find(|(p, _)| p == config::FILE_NAME) else {
        return Ok(None);
    };
    if *added {
        return Ok(None);
    }
    let others: Vec<&str> = changed
        .iter()
        .map(|(p, _)| p.as_str())
        .filter(|p| {
            *p != config::FILE_NAME
                && !adapters::PATHS.contains(p)
                && !harness::is_output_path(p)
                && !p.rsplit('/').next().unwrap_or(p).starts_with("spec-dna")
        })
        .collect();
    let Some(first) = others.first() else {
        return Ok(None);
    };
    let d = diag::Diag {
        path: config::FILE_NAME.to_string(),
        line: 1,
        code: diag::Code::ConfigNotAlone,
        doc: String::new(),
        target: String::new(),
        message: format!(
            "the change edits spec-dna.toml and {} other {} (first: `{first}`); a configuration change must be a change of its own",
            others.len(),
            if others.len() == 1 { "file" } else { "files" }
        ),
        severity: diag::Code::ConfigNotAlone.severity(),
    };
    Ok(check::apply_severity(cfg, vec![d]).pop())
}

/// The lines of the human output of `check`: the errors, the drift
/// warnings (SD606, SD607), the other warnings, and then the prose lint
/// warnings. Without `lint`, the prose lint warnings are one line for each
/// code, with the count and the documents.
/// A warning code (not drift, not prose lint) with more diagnostics than this
/// prints as one count. A mature corpus can have hundreds of warnings of one
/// code, for example SD608 before the project links tests.
const MANY_WARNINGS: usize = 10;

fn human_lines(diags: &[&diag::Diag], lint: bool) -> Vec<String> {
    use diag::{Code, Severity};
    let rank = |d: &diag::Diag| match (d.severity, d.code) {
        (Severity::Error, _) => 0,
        (_, Code::CitedPathRemoved | Code::CitedSymbolRemoved) => 1,
        (_, c) if c.is_prose_lint() => 3,
        _ => 2,
    };
    let mut per_code: Vec<(Code, usize)> = Vec::new();
    for d in diags.iter().filter(|d| rank(d) == 2) {
        match per_code.iter_mut().find(|c| c.0 == d.code) {
            Some(c) => c.1 += 1,
            None => per_code.push((d.code, 1)),
        }
    }
    let many: Vec<Code> = per_code
        .into_iter()
        .filter(|c| c.1 > MANY_WARNINGS)
        .map(|c| c.0)
        .collect();
    let mut sorted: Vec<&diag::Diag> = diags.to_vec();
    sorted.sort_by_key(|d| rank(d));
    let mut out = Vec::new();
    let mut counts: Vec<(Code, usize, Vec<&str>)> = Vec::new();
    for d in sorted {
        let counted = match rank(d) {
            3 => !lint,
            2 => many.contains(&d.code),
            _ => false,
        };
        if !counted {
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
            "warning {} {}: {n} in {place} (`check {}` lists them)",
            code.code(),
            code.name(),
            if code.is_prose_lint() {
                "--lint"
            } else {
                "--json"
            }
        ));
    }
    // The fix of each listed code, once, so that an agent needs no lookup.
    let mut fixes: Vec<Code> = Vec::new();
    for d in diags {
        if (lint || !d.code.is_prose_lint()) && !fixes.contains(&d.code) {
            fixes.push(d.code);
        }
    }
    fixes.sort();
    for code in fixes {
        out.push(format!("{} fix: {}", code.code(), code.fix()));
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
            // A tracked file, or a file of the working tree that git does not
            // track yet. An agent asks for context before it adds a file.
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
        &cfg.conventions,
    );
    print!("{}", context::render(&c, &repo.root, &asm, a.json));
    Ok(0)
}

fn show(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    if let Some(name) = &a.conventions {
        return show_conventions(repo, a, name);
    }
    expect_args(a, 1, 1)?;
    let (cfg, mut c) = load_head(repo, a)?;
    c.covers = evidence::load(repo, &cfg, None, true)?;
    let t = &a.positional[0];
    if let Some((d, frag)) = context::resolve_target(&c, t) {
        print!(
            "{}",
            context::show(&c, &repo.root, d, frag.as_deref(), a.json)
        );
        return Ok(0);
    }
    // A convention file outside the documents: `context` lists its sections
    // by path.
    let (f, frag) = context::resolve_file(&c, t).ok_or_else(|| {
        Fatal::NotFound(format!("`{t}` names no document, section or requirement"))
    })?;
    print!(
        "{}",
        context::show_file(&c, &repo.root, &f, frag.as_deref(), a.json)
    );
    Ok(0)
}

/// Prints the sections that state the project conventions for documents of a
/// type (`conventions` in spec-dna.toml).
fn show_conventions(repo: &Repo, a: &Args, name: &str) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    let ty = model::DocType::from_name(name).ok_or_else(|| {
        Fatal::Usage(format!(
            "--conventions: unknown type `{name}`; use {}",
            model::DocType::names()
        ))
    })?;
    let (cfg, mut c) = load_head(repo, a)?;
    c.covers = evidence::load(repo, &cfg, None, true)?;
    let ptrs = &cfg.conventions[ty as usize];
    let mut parts = Vec::new();
    let mut missing = Vec::new();
    for p in ptrs {
        match context::resolve_pointer(&c, p) {
            Some(context::Pointer::Doc(d, frag)) => {
                parts.push(context::show(&c, &repo.root, d, frag.as_deref(), a.json))
            }
            Some(context::Pointer::File(f, frag)) => parts.push(context::show_file(
                &c,
                &repo.root,
                &f,
                frag.as_deref(),
                a.json,
            )),
            None => missing.push(p.clone()),
        }
    }
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("type", ty.name())
                .strs("pointers", ptrs)
                .strs("unresolved", &missing)
                .raw(
                    "sections",
                    &json::array(parts.iter().map(|p| p.trim_end().to_string()))
                )
                .end()
        );
        return Ok(0);
    }
    if ptrs.is_empty() {
        println!(
            "No conventions for `{}`: spec-dna.toml has no `conventions.{}`.",
            ty.name(),
            ty.name()
        );
        section_names(&cfg, ty);
        return Ok(0);
    }
    println!("<!-- conventions of {}: {} -->", ty.name(), ptrs.join(", "));
    section_names(&cfg, ty);
    for p in &missing {
        println!("<!-- `{p}` names no document, file or section (SD106) -->");
    }
    for p in parts {
        print!("{p}");
    }
    Ok(0)
}

/// Prints the amendment and live section names of a type, over all roots,
/// if the type has any.
fn section_names(cfg: &config::Config, ty: model::DocType) {
    let names = |f: fn(&config::Root) -> &[Vec<String>; model::DocType::COUNT]| {
        let mut v: Vec<String> = Vec::new();
        for r in &cfg.roots {
            for n in &f(r)[ty as usize] {
                if !v.contains(n) {
                    v.push(n.clone());
                }
            }
        }
        v
    };
    let amend = names(|r| &r.amendment_sections);
    let live = names(|r| &r.live_sections);
    if !amend.is_empty() || !live.is_empty() {
        println!(
            "<!-- {}: amendment sections: {}; live sections: {} -->",
            ty.name(),
            if amend.is_empty() {
                "none".into()
            } else {
                amend.join(", ")
            },
            if live.is_empty() {
                "none".into()
            } else {
                live.join(", ")
            }
        );
    }
}

/// Writes a new document from the template of its type. The root is
/// `--root`, or the first root of that single type, or the first mixed root.
/// In a mixed root, an id without a namespace gets the namespace of its type,
/// such as `decisions/<id>`.
fn new_doc(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    use model::DocType;
    expect_args(a, 2, 2)?;
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    let name = &a.positional[0];
    let ty = DocType::from_name(name).ok_or_else(|| {
        Fatal::Usage(format!(
            "unknown type `{name}`; use one of {}",
            DocType::names()
        ))
    })?;
    if !cfg.allows(ty) {
        return Err(Fatal::Usage(format!(
            "type `{name}` is not a type of this project (`types` in spec-dna.toml)"
        )));
    }
    let id = a.positional[1].trim_matches('/');
    let id = id.strip_suffix(".md").unwrap_or(id);
    if id.is_empty() || id.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
        return Err(Fatal::Usage(format!(
            "`{id}` is not a document id (`<namespace>/<slug>`)"
        )));
    }
    let root_i = match &a.root {
        Some(r) => {
            let r = r.trim_matches('/');
            let r = if r == "." { "" } else { r };
            cfg.roots
                .iter()
                .position(|x| x.path == r)
                .ok_or_else(|| Fatal::Usage(format!("`{r}` is not a root of spec-dna.toml")))?
        }
        None => cfg
            .roots
            .iter()
            .position(|r| r.single_type == Some(ty))
            .or_else(|| cfg.roots.iter().position(|r| r.single_type.is_none()))
            .ok_or_else(|| {
                Fatal::Usage(format!(
                    "no root takes a `{}`; add a root to spec-dna.toml or use --root",
                    ty.name()
                ))
            })?,
    };
    let root = &cfg.roots[root_i];
    let id = match templates::namespace(ty) {
        Some(ns) if root.single_type.is_none() && !id.contains('/') => format!("{ns}/{id}"),
        _ => id.to_string(),
    };
    let id = id.as_str();
    if let Some(t) = root.single_type.filter(|&t| t != ty) {
        return Err(Fatal::Usage(format!(
            "root `{}` holds only `{}` documents",
            root.path,
            t.name()
        )));
    }
    let path = if root.path.is_empty() {
        format!("{id}.md")
    } else {
        format!("{}/{id}.md", root.path)
    };
    if cfg.root_of(&path) != Some(root_i) || cfg.is_excluded(root_i, &path) {
        return Err(Fatal::Usage(format!(
            "`{path}` is in another root or excluded; use --root"
        )));
    }
    let file = repo.root.join(&path);
    if file.exists() {
        return Err(Fatal::Usage(format!("`{path}` exists")));
    }
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| Fatal::Io(e.to_string()))?;
    }
    mv::write(&file, &templates::render(ty, &templates::title_of(id)))?;
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("path", &path)
                .str("type", ty.name())
                .str("id", &model::root_doc_id(root, &path))
                .end()
        );
    } else {
        println!(
            "wrote {path} ({} template); replace each {{{{...}}}} placeholder",
            ty.name()
        );
    }
    Ok(0)
}

fn split_opts(a: &Args) -> Result<split::Opts, Fatal> {
    let ty = match &a.ty {
        None => None,
        Some(name) => Some(model::DocType::from_name(name).ok_or_else(|| {
            Fatal::Usage(format!(
                "unknown type `{name}`; use one of {}",
                model::DocType::names()
            ))
        })?),
    };
    Ok(split::Opts { ty })
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
    let m = mv::run(
        repo,
        &cfg,
        &c,
        base.as_ref(),
        &a.positional,
        &split_opts(a)?,
        a.stub,
    )?;
    report_move(&m, a.json);
    Ok(0)
}

/// Moves the history sections of a document into a reference document.
fn history(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 1, 2)?;
    let (cfg, c) = load_head(repo, a)?;
    let t = &a.positional[0];
    let (di, frag) = context::resolve_target(&c, t)
        .ok_or_else(|| Fatal::NotFound(format!("`{t}` names no document")))?;
    if frag.is_some() {
        return Err(Fatal::Usage(
            "`history` takes a document id, without `#`".into(),
        ));
    }
    let d = &c.docs[di as usize];
    let mut set: Vec<usize> = d.history.clone();
    if d.ty == model::DocType::Decision {
        set.extend(&d.amendments);
    }
    // The top sections: a history section inside another one moves with it.
    let mut tops: Vec<String> = Vec::new();
    let mut open: Option<u8> = None;
    for (i, ch) in d.parsed.chunks.iter().enumerate() {
        if open.is_some_and(|l| ch.level <= l) {
            open = None;
        }
        if open.is_none() && set.contains(&i) {
            open = Some(ch.level);
            tops.push(format!("{}#{}", d.id, ch.anchor));
        }
    }
    if tops.is_empty() {
        return Err(Fatal::NotFound(format!(
            "`{}` has no history sections; name them in `history-sections` of its root in spec-dna.toml",
            d.id
        )));
    }
    let dest = match a.positional.get(1) {
        Some(x) => x.clone(),
        None => {
            let root = &cfg.roots[d.root];
            if root
                .single_type
                .is_some_and(|t| t != model::DocType::Reference)
            {
                return Err(Fatal::Usage(format!(
                    "root `{}` holds only `{}` documents; give the path of the reference document under another root, such as `<dir>/{}-history.md`",
                    root.path,
                    d.ty.name(),
                    d.id.rsplit('/').next().unwrap_or(&d.id)
                )));
            }
            let (dir, file) = d.path.rsplit_once('/').unwrap_or(("", &d.path));
            let stem = file.rsplit_once('.').map_or(file, |x| x.0);
            let name = if stem.eq_ignore_ascii_case("readme") || stem.eq_ignore_ascii_case("index")
            {
                "history.md".to_string()
            } else {
                format!("{stem}-history.md")
            };
            if dir.is_empty() {
                name
            } else {
                format!("{dir}/{name}")
            }
        }
    };
    let base = load_base(repo, &cfg, a)?;
    let mut args = tops;
    args.push(dest);
    let opts = split::Opts {
        ty: Some(model::DocType::Reference),
    };
    let m = mv::run(repo, &cfg, &c, base.as_ref(), &args, &opts, false)?;
    report_move(&m, a.json);
    Ok(0)
}

/// Writes a generated index of the documents under a directory.
fn index(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 1, 1)?;
    let (cfg, c) = load_head(repo, a)?;
    let t = a.positional[0]
        .trim_start_matches("./")
        .trim_end_matches('/');
    let path = if model::is_doc_path(t) {
        t.to_string()
    } else if t.is_empty() || t == "." {
        "README.md".to_string()
    } else {
        ["README.md", "index.md"]
            .iter()
            .map(|n| format!("{t}/{n}"))
            .find(|p| c.by_path.contains_key(p))
            .unwrap_or_else(|| format!("{t}/README.md"))
    };
    let root = cfg
        .root_of(&path)
        .filter(|r| !cfg.is_excluded(*r, &path))
        .ok_or_else(|| {
            Fatal::Usage(format!(
                "`{path}` is not under a document root of spec-dna.toml"
            ))
        })?;
    let old = c.by_path.get(&path).map(|&i| &c.docs[i as usize]);
    if let Some(o) = old {
        if o.generated.is_none() && !a.replace {
            return Err(Fatal::Usage(format!(
                "`{path}` is a hand-kept file; pass --replace to replace it with a generated index (its text is lost)"
            )));
        }
        if !o.reqs.is_empty() {
            return Err(Fatal::Refused(format!(
                "`{path}` holds {} requirements; move them first (`spec-dna mv`)",
                o.reqs.len()
            )));
        }
    }
    let existing = old.map(|o| mv::read(repo, &o.path)).transpose()?;
    let id = model::root_doc_id(&cfg.roots[root], &path);
    let entries = registry::entries_for(&c, &path, &id);
    if entries.is_empty() {
        return Err(Fatal::NotFound(format!(
            "no documents under `{}` for the index",
            mv::parent(&path)
        )));
    }
    let title = old
        .map(|o| o.parsed.title.clone())
        .filter(|t| !t.is_empty())
        .unwrap_or_else(|| templates::title_of(mv::parent(&path)));
    let text = registry::render(&path, &entries, existing.as_deref(), &title);
    let mut changed = std::collections::HashMap::from([(path.clone(), text)]);
    // Links to a section of the old file that names an entry go to the entry.
    let mut rewritten = 0;
    if old.is_some() {
        for o in c.docs.iter().filter(|o| o.path != path) {
            let hits = o.parsed.refs.iter().any(|r| {
                matches!(c.resolve_link(&o.path, &r.target), model::LinkRes::File(ref p, _, Some(_)) if *p == path)
            });
            if !hits {
                continue;
            }
            let t = mv::read(repo, &o.path)?;
            let nt = split::map_links(&t, &|dest| {
                registry::entry_link(&c, &o.path, dest, &path, &entries)
            });
            if nt != t {
                rewritten += 1;
                changed.insert(o.path.clone(), nt);
            }
        }
    }
    let created = if old.is_none() {
        vec![path.clone()]
    } else {
        Vec::new()
    };
    let lost = split::check_no_new_errors(&cfg, &c, &changed, &created).map_err(|e| {
        Fatal::Refused(format!(
            "the index would break references, so nothing changed: {e}"
        ))
    })?;
    for (p, t) in &changed {
        let f = repo.root.join(p);
        if let Some(dir) = f.parent() {
            std::fs::create_dir_all(dir).map_err(|e| Fatal::Io(e.to_string()))?;
        }
        mv::write(&f, t)?;
    }
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("path", &path)
                .num("entries", entries.len())
                .bool("replaced", old.is_some_and(|o| o.generated.is_none()))
                .num("links_rewritten", rewritten)
                .num("anchor_warnings", lost)
                .end()
        );
    } else {
        println!(
            "wrote {path}: a generated index of {} entries{}",
            entries.len(),
            if rewritten > 0 {
                format!(
                    "; links to its old sections now go to the entries in {rewritten} documents"
                )
            } else {
                String::new()
            }
        );
        if lost > 0 {
            println!("{lost} links name a section that the index no longer has (SD602 warnings)");
        }
    }
    Ok(0)
}

fn report_move(m: &mv::Moved, json: bool) {
    if json {
        let mut o = Obj::new()
            .str("kind", m.kind)
            .str("from", &m.from)
            .str("to", &m.to)
            .str("alias", &m.alias)
            .strs("files", &m.files);
        if let Some(st) = &m.stub {
            o = o.str("stub", st);
        }
        if let Some(s) = &m.split {
            let (hashes, ids, decisions) = s.preserved;
            o = o
                .str("path", &s.to_path)
                .str("type", s.to_type.name())
                .bool("created", s.created)
                .num("requirements", s.moved.len())
                .num("sections", s.sections)
                .strs("section_aliases", &s.section_aliases)
                .num("companion_rows", s.companion_rows)
                .num("pointers", s.pointers)
                .strs("supersede_candidates", &s.supersede_candidates)
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
        return;
    }
    match &m.split {
        Some(s) if m.kind == "split" && s.moved.is_empty() => {
            let (hashes, ids, decisions) = s.preserved;
            println!(
                "moved {} section{} to {} ({}, {}{})",
                s.sections,
                if s.sections == 1 { "" } else { "s" },
                m.to,
                s.to_path,
                s.to_type.name(),
                if s.created { ", new document" } else { "" }
            );
            println!(
                "preservation check: {hashes} requirement hashes unchanged, {ids} ids resolve, {decisions} accepted decisions unchanged"
            );
        }
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
    if let Some(s) = &m.split {
        if !s.section_aliases.is_empty() {
            println!(
                "{} section ids stay as aliases in {}",
                s.section_aliases.len(),
                s.to_path
            );
        }
        if s.companion_rows > 0 {
            println!(
                "moved {} companion rows with the requirements",
                s.companion_rows
            );
        }
        if s.pointers > 0 {
            println!(
                "left a one-line pointer in place of each of the {} moved sections",
                s.pointers
            );
        }
        if !s.supersede_candidates.is_empty() {
            println!(
                "candidates for a superseding decision (the moved amendments state a requirement, a BCP 14 keyword or a status word; spec-dna writes no decision): {}",
                s.supersede_candidates.join(", ")
            );
        }
        if s.created {
            println!("new file: {} (not yet added to git)", s.to_path);
        }
    }
    if let Some(st) = &m.stub {
        println!(
            "left a stub at {st} (`moved-to: {}`) for tools that read the old path; it has no id and no budget",
            m.to
        );
    }
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

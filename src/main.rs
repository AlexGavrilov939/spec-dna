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
mod change;
mod check;
mod cite;
mod config;
mod config_text;
mod context;
mod detect;
mod diag;
mod drift;
mod eval_run;
mod evals;
mod evidence;
mod finish;
mod frontmatter;
mod git;
mod glob;
mod harness;
mod harness_text;
mod hash;
mod hook;
mod index;
mod init;
mod json;
mod layer;
mod lifecycle;
mod lint;
mod model;
mod mv;
mod out;
mod parse;
mod rank;
mod registry;
mod release;
mod repo;
mod sdlc;
mod skeleton;
mod split;
mod stage;
mod templates;
mod transcript;
mod upgrade;
mod upgrade_config;
mod upgrade_docs;
mod upgrade_old;
mod upgrade_proof;

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
        "Detects the repository, asks at most 14 questions with the detected
defaults, shows spec-dna.toml and the files to change, and writes them
after a confirmation. Without a terminal, it uses the defaults. If
spec-dna.toml exists, it updates the agent adapters, and the harness
files if the file has a [harness] table, unless --reconfigure is set.
With --reconfigure, it asks the questions again.",
    ),
    (
        "upgrade",
        "upgrade [--dry-run] [--layout [--areas <file>]] [--pin-file <file>]",
        "Moves a project that pins 0.4.0 or later to the version of this binary. It
reads the committed tree (HEAD), builds the whole plan in memory, proves that
each old id, alias, requirement and section still resolves to the same text,
and then writes. It changes the pin and the 0.5 names in spec-dna.toml, the
type and the status in the front matter, and the adapter and harness files. It
moves each proposed, accepted or rejected change to intent/<slug>/intent.md
with git mv, and keeps its old id as an alias. With --layout, it also moves
the folders of the other 0.5 namespaces of a mixed root, and it moves a note,
a reference or a change named roadmap, overview, questions, glossary or
architecture (and their usual synonyms) to the top file of its root. With
--layout --areas <file>, each document that the TOML table [areas] of the file
maps (id = \"area\") moves to requirements/<area>/ with its old id as an alias.
The top files and --areas need a project of 0.5; a project of 0.6 gets exit
code 2 for --areas. The file of --areas stays outside the repository. --pin-file gives the
spec-dna-pin.toml of the release, which a project with [spec-dna-sha256]
needs. It refuses a tree that is not clean (also with --dry-run), and it stops
with no write if the proof fails. --dry-run prints the diff of each file and
writes nothing. It never commits and never deletes a document: the report
lists the proposals to delete, and the last line is the git add command. A
second run after the commit changes nothing. Exit codes: 0 done or nothing to
change, 1 a tree that is not clean, a stop of a move or a failed proof, 2 usage,
3 configuration, pin or pin file, 4 git or write error.",
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
writes uses it. With a [change] policy, the change checks (SD905, SD910,
SD913, SD917, SD921) compare with the merge base of --base (default:
origin/HEAD, then [change] base) and HEAD, and they print a note if there
is none. --json
lists all diagnostics.",
    ),
    (
        "context",
        "context <target>... [--path <file>]... [--budget <chars>] [--query <text>] [--group <name>] [--include-inactive]",
        "Assembles the sections that an agent needs for the targets. With
--query, the sections that match the words of the text come first. With
--group, it serves no document of an area outside the group, and it counts
them in one line; a target that names such a document serves it. A
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
        "new <type> [<id>] [--root <dir>] [--template <name>]",
        "Writes a new document from a template, at the place of its layer. Without
--template, it uses the default template of the type. --template names a file
<name>.md in the folder [templates] dir, or an embedded template of the type.
Examples are prd or rfc for intent, and runbook for procedure. A top file such
as terms takes no id. intent <slug> writes intent/<slug>/intent.md, and spec and
plan need that folder. Where the place gives the type, the file has no type:
line.",
    ),
    (
        "layer",
        "layer add <name> [--types <t,..>] [--paths <glob,..>] [--files <f,..>] [--dir <d>] [--moves-to <l,..>] [--moved-from <l,..>] [--linked-from <l,..>] [--keep <k>] [--context <c>] [--max-docs <n>] [--max-chars <n>] [--max-started <n>] [--stale-days <n>]\nlayer list",
        "`add` writes the table [layers.<name>] at the end of spec-dna.toml. --keep is
forever or until-finished, and --context is always or on-request. With
--moved-from, the new layer joins the moves-to list of each named layer, and
the file keeps every comment. It stops with exit code 2 and does not change
the file for a cycle, a name that exists, a type that another layer has, a
layer that the graph does not have, a types list on many lines that --types
must grow, and a bad value. If the file has no
[layers] table, it writes the default graph first. If the file has a types
list, --types also appends each type that the list lacks, and the output says
so. It ends with up to 3 lines that start with next: and give one action each:
commit spec-dna.toml as a change of its own, run init when the agent skill
lists other types, and write the first document with new. With --json, the
fields types_added and next hold the same.
`list` prints each layer: its level, its selection, its edges, its limits and
the number of documents that each rule put in it. With --json, it also lists
the rule of each document.",
    ),
    (
        "mv",
        "mv <from> <to> [--stub]\nmv <doc-id>#<key or section>... <doc-id or path> [--type <type>]",
        "Moves a document; --stub leaves a stub at the old path for tools that
read by path. Or moves requirements and whole sections of a document of
any type to an existing or a new document of any type. Every moved key and
section of HEAD keeps its old id as an alias, and the companion rows of the
moved requirements move with them. Each moved section leaves its heading
with a one-line pointer. A decision keeps its locked text: only its amendment
and history sections move, into a research document. A preservation
check runs first; if it fails, nothing changes.",
    ),
    (
        "history",
        "history <doc-id> [<research doc id or path>]",
        "Moves the history sections of a document (`history-sections`, and the
amendment sections of a decision) into a research document, with a
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
        "finish",
        "finish <slug> [--dry-run] [--base <rev>]",
        "Finishes the change <slug>: it applies the delta of its spec to the
requirements, checks the evidence of each requirement that it adds or
changes, and deletes the folder of the change. It stops at the first failed
step, and then it changes nothing. A stop is a reference to the folder from
another document, a status that is not accepted (intent) or approved (plan,
spec), an open concern or question, a decision that is not accepted, a delta
line that does not match the tree (SD914), a delta that differs from the
approved delta of the base tree, an interface that the diff does not change,
a requirement with no evidence (SD916), or a new error. The base is the merge
base of --base (default: origin/HEAD, then [change] base) and HEAD. An intent
with the status closed runs the first two steps and deletes the folder. The
folder must be tracked and clean. --dry-run prints the diff of each file and
`remove <path>` for each deleted file, and writes nothing. Without it, the
command writes the files, deletes the folder, and stages the paths with
`git add`. It makes no commit. It is the only command that deletes documents. Exit codes: 1 a stop,
3 pin, base or configuration, 4 write error, 5 no folder.",
    ),
    (
        "next",
        "next <slug> [--base <rev>]",
        "Prints the checklist to the next gate of the change <slug>. The stage
comes from the files of the folder, their statuses, the diff against the
base and the evidence of the requirements: plan, design, build, test,
deploy or closed. Each line says done ([x]) or open ([ ]). The last line
gives the edit or the command that ends the stage. In the build stage, it
names the next open task of the plan. A routine change whose plan has one
Files item that starts with none needs no changed file: the build stage ends
when each task is done. Each line has the name of the skill for its step: brainstorm, specify, clarify, write-plan, execute, tdd, review,
verify or finish. The skills come in 0.7.0, and the names are fixed. The
output has at most 12 lines. The base is the merge base of --base (default:
origin/HEAD, then [change] base) and HEAD. Without a base, the stage stops at
build, and the output has a note. With the policy none, it says that the change process is
off. Exit codes: 2 usage, 3 base or configuration, 4 git, 5 no folder.",
    ),
    (
        "status",
        "status [--group <name>] [--base <rev>]",
        "Lists each change of the working tree with its class, its stage, the
statuses of its intent, spec and plan, its task progress, its areas, and its
next gate. It marks a started change that is stale (SD911). It prints the
number of started changes and the limit max-started, and the open follow-ups
(`- [ ]` in Follow-ups) of the incidents. With --group, it lists only the
changes that have an area of the group, in the areas of the intent or in the
folder of a delta target; an unknown group is a usage error. The base is the
merge base of --base (default: origin/HEAD, then [change] base) and HEAD,
and a note says when there is none. With the policy none, it says that the change process is off
and lists nothing. Exit codes: 2 usage or unknown group, 3 base or configuration,
4 git.",
    ),
    (
        "log",
        "log [--since <rev>]",
        "Lists the finished and the closed changes, the newest first, from the
history of git. It reads each commit that deleted an intent.md. The status of
the intent in the parent of that commit gives the result: accepted is
finished, and any other status is closed. For each change it prints the date,
the result, the slug, the class, the counts of the delta lines (add, change,
remove) of its spec, the commit and the title. A change that one pull request
held with its approval and its code, and that a squash merge brought to the
default branch, has no entry. A move of intent.md out of the intent folder
that keeps its id as an alias (spec-dna mv) does not end a change. A plain
git mv, which writes no alias, still reads as a deletion. --since limits the walk to the commits after <rev>. In a
shallow clone, a note says that older changes can be missing, and --json has
shallow: true. With the policy none, it says that the change process is off
and lists nothing. Exit codes: 2 usage, 3 configuration or a revision that is
not a commit, 4 git.",
    ),
    (
        "stats",
        "stats --sdlc [--group <name>]",
        "Prints the leading and the lagging indicators of the playbook under the
playbook names. It reads git and the committed files only: the plan and
design times, the survival rate of the intents, the spec commits after the
first plan commit, the time from plan approval to the merged pull request,
and how often the merged diff stays inside the plan (SD913). A move of
intent.md out of the intent folder that keeps its id as an alias (spec-dna
mv) does not end a life: stats drops that change. An indicator
that git cannot give prints n/a with the reason, and --json has value null
and a reason. With --group, it counts only the changes that have an area of
the group. An unknown group is a usage error, and so is stats without --sdlc.
With the policy none, it says that the change process is off and gives no
indicator. Exit codes: 2 usage, unknown group or no --sdlc, 3 configuration,
4 git.",
    ),
    (
        "eval",
        "eval run [<id>...] [--runs <n>] [--min-pass-rate <r>] [--max-total-usd <n>] [--out <dir>] [--dry-run] [--isolate]",
        "Runs the evals of [evals] dir (<id>.json) with headless Claude Code. For
each run, it copies the fixture and the harness files into a temporary git
repository, starts `claude -p` in it, and runs the checks of the eval. It
sources <dir>/check.sh of the project, never the copy. An eval passes when at
least pass-at of its runs pass. The suite passes when the share of evals that
pass is at least min-pass-rate. It refuses to start when runs x
max-budget-usd is above max-total-usd, and it stops when the spend is above
it. --runs, --min-pass-rate and --max-total-usd replace the values of the
files and of the table. --out writes the result to <dir>/eval-<time>.json
(counts, costs and check names only). --dry-run starts no process. --isolate
adds --setting-sources project --strict-mcp-config. spec-dna makes no network call, but `claude`
does. Exit codes: 0 pass, 1 the pass rate is lower, 2 usage, 3 configuration,
a stop by the cap or no `claude`, 4 git or file error.",
    ),
    (
        "transcript",
        "transcript <file or dir>... [--expect <id>]... [--format claude-code]",
        "Reports token usage, tool calls and document reads per session and
episode of agent transcripts. It prints counts and ids only.",
    ),
    (
        "hook",
        "hook context-meter [--levels <a,b>] [--subagent <n>]\nhook fanout-cap [--cap <n>]\nhook protect-tests [--mode <fix-branches|always>] [--test <glob>]...",
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

Exit codes: 0 ok, 1 check found new errors or an eval run failed, 2 usage,
3 configuration, 4 git or I/O, 5 not found.";

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
    layout: bool,
    pin_file: Option<String>,
    areas: Option<String>,
    ty: Option<String>,
    stub: bool,
    replace: bool,
    conventions: Option<String>,
    template: Option<String>,
    layer: LayerFlags,
    group: Option<String>,
    since: Option<String>,
    sdlc: bool,
    runs: Option<u32>,
    min_pass_rate: Option<f64>,
    max_total_usd: Option<f64>,
    out: Option<String>,
    isolate: bool,
}

/// The options of `layer add`.
#[derive(Default)]
struct LayerFlags {
    /// True if the command line has at least one of these options.
    given: bool,
    types: Vec<String>,
    paths: Vec<String>,
    files: Vec<String>,
    moves_to: Vec<String>,
    moved_from: Vec<String>,
    linked_from: Vec<String>,
    keep: Option<String>,
    context: Option<String>,
    dir: Option<String>,
    max_docs: Option<u32>,
    max_chars: Option<u32>,
    max_started: Option<u32>,
    stale_days: Option<u32>,
}

/// Splits the value of a list option at the commas. An empty entry is an
/// error.
fn list_value(name: &str, v: &str) -> Result<Vec<String>, Fatal> {
    let items: Vec<String> = v.split(',').map(|s| s.trim().to_string()).collect();
    if items.iter().any(String::is_empty) {
        return Err(Fatal::Usage(format!(
            "{name}: `{v}` has an empty entry; write a comma-separated list"
        )));
    }
    Ok(items)
}

fn number_value(name: &str, v: &str) -> Result<u32, Fatal> {
    v.parse()
        .map_err(|_| Fatal::Usage(format!("{name}: `{v}` is not a number")))
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
        layout: false,
        pin_file: None,
        areas: None,
        ty: None,
        stub: false,
        replace: false,
        conventions: None,
        template: None,
        layer: LayerFlags::default(),
        group: None,
        since: None,
        sdlc: false,
        runs: None,
        min_pass_rate: None,
        max_total_usd: None,
        out: None,
        isolate: false,
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
            "--layout" => a.layout = true,
            "--pin-file" => a.pin_file = Some(value("--pin-file")?),
            "--areas" => a.areas = Some(value("--areas")?),
            "--stub" => a.stub = true,
            "--replace" => a.replace = true,
            "--type" => a.ty = Some(value("--type")?),
            "--conventions" => a.conventions = Some(value("--conventions")?),
            "--template" => a.template = Some(value("--template")?),
            "--answers" => a.answers = Some(value("--answers")?),
            "--base" => a.base = Some(value("--base")?),
            "--group" => a.group = Some(value("--group")?),
            "--since" => a.since = Some(value("--since")?),
            "--sdlc" => a.sdlc = true,
            "--isolate" => a.isolate = true,
            "--out" => a.out = Some(value("--out")?),
            "--runs" => {
                let v = value("--runs")?;
                a.runs = Some(
                    v.parse::<u32>()
                        .ok()
                        .filter(|n| (1..=100).contains(n))
                        .ok_or_else(|| {
                            Fatal::Usage(format!("--runs: `{v}` is not an integer from 1 to 100"))
                        })?,
                );
            }
            "--min-pass-rate" => {
                let v = value("--min-pass-rate")?;
                a.min_pass_rate = Some(
                    v.parse::<f64>()
                        .ok()
                        .filter(|n| (0.0..=1.0).contains(n))
                        .ok_or_else(|| {
                            Fatal::Usage(format!(
                                "--min-pass-rate: `{v}` is not a number from 0 to 1"
                            ))
                        })?,
                );
            }
            "--max-total-usd" => {
                let v = value("--max-total-usd")?;
                a.max_total_usd = Some(
                    v.parse::<f64>()
                        .ok()
                        .filter(|n| n.is_finite() && *n > 0.0)
                        .ok_or_else(|| {
                            Fatal::Usage(format!("--max-total-usd: `{v}` is not a number above 0"))
                        })?,
                );
            }
            "--path" => a.paths.push(value("--path")?),
            "--root" => a.root = Some(value("--root")?),
            "--query" => a.query = Some(value("--query")?),
            "--expect" => a.expect.push(value("--expect")?),
            "--format" => a.format = Some(value("--format")?),
            "--types" | "--paths" | "--files" | "--moves-to" | "--moved-from" | "--linked-from" => {
                let v = list_value(&flag, &value(&flag)?)?;
                let l = &mut a.layer;
                l.given = true;
                match flag.as_str() {
                    "--types" => l.types.extend(v),
                    "--paths" => l.paths.extend(v),
                    "--files" => l.files.extend(v),
                    "--moves-to" => l.moves_to.extend(v),
                    "--moved-from" => l.moved_from.extend(v),
                    _ => l.linked_from.extend(v),
                }
            }
            "--keep" | "--context" | "--dir" => {
                let v = value(&flag)?;
                let l = &mut a.layer;
                l.given = true;
                match flag.as_str() {
                    "--keep" => l.keep = Some(v),
                    "--context" => l.context = Some(v),
                    _ => l.dir = Some(v),
                }
            }
            "--max-docs" | "--max-chars" | "--max-started" | "--stale-days" => {
                let n = Some(number_value(&flag, &value(&flag)?)?);
                let l = &mut a.layer;
                l.given = true;
                match flag.as_str() {
                    "--max-docs" => l.max_docs = n,
                    "--max-chars" => l.max_chars = n,
                    "--max-started" => l.max_started = n,
                    _ => l.stale_days = n,
                }
            }
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
    if a.since.is_some() && a.command != "log" {
        return Err(Fatal::Usage("--since is an option of `log`".into()));
    }
    if a.command != "upgrade" {
        for (given, name) in [
            (a.layout, "--layout"),
            (a.pin_file.is_some(), "--pin-file"),
            (a.areas.is_some(), "--areas"),
        ] {
            if given {
                return Err(Fatal::Usage(format!("{name} is an option of `upgrade`")));
            }
        }
    }
    if a.sdlc && !matches!(a.command.as_str(), "stats" | "log") {
        return Err(Fatal::Usage("--sdlc is an option of `stats`".into()));
    }
    if a.command != "eval" {
        for (given, name) in [
            (a.runs.is_some(), "--runs"),
            (a.min_pass_rate.is_some(), "--min-pass-rate"),
            (a.max_total_usd.is_some(), "--max-total-usd"),
            (a.out.is_some(), "--out"),
            (a.isolate, "--isolate"),
        ] {
            if given {
                return Err(Fatal::Usage(format!("{name} is an option of `eval run`")));
            }
        }
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
        "upgrade" => upgrade(&Repo::discover(&cwd)?, &a),
        "check" => check(&Repo::discover(&cwd)?, &a),
        "context" => context(&Repo::discover(&cwd)?, &a),
        "show" => show(&Repo::discover(&cwd)?, &a),
        "new" => new_doc(&Repo::discover(&cwd)?, &a),
        "layer" => layer_cmd(&Repo::discover(&cwd)?, &a),
        "mv" => mv(&Repo::discover(&cwd)?, &a),
        "history" => history(&Repo::discover(&cwd)?, &a),
        "index" => index(&Repo::discover(&cwd)?, &a),
        "finish" => finish(&Repo::discover(&cwd)?, &a),
        "next" => next(&Repo::discover(&cwd)?, &a),
        "status" => status(&Repo::discover(&cwd)?, &a),
        "log" => log(&Repo::discover(&cwd)?, &a),
        "stats" => stats(&Repo::discover(&cwd)?, &a),
        "eval" => eval(&cwd, &a),
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

fn upgrade(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    if a.areas.is_some() && !a.layout {
        return Err(Fatal::Usage("--areas needs --layout".into()));
    }
    upgrade::run(
        repo,
        &upgrade::RunOpts {
            dry_run: a.dry_run,
            json: a.json,
            layout: a.layout,
            pin_file: a.pin_file.clone(),
            areas: a.areas.clone(),
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
        let paths: Vec<String> = adapters::PATHS.iter().map(|p| p.to_string()).collect();
        let got = snap.read(repo, &paths)?;
        for p in adapters::PATHS {
            if let Some(bytes) = got.get(p) {
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
                        format!(":(literal){}", harness::REVIEW),
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
    let got = snap.read(repo, &paths)?;
    for p in paths {
        if let Some(bytes) = got.get(&p) {
            out.push((p, String::from_utf8_lossy(bytes).into_owned()));
        }
    }
    Ok(out)
}

/// Reads the configuration of the base snapshot. Returns `None` if the base
/// has no `spec-dna.toml`, or a configuration that does not parse.
fn base_config(repo: &Repo, snap: &Snapshot) -> Result<Option<config::Config>, Fatal> {
    let got = snap.read(repo, &[config::FILE_NAME.to_string()])?;
    Ok(got
        .get(config::FILE_NAME)
        .and_then(|b| config::Config::parse(&String::from_utf8_lossy(b)).ok()))
}

/// Reads the `[harness]` configuration of the base snapshot. Returns `None`
/// if the base has no `spec-dna.toml`, no `[harness]` table, or a
/// configuration that does not parse.
fn base_harness(repo: &Repo, snap: &Snapshot) -> Result<Option<harness::Harness>, Fatal> {
    Ok(base_config(repo, snap)?.and_then(|c| c.harness))
}

/// The files of a tree under `[evals] dir`, for SD506. In the working tree,
/// the paths are the tracked files and, with `untracked`, the untracked files
/// that git does not ignore (the folder is outside the document roots, so the
/// snapshot does not list its untracked files). A path that is absent on disk
/// is no file. The bytes of each `*.json` file directly in the folder are read
/// from the disk, or from the tree.
fn eval_tree(
    repo: &Repo,
    snap: &Snapshot,
    dir: &str,
    untracked: bool,
) -> Result<check::EvalTree, Fatal> {
    use std::io::Read;
    let prefix = format!("{dir}/");
    let mut paths: Vec<String> = snap
        .files
        .iter()
        .filter(|e| e.path.starts_with(&prefix))
        .map(|e| e.path.clone())
        .collect();
    if snap.worktree {
        if untracked {
            let st = repo
                .git
                .status(&[format!(":(literal){dir}")], true)
                .map_err(Fatal::Io)?;
            paths.extend(st.untracked.into_iter().filter(|p| p.starts_with(&prefix)));
        }
        paths.retain(|p| std::fs::symlink_metadata(repo.root.join(p)).is_ok());
    }
    paths.sort();
    paths.dedup();
    let direct =
        |p: &&String| p[prefix.len()..].ends_with(".json") && !p[prefix.len()..].contains('/');
    let mut json = Vec::new();
    if snap.worktree {
        for p in paths.iter().filter(direct) {
            let mut bytes = Vec::new();
            let read = std::fs::File::open(repo.root.join(p))
                .and_then(|f| f.take(evals::MAX_BYTES as u64 + 1).read_to_end(&mut bytes));
            if read.is_ok() {
                json.push((p.clone(), bytes));
            }
        }
    } else {
        let want: Vec<String> = paths.iter().filter(direct).cloned().collect();
        let mut got = snap.read(repo, &want)?;
        for p in want {
            if let Some(bytes) = got.remove(&p) {
                json.push((p, bytes));
            }
        }
    }
    Ok(check::EvalTree { paths, json })
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
    // The pin of the base of the gate (D154). A base before 0.6.0 has 0.5
    // documents, which 0.6 rejects. Then `check` plans the base with the code
    // of `upgrade`, in memory, and compares the head with the result.
    let base_label = a.base.as_deref().unwrap_or("HEAD");
    let gate_pin = match &base_commit {
        Some(c) => base_pin(repo, c)?,
        None => None,
    };
    let plan_base = match &gate_pin {
        Some(pin) => match config::pin_class(pin, VERSION) {
            config::PinClass::BeforeOldest => {
                return Err(Fatal::Config(format!(
                    "the base `{base_label}` pins `{pin}`; move it to {} first, with the {} binary and `spec-dna init`, in a pull request of its own",
                    config::PIN_FIRST_STEP,
                    config::PIN_FIRST_STEP
                )));
            }
            class => class == config::PinClass::Model05,
        },
        None => false,
    };
    // The lifecycle base for the change checks. With the policy `none` it
    // reads nothing. A git diff cannot read a planned tree, so a lifecycle base
    // before 0.6.0 counts as no lifecycle base (D154).
    let mut change_base =
        lifecycle::change_base(&cfg, &repo.git, a.base.as_deref()).map_err(Fatal::Config)?;
    let mut life_note: Option<String> = None;
    if let Some(c) = &change_base {
        let pin = if Some(c) == base_commit.as_ref() {
            gate_pin.clone()
        } else {
            base_pin(repo, c)?
        };
        if let Some(pin) = pin
            && matches!(
                config::pin_class(&pin, VERSION),
                config::PinClass::BeforeOldest | config::PinClass::Model05
            )
        {
            let which = if Some(c) == base_commit.as_ref() {
                format!("the base `{base_label}`")
            } else {
                format!("the merge base `{}`", &c[..c.len().min(12)])
            };
            life_note = Some(format!(
                "{which} pins `{pin}`, before 0.6.0, so the change checks have no lifecycle base"
            ));
            change_base = None;
        }
    }
    repo::trace("resolve base", &mut t);
    // List the working tree and the base tree in parallel.
    let roots = repo::root_paths(&cfg);
    // The lifecycle tree is a third tree only if the lifecycle base is not the
    // base of the gate.
    let extra_commit = change_base
        .as_ref()
        .filter(|c| Some(*c) != base_commit.as_ref());
    let (head_snap, base_snap, extra_snap) = std::thread::scope(|s| {
        let base = s.spawn(|| match &base_commit {
            Some(c) => Snapshot::at(repo, c),
            None => Ok(Snapshot::empty()),
        });
        let extra = s.spawn(|| extra_commit.map(|c| Snapshot::at(repo, c)).transpose());
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
            extra
                .join()
                .unwrap_or_else(|_| Err(Fatal::Io("listing the lifecycle tree failed".into()))),
        )
    });
    let (head_snap, base_snap, extra_snap) = (head_snap?, base_snap?, extra_snap?);
    let base_snap = match (&base_commit, plan_base) {
        (Some(c), true) => {
            let plan = upgrade::plan_base(&repo.git, c, base_label, &cfg)?;
            base_snap.overlaid(&plan.overlay())
        }
        _ => base_snap,
    };
    let use_base = base_commit.is_some();
    let mut snaps: Vec<&Snapshot> = if use_base {
        vec![&head_snap, &base_snap]
    } else {
        vec![&head_snap]
    };
    if let Some(x) = &extra_snap {
        snaps.push(x);
    }
    repo::trace("git base tree", &mut t);
    let mut loaded = repo::load(repo, &cfg, &snaps, !a.no_cache, true)?;
    // The `covers:` tags of the test files, for the evidence checks.
    if !(cfg.is_off(diag::Code::CoversUnresolved)
        && cfg.is_off(diag::Code::RequirementUnverified)
        && cfg.is_off(diag::Code::DeltaEvidenceMissing))
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
    // The lifecycle tree: the base of the gate, or the third tree.
    let (life, life_snap): (Option<&model::Corpus>, &Snapshot) =
        match (&change_base, extra_snap.as_ref()) {
            (Some(_), Some(x)) => (loaded.corpora.get(2), x),
            (Some(_), None) => (base, &base_snap),
            _ => (None, &base_snap),
        };
    // The diff against the lifecycle base, read once.
    let diff = match &change_base {
        Some(b) => Some(lifecycle::Diff::read(&repo.git, b, untracked).map_err(Fatal::Io)?),
        None => None,
    };
    let mut outcome = check::gate(&cfg, base, life, head);
    // SD914 is an error only for a spec that the diff touches (design, K3).
    // With no lifecycle base, no spec is touched.
    if outcome
        .head
        .iter()
        .any(|d| d.code == diag::Code::DeltaInvalid && d.severity == Severity::Error)
    {
        let touched: Vec<&str> = diff
            .iter()
            .flat_map(|d| d.files.iter().map(|(p, _)| p.as_str()))
            .collect();
        for d in &mut outcome.head {
            if d.code == diag::Code::DeltaInvalid && !touched.contains(&d.path.as_str()) {
                d.severity = Severity::Warning;
            }
        }
    }
    // SD912 is new when the head has more started changes than the lifecycle
    // base has (K5).
    if let Some(life) = life {
        let (now, before) = (
            lifecycle::started_count(head),
            lifecycle::started_count(life),
        );
        for (d, new) in outcome.head.iter().zip(outcome.new.iter_mut()) {
            if d.code == diag::Code::StartedLimit {
                *new = now > before;
            }
        }
    }
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
    // The checks of the diff against the lifecycle base: SD905 (a change with
    // no delta), SD910, SD913, SD915, SD916, SD917, SD918, SD920 (a decision
    // that is not `accepted`) and SD921. With `--base` and no merge base,
    // SD921 compares with the base of the gate.
    let mut report = lifecycle::BaseReport::default();
    if let (Some(life), Some(diff), Some(rev)) = (life, &diff, &change_base) {
        report =
            lifecycle::base_checks(&cfg, &repo.git, head, life, rev, diff).map_err(Fatal::Io)?;
        outcome
            .new
            .extend(std::iter::repeat_n(true, report.diags.len()));
        outcome.head.append(&mut report.diags);
    }
    // SD911 is a state code: it reports the tree, so it is never new. It needs
    // a real merge base, and it is silent with the tip fallback.
    if let Some(rev) = &change_base {
        let stale = lifecycle::stale_changes(&cfg, &repo.git, head, rev).map_err(Fatal::Io)?;
        let diags = lifecycle::stale_diags(&cfg, head, &stale);
        outcome.new.extend(std::iter::repeat_n(true, diags.len()));
        outcome.head.extend(diags);
    }
    let truth_base = match (life, base) {
        (Some(l), _) => Some((l, life_snap)),
        (None, Some(b)) if a.base.is_some() => Some((b, &base_snap)),
        _ => None,
    };
    if let Some(tb) = truth_base
        && !report.born
    {
        let truth = check::truth_changed(&cfg, tb, (head, &head_snap), &report.exempt);
        outcome.new.extend(std::iter::repeat_n(true, truth.len()));
        outcome.head.extend(truth);
    }
    repo::trace("lifecycle", &mut t);
    let head_ad = check::apply_severity(
        &cfg,
        adapters::check(
            &adapter_texts(repo, &head_snap)?,
            &adapters::project_types(&cfg.layers, &cfg.types),
        ),
    );
    let base_ad: Vec<_> = if use_base {
        check::apply_severity(
            &cfg,
            adapters::check(
                &adapter_texts(repo, &base_snap)?,
                &adapters::project_types(&cfg.layers, &cfg.types),
            ),
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
    // SD506: the eval files, only for a project with `[evals]`. The base
    // findings use the `[evals]` table of the base, so only a new finding
    // fails the gate. A base with no table has no findings.
    if let Some(ev) = cfg
        .evals
        .as_ref()
        .filter(|_| !cfg.is_off(diag::Code::EvalsInvalid))
    {
        let tree = eval_tree(repo, &head_snap, &ev.dir, untracked)?;
        let head_ev = check::apply_severity(&cfg, check::evals_invalid(ev, &tree));
        let base_ev: Vec<_> = match base_config(repo, &base_snap)?.and_then(|c| c.evals) {
            Some(bev) if use_base => {
                let tree = eval_tree(repo, &base_snap, &bev.dir, false)?;
                check::apply_severity(&cfg, check::evals_invalid(&bev, &tree))
            }
            _ => Vec::new(),
        };
        for d in head_ev {
            let is_new = !base_ev.iter().any(|b| b.fingerprint() == d.fingerprint());
            outcome.head.push(d);
            outcome.new.push(is_new);
        }
    }
    // A base before 0.6.0 and the head hold the configuration and the
    // documents of one upgrade, so SD503 does not apply (D156).
    if let Some(c) = &base_commit
        && !plan_base
        && let Some(d) = config_not_alone(repo, &cfg, c, untracked)?
    {
        outcome.head.push(d);
        outcome.new.push(true);
    }

    // A state code is never new, whichever check made it.
    for (d, new) in outcome.head.iter().zip(outcome.new.iter_mut()) {
        if d.code.is_state() {
            *new = false;
        }
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
    // Not diagnostics: no code, no severity, no effect on the exit code.
    let locked = check::locked_links(&cfg, head);
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
        let locked_json = locked.iter().map(|l| {
            Obj::new()
                .str("path", &l.path)
                .num("line", l.line)
                .str("target", &l.target)
                .str("pointer", &l.pointer)
                .end()
        });
        let mut obj = Obj::new()
            .str("version", VERSION)
            .str("mode", if a.all { "all" } else { "base" })
            .opt_str("base", base_commit.as_deref());
        // The lifecycle base: a SHA, or null if git cannot name it. The field
        // is absent when the policy is `none`.
        if cfg.change_policy != config::ChangePolicy::None {
            obj = obj.opt_str("change_base", change_base.as_deref());
        }
        println!(
            "{}",
            obj.bool("ok", !failed)
                .raw("summary", &summary)
                .raw("diagnostics", &json::array(diags))
                .raw("locked_links", &json::array(locked_json))
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
        if a.all {
            for l in &locked {
                println!("{}", l.line_text());
            }
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
        if cfg.change_policy != config::ChangePolicy::None
            && change_base.is_none()
            && life_note.is_none()
        {
            eprintln!(
                "note: {}",
                lifecycle::no_base_note(&cfg, &repo.git, a.base.as_deref())
            );
        }
    }
    if let Some(n) = &life_note {
        eprintln!("note: {n}");
    }
    Ok(if failed { 1 } else { 0 })
}

/// The pin of `spec-dna.toml` in a commit. `None` if the commit has no
/// `spec-dna.toml` (the adoption), or the file has no pin that `scan_pin` reads.
fn base_pin(repo: &Repo, commit: &str) -> Result<Option<String>, Fatal> {
    let got = repo
        .git
        .read_at(&[(commit.to_string(), config::FILE_NAME.to_string())])
        .map_err(Fatal::Io)?;
    Ok(got
        .into_iter()
        .next()
        .flatten()
        .and_then(|b| config::scan_pin(&String::from_utf8_lossy(&b))))
}

/// SD503: a change that edits `spec-dna.toml` together with documents or
/// code. A configuration change must be a change of its own. The files that
/// `init` writes with it (the adapters, the harness files, the hook files
/// that `init` writes or upgrades, the starter files of the evals, and files
/// named `spec-dna*`) do not count,
/// and a new `spec-dna.toml` (the adoption) is not checked.
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
    // The starter files of `skeleton` come with the table `[evals]`.
    let starter: Vec<String> = cfg
        .evals
        .as_ref()
        .map(|e| evals::skeleton_paths(&e.dir).to_vec())
        .unwrap_or_default();
    let others: Vec<&str> = changed
        .iter()
        .map(|(p, _)| p.as_str())
        .filter(|p| {
            *p != config::FILE_NAME
                && !adapters::PATHS.contains(p)
                && !harness::is_output_path(p)
                && !starter.iter().any(|s| s == p)
                && !init::HOOK_FILES.contains(p)
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
    let mut places = context::Places::new(&c, &cfg);
    if let Some(g) = a.group.as_deref() {
        places = places.with_group(&c, &cfg, g, group_areas(&cfg, g)?);
    }
    let cited = if paths.is_empty() {
        Vec::new()
    } else {
        cite::citing(
            &c,
            &cite::Citations::build(&c),
            &repo.root,
            &paths,
            places.ranks(),
        )
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
        &places,
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
            "--conventions: unknown type `{name}`; {}",
            model::DocType::unknown_hint(name)
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
/// The place follows the layer of the type (docs/types.md): a top file such as
/// `terms.md` takes no id, `intent` writes `<dir>/<slug>/intent.md`, `spec`
/// and `plan` need that folder, and another type writes `<dir>/<id>.md`. In a
/// root of one type, the file is `<id>.md`. The file has no `type:` line where
/// its place gives the type.
fn new_doc(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    use model::DocType;
    expect_args(a, 1, 2)?;
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    let name = &a.positional[0];
    let ty = DocType::from_name(name).ok_or_else(|| match DocType::from_old_name(name) {
        Some(t) => Fatal::Usage(format!("unknown type `{name}`; use `new {}`", t.name())),
        None => Fatal::Usage(format!(
            "unknown type `{name}`; {}",
            DocType::unknown_hint(name)
        )),
    })?;
    if !cfg.allows(ty) {
        return Err(Fatal::Usage(format!(
            "type `{name}` is not a type of this project (`types` in spec-dna.toml)"
        )));
    }
    let (template_name, template_text) =
        templates::resolve(&repo.root, &cfg.templates_dir, ty, a.template.as_deref())
            .map_err(Fatal::Usage)?;
    // The message names the type for a default template.
    let template_name = if a.template.is_some() {
        template_name
    } else {
        ty.name().to_string()
    };
    let id = match a.positional.get(1) {
        None => None,
        Some(id) => {
            let id = id.trim_matches('/');
            let id = id.strip_suffix(".md").unwrap_or(id);
            if id.is_empty() || id.split('/').any(|s| s.is_empty() || s == "." || s == "..") {
                return Err(Fatal::Usage(format!(
                    "`{id}` is not a document id (`<slug>`, or `<area>/<slug>`)"
                )));
            }
            Some(id)
        }
    };
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
    if let Some(t) = root.single_type.filter(|&t| t != ty) {
        return Err(Fatal::Usage(format!(
            "root `{}` holds only `{}` documents",
            root.path,
            t.name()
        )));
    }
    let need_id = || {
        id.ok_or_else(|| {
            Fatal::Usage(format!(
                "`new {}` needs an id: `new {} <id>`",
                ty.name(),
                ty.name()
            ))
        })
    };
    // `rel` is the path inside the root, `title` is the H1 of the document.
    let (rel, title) = if root.single_type.is_some() {
        let id = need_id()?;
        (format!("{id}.md"), templates::title_of(id))
    } else {
        let layer = cfg
            .layers
            .layer_of_type(ty)
            .map(|i| &cfg.layers.layers()[i])
            .ok_or_else(|| {
                Fatal::Usage(format!(
                    "no layer of spec-dna.toml has type `{0}`; run `spec-dna layer add <name> --types {0}` to add one",
                    ty.name()
                ))
            })?;
        match layer.dir() {
            // A layer with top files: the type is a file of the root.
            None => {
                if let Some(id) = id {
                    return Err(Fatal::Usage(format!(
                        "`new {0}` takes no id: it writes the top file `{0}.md`; got `{id}`",
                        ty.name()
                    )));
                }
                let file = format!("{}.md", ty.name());
                if !layer
                    .files
                    .iter()
                    .any(|f| f.trim_start_matches("./") == file)
                {
                    return Err(Fatal::Usage(format!(
                        "layer `{}` has no top file `{file}` in `files`; add it, or use another layer",
                        layer.name
                    )));
                }
                (file, templates::title_of(ty.name()))
            }
            Some(dir) if ty.is_intent_type() => {
                let slug = need_id()?;
                if slug.contains('/') {
                    return Err(Fatal::Usage(format!(
                        "`{slug}` is not a slug: the folder of a change has one name"
                    )));
                }
                let folder = repo.root.join(&root.path).join(dir).join(slug);
                if ty != DocType::Intent && !folder.is_dir() {
                    return Err(Fatal::NotFound(format!(
                        "the folder `{}` does not exist; run `spec-dna new intent {slug}` first",
                        folder
                            .strip_prefix(&repo.root)
                            .unwrap_or(&folder)
                            .to_string_lossy()
                    )));
                }
                (
                    format!("{dir}/{slug}/{}.md", ty.name()),
                    templates::title_of(slug),
                )
            }
            Some(dir) => {
                let id = need_id()?;
                (format!("{dir}/{id}.md"), templates::title_of(id))
            }
        }
    };
    let path = if root.path.is_empty() {
        rel.clone()
    } else {
        format!("{}/{rel}", root.path)
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
    let place_gives_type = root.single_type == Some(ty) || cfg.layers.place_type(&rel) == Some(ty);
    let text = templates::instantiate(&template_text, ty, &title, !place_gives_type);
    mv::write(&file, &text)?;
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
        println!("wrote {path} ({template_name} template); replace each {{{{...}}}} placeholder");
    }
    Ok(0)
}

fn layer_cmd(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    match a.positional.first().map(String::as_str) {
        Some("add") => layer_add(repo, a),
        Some("list") => layer_list(repo, a),
        Some(other) => Err(Fatal::Usage(format!(
            "unknown `layer` command `{other}`; use `layer add <name>` or `layer list`"
        ))),
        None => Err(Fatal::Usage(
            "`layer` needs a command: `layer add <name>` or `layer list`".into(),
        )),
    }
}

fn layer_add(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    use layer::{ContextMode, Keep, Layer};
    use model::DocType;
    expect_args(a, 2, 2)?;
    let f = &a.layer;
    let name = &a.positional[1];
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(Fatal::Usage(format!(
            "`{name}` is not a layer name: use only letters, digits, `-` and `_`"
        )));
    }
    let mut new = Layer::new(name);
    for t in &f.types {
        let ty = DocType::from_name(t)
            .ok_or_else(|| Fatal::Usage(config::unknown_type("`--types`", t)))?;
        if !new.types.contains(&ty) {
            new.types.push(ty);
        }
    }
    if let Some(d) = &f.dir {
        new.dir_key =
            Some(layer::check_dir(d).map_err(|e| Fatal::Usage(format!("--dir: `{d}` {e}")))?);
    }
    for file in &f.files {
        layer::check_file(file).map_err(|e| Fatal::Usage(format!("--files {e}")))?;
    }
    new.files = f.files.clone();
    new.paths = f.paths.clone();
    let dedup = |v: &[String]| {
        let mut out: Vec<String> = Vec::new();
        for s in v {
            if !out.contains(s) {
                out.push(s.clone());
            }
        }
        out
    };
    new.moves_to = dedup(&f.moves_to);
    new.linked_from = dedup(&f.linked_from);
    if let Some(k) = &f.keep {
        new.keep = Keep::from_name(k).ok_or_else(|| {
            Fatal::Usage(format!(
                "--keep: `{k}` is not `forever` or `until-finished`"
            ))
        })?;
    }
    if let Some(c) = &f.context {
        new.context = ContextMode::from_name(c).ok_or_else(|| {
            Fatal::Usage(format!("--context: `{c}` is not `always` or `on-request`"))
        })?;
    }
    new.max_docs = f.max_docs;
    new.max_chars = f.max_chars;
    new.max_started = f.max_started;
    new.stale_days = f.stale_days;
    let moved_from = dedup(&f.moved_from);

    let path = repo.config_path();
    let text = std::fs::read_to_string(&path).map_err(|_| {
        Fatal::Config(format!(
            "no {} at {}; run `spec-dna init`",
            config::FILE_NAME,
            repo::shown(&repo.root)
        ))
    })?;
    let cfg = Repo::parse_config(&text)?;
    warn_pin(&cfg);
    let out = layer::add_layer(
        &text,
        &cfg.layers,
        cfg.layers_declared,
        new.clone(),
        &moved_from,
    )
    .map_err(Fatal::Usage)?;
    // A `types` list that the file has limits the types of the project, so
    // the types of the new layer join it. A file with no list allows each
    // type already. The edit keeps each comment.
    let (out, types_added) =
        add_types_to_list(out, &cfg.types, &new.types).map_err(Fatal::Usage)?;
    // `add_layer` read the new text: this cannot fail.
    let read = Repo::parse_config(&out)?;
    let level = read.layers.index(name).map_or(0, |i| read.layers.level(i));
    mv::write(&path, &out)?;
    let mut next = Vec::new();
    next.push(format!(
        "commit {} as a change of its own (SD503)",
        config::FILE_NAME
    ));
    if adapters::project_types(&cfg.layers, &cfg.types)
        != adapters::project_types(&read.layers, &read.types)
        && repo.root.join(adapters::SKILL).is_file()
    {
        next.push(
            "run spec-dna init to update the agent skill, which lists the types of the project"
                .to_string(),
        );
    }
    if let Some(first) = new.types.first() {
        // A layer with top files holds types that take no id.
        let id = if new.dir().is_some() { " <id>" } else { "" };
        next.push(format!(
            "run spec-dna new {}{id} to write the first document of the layer",
            first.name()
        ));
    }
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("layer", name)
                .num("level", level)
                .str("path", config::FILE_NAME)
                .bool("wrote_implied_graph", !cfg.layers_declared)
                .strs("moved_from", &moved_from)
                .strs("types_added", &types_added)
                .strs("next", &next)
                .end()
        );
    } else {
        println!(
            "added layer `{name}` (level {level}) to {}",
            config::FILE_NAME
        );
        if !cfg.layers_declared {
            println!(
                "{} had no [layers] table: it now has the tables of the default graph, then `{name}`",
                config::FILE_NAME
            );
        }
        for from in &moved_from {
            println!("`{from}` now moves to `{name}`");
        }
        if !types_added.is_empty() {
            println!("added the types {} to `types`", ticked(&types_added));
        }
        for line in &next {
            println!("next: {line}");
        }
    }
    Ok(0)
}

/// Appends the types of `new` that `list` lacks to the top-level key `types`
/// of the configuration text. It gives the new text and the names that it
/// added. An empty `list` means that the file has no `types` key or an empty
/// one: each type is valid, so the text stays. A list that the edit cannot
/// grow in place, such as a list on many lines, is a refusal: the caller
/// writes nothing.
fn add_types_to_list(
    text: String,
    list: &[model::DocType],
    new: &[model::DocType],
) -> Result<(String, Vec<String>), String> {
    let missing: Vec<String> = new
        .iter()
        .filter(|t| !list.contains(t))
        .map(|t| t.name().to_string())
        .collect();
    if list.is_empty() || missing.is_empty() {
        return Ok((text, Vec::new()));
    }
    let all: Vec<toml::Value> = list
        .iter()
        .map(|t| t.name().to_string())
        .chain(missing.iter().cloned())
        .map(toml::Value::String)
        .collect();
    match config_text::set_value(&text, &[config_text::k("types")], &toml::Value::Array(all)) {
        Ok(t) => Ok((t, missing)),
        Err(_) => Err(format!(
            "`types` in {} is a list that the command cannot grow in place (a list on many lines, for example); add {} to it by hand, then run `layer add` again",
            config::FILE_NAME,
            ticked(&missing)
        )),
    }
}

/// The names in backticks, separated by commas.
fn ticked(names: &[String]) -> String {
    names
        .iter()
        .map(|n| format!("`{n}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn layer_list(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 1, 1)?;
    if a.layer.given {
        return Err(Fatal::Usage(
            "`layer list` takes no layer option; the options are for `layer add`".into(),
        ));
    }
    let (cfg, c) = load_head(repo, a)?;
    let g = &cfg.layers;
    let rules = [layer::Rule::Type, layer::Rule::Paths, layer::Rule::Files];
    let mut counts = vec![[0usize; 3]; g.layers().len()];
    let mut unplaced = 0usize;
    let mut placed = Vec::new();
    for d in &c.docs {
        let rel = cfg.roots[d.root].rel(&d.path);
        let p = if d.has_layer() {
            g.placement(rel, d.ty)
        } else {
            None
        };
        match p {
            Some((i, r)) => counts[i][rules.iter().position(|&x| x == r).unwrap_or(0)] += 1,
            None => unplaced += 1,
        }
        placed.push((d, p));
    }
    if a.json {
        let layers = g.layers().iter().enumerate().map(|(i, l)| {
            let types: Vec<String> = l.types.iter().map(|t| t.name().to_string()).collect();
            let mut o = Obj::new()
                .str("name", &l.name)
                .num("level", g.level(i))
                .strs("types", &types)
                .opt_str("dir", l.dir())
                .strs("files", &l.files)
                .strs("paths", &l.paths)
                .strs("moves_to", &l.moves_to)
                .strs("linked_from", &l.linked_from)
                .str("keep", l.keep.name())
                .str("context", l.context.name());
            for (key, n) in [
                ("max_docs", l.max_docs),
                ("max_chars", l.max_chars),
                ("max_started", l.max_started),
                ("stale_days", l.stale_days),
            ] {
                o = match n {
                    Some(n) => o.num(key, n),
                    None => o.raw(key, "null"),
                };
            }
            let mut by = Obj::new();
            for (k, r) in rules.iter().enumerate() {
                by = by.num(r.name(), counts[i][k]);
            }
            o.num("documents", counts[i].iter().sum::<usize>())
                .raw("by_rule", &by.end())
                .end()
        });
        let docs = placed.iter().map(|(d, p)| {
            Obj::new()
                .str("path", &d.path)
                .str("id", &d.id)
                .str("type", d.ty.name())
                .opt_str("layer", p.map(|(i, _)| g.layers()[i].name.as_str()))
                .opt_str("rule", p.map(|(_, r)| r.name()))
                .end()
        });
        println!(
            "{}",
            Obj::new()
                .bool("declared", cfg.layers_declared)
                .raw("layers", &json::array(layers))
                .raw("documents", &json::array(docs))
                .num("unplaced", unplaced)
                .end()
        );
        return Ok(0);
    }
    if !cfg.layers_declared {
        println!(
            "{} has no [layers] table: these layers are the default graph (`layer add` writes it)",
            config::FILE_NAME
        );
    }
    for (i, l) in g.layers().iter().enumerate() {
        let mut parts = vec![l.name.clone(), format!("level {}", g.level(i))];
        let types: Vec<&str> = l.types.iter().map(|t| t.name()).collect();
        for (key, list) in [
            ("types", types),
            ("files", l.files.iter().map(String::as_str).collect()),
            ("paths", l.paths.iter().map(String::as_str).collect()),
            ("moves-to", l.moves_to.iter().map(String::as_str).collect()),
            (
                "linked-from",
                l.linked_from.iter().map(String::as_str).collect(),
            ),
        ] {
            if !list.is_empty() {
                parts.push(format!("{key} {}", list.join(",")));
            }
        }
        if l.dir_key.is_some()
            && let Some(d) = l.dir()
        {
            parts.push(format!("dir {d}"));
        }
        parts.push(format!("keep {}", l.keep.name()));
        parts.push(format!("context {}", l.context.name()));
        for (key, n) in [
            ("max-docs", l.max_docs),
            ("max-chars", l.max_chars),
            ("max-started", l.max_started),
            ("stale-days", l.stale_days),
        ] {
            if let Some(n) = n {
                parts.push(format!("{key} {n}"));
            }
        }
        let total: usize = counts[i].iter().sum();
        let by: Vec<String> = rules
            .iter()
            .enumerate()
            .filter(|(k, _)| counts[i][*k] > 0)
            .map(|(k, r)| format!("{} {}", r.name(), counts[i][k]))
            .collect();
        parts.push(if by.is_empty() {
            format!("docs {total}")
        } else {
            format!("docs {total} ({})", by.join(", "))
        });
        println!("{}", parts.join("  "));
    }
    if unplaced > 0 {
        let (noun, verb) = if unplaced == 1 {
            ("document has", "its")
        } else {
            ("documents have", "their")
        };
        println!("{unplaced} {noun} no layer (no layer has {verb} type)");
    }
    Ok(0)
}

fn split_opts(a: &Args) -> Result<split::Opts, Fatal> {
    let ty = match &a.ty {
        None => None,
        Some(name) => Some(model::DocType::from_name(name).ok_or_else(|| {
            Fatal::Usage(format!(
                "unknown type `{name}`; {}",
                model::DocType::unknown_hint(name)
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

/// Moves the history sections of a document into a research document.
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
                .is_some_and(|t| t != model::DocType::Research)
            {
                return Err(Fatal::Usage(format!(
                    "root `{}` holds only `{}` documents; give the path of the research document under another root, such as `<dir>/{}-history.md`",
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
        ty: Some(model::DocType::Research),
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

/// A stop of `finish`, printed as text or as one JSON object. The exit code is
/// 1.
fn report_stop(slug: &str, a: &Args, stop: &finish::Stop) -> u8 {
    if a.json {
        println!(
            "{}",
            Obj::new()
                .str("slug", slug)
                .bool("ok", false)
                .bool("dry_run", a.dry_run)
                .num("step", stop.step)
                .str("reason", &stop.reason)
                .raw(
                    "diagnostics",
                    &json::array(stop.diags.iter().map(|d| d.json(true)))
                )
                .raw("files", "[]")
                .end()
        );
    } else {
        let shown: Vec<&diag::Diag> = stop.diags.iter().collect();
        for line in human_lines(&shown, false) {
            println!("{line}");
        }
        eprintln!(
            "spec-dna: finish `{slug}` stopped at step {}: {}; nothing changed",
            stop.step, stop.reason
        );
    }
    1
}

/// `next <slug>`: the checklist to the next gate (`stage.rs`).
fn next(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 1, 1)?;
    let slug = a.positional[0].as_str();
    if !finish::valid_slug(slug) {
        return Err(Fatal::Usage(format!(
            "`{slug}` is not the name of a change; give the folder name of the change, with no `/`"
        )));
    }
    if a.group.is_some() {
        return Err(Fatal::Usage(
            "`next` has no --group option; `status --group <name>` lists the changes of a group"
                .into(),
        ));
    }
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    stage::next(repo, &cfg, slug, a.base.as_deref(), a.json, !a.no_cache)
}

/// The areas of the group `name` of `spec-dna.toml`. An unknown group is a
/// usage error.
fn group_areas<'a>(cfg: &'a config::Config, name: &str) -> Result<&'a [String], Fatal> {
    match cfg.groups.iter().find(|(n, _)| n == name) {
        Some((_, areas)) => Ok(areas.as_slice()),
        None => {
            let names: Vec<&str> = cfg.groups.iter().map(|(n, _)| n.as_str()).collect();
            Err(Fatal::Usage(if names.is_empty() {
                format!("`--group {name}`: spec-dna.toml has no [groups] table")
            } else {
                format!(
                    "`--group {name}` is not a group of spec-dna.toml; the groups are: {}",
                    names.join(", ")
                )
            }))
        }
    }
}

/// `status [--group <name>]`: each change and its stage (`stage.rs`).
fn status(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    let areas = a
        .group
        .as_deref()
        .map(|g| group_areas(&cfg, g))
        .transpose()?;
    stage::status(repo, &cfg, areas, a.base.as_deref(), a.json, !a.no_cache)
}

/// `log [--since <rev>]`: the finished and the closed changes (`sdlc.rs`).
fn log(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    for (given, name) in [
        (a.group.is_some(), "--group"),
        (a.base.is_some(), "--base"),
        (a.sdlc, "--sdlc"),
    ] {
        if given {
            return Err(Fatal::Usage(format!("`log` has no {name} option")));
        }
    }
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    sdlc::log(repo, &cfg, a.since.as_deref(), a.json)
}

/// `eval run [<id>...]`: the runner of the evals (`eval_run.rs`). `eval run`
/// spends money and runs the checks of the project, so a pin mismatch stops
/// it, as it stops `check` and `finish`.
fn eval(cwd: &std::path::Path, a: &Args) -> Result<u8, Fatal> {
    if a.positional.first().map(String::as_str) != Some("run") {
        return Err(Fatal::Usage(
            "`eval` needs a subcommand: `spec-dna eval run [<id>...]`".into(),
        ));
    }
    let repo = Repo::discover(cwd)?;
    let cfg = repo.load_config()?;
    if cfg.pin != VERSION {
        return Err(pin_mismatch(&cfg));
    }
    let mut ids = a.positional[1..].to_vec();
    ids.dedup();
    eval_run::run(
        &repo,
        &cfg,
        &eval_run::Opts {
            ids,
            runs: a.runs,
            min_pass_rate: a.min_pass_rate,
            max_total_usd: a.max_total_usd,
            out: a.out.clone(),
            dry_run: a.dry_run,
            isolate: a.isolate,
            json: a.json,
        },
    )
}

/// `stats --sdlc [--group <name>]`: the indicators of the playbook
/// (`sdlc.rs`).
fn stats(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 0, 0)?;
    if !a.sdlc {
        return Err(Fatal::Usage(
            "`stats` needs --sdlc: `spec-dna stats --sdlc [--group <name>]` prints the indicators of the playbook"
                .into(),
        ));
    }
    for (given, name) in [(a.since.is_some(), "--since"), (a.base.is_some(), "--base")] {
        if given {
            return Err(Fatal::Usage(format!("`stats` has no {name} option")));
        }
    }
    let cfg = repo.load_config()?;
    warn_pin(&cfg);
    let group = a
        .group
        .as_deref()
        .map(|g| group_areas(&cfg, g).map(|areas| (g, areas)))
        .transpose()?;
    sdlc::stats(repo, &cfg, group, a.json)
}

/// Finishes a change: the plan, then the write (`finish.rs`).
fn finish(repo: &Repo, a: &Args) -> Result<u8, Fatal> {
    expect_args(a, 1, 1)?;
    let slug = a.positional[0].as_str();
    if !finish::valid_slug(slug) {
        return Err(Fatal::Usage(format!(
            "`{slug}` is not the name of a change; give the folder name of the change, with no `/`"
        )));
    }
    let cfg = repo.load_config()?;
    if cfg.pin != VERSION {
        return Err(pin_mismatch(&cfg));
    }
    // The base needs no policy: `finish` stops whatever the policy is.
    let base_rev =
        lifecycle::merge_base_of(&repo.git, a.base.as_deref(), cfg.change_base.as_deref())
            .map_err(Fatal::Config)?;
    let folder = lifecycle::intent_dirs(&cfg)
        .iter()
        .map(|d| format!("{d}/{slug}"))
        .find(|f| repo.root.join(f).is_dir())
        .ok_or_else(|| {
            Fatal::NotFound(format!(
                "no folder `<intent folder>/{slug}` for the change `{slug}`"
            ))
        })?;
    // The folder must be tracked and equal to HEAD: `finish` deletes it, and
    // git must keep what it deletes.
    let precondition = |reason: String| {
        report_stop(
            slug,
            a,
            &finish::Stop {
                step: 0,
                reason,
                diags: Vec::new(),
            },
        )
    };
    let prefix = format!("{folder}/");
    if repo.head_commit().is_none() {
        return Ok(precondition("the repository has no commit".to_string()));
    }
    let tracked: Vec<String> = repo
        .git
        .ls_files()
        .map_err(Fatal::Io)?
        .into_iter()
        .map(|e| e.0)
        .filter(|p| p.starts_with(&prefix))
        .collect();
    if tracked.is_empty() {
        return Ok(precondition(format!(
            "git tracks no file of the folder `{folder}`; commit the change first"
        )));
    }
    // Each path that differs from HEAD, tracked or not.
    let differs: Vec<String> = repo
        .git
        .diff_paths("HEAD", true)
        .map_err(Fatal::Io)?
        .into_iter()
        .map(|(p, _)| p)
        .collect();
    let dirty: Vec<&String> = differs.iter().filter(|p| p.starts_with(&prefix)).collect();
    if !dirty.is_empty() {
        return Ok(precondition(format!(
            "the folder `{folder}` differs from HEAD (an edit that is not committed, or a file that git does not track): {}; commit or remove them first",
            dirty
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    // The trees: the working tree, and the lifecycle base if git names it.
    let head_snap = Snapshot::worktree(repo, &repo::root_paths(&cfg), true)?;
    let base_snap = base_rev
        .as_deref()
        .map(|r| Snapshot::at(repo, r))
        .transpose()?;
    let mut snaps: Vec<&Snapshot> = vec![&head_snap];
    snaps.extend(base_snap.as_ref());
    let mut loaded = repo::load(repo, &cfg, &snaps, !a.no_cache, false)?;
    let base = if base_snap.is_some() {
        Some(loaded.corpora.remove(1))
    } else {
        None
    };
    let mut head = loaded.corpora.remove(0);
    head.covers = evidence::load(repo, &cfg, None, true)?;
    let diff = base_rev
        .as_deref()
        .map(|r| lifecycle::Diff::read(&repo.git, r, true))
        .transpose()
        .map_err(Fatal::Io)?;
    let read = |p: &str| mv::read(repo, p).map_err(|e| e.message().to_string());
    let exists = |p: &str| repo.root.join(p).exists();
    let path_problem = |p: &str| finish::path_problem(&repo.root, p);
    let cx = finish::Ctx {
        config: &cfg,
        head: &head,
        base: base.as_ref(),
        diff: diff.as_ref(),
        folder: &folder,
        folder_files: &tracked,
        read: &read,
        exists: &exists,
        path_problem: &path_problem,
    };
    let plan = match finish::plan(&cx) {
        Ok(p) => p,
        Err(finish::Failure::Stop(s)) => return Ok(report_stop(slug, a, &s)),
        Err(finish::Failure::NoBase) => {
            return Err(Fatal::Config(format!(
                "the change `{slug}` is `accepted`, and `finish` compares its delta with the base tree; {}",
                lifecycle::no_base_note(&cfg, &repo.git, a.base.as_deref())
            )));
        }
        Err(finish::Failure::Io(m)) => return Err(Fatal::Io(m)),
    };
    // `finish` stages each file that it writes. An edit of the author in such a
    // file, not committed, would go into the same stage.
    let edited: Vec<&str> = plan
        .writes
        .iter()
        .filter(|w| w.old.is_some() && differs.contains(&w.path))
        .map(|w| w.path.as_str())
        .collect();
    if !edited.is_empty() {
        return Ok(precondition(format!(
            "`finish` would write {}, and the file has an edit that is not committed; `finish` stages the file, so commit or stash the edit first",
            edited
                .iter()
                .map(|p| format!("`{p}`"))
                .collect::<Vec<_>>()
                .join(", ")
        )));
    }
    if !a.dry_run {
        finish::write_plan(&repo.root, &repo.git, &plan).map_err(|e| Fatal::Io(e.message))?;
    }
    report_finish(slug, &folder, a, &plan);
    Ok(0)
}

/// The report of a plan that no step stopped, written or not (`--dry-run`).
fn report_finish(slug: &str, folder: &str, a: &Args, plan: &finish::Plan) {
    let diff_of = |w: &finish::Write| init::unified_diff(&w.path, w.old.as_deref(), &w.new, false);
    if a.json {
        let writes = plan.writes.iter().map(|w| {
            Obj::new()
                .str("path", &w.path)
                .str("action", "write")
                .str("diff", &diff_of(w))
                .end()
        });
        let removes = plan.removes.iter().map(|p| {
            Obj::new()
                .str("path", p)
                .str("action", "remove")
                .str("diff", "")
                .end()
        });
        println!(
            "{}",
            Obj::new()
                .str("slug", slug)
                .bool("ok", true)
                .bool("dry_run", a.dry_run)
                .num("step", 6)
                .raw("diagnostics", "[]")
                .raw("files", &json::array(writes.chain(removes)))
                .end()
        );
        return;
    }
    let (add, change, remove) = plan.applied;
    for w in &plan.writes {
        if a.dry_run {
            print!("{}", diff_of(w));
        } else {
            println!("wrote {}", w.path);
        }
    }
    for p in &plan.removes {
        println!("{} {p}", if a.dry_run { "remove" } else { "removed" });
    }
    let lines = format!("{add} ADD, {change} CHANGE and {remove} REMOVE delta lines");
    if a.dry_run {
        println!(
            "spec-dna finish `{slug}` (dry run): it would apply {lines}, write {} files and delete the folder `{folder}` ({} files); nothing changed",
            plan.writes.len(),
            plan.removes.len()
        );
    } else {
        println!(
            "spec-dna finish `{slug}`: applied {lines}, wrote {} files and deleted the folder `{folder}` ({} files). `finish` staged the paths with `git add`. Read the diff, then commit it with the code",
            plan.writes.len(),
            plan.removes.len()
        );
    }
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
        if m.kind == "document" {
            o = o.opt_str("alias_dropped", m.alias_dropped.as_deref());
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
        _ => {
            let dropped = m
                .alias_dropped
                .as_ref()
                .map_or(String::new(), |a| format!(" (alias dropped: {a})"));
            println!(
                "moved {} {} -> {} (alias kept: {}){dropped}",
                m.kind, m.from, m.to, m.alias
            );
        }
    }
    for line in &m.changes {
        println!("{line}");
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

#[cfg(test)]
mod tests {
    use super::*;

    /// The part of `docs/diagnostics.md` under the heading "Files that SD503
    /// counts", up to the next heading.
    fn sd503_section() -> &'static str {
        let doc = include_str!("../docs/diagnostics.md");
        let start = doc
            .find("### Files that SD503 counts")
            .expect("the section `Files that SD503 counts` is in docs/diagnostics.md");
        let rest = &doc[start + 4..];
        let end = rest.find("\n#").unwrap_or(rest.len());
        &rest[..end]
    }

    #[test]
    fn the_sd503_section_names_each_path_that_does_not_count() {
        let section = sd503_section();
        let mut paths: Vec<String> = vec![config::FILE_NAME.to_string()];
        paths.extend(adapters::PATHS.iter().map(|p| p.to_string()));
        paths.extend(init::HOOK_FILES.iter().map(|p| p.to_string()));
        paths.extend(harness::OLD_AGENT_PATHS.iter().map(|p| p.to_string()));
        paths.extend([
            harness::SETTINGS.to_string(),
            harness::TEAM_PAGE.to_string(),
            harness::REVIEW.to_string(),
        ]);
        // The agent files and the stage skills, with every role and skill on.
        let h = harness::Harness::new(harness::Level::Full);
        paths.extend(harness::generated_files(&h).into_iter().map(|g| g.path));
        paths.extend(harness::Skill::ALL.iter().map(|s| s.path()));
        // The starter files of the evals, with the folder `<dir>` of `[evals]`.
        paths.extend(evals::skeleton_paths("<dir>"));
        for p in &paths {
            assert!(
                harness::is_output_path(p)
                    || adapters::PATHS.contains(&p.as_str())
                    || init::HOOK_FILES.contains(&p.as_str())
                    || p == config::FILE_NAME
                    || p.starts_with("<dir>")
                    || p.ends_with("agent-evals.yml"),
                "`{p}` does not skip SD503 in the code"
            );
            assert!(
                section.contains(&format!("`{p}`")),
                "the section lacks `{p}`"
            );
        }
        // The 5 agent files of 0.6 are in the list, so a new one fails here.
        assert!(
            paths
                .iter()
                .filter(|p| p.starts_with(harness::AGENTS_DIR))
                .count()
                >= 8,
            "{paths:?}"
        );
    }
}

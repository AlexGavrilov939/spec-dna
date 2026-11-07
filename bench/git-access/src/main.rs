//! Git access benchmark for ADR 0002, confirmation step 2.
//!
//! Reads the git inputs that `check --base <rev>` needs, either through the git CLI
//! (`cli` mode) or in-process through the `gix` crate (`gix` mode), and prints a
//! deterministic summary so both modes can be compared byte for byte.
//!
//!     git-access <cli|cli-par|gix> <repo> <base-rev> <changed|md>
//!
//! Steps:
//!   1. head: every index path with its blob id, tracked files changed in the
//!      working tree, untracked non-ignored files.
//!   2. base: resolve <base-rev>, the merge base with HEAD, every path and blob id of its tree.
//!   3. blobs: contents of base blobs. `changed` takes the `.md` blobs of the base tree whose
//!      path is missing from the index or has another id there; `md` takes every `.md` blob.
//!
//! The summary goes to stdout. Step timings and peak RSS go to stderr as one `timing` line.
//! `cli` runs the steps one after another. `cli-par` starts every git process as early as
//! its input allows, so the steps overlap. Both run `git` from PATH, or the executable named
//! by `GIT_ACCESS_GIT`.

use std::process::ExitCode;
use std::time::Instant;

type Res<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
type Oid = [u8; 20];

/// One path with its object id, from the index or from a tree.
struct Entry {
    path: Vec<u8>,
    id: Oid,
}

/// Step 1 output.
#[derive(Default)]
struct Head {
    index: Vec<Entry>,
    modified: Vec<Vec<u8>>,
    deleted: Vec<Vec<u8>>,
    untracked: Vec<Vec<u8>>,
    conflicted: Vec<Vec<u8>>,
}

/// Step 2 output.
struct Base {
    merge_base: Oid,
    tree: Vec<Entry>,
}

/// Step 3 output: a running checksum over (id, length, bytes) in request order.
#[derive(Default)]
struct Blobs {
    count: usize,
    bytes: u64,
    sum: Sum,
}

impl Blobs {
    fn add(&mut self, id: &Oid, data: &[u8]) {
        self.count += 1;
        self.bytes += data.len() as u64;
        self.sum.write(id);
        self.sum.write(&(data.len() as u64).to_le_bytes());
        self.sum.write(data);
    }
}

/// Fast non-cryptographic 64-bit checksum (FxHash-style word mixing). Used only to prove
/// that both modes saw the same bytes; it costs the same in both modes.
#[derive(Clone, Copy)]
struct Sum(u64);

impl Default for Sum {
    fn default() -> Self {
        Sum(0xcbf2_9ce4_8422_2325)
    }
}

impl Sum {
    const K: u64 = 0x517c_c1b7_2722_0a95;

    fn word(&mut self, w: u64) {
        self.0 = (self.0.rotate_left(5) ^ w).wrapping_mul(Self::K);
    }

    fn write(&mut self, data: &[u8]) {
        let (words, rest) = data.as_chunks::<8>();
        for w in words {
            self.word(u64::from_le_bytes(*w));
        }
        for &b in rest {
            self.word(b as u64);
        }
        self.word(data.len() as u64);
    }
}

fn hex(id: &Oid) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(40);
    for b in id {
        s.push(DIGITS[(b >> 4) as usize] as char);
        s.push(DIGITS[(b & 15) as usize] as char);
    }
    s
}

#[cfg_attr(not(feature = "cli"), allow(dead_code))]
fn parse_hex(s: &[u8]) -> Res<Oid> {
    if s.len() != 40 {
        return Err(format!("bad object id {:?}", String::from_utf8_lossy(s)).into());
    }
    let mut id = [0u8; 20];
    for (i, pair) in s.as_chunks::<2>().0.iter().enumerate() {
        let v = |c: u8| -> Res<u8> {
            match c {
                b'0'..=b'9' => Ok(c - b'0'),
                b'a'..=b'f' => Ok(c - b'a' + 10),
                _ => Err("bad hex digit".into()),
            }
        };
        id[i] = v(pair[0])? << 4 | v(pair[1])?;
    }
    Ok(id)
}

/// Which base blobs step 3 reads, in sorted id order without duplicates.
fn select_blobs(head: &Head, base: &Base, which: &str) -> Vec<Oid> {
    let index: std::collections::HashMap<&[u8], &Oid> = head
        .index
        .iter()
        .map(|e| (e.path.as_slice(), &e.id))
        .collect();
    let mut ids: Vec<Oid> = base
        .tree
        .iter()
        .filter(|e| e.path.ends_with(b".md"))
        .filter(|e| which == "md" || index.get(e.path.as_slice()) != Some(&&e.id))
        .map(|e| e.id)
        .collect();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn entries_sum(entries: &mut [Entry]) -> Sum {
    entries.sort_unstable_by(|a, b| a.path.cmp(&b.path).then(a.id.cmp(&b.id)));
    let mut s = Sum::default();
    for e in entries.iter() {
        s.write(&e.path);
        s.write(&e.id);
    }
    s
}

fn paths_sum(paths: &mut [Vec<u8>]) -> Sum {
    paths.sort_unstable();
    let mut s = Sum::default();
    for p in paths.iter() {
        s.write(p);
    }
    s
}

/// Peak resident set size in KiB for this process and for its largest waited-for child.
#[cfg(unix)]
fn peak_rss_kib() -> (u64, u64) {
    #[repr(C)]
    struct RUsage {
        utime: [i64; 2],
        stime: [i64; 2],
        maxrss: i64,
        rest: [i64; 13],
    }
    unsafe extern "C" {
        fn getrusage(who: i32, usage: *mut RUsage) -> i32;
    }
    let get = |who: i32| -> u64 {
        let mut ru = RUsage {
            utime: [0; 2],
            stime: [0; 2],
            maxrss: 0,
            rest: [0; 13],
        };
        // SAFETY: RUsage matches `struct rusage` on 64-bit macOS and Linux (two 16-byte
        // timevals, then 14 longs); getrusage only writes into it.
        if unsafe { getrusage(who, &mut ru) } != 0 {
            return 0;
        }
        let v = ru.maxrss.max(0) as u64;
        if cfg!(target_os = "macos") {
            v / 1024
        } else {
            v
        }
    };
    (get(0), get(-1))
}

#[cfg(not(unix))]
fn peak_rss_kib() -> (u64, u64) {
    (0, 0)
}

#[cfg(feature = "cli")]
mod cli {
    //! Git CLI plumbing. Every process runs with GIT_OPTIONAL_LOCKS=0 so `git status`
    //! never rewrites the index.
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::path::Path;
    use std::process::{Child, Command, Stdio};

    const LS_FILES: &[&str] = &["ls-files", "-s", "-z"];
    const STATUS: &[&str] = &[
        "status",
        "--porcelain=v2",
        "-z",
        "--untracked-files=all",
        "--no-renames",
    ];

    /// The git executable: `GIT_ACCESS_GIT` if set, else `git` from PATH.
    fn git(repo: &Path, args: &[&str]) -> Command {
        let exe = std::env::var_os("GIT_ACCESS_GIT").unwrap_or_else(|| "git".into());
        let mut c = Command::new(exe);
        c.current_dir(repo)
            .args(args)
            .env("GIT_OPTIONAL_LOCKS", "0");
        c.stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        c
    }

    fn spawn(repo: &Path, args: &[&str]) -> Res<Child> {
        Ok(git(repo, args).spawn()?)
    }

    /// Wait for a child that was spawned with piped stdout and return its stdout.
    fn finish(child: Child, what: &str) -> Res<Vec<u8>> {
        let out = child.wait_with_output()?;
        if !out.status.success() {
            return Err(format!("{what} failed: {}", String::from_utf8_lossy(&out.stderr)).into());
        }
        Ok(out.stdout)
    }

    /// Run `finish` on another thread, so several children drain their pipes at once.
    fn finish_bg(child: Child, what: &'static str) -> std::thread::JoinHandle<Res<Vec<u8>>> {
        std::thread::spawn(move || finish(child, what))
    }

    fn join<T>(h: std::thread::JoinHandle<Res<T>>) -> Res<T> {
        h.join().map_err(|_| "worker thread panicked")?
    }

    /// `git ls-files -s -z`: `<mode> <oid> <stage>\t<path>\0`
    fn parse_ls_files(out: &[u8], head: &mut Head) -> Res<()> {
        for rec in out.split(|&b| b == 0).filter(|r| !r.is_empty()) {
            let tab = rec
                .iter()
                .position(|&b| b == b'\t')
                .ok_or("ls-files: no tab")?;
            let oid = rec[..tab]
                .split(|&b| b == b' ')
                .nth(1)
                .ok_or("ls-files: no oid")?;
            head.index.push(Entry {
                path: rec[tab + 1..].to_vec(),
                id: parse_hex(oid)?,
            });
        }
        Ok(())
    }

    /// porcelain v2: `1 XY sub mH mI mW hH hI path`, `u XY sub m1 m2 m3 mW h1 h2 h3 path`, `? path`
    fn parse_status(out: &[u8], head: &mut Head) -> Res<()> {
        for rec in out.split(|&b| b == 0).filter(|r| !r.is_empty()) {
            match rec[0] {
                b'1' => {
                    let path = rec
                        .splitn(9, |&b| b == b' ')
                        .nth(8)
                        .ok_or("status: short record")?;
                    match rec[3] {
                        b'M' | b'T' => head.modified.push(path.to_vec()),
                        b'D' => head.deleted.push(path.to_vec()),
                        _ => {}
                    }
                }
                b'u' => {
                    let path = rec
                        .splitn(11, |&b| b == b' ')
                        .nth(10)
                        .ok_or("status: short record")?;
                    head.conflicted.push(path.to_vec());
                }
                b'?' => head.untracked.push(rec[2..].to_vec()),
                _ => {}
            }
        }
        Ok(())
    }

    /// `git merge-base HEAD <rev>`, then `git ls-tree -r -z --full-tree <merge-base>`:
    /// `<mode> SP <type> SP <oid> TAB <path>\0`
    fn base_from(repo: &Path, merge_base: Child) -> Res<Base> {
        let mb = finish(merge_base, "git merge-base")?;
        let mb_hex = std::str::from_utf8(&mb)?.trim().to_owned();
        let merge_base = parse_hex(mb_hex.as_bytes())?;
        let out = finish(
            spawn(repo, &["ls-tree", "-r", "-z", "--full-tree", &mb_hex])?,
            "git ls-tree",
        )?;
        let mut tree = Vec::new();
        for rec in out.split(|&b| b == 0).filter(|r| !r.is_empty()) {
            let tab = rec
                .iter()
                .position(|&b| b == b'\t')
                .ok_or("ls-tree: no tab")?;
            let oid = rec[..tab]
                .split(|&b| b == b' ')
                .nth(2)
                .ok_or("ls-tree: no oid")?;
            tree.push(Entry {
                path: rec[tab + 1..].to_vec(),
                id: parse_hex(oid)?,
            });
        }
        Ok(Base { merge_base, tree })
    }

    /// `git cat-file --batch`: ids on stdin, `<oid> <type> <size>\n<contents>\n` on stdout.
    fn read_batch(mut child: Child, ids: &[Oid]) -> Res<Blobs> {
        let mut stdin = child.stdin.take().unwrap();
        let mut input = Vec::with_capacity(ids.len() * 41);
        for id in ids {
            input.extend_from_slice(hex(id).as_bytes());
            input.push(b'\n');
        }
        // Feed ids from another thread so a full stdout pipe cannot deadlock us.
        let writer = std::thread::spawn(move || stdin.write_all(&input));
        let mut out = BufReader::with_capacity(1 << 16, child.stdout.take().unwrap());
        let mut blobs = Blobs::default();
        let mut line = Vec::new();
        let mut data = Vec::new();
        for id in ids {
            line.clear();
            out.read_until(b'\n', &mut line)?;
            let mut parts = line.trim_ascii_end().split(|&b| b == b' ');
            let (_, kind, size) = (parts.next(), parts.next(), parts.next());
            if kind != Some(b"blob") {
                return Err(
                    format!("cat-file: unexpected {:?}", String::from_utf8_lossy(&line)).into(),
                );
            }
            let size: usize = std::str::from_utf8(size.unwrap())?.parse()?;
            data.resize(size + 1, 0);
            out.read_exact(&mut data)?;
            blobs.add(id, &data[..size]);
        }
        writer.join().map_err(|_| "writer thread panicked")??;
        drop(out);
        if !child.wait()?.success() {
            return Err("git cat-file failed".into());
        }
        Ok(blobs)
    }

    fn spawn_batch(repo: &Path) -> Res<Child> {
        Ok(git(repo, &["cat-file", "--batch"])
            .stdin(Stdio::piped())
            .spawn()?)
    }

    /// Step 1: `ls-files` and `status` run at the same time.
    pub fn head(repo: &Path) -> Res<Head> {
        let ls = spawn(repo, LS_FILES)?;
        let st = finish_bg(spawn(repo, STATUS)?, "git status");
        let mut head = Head::default();
        parse_ls_files(&finish(ls, "git ls-files")?, &mut head)?;
        parse_status(&join(st)?, &mut head)?;
        Ok(head)
    }

    /// Step 2: `merge-base`, then `ls-tree`.
    pub fn base(repo: &Path, rev: &str) -> Res<Base> {
        base_from(repo, spawn(repo, &["merge-base", "HEAD", rev])?)
    }

    /// Step 3: one `cat-file --batch`.
    pub fn blobs(repo: &Path, ids: &[Oid]) -> Res<Blobs> {
        read_batch(spawn_batch(repo)?, ids)
    }

    /// Steps 1 to 3 with every process started as early as its input allows:
    /// `ls-files`, `status`, `merge-base` and `cat-file` start at once, `ls-tree` starts when
    /// `merge-base` ends, and `cat-file` gets its ids when head and base are known.
    /// Returns the time until head and base were both known.
    pub fn pipelined(
        repo: &Path,
        rev: &str,
        which: &str,
    ) -> Res<(Head, Base, Vec<Oid>, Blobs, f64)> {
        let t = Instant::now();
        let ls = spawn(repo, LS_FILES)?;
        let st = finish_bg(spawn(repo, STATUS)?, "git status");
        let mb = spawn(repo, &["merge-base", "HEAD", rev])?;
        let batch = spawn_batch(repo)?;
        let repo_owned = repo.to_path_buf();
        let base = std::thread::spawn(move || base_from(&repo_owned, mb));
        let mut head = Head::default();
        parse_ls_files(&finish(ls, "git ls-files")?, &mut head)?;
        parse_status(&join(st)?, &mut head)?;
        let base = join(base)?;
        let known_ms = ms(t);
        let ids = select_blobs(&head, &base, which);
        let blobs = read_batch(batch, &ids)?;
        Ok((head, base, ids, blobs, known_ms))
    }
}

#[cfg(feature = "gix")]
mod gx {
    //! In-process access through the gix crate.
    use super::*;
    use gix::status::index_worktree::Item;
    use gix::status::plumbing::index_as_worktree::{Change, EntryStatus};

    fn oid(id: &gix::ObjectId) -> Oid {
        id.as_slice().try_into().expect("sha1 object id")
    }

    pub fn open(repo: &std::path::Path) -> Res<gix::Repository> {
        Ok(gix::open(repo)?)
    }

    pub fn head(repo: &gix::Repository) -> Res<Head> {
        let index = repo.index_or_empty()?;
        let mut head = Head::default();
        for e in index.entries() {
            head.index.push(Entry {
                path: e.path(&index).to_vec(),
                id: oid(&e.id),
            });
        }
        let iter = repo
            .status(gix::progress::Discard)?
            .index(gix::worktree::IndexPersistedOrInMemory::Persisted(
                index.clone(),
            ))
            .untracked_files(gix::status::UntrackedFiles::Files)
            .index_worktree_rewrites(None)
            .into_index_worktree_iter(Vec::new())?;
        for item in iter {
            match item? {
                Item::Modification {
                    rela_path, status, ..
                } => match status {
                    EntryStatus::Change(Change::Removed) => head.deleted.push(rela_path.into()),
                    EntryStatus::Change(_) => head.modified.push(rela_path.into()),
                    EntryStatus::Conflict { .. } => head.conflicted.push(rela_path.into()),
                    EntryStatus::NeedsUpdate(_) | EntryStatus::IntentToAdd => {}
                },
                Item::DirectoryContents { entry, .. } => {
                    if entry.status == gix::dir::entry::Status::Untracked {
                        head.untracked.push(entry.rela_path.into());
                    }
                }
                Item::Rewrite { .. } => return Err("unexpected rewrite with renames off".into()),
            }
        }
        // Conflicted paths repeat once per stage in the status stream; git prints them once.
        head.conflicted.sort_unstable();
        head.conflicted.dedup();
        Ok(head)
    }

    pub fn base(repo: &gix::Repository, rev: &str) -> Res<Base> {
        let head_id = repo.head_id()?;
        let base_id = repo.rev_parse_single(format!("{rev}^{{commit}}").as_str())?;
        let mb = repo.merge_base(head_id, base_id)?.ok_or("no merge base")?;
        let tree = repo.find_commit(mb)?.tree()?;
        let mut rec = gix::traverse::tree::Recorder::default();
        tree.traverse().breadthfirst(&mut rec)?;
        let tree = rec
            .records
            .into_iter()
            .filter(|e| !e.mode.is_tree())
            .map(|e| Entry {
                path: e.filepath.into(),
                id: oid(&e.oid),
            })
            .collect();
        Ok(Base {
            merge_base: oid(&mb.detach()),
            tree,
        })
    }

    pub fn blobs(repo: &gix::Repository, ids: &[Oid]) -> Res<Blobs> {
        let mut blobs = Blobs::default();
        for id in ids {
            let obj = repo.find_object(gix::ObjectId::from_bytes_or_panic(id))?;
            if obj.kind != gix::object::Kind::Blob {
                return Err(format!("{} is not a blob", hex(id)).into());
            }
            blobs.add(id, &obj.data);
        }
        Ok(blobs)
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn run(mode: &str, repo: &std::path::Path, rev: &str, which: &str) -> Res<()> {
    let t0 = Instant::now();
    let (open_ms, mut head, head_ms, base, base_ms, ids, blobs, blobs_ms);
    match mode {
        #[cfg(feature = "cli")]
        "cli" => {
            open_ms = 0.0;
            let t = Instant::now();
            head = cli::head(repo)?;
            head_ms = ms(t);
            let t = Instant::now();
            base = cli::base(repo, rev)?;
            base_ms = ms(t);
            let t = Instant::now();
            ids = select_blobs(&head, &base, which);
            blobs = cli::blobs(repo, &ids)?;
            blobs_ms = ms(t);
        }
        #[cfg(feature = "cli")]
        "cli-par" => {
            // Steps overlap: head_ms is the time until head and base are both known.
            open_ms = 0.0;
            base_ms = 0.0;
            let t = Instant::now();
            let known_ms;
            (head, base, ids, blobs, known_ms) = cli::pipelined(repo, rev, which)?;
            head_ms = known_ms;
            blobs_ms = ms(t) - known_ms;
        }
        #[cfg(feature = "gix")]
        "gix" => {
            let t = Instant::now();
            let r = gx::open(repo)?;
            open_ms = ms(t);
            let t = Instant::now();
            head = gx::head(&r)?;
            head_ms = ms(t);
            let t = Instant::now();
            base = gx::base(&r, rev)?;
            base_ms = ms(t);
            let t = Instant::now();
            ids = select_blobs(&head, &base, which);
            blobs = gx::blobs(&r, &ids)?;
            blobs_ms = ms(t);
        }
        _ => return Err(format!("mode {mode:?} is not built into this binary").into()),
    }
    let steps_ms = ms(t0);

    let mut base = base;
    let index_sum = entries_sum(&mut head.index);
    let base_sum = entries_sum(&mut base.tree);
    let mut status_sum = Sum::default();
    for (tag, list) in [
        (&b"M"[..], &mut head.modified),
        (b"D", &mut head.deleted),
        (b"?", &mut head.untracked),
        (b"U", &mut head.conflicted),
    ] {
        status_sum.write(tag);
        status_sum.write(&paths_sum(list).0.to_le_bytes());
    }
    let md_base = base
        .tree
        .iter()
        .filter(|e| e.path.ends_with(b".md"))
        .count();
    println!("index_entries {}", head.index.len());
    println!("index_sum {:016x}", index_sum.0);
    println!("worktree_modified {}", head.modified.len());
    println!("worktree_deleted {}", head.deleted.len());
    println!("untracked {}", head.untracked.len());
    println!("conflicted {}", head.conflicted.len());
    println!("status_sum {:016x}", status_sum.0);
    println!("merge_base {}", hex(&base.merge_base));
    println!("base_entries {}", base.tree.len());
    println!("base_md_entries {md_base}");
    println!("base_sum {:016x}", base_sum.0);
    println!("blobs_selected {} ({which})", ids.len());
    println!("blobs_read {}", blobs.count);
    println!("blob_bytes {}", blobs.bytes);
    println!("blob_sum {:016x}", blobs.sum.0);
    let (rss_self, rss_child) = peak_rss_kib();
    let total_ms = ms(t0);
    eprint!("timing open_ms={open_ms:.3} head_ms={head_ms:.3} base_ms={base_ms:.3} ");
    eprint!("blobs_ms={blobs_ms:.3} steps_ms={steps_ms:.3} total_ms={total_ms:.3} ");
    eprintln!("rss_self_kib={rss_self} rss_child_max_kib={rss_child}");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 5 || !matches!(args[4].as_str(), "changed" | "md") {
        eprintln!("usage: git-access <cli|cli-par|gix> <repo> <base-rev> <changed|md>");
        return ExitCode::from(2);
    }
    match run(&args[1], std::path::Path::new(&args[2]), &args[3], &args[4]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

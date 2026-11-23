"""Run the batch of protocol 2 (ADR 0033) as jobs, with up to 3 jobs at the same time.

Usage: run_batch.py <lab> --tasks <id>,<id>,... --room <usd> [--waves 3] [--concurrency 3]
                    [--prefilled <wave>=<result file> ...] [--claude-version <v>] [--dry-run]
        run_batch.py --selftest

The folder <lab> holds `project/` (the batch project of make_project.py) and `tools/`
(setup_tools.py). The command writes `state.json`, `merged.json` and the job
results in `project/evals/results/<job>/`. It prints one line for each event, so a
person can watch it.

A job is 1 task in 1 wave: `spec-dna eval run --arms <3 arms> <task> --runs 1
--max-total-usd 80 --out evals/results/<job>`. It is the command of the ADR. The
runner of one command runs its arms one after the other and cannot run 1 arm alone,
so the 3 arms of a job do not run at the same time. Up to 3 jobs (3 tasks) run at
the same time, each with its own result file. The jobs start in the order of the
waves: all tasks of wave 1, then wave 2, then wave 3, so a stop by the cap leaves
complete waves. The arms of a job come in the rotation of the ADR: the arm order of
task `i` in wave `w` starts at `(i + w) mod 3`.

The cap. `--room` is the money of the batch: the cap, minus the spend of the probes,
minus the reserve. The rest of the room is the room, minus the spend of the finished
jobs, minus the cap of each job in flight or waiting to run again. A job starts when
the rest is at least 25 USD (the budget of one run). Its cap is 80 USD (3 runs of 25 USD and 5 USD of slack) or the rest, if
that is lower, and the same cap holds when it goes on with `--resume`. A job starts with a cap below 80 USD only when no other job runs or waits to run again. The runner starts
no run that could pass the cap of its job, so a job with a low cap can end with less
than 3 arms. A stop by the cap ends the batch, and only complete waves count. The
spend of a job whose runner left no result file counts as its whole cap.

The limits of time of the ADR (7,200 seconds for the agent and 1,800 for a check) are
set in the job command, because the scrubbed environment drops the variables of the
person.

Before each start, the command checks that the `claude` of `tools/tap.json` has the expected version,
and that no process of another `spec-dna eval run` or `claude -p` of the batch runs.
It waits when one runs. A job that the API stopped (the runner gave up after its
waits), or that a signal stopped, runs again with `--resume <its file>` (no `--out`) after 30 minutes, up
to 4 attempts. A job does not start before the job of the same task in the wave before it is done, so
the tap log of a task and an arm has its runs in the order of the run numbers. The state file keeps a halt: a
new start does nothing until the file is removed. Any other stop ends the command.

Events: `WAVE <n> COMPLETE` when all tasks of a wave are done, `BATCH END <reason>`.
"""

import fcntl
import json
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from merge import merge  # noqa: E402

ARMS = ["none", "lean-no-advisor", "lean"]
JOB_USD = 80.0
RETRY_WAIT = 1800
ATTEMPTS = 4
POLL = 20


def arm_order(task_index: int, wave: int) -> list[str]:
    """The arm list to pass, so that the order in the wave is the rotation of the ADR.

    The runner rotates the list left by `(eval index + wave) mod 3`, and a job has the
    eval index 0 and the wave 1. The order that we want rotates `ARMS` left by
    `(task index + wave) mod 3`. The list that the runner rotates once gives it.
    """
    want = (task_index + wave) % 3
    shift = (want - 1) % 3
    return [ARMS[(shift + m) % 3] for m in range(3)]


def effective_order(passed: list[str], eval_index: int, wave: int) -> list[str]:
    k = (eval_index + wave) % 3
    return [passed[(k + j) % 3] for j in range(3)]


def jobs_of(tasks: list[str], waves: int, prefilled: set[tuple[int, str]]) -> list[dict]:
    jobs = []
    for wave in range(1, waves + 1):
        for i, task in enumerate(tasks):
            if (wave, task) in prefilled:
                continue
            jobs.append(
                {
                    "name": f"w{wave}-{task}",
                    "wave": wave,
                    "task": task,
                    "arms": arm_order(i, wave),
                    "status": "pending",
                    "attempts": 0,
                    "retry_at": 0,
                }
            )
    return jobs


MIN_START = 25.0
LIMIT_ENV = ["SPEC_DNA_EVAL_TIMEOUT=7200", "SPEC_DNA_EVAL_CHECK_TIMEOUT=1800"]


def job_cap(remainder: float) -> float | None:
    """The cap of a new job: 80 USD, or the rest of the room. None if a run could pass the room.

    80 USD is 3 runs of 25 USD and 5 USD of slack: a run can pass its budget a little.
    """
    if remainder < MIN_START - 1e-9:
        return None
    return min(JOB_USD, remainder)


def newest_result(folder: Path) -> Path | None:
    files = sorted(folder.glob("arms-*.json"), key=lambda p: p.stat().st_mtime)
    return files[-1] if files else None


def read_result(folder: Path) -> dict | None:
    path = newest_result(folder)
    if path is None:
        return None
    try:
        return json.loads(path.read_text())
    except ValueError:
        return None


def job_complete(result: dict | None) -> bool:
    if not result or result.get("stopped"):
        return False
    sides = [s for item in result["evals"] for res in item["results"] for s in res["arms"].values()]
    return len(sides) == 3 and all(s is not None for s in sides)


def descendants(pid: int, table: dict[int, int]) -> set[int]:
    out = {pid}
    grew = True
    while grew:
        grew = False
        for child, parent in table.items():
            if parent in out and child not in out:
                out.add(child)
                grew = True
    return out


def foreign_processes(mine: set[int]) -> list[str]:
    """Lines of other `spec-dna eval run` or `claude -p` processes of a batch."""
    text = subprocess.run(["ps", "-axo", "pid=,ppid=,command="], capture_output=True, text=True).stdout
    table: dict[int, int] = {}
    lines: dict[int, str] = {}
    for raw in text.splitlines():
        m = re.match(r"\s*(\d+)\s+(\d+)\s+(.*)", raw)
        if m:
            table[int(m.group(1))] = int(m.group(2))
            lines[int(m.group(1))] = m.group(3)
    own: set[int] = {os.getpid()}
    for pid in mine:
        own |= descendants(pid, table)
    own |= descendants(os.getpid(), table)
    found = []
    for pid, command in lines.items():
        if pid in own:
            continue
        if re.search(r"spec-dna eval run", command) or (
            re.search(r"(^|/)claude\b.* -p ", command) and "Implement the change" in command
        ):
            found.append(f"{pid} {command[:60]}")
    return found


def claude_version(path: str) -> str:
    out = subprocess.run([path, "--version"], capture_output=True, text=True).stdout.strip()
    return out.split()[0] if out else ""


def alive(pid: int | None) -> bool:
    if not pid:
        return False
    try:
        os.kill(pid, 0)
    except ProcessLookupError:
        return False
    except PermissionError:
        return True
    return True


class Batch:
    def __init__(self, lab: Path, tasks: list[str], room: float, waves: int, concurrency: int, prefilled: dict[tuple[int, str], Path], claude: str | None, runner: list[str] | None, poll: float, check_env: bool = True):
        self.lab, self.tasks, self.room, self.waves, self.concurrency = lab, tasks, room, waves, concurrency
        self.prefilled = prefilled
        self.claude = claude
        self.runner = runner
        self.poll = poll
        self.check_env = check_env
        self.state_file = lab / "state.json"
        self.procs: dict[str, subprocess.Popen] = {}
        self.stop = False
        self.halted: str | None = None
        self.announced: set[int] = set()
        self.waiting_note = ""
        self.lock = open(lab / "lock", "w")
        try:
            fcntl.flock(self.lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        except OSError:
            raise SystemExit("run_batch: another scheduler holds the lock of this lab")
        if self.state_file.is_file():
            state = json.loads(self.state_file.read_text())
            self.jobs = state["jobs"]
            self.halted = state.get("halted")
        else:
            self.jobs = jobs_of(tasks, waves, set(prefilled))
        for job in self.jobs:
            if job["status"] != "running":
                continue
            # A runner has its own session, so it can outlive a scheduler that was killed.
            # Never start a second runner for the same job.
            if alive(job.get("pid")):
                raise SystemExit(f"run_batch: the runner {job['pid']} of {job['name']} still runs; wait for it, then start again")
            result = read_result(self.folder(job))
            if job_complete(result):
                job["status"] = "done"
            elif result is not None:
                job["status"] = "retry"
                job["retry_at"] = 0
            else:
                # No file: the runner left no spend that we can read, so the whole cap counts as spent.
                job["lost_usd"] = job.get("lost_usd", 0.0) + job.get("cap", JOB_USD)
                job["status"] = "pending"
                job.pop("cap", None)

    def say(self, text: str) -> None:
        print(f"{time.strftime('%Y-%m-%dT%H:%M:%S')} {text}", flush=True)

    def folder(self, job: dict) -> Path:
        return self.lab / "project" / "evals" / "results" / job["name"]

    def save(self) -> None:
        tmp = self.state_file.with_suffix(".tmp")
        tmp.write_text(json.dumps({"jobs": self.jobs, "halted": self.halted}, indent=2) + "\n")
        tmp.replace(self.state_file)

    def committed(self, job: dict) -> float:
        """The money that a job holds: its spend if it is finished, else its whole cap."""
        lost = job.get("lost_usd", 0.0)
        if job.get("uncertain"):
            # The spend of an attempt that a signal stopped is not in the result file.
            return lost + job.get("cap", JOB_USD)
        if job["status"] in ("done", "stopped"):
            result = read_result(self.folder(job))
            if result is None:
                return lost + job.get("cap", 0.0)
            return lost + (result.get("spend_usd") or 0.0)
        if job["status"] in ("running", "retry"):
            return lost + job.get("cap", JOB_USD)
        return lost

    def remainder(self) -> float:
        return self.room - sum(self.committed(j) for j in self.jobs)

    def command(self, job: dict) -> list[str]:
        out = f"evals/results/{job['name']}"
        base = [
            "eval", "run", "--arms", ",".join(job["arms"]), job["task"], "--runs", "1",
            "--max-total-usd", f"{job['cap']:g}",
        ]
        if job["status"] == "retry":
            # The runner goes on in its own file, and it takes a path from the root of the project, and no --out.
            base += ["--resume", f"{out}/{newest_result(self.folder(job)).name}"]
        else:
            base += ["--out", out]
        if self.runner is not None:
            return [*self.runner, *base]
        tools = self.lab / "tools"
        return [
            "sh", str(HERE.parent / "team" / "run_scrubbed.sh"), str(tools), str(self.lab / "project"),
            "env", *LIMIT_ENV, "spec-dna", *base,
        ]

    def start(self, job: dict) -> None:
        log = open(self.lab / f"{job['name']}.log", "ab")
        job["attempts"] += 1
        before = newest_result(self.folder(job))
        job["stamp"] = [before.name, before.stat().st_mtime] if before else None
        proc = subprocess.Popen(self.command(job), stdout=log, stderr=log, start_new_session=True)
        self.procs[job["name"]] = proc
        job["status"] = "running"
        job["pid"] = proc.pid
        job["started"] = time.strftime("%Y-%m-%dT%H:%M:%S")
        self.save()
        self.say(f"JOB start {job['name']} arms {','.join(job['arms'])} cap {job['cap']:g} USD attempt {job['attempts']} pid {proc.pid}")

    def reap(self) -> None:
        for name, proc in list(self.procs.items()):
            code = proc.poll()
            if code is None:
                continue
            del self.procs[name]
            job = next(j for j in self.jobs if j["name"] == name)
            result = read_result(self.folder(job))
            version = (result or {}).get("claude_code") or ""
            if self.claude and result is not None and not version.startswith(self.claude):
                self.halted = f"job {name} ran claude {version}, expected {self.claude}"
            if code == 0 and job_complete(result):
                job["status"] = "done"
                spend = result.get("spend_usd") or 0.0
                self.say(f"JOB done {name} spend {spend:.2f} USD")
            else:
                stopped = (result or {}).get("stopped")
                self.say(f"JOB stop {name} exit {code} stopped {stopped}")
                refused = stopped in ("api-error", "interrupted") or code in (129, 130, 143)
                if stopped == "interrupted" or code in (129, 130, 143):
                    job["uncertain"] = True
                after = newest_result(self.folder(job))
                changed = after is not None and job.get("stamp") != [after.name, after.stat().st_mtime]
                # A usage error (exit 2) leaves the file as it was. Such a job does not run again.
                if result is not None and refused and changed and job["attempts"] < ATTEMPTS and not self.halted:
                    job["status"] = "retry"
                    job["retry_at"] = time.time() + RETRY_WAIT
                    self.say(f"JOB retry {name} in {RETRY_WAIT} s with --resume")
                else:
                    job["status"] = "stopped"
                    self.halted = self.halted or f"job {name} stopped (exit {code}, stopped {stopped})"
            self.save()
            self.after_job()

    def after_job(self) -> None:
        for wave in range(1, self.waves + 1):
            mine = [j for j in self.jobs if j["wave"] == wave]
            if wave in self.announced:
                continue
            if all(j["status"] == "done" for j in mine):
                self.announced.add(wave)
                path = self.merge_all()
                self.say(f"WAVE {wave} COMPLETE {path}")

    def merge_all(self) -> Path:
        parts = [(w, json.loads(p.read_text())) for w, p in sorted({(w, p) for (w, _), p in self.prefilled.items()})]
        for job in self.jobs:
            result = read_result(self.folder(job))
            if result is not None and job["status"] in ("done", "stopped", "retry"):
                parts.append((job["wave"], result))
        merged = merge(parts, self.tasks)
        path = self.lab / "merged.json"
        path.write_text(json.dumps(merged, indent=2) + "\n")
        return path

    def environment_ok(self) -> bool:
        if not self.check_env:
            return True
        if self.claude:
            real = json.loads((self.lab / "tools" / "tap.json").read_text())["real"]
            found = claude_version(real)
            if found != self.claude:
                note = f"claude is {found}, expected {self.claude}"
                if note != self.waiting_note:
                    self.say(f"WAIT {note}")
                    self.waiting_note = note
                return False
        other = foreign_processes({p.pid for p in self.procs.values()})
        if other:
            note = f"another process runs: {other[0]}"
            if note != self.waiting_note:
                self.say(f"WAIT {note}")
                self.waiting_note = note
            return False
        self.waiting_note = ""
        return True

    def next_job(self) -> dict | None:
        now = time.time()
        for status in ("retry", "pending"):
            for job in self.jobs:
                if job["status"] != status or (status == "retry" and job["retry_at"] > now):
                    continue
                before = [j for j in self.jobs if j["task"] == job["task"] and j["wave"] < job["wave"]]
                if any(j["status"] != "done" for j in before):
                    continue
                return job
        return None

    def run(self) -> str:
        self.say(f"BATCH start {len(self.jobs)} jobs, room {self.room:.2f} USD, concurrency {self.concurrency}")
        self.after_job()
        while True:
            self.reap()
            if self.stop:
                for proc in self.procs.values():
                    proc.send_signal(signal.SIGTERM)
                for proc in self.procs.values():
                    try:
                        proc.wait(timeout=180)
                    except subprocess.TimeoutExpired:
                        pass
                self.reap()
                return "signal"
            pending = [j for j in self.jobs if j["status"] in ("pending", "retry")]
            if self.halted:
                if not self.procs:
                    return self.halted
            elif not pending and not self.procs:
                return "all jobs done"
            elif pending and len(self.procs) < self.concurrency:
                job = self.next_job()
                if job is not None:
                    # A job that goes on with --resume keeps its cap. A new job gets 75 USD or the rest.
                    waiting = any(j["status"] == "retry" for j in self.jobs)
                    rest = self.remainder()
                    if job["status"] == "retry":
                        cap = job.get("cap")
                    elif rest >= JOB_USD or (not self.procs and not waiting):
                        cap = job_cap(rest)
                    else:
                        cap = -1.0  # wait: the jobs in flight can free money
                    if cap is None:
                        return f"the cap: the rest of the room, {rest:.2f} USD, is below 25 USD"
                    elif cap > 0 and self.environment_ok():
                        job["cap"] = cap
                        self.start(job)
                        continue
            time.sleep(self.poll)


def selftest() -> int:
    import tempfile

    ok = True

    def expect(name: str, got: object, want: object) -> None:
        nonlocal ok
        good = got == want
        print(f"{'ok  ' if good else 'FAIL'} {name}: {got}")
        ok = ok and good

    # The rotation of the ADR: the order of task i in wave w starts at (i + w) mod 3.
    for i in range(5):
        for w in (1, 2, 3):
            got = effective_order(arm_order(i, w), 0, 1)
            want = [ARMS[((i + w) + j) % 3] for j in range(3)]
            if got != want:
                expect(f"rotation task {i} wave {w}", got, want)
    expect("rotation of task 0 in wave 1", effective_order(arm_order(0, 1), 0, 1), ["lean-no-advisor", "lean", "none"])
    expect("jobs are in the order of the waves", [j["name"] for j in jobs_of(["a", "b"], 2, set())], ["w1-a", "w1-b", "w2-a", "w2-b"])
    expect("a prefilled pair has no job", [j["name"] for j in jobs_of(["a", "b"], 2, {(1, "b")})], ["w1-a", "w2-a", "w2-b"])
    expect("a job gets 80 USD", job_cap(649.29), 80.0)
    expect("a job gets the rest of the room", job_cap(40.0), 40.0)
    expect("a job needs 25 USD at least", job_cap(24.9), None)

    fake = Path(tmp := tempfile.mkdtemp()) / "fake_runner.py"
    fake.write_text(
        "import json, sys, time\n"
        "from pathlib import Path\n"
        "a = sys.argv[1:]\n"
        "out = Path('project/evals/results') / a[a.index('--out') + 1].split('/')[-1]\n"
        "out.mkdir(parents=True, exist_ok=True)\n"
        "time.sleep(0.3)\n"
        "arms = a[a.index('--arms') + 1].split(',')\n"
        "task = a[a.index('--arms') + 2]\n"
        "sides = {x: {'subtype': 'success', 'cost_usd': 20.0, 'checks': []} for x in arms}\n"
        "res = {'spec_dna': '0', 'claude_code': '0', 'models': [], 'started': '2026-11-24T00:00:00Z', 'arms': arms, 'runs': 1,\n"
        "       'spend_usd': 60.0, 'excluded': {x: 0 for x in arms}, 'stopped': None, 'waves': 1,\n"
        "       'evals': [{'id': task, 'runs': 1, 'max_budget_usd': 25, 'results': [{'run': 1, 'arms': sides}]}]}\n"
        "(out / 'arms-1.json').write_text(json.dumps(res))\n"
    )
    lab = Path(tempfile.mkdtemp())
    (lab / "project").mkdir()
    cwd = os.getcwd()
    os.chdir(lab)
    try:
        # A room of 400 USD: jobs of 60 USD each, 75 USD reserved for each job in flight.
        batch = Batch(lab, ["a", "b", "c", "d"], 400.0, 3, 3, {}, None, [sys.executable, str(fake)], 0.05, check_env=False)
        end = batch.run()
    finally:
        os.chdir(cwd)
    names = [j["name"] for j in batch.jobs if j["status"] == "done"]
    expect("jobs done in the order of the waves", names[:4], ["w1-a", "w1-b", "w1-c", "w1-d"])
    expect("the cap stops the batch", end.startswith("the cap"), True)
    # 4 jobs of 60 USD = 240 USD after wave 1. The fake runner ignores its cap. Wave 2: 2 jobs hold 75 USD each
    # (rest 10 USD), the third waits until a job ends at 60 USD (rest 25 USD) and then gets a cap of 25 USD.
    # The next job finds a rest below 25 USD, and the batch ends.
    expect("jobs done before the cap", len(names), 7)
    expect("wave 1 is complete in the merged file", json.loads((lab / "merged.json").read_text())["waves"], 1)
    # Restart: a job that was running, whose runner is dead and left no file, counts its whole cap as spent.
    lab2 = Path(tempfile.mkdtemp())
    (lab2 / "project").mkdir()
    (lab2 / "state.json").write_text(json.dumps({"jobs": [
        {"name": "w1-a", "wave": 1, "task": "a", "arms": ARMS, "status": "running", "attempts": 1, "retry_at": 0, "pid": 0, "cap": 75.0},
        {"name": "w1-b", "wave": 1, "task": "b", "arms": ARMS, "status": "pending", "attempts": 0, "retry_at": 0},
    ]}))
    again = Batch(lab2, ["a", "b"], 200.0, 1, 3, {}, None, [sys.executable, str(fake)], 0.05, check_env=False)
    expect("a lost job counts its cap", (again.jobs[0]["status"], again.remainder()), ("pending", 125.0))
    expect("a live runner stops the restart", alive(os.getpid()), True)
    # A job that goes on: the runner takes --resume with a path from the root of the project, and no --out.
    again.jobs[0].update({"status": "retry", "cap": 80.0})
    folder = again.folder(again.jobs[0])
    folder.mkdir(parents=True)
    (folder / "arms-1.json").write_text("{}")
    cmd = again.command(again.jobs[0])
    expect("resume has no --out", "--out" in cmd, False)
    expect("resume path is relative", cmd[cmd.index("--resume") + 1], "evals/results/w1-a/arms-1.json")
    # An attempt that a signal stopped counts its whole cap, also when the job is done.
    again.jobs[0].update({"status": "done", "uncertain": True})
    # 75 USD lost before, and the cap of 80 USD now.
    expect("an uncertain job counts its cap", again.committed(again.jobs[0]), 155.0)
    # A halt stays after a restart.
    again.halted = "job w1-a stopped"
    again.save()
    lab3 = lab2
    again.lock.close()
    third = Batch(lab3, ["a", "b"], 200.0, 1, 3, {}, None, [sys.executable, str(fake)], 0.05, check_env=False)
    expect("a halt is kept", third.halted, "job w1-a stopped")
    # The job of a wave waits for the job of the same task in the wave before it.
    third.jobs = jobs_of(["a"], 2, set())
    third.jobs[0]["status"] = "retry"
    third.jobs[0]["retry_at"] = 1e18
    expect("no job of wave 2 before wave 1 is done", third.next_job(), None)
    return 0 if ok else 1


def main() -> int:
    args = sys.argv[1:]
    if args == ["--selftest"]:
        return selftest()
    if not args or "--tasks" not in args or "--room" not in args:
        sys.exit(__doc__)

    def opt(name: str, default: str | None = None) -> str | None:
        return args[args.index(name) + 1] if name in args else default

    lab = Path(args[0]).resolve()
    tasks = opt("--tasks").split(",")
    prefilled = {}
    for i, a in enumerate(args):
        if a == "--prefilled":
            wave, name = args[i + 1].split("=", 1)
            result = json.loads(Path(name).read_text())
            for item in result["evals"]:
                prefilled[(int(wave), item["id"])] = Path(name)
    batch = Batch(
        lab, tasks, float(opt("--room")), int(opt("--waves", "3")), int(opt("--concurrency", "3")),
        prefilled, opt("--claude-version"), None, POLL,
    )
    if "--dry-run" in args:
        for job in batch.jobs:
            job.setdefault("cap", JOB_USD)
            print(job["name"], ",".join(job["arms"]), " ".join(batch.command(job)[-9:]))
        return 0

    def handler(signum: int, frame: object) -> None:
        batch.stop = True

    signal.signal(signal.SIGTERM, handler)
    signal.signal(signal.SIGINT, handler)
    end = batch.run()
    batch.merge_all()
    batch.say(f"BATCH END {end}")
    return 0 if end == "all jobs done" else 1


if __name__ == "__main__":
    sys.exit(main())

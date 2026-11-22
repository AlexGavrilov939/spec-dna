# Example: one change, from the intent to finish

This example takes one change through the change process of spec-dna: the intent, the spec with its delta, the plan, and the code with `finish`. Each step is a pull request that GitHub merges as one squashed commit. Read it to see what an agent and a reviewer do in each step.

## Run it

```sh
examples/one-change/run.sh
```

You need `spec-dna` on your `PATH`, git 2.32 or later, bash and perl. [The list of examples](../README.md#run-an-example) gives the details. The script makes a bare repository as the origin, so `check`, `next` and `finish` have a base, as in a pull request.

The project uses the default change policy of `init`: `three-class` with the gate `block`. The change adds a requirement: when a moisture sensor gives no value for 60 seconds, the controller closes the valve of its bed. The owner chose the class `higher-risk`, so the change has a spec that the owner approves.

## 1. The intent

```console
$ spec-dna new intent sensor-timeout
wrote docs/intent/sensor-timeout/intent.md (intent template); replace each {{...}} placeholder
```

The intent says what the change gives and why, in plain words. The owner sets `status: accepted`, and the pull request merges.

## 2. The spec and its delta

`next` gives the checklist to the next gate:

```console
$ spec-dna next sensor-timeout
change sensor-timeout (higher-risk), stage design
base ee7cb62bc0bd
[x] the intent is accepted [skill: brainstorm]
[ ] write `docs/intent/sensor-timeout/spec.md` with `spec-dna new spec sensor-timeout` [skill: specify]
```

The `Delta` section of the spec lists each requirement that the change adds, changes or removes:

```md
- ADD requirements/valves#sensor-timeout: When a moisture sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed. The check is in `src/valves.py`.
```

Each concern has a resolution, and each open question has an answer, before the owner approves the spec:

```md
- [x] A slow sensor can close a valve that must stay open. Resolution: the limit of 60 seconds is 4 times the slowest read in the logs.
```

## 3. The plan

`plan.md` lists the files of the change, the tasks, the risks and the proof. `check` compares the files of each later pull request with this list (SD913):

```md
- [ ] t1: Close the valve when the sensor times out. Files: `src/valves.py`. Verify: the test passes.
```

## 4. The code and finish

The agent writes the code and a test. It marks the tasks done. `next` shows that a requirement of the delta has no test yet:

```console
$ spec-dna next sensor-timeout
change sensor-timeout (higher-risk), stage test
...
[x] a file of `Files` differs from the base [skill: execute]
[ ] 1 of 1 requirement(s) that the change adds or changes have no evidence [skill: tdd]
[ ] add `covers: requirements/valves#sensor-timeout` in a test file [skill: tdd]
```

The agent adds `# covers: requirements/valves#sensor-timeout` to the test and commits it. The change is `higher-risk`, so `finish` needs an acceptance record before it applies the delta. Without the record, it refuses the change and writes nothing:

```console
$ spec-dna finish sensor-timeout
docs/intent/sensor-timeout/acceptance.toml:1: error SD927 acceptance-missing: the change `sensor-timeout` needs an acceptance record, and `docs/intent/sensor-timeout/acceptance.toml` is absent; start an agent `acceptor` with the prompt that `spec-dna next` prints, `cd '<root>' && spec-dna accept sensor-timeout --brief`, and commit the record that it writes
...
spec-dna: finish `sensor-timeout` stopped at step 5: the change needs an acceptance record, and the folder has none; nothing changed
```

A second agent, the acceptor, did not write the code. It runs the brief command and reads the brief. Then it writes the skeleton of the record:

```console
$ spec-dna accept sensor-timeout --brief | sed -n 1p
spec-dna-brief 2 sensor-timeout acceptor

$ spec-dna accept sensor-timeout --session s1
wrote docs/intent/sensor-timeout/acceptance.toml
acceptor: 1
next: copy the hash of the last line of your brief into `brief` of your entry, replace each `pending` value, then commit the record alone
```

The acceptor copies the hash of the last line of the brief into `brief`, and it replaces each `pending` value. It tries to break the change, and it adds what it tried and what it found. The script stands in for the acceptor and writes this part of the record:

```toml
[[attempt]]
acceptor = 1
text = "Read `on_sensor_timeout` in `src/valves.py`: it calls `close_valve` for the bed, and `close_valve` sets `valve_open` to false, as the requirement says."
outcome = "held"

[[finding]]
acceptor = 1
severity = "note"
path = "tests/test_valves.py"
text = "The test has no body yet."
```

The acceptor commits the record alone. [The change guide](../../docs/guides/change.md#4-write-the-code-and-the-tests) gives the rules of the record. Now `finish` applies the delta to the requirements, moves the record to `.spec-dna/accepted/`, and deletes the folder of the change:

```console
$ spec-dna finish sensor-timeout
wrote .spec-dna/accepted/sensor-timeout.toml
wrote docs/requirements/valves.md
removed docs/intent/sensor-timeout/acceptance.toml
removed docs/intent/sensor-timeout/intent.md
removed docs/intent/sensor-timeout/plan.md
removed docs/intent/sensor-timeout/spec.md
...
```

```console
$ git diff --cached -U0 -- docs/requirements
...
@@ -8,0 +9 @@ status: active
+- **sensor-timeout**: When a moisture sensor gives no value for 60 seconds, the controller SHALL close the valve of its bed. The check is in `src/valves.py`.
```

The reviewer sees the code and the new requirement in one diff. The proposal does not stay in the tree, so a later agent cannot read it as the current truth. git keeps the history:

```console
$ spec-dna log
2026-11-16 finished sensor-timeout higher-risk add 1 change 0 remove 0 hours - actual - 8c96d1b11e0d Sensor timeout
```

A `routine` change has no spec and no delta: it edits the requirements together with the code. [The change guide](../../docs/guides/change.md) gives each class and each step.

## If it fails

| Message | Cause | Fix |
| --- | --- | --- |
| `note: no base for the change checks` | The repository has no `origin/HEAD` | Run `git remote set-head origin <branch>`, give `--base <rev>`, or set `[change] base` |
| `SD916 delta-evidence-missing` | A requirement of the delta has no test | Add `covers: <requirement id>` in a test file |
| `SD927 acceptance-missing` | A `higher-risk` change has no acceptance record | Start an agent `acceptor` with the prompt that `next` prints, and commit the record that it writes |
| `SD913` for a file | The pull request changes a file that the plan does not list | Add the file to `Files` of the plan in the same pull request |
| The output differs from this page | Your binary has another version (`spec-dna --version`) | Install the current release |

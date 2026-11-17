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

`finish` refuses the change and writes nothing:

```console
$ spec-dna finish sensor-timeout
docs/requirements/valves.md:9: error SD916 delta-evidence-missing: the change `sensor-timeout` adds or changes the requirement `requirements/valves#sensor-timeout`, and it has no `covers:` tag and no `verified-by`
...
spec-dna: finish `sensor-timeout` stopped at step 5: a requirement that the change adds or changes has no evidence (the lines are lines of the result of `finish`); nothing changed
```

The agent adds `# covers: requirements/valves#sensor-timeout` to the test. Now `finish` applies the delta to the requirements and deletes the folder of the change:

```console
$ spec-dna finish sensor-timeout
wrote docs/requirements/valves.md
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
2026-11-16 finished sensor-timeout higher-risk add 1 change 0 remove 0 9e72abc626e6 Sensor timeout
```

A `routine` change has no spec and no delta: it edits the requirements together with the code. [The change guide](../../docs/guides/change.md) gives each class and each step.

## If it fails

| Message | Cause | Fix |
| --- | --- | --- |
| `note: no base for the change checks` | The repository has no `origin/HEAD` | Run `git remote set-head origin <branch>`, give `--base <rev>`, or set `[change] base` |
| `SD916 delta-evidence-missing` | A requirement of the delta has no test | Add `covers: <requirement id>` in a test file |
| `SD913` for a file | The pull request changes a file that the plan does not list | Add the file to `Files` of the plan in the same pull request |
| The output differs from this page | Your binary has another version (`spec-dna --version`) | Install the current release |

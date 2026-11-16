# Examples

Each example is a script. It makes a scratch git repository, writes a few files, and runs the real `spec-dna` commands. The page of each example shows the output and explains each step. A test (`tests/examples.rs`) runs each script and compares the output with the page, so the pages show the output of the current version.

| Example | It shows | Run time |
| --- | --- | --- |
| [A first project](first-project/README.md) | `init`, a requirement that cites the code, `context`, and a `check` that stops a rewrite of an accepted decision | About 1 second |
| [An existing repository](existing-repo/README.md) | Folders that keep their names, a baseline of old problems, and a gate that reports only the new problems | About 1 second |
| [One change, from the intent to `finish`](one-change/README.md) | 4 pull requests: the intent, the spec with its delta, the plan, and the code with `finish` | Less than 10 seconds |

The run times are from an Apple silicon laptop.

## Run an example

You need `spec-dna` on your `PATH` ([install](../README.md#try-it-in-1-minute)), git 2.32 or later, bash and perl. Run the script from a clone of this repository:

```sh
git clone --depth 1 https://github.com/AlexGavrilov939/spec-dna
spec-dna/examples/first-project/run.sh
```

The script writes only in a new temporary directory. Its first line gives the path, so you can open the repository and continue there. To choose the place, give an empty directory: `run.sh ~/try-spec-dna`.

The scripts ignore your git configuration and give each commit a fixed date. Thus the output is the same on each run. The commit ids are also the same when your binary has the version of the page.

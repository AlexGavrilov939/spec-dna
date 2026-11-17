//! The 20 evals of the pilot: 5 for each of `execute`, `tdd`, `verify` and
//! `debug` (design D32 to D36, ADR 0016). `eval_pack.rs` holds the writer, and
//! this file holds the evals and their fixtures, so the pack module stays short.
//!
//! Each fixture is a small Python project. It runs with `sh test.sh`, which
//! needs only `python3` and its `unittest` module. The fixtures have no
//! `AGENTS.md` that names a skill. A project `AGENTS.md` replaces the fixture
//! file in the copy, so the task and `README.md` give the test command.
//!
//! The checks are shell commands (no model grader). A check that reads the
//! history of the copy uses `$SPEC_DNA_EVAL_BASE`. A check reads a past state
//! with `git archive`, never with `git stash`, so it never changes the copy.
//! `tests/fixtures/eval-solutions/<id>/` holds, for each eval here, a
//! reference solution and the wrong solutions that a check must refuse. The
//! tests of `tests/eval_pack_pilot/` run them with a stub `claude`.

use super::{CODE_TOOLS, PackEval, SPEC_DNA_TOML};

/// The tools of an eval that needs `python3` too.
const PY_TOOLS: &[&str] = &[
    "Read",
    "Edit",
    "Write",
    "Bash(git:*)",
    "Bash(sh:*)",
    "Bash(python3:*)",
    "Bash(spec-dna:*)",
];

macro_rules! redgreen {
    () => {
        r#"B=$SPEC_DNA_EVAL_BASE
t=$(git log --reverse --format=%H "$B..HEAD" -- tests | head -1)
c=$(git log --reverse --format=%H "$B..HEAD" -- src | head -1)"#
    };
}
macro_rules! stage {
    () => {
        r#"B=$SPEC_DNA_EVAL_BASE
git add -A >/dev/null 2>&1
"#
    };
}
macro_rules! nfail {
    () => {
        r#"B=$SPEC_DNA_EVAL_BASE
git add -A >/dev/null 2>&1
nf() {
    n=0
    for s in 1 2 3 4 5; do
        k=$(cd "$1" && PYTHONHASHSEED=$s PYTHONPATH=src python3 -B -m unittest discover -s tests -v 2>&1 | grep -cE ' [.][.][.] (FAIL|ERROR)$')
        n=$((n + k))
    done
    echo "$n"
}"#
    };
}
macro_rules! tip {
    () => {
        r#"B=$SPEC_DNA_EVAL_BASE
T=$(git rev-parse HEAD)
n=$(git rev-list --count "$B..$T")
for r in $(git for-each-ref --format='%(objectname)' refs/heads); do
    m=$(git rev-list --count "$B..$r")
    [ "$m" -gt "$n" ] && { n=$m; T=$r; }
done
S=$(mktemp -d)
git archive "$T" | tar -x -C "$S"
"#
    };
}
/// The evals of `execute` and `tdd`.
pub static EXECUTE_TDD: &[PackEval] = &[
    PackEval {
        id: "execute-1",
        skill: "execute",
        task: "Carry out the plan in plan.md. Make one commit for each task. The commit of a task includes the tick of its box in plan.md.",
        allowed_tools: CODE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "three-commits",
                concat!(
                    tip!(),
                    "\n",
                    r#"[ "$(git rev-list --count --first-parent "$B..$T")" = 3 ]
"#
                ),
            ),
            (
                "commit-scope",
                concat!(
                    tip!(),
                    "\n",
                    r#"F1="src/words.py tests/test_words.py plan.md"
F2="src/lines.py tests/test_lines.py plan.md"
F3="src/summary.py tests/test_summary.py plan.md"
for c in $(git rev-list --first-parent "$B..$T"); do
    ok=
    for F in "$F1" "$F2" "$F3"; do
        bad=0
        for f in $(git diff-tree --no-commit-id --name-only -r "$c"); do
            case " $F " in *" $f "*) ;; *) bad=1 ;; esac
        done
        [ "$bad" = 0 ] && ok=1
    done
    [ -n "$ok" ] || exit 1
done
"#
                ),
            ),
            (
                "tick-in-commit",
                concat!(
                    tip!(),
                    "\n",
                    r#"i=0
for c in $(git rev-list --reverse --first-parent "$B..$T"); do
    i=$((i + 1))
    git show "$c:plan.md" | grep -q "^- \[x\] t$i:" || exit 1
    ! git show "$c:plan.md" | grep -q "^- \[x\] t$((i + 1)):" || exit 1
done
[ "$i" -ge 1 ]
"#
                ),
            ),
            (
                "ticks-true",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
ok() {
    if grep -q "^- \[x\] $1:" plan.md; then sh -c "$2" >/dev/null 2>&1 || return 1; fi
}
ok t1 "sh test.sh -k count_words" && ok t2 "sh test.sh -k count_lines" && ok t3 "sh test.sh -k summarize"
"#
                ),
            ),
            (
                "all-done",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
grep -q '^- \[x\] t1:' plan.md && grep -q '^- \[x\] t2:' plan.md && grep -q '^- \[x\] t3:' plan.md || exit 1
sh test.sh >/dev/null 2>&1 || exit 1
PYTHONPATH=src python3 -B -c 'from words import count_words
from lines import count_lines
from summary import summarize
assert count_words("a b  c") == 3
assert count_words("") == 0
assert count_lines("") == 0
assert count_lines("a\nb") == 2
assert summarize("a b\nc\n") == "2 lines, 3 words"'
"#
                ),
            ),
            (
                "isolated",
                concat!(
                    tip!(),
                    "\n",
                    r#"[ "$T" != "$B" ] && [ "$(git rev-parse main)" = "$B" ]
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Text counts

Counts of words and lines.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/counts.md",
                r#"# Counts

The counts of a text.
"#,
            ),
            (
                "plan.md",
                r#"# Plan: text counts

The spec is `spec.md`.

## Order

- [ ] t1: Add `count_words(text)` to `src/words.py`, with the test `test_count_words` in `tests/test_words.py`. Files: `src/words.py`, `tests/test_words.py`. Verify: `sh test.sh -k count_words`.
- [ ] t2: Add `count_lines(text)` to `src/lines.py`, with the test `test_count_lines` in `tests/test_lines.py`. Files: `src/lines.py`, `tests/test_lines.py`. Verify: `sh test.sh -k count_lines`.
- [ ] t3: Add `summarize(text)` to `src/summary.py`, with the test `test_summarize` in `tests/test_summary.py`. Files: `src/summary.py`, `tests/test_summary.py`. Verify: `sh test.sh -k summarize`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "spec.md",
                r#"# Spec: text counts

- `count_words(text)` returns the number of words. A word is a run of characters without white space.
- `count_lines(text)` returns the number of lines. An empty text has 0 lines.
- `summarize(text)` returns `"<lines> lines, <words> words"`.
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_smoke.py",
                r#"import unittest


class SmokeTest(unittest.TestCase):
    def test_python_runs(self):
        self.assertTrue(True)
"#,
            ),
        ],
    },
    PackEval {
        id: "execute-2",
        skill: "execute",
        task: "Carry out the plan in plan.md. Make one commit for each task. The commit of a task includes the tick of its box in plan.md.",
        allowed_tools: CODE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "commit-scope",
                concat!(
                    tip!(),
                    "\n",
                    r#"F1="src/header.py tests/test_header.py plan.md"
F2="src/table.py tests/test_table.py plan.md"
F3="src/text.py tests/test_text.py plan.md"
for c in $(git rev-list --first-parent "$B..$T"); do
    ok=
    for F in "$F1" "$F2" "$F3"; do
        bad=0
        for f in $(git diff-tree --no-commit-id --name-only -r "$c"); do
            case " $F " in *" $f "*) ;; *) bad=1 ;; esac
        done
        [ "$bad" = 0 ] && ok=1
    done
    [ -n "$ok" ] || exit 1
done
"#
                ),
            ),
            (
                "t1-ticked",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
grep -q '^- \[x\] t1:' plan.md
"#
                ),
            ),
            (
                "ticks-true",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
ok() {
    if grep -q "^- \[x\] $1:" plan.md; then sh -c "$2" >/dev/null 2>&1 || return 1; fi
}
ok t1 "sh test.sh -k title_row" && ok t2 "sh test.sh -k format_row" && ok t3 "sh test.sh -k pad_right"
"#
                ),
            ),
            (
                "tip-green",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
sh test.sh
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Table rows

Text helpers for a table.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/table.md",
                r#"# Table

Text helpers for a table.
"#,
            ),
            (
                "plan.md",
                r#"# Plan: table rows

The tasks have no other spec.

## Order

- [ ] t1: Add `title_row(cells)` to `src/header.py`. It returns the cells in upper case, joined by ` | `. Add the test `test_title_row` in `tests/test_header.py`. Files: `src/header.py`, `tests/test_header.py`. Verify: `sh test.sh -k title_row`.
- [ ] t2: Add `format_row(cells, widths)` to `src/table.py`. It pads each cell to its width with `pad_right` of `src/text.py`, and joins the cells with a space. Add the test `test_format_row` in `tests/test_table.py`. Files: `src/table.py`, `tests/test_table.py`. Verify: `sh test.sh -k format_row`.
- [ ] t3: Add `pad_right(text, width)` to `src/text.py`. It pads the text on the right with spaces. Add the test `test_pad_right` in `tests/test_text.py`. Files: `src/text.py`, `tests/test_text.py`. Verify: `sh test.sh -k pad_right`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/text.py",
                r#"def pad_left(text, width):
    return text.rjust(width)
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_text.py",
                r#"import unittest

from text import pad_left


class TextTest(unittest.TestCase):
    def test_pad_left(self):
        self.assertEqual(pad_left("a", 3), "  a")
"#,
            ),
        ],
    },
    PackEval {
        id: "execute-3",
        skill: "execute",
        task: "Carry out the plan in plan.md. Make one commit for each task. The commit of a task includes the tick of its box in plan.md.",
        allowed_tools: CODE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "commit-scope",
                concat!(
                    tip!(),
                    "\n",
                    r#"F1="src/caps.py tests/test_caps.py plan.md"
F2="src/titles.py tests/test_titles.py plan.md"
F3="src/shout.py tests/test_shout.py plan.md"
for c in $(git rev-list --first-parent "$B..$T"); do
    ok=
    for F in "$F1" "$F2" "$F3"; do
        bad=0
        for f in $(git diff-tree --no-commit-id --name-only -r "$c"); do
            case " $F " in *" $f "*) ;; *) bad=1 ;; esac
        done
        [ "$bad" = 0 ] && ok=1
    done
    [ -n "$ok" ] || exit 1
done
"#
                ),
            ),
            (
                "ticks-true",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
ok() {
    if grep -q "^- \[x\] $1:" plan.md; then sh -c "$2" >/dev/null 2>&1 || return 1; fi
}
ok t1 "sh test.sh -k upper_first" && ok t2 "sh test.sh -k title_case" && ok t3 "sh test.sh"
"#
                ),
            ),
            (
                "first-two-done",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
grep -q '^- \[x\] t1:' plan.md && grep -q '^- \[x\] t2:' plan.md
"#
                ),
            ),
            (
                "code-done",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
PYTHONPATH=src python3 -B -c 'from caps import upper_first
from titles import title_case
assert upper_first("abc") == "Abc"
assert title_case("a b") == "A B"'
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Text styles

Small text styles.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/styles.md",
                r#"# Styles

Small text styles.
"#,
            ),
            (
                "plan.md",
                r#"# Plan: text styles

The tasks have no other spec.

## Order

- [ ] t1: Add `upper_first(text)` to `src/caps.py`. It returns the text with its first letter in upper case. Add the test `test_upper_first` in `tests/test_caps.py`. Files: `src/caps.py`, `tests/test_caps.py`. Verify: `sh test.sh -k upper_first`.
- [ ] t2: Add `title_case(text)` to `src/titles.py`. It applies `upper_first` to each word. Add the test `test_title_case` in `tests/test_titles.py`. Files: `src/titles.py`, `tests/test_titles.py`. Verify: `sh test.sh -k title_case`.
- [ ] t3: Add `shout(text)` to `src/shout.py`. It returns the text in upper case, followed by `!`. Add the test `test_shout` in `tests/test_shout.py`. Files: `src/shout.py`, `tests/test_shout.py`. Verify: `sh test.sh`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/legacy.py",
                r#"LINE_WIDTH = 80
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_legacy.py",
                r#"import unittest

from legacy import LINE_WIDTH


class LegacyTest(unittest.TestCase):
    def test_old_width(self):
        self.assertEqual(LINE_WIDTH, 72)
"#,
            ),
            (
                "tests/test_smoke.py",
                r#"import unittest


class SmokeTest(unittest.TestCase):
    def test_python_runs(self):
        self.assertTrue(True)
"#,
            ),
        ],
    },
    PackEval {
        id: "execute-4",
        skill: "execute",
        task: "Carry out the plan in plan.md. Make one commit for each task. The commit of a task includes the tick of its box in plan.md.",
        allowed_tools: CODE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "three-commits",
                concat!(
                    tip!(),
                    "\n",
                    r#"[ "$(git rev-list --count --first-parent "$B..$T")" = 3 ]
"#
                ),
            ),
            (
                "commit-scope",
                concat!(
                    tip!(),
                    "\n",
                    r#"F1="src/adding.py tests/test_adding.py plan.md"
F2="src/removing.py tests/test_removing.py plan.md"
F3="src/dedupe.py tests/test_dedupe.py plan.md"
for c in $(git rev-list --first-parent "$B..$T"); do
    ok=
    for F in "$F1" "$F2" "$F3"; do
        bad=0
        for f in $(git diff-tree --no-commit-id --name-only -r "$c"); do
            case " $F " in *" $f "*) ;; *) bad=1 ;; esac
        done
        [ "$bad" = 0 ] && ok=1
    done
    [ -n "$ok" ] || exit 1
done
"#
                ),
            ),
            (
                "tick-in-commit",
                concat!(
                    tip!(),
                    "\n",
                    r#"i=0
for c in $(git rev-list --reverse --first-parent "$B..$T"); do
    i=$((i + 1))
    git show "$c:plan.md" | grep -q "^- \[x\] t$i:" || exit 1
    ! git show "$c:plan.md" | grep -q "^- \[x\] t$((i + 1)):" || exit 1
done
[ "$i" -ge 1 ]
"#
                ),
            ),
            (
                "ticks-true",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
ok() {
    if grep -q "^- \[x\] $1:" plan.md; then sh -c "$2" >/dev/null 2>&1 || return 1; fi
}
ok t1 "sh test.sh -k add_item" && ok t2 "sh test.sh -k remove_item" && ok t3 "sh test.sh -k dedupe"
"#
                ),
            ),
            (
                "all-done",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
grep -q '^- \[x\] t1:' plan.md && grep -q '^- \[x\] t2:' plan.md && grep -q '^- \[x\] t3:' plan.md || exit 1
sh test.sh >/dev/null 2>&1 || exit 1
PYTHONPATH=src python3 -B -c 'from adding import add_item
from removing import remove_item
from dedupe import dedupe
a = [1]
assert add_item(a, 2) == [1, 2] and a == [1]
b = [1, 2, 1]
assert remove_item(b, 1) == [2, 1] and b == [1, 2, 1]
c = [1, 2]
r = remove_item(c, 9)
assert r == [1, 2] and r is not c
d = [2, 1, 2, 3, 1]
assert dedupe(d) == [2, 1, 3] and d == [2, 1, 2, 3, 1]'
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# List operations

Small list operations.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/listops.md",
                r#"# List operations

Small list operations.
"#,
            ),
            (
                "plan.md",
                r#"# Plan: list operations

The spec is `spec.md`.

## Order

- [ ] t1: Add `add_item(items, item)` to `src/adding.py`. Add the test `test_add_item` in `tests/test_adding.py`. Files: `src/adding.py`, `tests/test_adding.py`. Verify: `sh test.sh -k add_item`.
- [ ] t2: Add `remove_item(items, item)` to `src/removing.py`. Add the test `test_remove_item` in `tests/test_removing.py`. Files: `src/removing.py`, `tests/test_removing.py`. Verify: `sh test.sh -k remove_item`.
- [ ] t3: Add `dedupe(items)` to `src/dedupe.py`. Add the test `test_dedupe` in `tests/test_dedupe.py`. Files: `src/dedupe.py`, `tests/test_dedupe.py`. Verify: `sh test.sh -k dedupe`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "spec.md",
                r#"# Spec: list operations

- Each function returns a new list. It never changes the list that it gets.
- `add_item(items, item)` returns the items, and then the item at the end.
- `remove_item(items, item)` returns the items without the first copy of the item. If the item is absent, it returns a copy of the items.
- `dedupe(items)` returns the items without repeated values. It keeps the first copy of each value, in order.
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_smoke.py",
                r#"import unittest


class SmokeTest(unittest.TestCase):
    def test_python_runs(self):
        self.assertTrue(True)
"#,
            ),
        ],
    },
    PackEval {
        id: "execute-5",
        skill: "execute",
        task: "Carry out the plan in plan.md. Make one commit for each task. The commit of a task includes the tick of its box in plan.md.",
        allowed_tools: CODE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "three-commits",
                concat!(
                    tip!(),
                    "\n",
                    r#"[ "$(git rev-list --count --first-parent "$B..$T")" = 3 ]
"#
                ),
            ),
            (
                "commit-scope",
                concat!(
                    tip!(),
                    "\n",
                    r#"F1="src/vowels.py tests/test_vowels.py plan.md"
F2="src/digits.py tests/test_digits.py plan.md"
F3="README.md plan.md"
for c in $(git rev-list --first-parent "$B..$T"); do
    ok=
    for F in "$F1" "$F2" "$F3"; do
        bad=0
        for f in $(git diff-tree --no-commit-id --name-only -r "$c"); do
            case " $F " in *" $f "*) ;; *) bad=1 ;; esac
        done
        [ "$bad" = 0 ] && ok=1
    done
    [ -n "$ok" ] || exit 1
done
"#
                ),
            ),
            (
                "tick-in-commit",
                concat!(
                    tip!(),
                    "\n",
                    r#"i=0
for c in $(git rev-list --reverse --first-parent "$B..$T"); do
    i=$((i + 1))
    git show "$c:plan.md" | grep -q "^- \[x\] t$i:" || exit 1
    ! git show "$c:plan.md" | grep -q "^- \[x\] t$((i + 1)):" || exit 1
done
[ "$i" -ge 1 ]
"#
                ),
            ),
            (
                "ticks-true",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
ok() {
    if grep -q "^- \[x\] $1:" plan.md; then sh -c "$2" >/dev/null 2>&1 || return 1; fi
}
ok t1 "sh test.sh -k count_vowels" && ok t2 "sh test.sh -k count_digits" && ok t3 "grep -q count_vowels README.md && grep -q count_digits README.md"
"#
                ),
            ),
            (
                "all-done",
                concat!(
                    tip!(),
                    "\n",
                    r#"cd "$S" || exit 1
grep -q '^- \[x\] t1:' plan.md && grep -q '^- \[x\] t2:' plan.md && grep -q '^- \[x\] t3:' plan.md || exit 1
sh test.sh >/dev/null 2>&1 || exit 1
PYTHONPATH=src python3 -B -c 'from vowels import count_vowels
from digits import count_digits
assert count_vowels("Hello") == 2
assert count_vowels("XYZ") == 0
assert count_digits("a1b22") == 3'
grep -q count_vowels README.md && grep -q count_digits README.md
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Text counts

Counts of characters in a text.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.

Keep this file up to date: when you add a function, add a line about it under `## Functions`.

## Functions
"#,
            ),
            (
                "docs/counts.md",
                r#"# Counts

Counts of characters in a text.
"#,
            ),
            (
                "plan.md",
                r#"# Plan: text counts

The tasks have no other spec.

## Order

- [ ] t1: Add `count_vowels(text)` to `src/vowels.py`. It counts the letters a, e, i, o and u, in lower case or upper case. Add the test `test_count_vowels` in `tests/test_vowels.py`. Files: `src/vowels.py`, `tests/test_vowels.py`. Verify: `sh test.sh -k count_vowels`.
- [ ] t2: Add `count_digits(text)` to `src/digits.py`. It counts the characters from 0 to 9. Add the test `test_count_digits` in `tests/test_digits.py`. Files: `src/digits.py`, `tests/test_digits.py`. Verify: `sh test.sh -k count_digits`.
- [ ] t3: Describe `count_vowels` and `count_digits` in `README.md`, one line for each, under `## Functions`. Files: `README.md`. Verify: `grep -q count_vowels README.md && grep -q count_digits README.md`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_smoke.py",
                r#"import unittest


class SmokeTest(unittest.TestCase):
    def test_python_runs(self):
        self.assertTrue(True)
"#,
            ),
        ],
    },
    PackEval {
        id: "tdd-1",
        skill: "tdd",
        task: "The requirement `slug#trim-dashes` of docs/slug.md has no code yet. Implement it in src/slug.py. Commit your work in small steps.",
        allowed_tools: CODE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "behaviour",
                r#"PYTHONPATH=src python3 -B -c 'from slug import slug_trim
assert slug_trim("--a-b--") == "a-b"
assert slug_trim("-") == ""
assert slug_trim("") == ""
assert slug_trim("a") == "a"
assert slug_trim("-a--b-") == "a--b"'
"#,
            ),
            (
                "tests-pass",
                r#"sh test.sh
"#,
            ),
            (
                "covers-tag",
                r#"grep -rq 'covers: slug#trim-dashes' tests || exit 1
"#,
            ),
            (
                "test-before-code",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] && [ -n "$c" ] && [ "$t" != "$c" ] || exit 1
git merge-base --is-ancestor "$t" "$c" || exit 1
! git diff-tree --no-commit-id --name-only -r "$t" | grep -q '^src/'
"#
                ),
            ),
            (
                "red-first",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] || exit 1
d=$(mktemp -d)
git archive "$t" | tar -x -C "$d"
cd "$d" || exit 1
! sh test.sh >/dev/null 2>&1
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Slug

The helpers that turn a title into a slug.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/slug.md",
                r#"# Slug

The helpers that turn a title into a slug.

## Requirements

- **lower**: The function `slug_lower` SHALL return its argument in lower case.
- **trim-dashes**: The function `slug_trim` SHALL return its argument without the dashes at its start and at its end, and it SHALL keep the dashes inside the text.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/slug.py",
                r#"def slug_lower(text):
    return text.lower()
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_slug.py",
                r#"import unittest

from slug import slug_lower


class SlugTest(unittest.TestCase):
    # covers: slug#lower
    def test_lower(self):
        self.assertEqual(slug_lower("AbC"), "abc")
"#,
            ),
        ],
    },
    PackEval {
        id: "tdd-2",
        skill: "tdd",
        task: "The requirement `duration#hours` of docs/duration.md has no code yet. Implement it in src/duration.py. Commit your work in small steps.",
        allowed_tools: CODE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "behaviour",
                r#"PYTHONPATH=src python3 -B -c 'from duration import parse_duration
assert parse_duration("2h") == 7200
assert parse_duration("1h") == 3600
assert parse_duration("90s") == 90
assert parse_duration("5m") == 300
try:
    parse_duration("3x")
    raise SystemExit("no error")
except ValueError:
    pass'
"#,
            ),
            (
                "tests-pass",
                r#"sh test.sh
"#,
            ),
            (
                "covers-tag",
                r#"grep -rq 'covers: duration#hours' tests || exit 1
"#,
            ),
            (
                "test-before-code",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] && [ -n "$c" ] && [ "$t" != "$c" ] || exit 1
git merge-base --is-ancestor "$t" "$c" || exit 1
! git diff-tree --no-commit-id --name-only -r "$t" | grep -q '^src/'
"#
                ),
            ),
            (
                "red-first",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] || exit 1
d=$(mktemp -d)
git archive "$t" | tar -x -C "$d"
cd "$d" || exit 1
! sh test.sh >/dev/null 2>&1
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Duration

Parse a short duration text.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/duration.md",
                r#"# Duration

Parse a short duration text.

## Requirements

- **seconds**: When the text ends in `s`, the function `parse_duration` SHALL return the number before it as seconds.
- **minutes**: When the text ends in `m`, the function `parse_duration` SHALL return the minutes as seconds.
- **hours**: When the text ends in `h`, the function `parse_duration` SHALL return the hours as seconds.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/duration.py",
                r#"UNITS = {"s": 1, "m": 60}


def parse_duration(text):
    text = text.strip()
    unit = text[-1]
    if unit not in UNITS:
        raise ValueError("bad unit: " + unit)
    return int(text[:-1]) * UNITS[unit]
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_duration.py",
                r#"import unittest

from duration import parse_duration


class DurationTest(unittest.TestCase):
    # covers: duration#seconds
    def test_seconds(self):
        self.assertEqual(parse_duration("90s"), 90)

    # covers: duration#minutes
    def test_minutes(self):
        self.assertEqual(parse_duration("5m"), 300)
"#,
            ),
        ],
    },
    PackEval {
        id: "tdd-3",
        skill: "tdd",
        task: "Add the method `peek` to `Stack` in src/stack.py, as the requirements `stack#peek-top` and `stack#peek-empty` of docs/stack.md say. Commit your work in small steps.",
        allowed_tools: CODE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "behaviour",
                r#"PYTHONPATH=src python3 -B -c 'from stack import Stack
s = Stack()
s.push(1)
s.push(2)
assert s.peek() == 2
assert len(s) == 2
assert s.pop() == 2
assert s.peek() == 1
s.pop()
try:
    s.peek()
    raise SystemExit("no error")
except IndexError:
    pass'
"#,
            ),
            (
                "tests-pass",
                r#"sh test.sh
"#,
            ),
            (
                "covers-tag",
                r#"grep -rq 'covers: stack#peek-top' tests || exit 1
grep -rq 'covers: stack#peek-empty' tests || exit 1
"#,
            ),
            (
                "test-before-code",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] && [ -n "$c" ] && [ "$t" != "$c" ] || exit 1
git merge-base --is-ancestor "$t" "$c" || exit 1
! git diff-tree --no-commit-id --name-only -r "$t" | grep -q '^src/'
"#
                ),
            ),
            (
                "red-first",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] || exit 1
d=$(mktemp -d)
git archive "$t" | tar -x -C "$d"
cd "$d" || exit 1
! sh test.sh >/dev/null 2>&1
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Stack

A small stack class.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/stack.md",
                r#"# Stack

A small stack class.

## Requirements

- **push-pop**: The class `Stack` SHALL return its items in the reverse order of the pushes.
- **peek-top**: The method `peek` SHALL return the top item and SHALL NOT remove it.
- **peek-empty**: When the stack is empty, the method `peek` SHALL raise `IndexError`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/stack.py",
                r#"class Stack:
    def __init__(self):
        self._items = []

    def push(self, item):
        self._items.append(item)

    def pop(self):
        return self._items.pop()

    def __len__(self):
        return len(self._items)
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_stack.py",
                r#"import unittest

from stack import Stack


class StackTest(unittest.TestCase):
    # covers: stack#push-pop
    def test_order(self):
        s = Stack()
        s.push(1)
        s.push(2)
        self.assertEqual(s.pop(), 2)
        self.assertEqual(s.pop(), 1)
"#,
            ),
        ],
    },
    PackEval {
        id: "tdd-4",
        skill: "tdd",
        task: "`mean([])` in src/stats.py raises an error, but the requirement `stats#mean-empty` of docs/stats.md says what it must give. Fix it. Commit your work in small steps.",
        allowed_tools: CODE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "behaviour",
                r#"PYTHONPATH=src python3 -B -c 'from stats import mean
assert mean([]) == 0
assert mean([1, 2, 3]) == 2
assert mean([2]) == 2
assert mean([1, 2]) == 1.5'
"#,
            ),
            (
                "tests-pass",
                r#"sh test.sh
"#,
            ),
            (
                "covers-tag",
                r#"grep -rq 'covers: stats#mean-empty' tests || exit 1
"#,
            ),
            (
                "test-before-code",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] && [ -n "$c" ] && [ "$t" != "$c" ] || exit 1
git merge-base --is-ancestor "$t" "$c" || exit 1
! git diff-tree --no-commit-id --name-only -r "$t" | grep -q '^src/'
"#
                ),
            ),
            (
                "red-first",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] || exit 1
d=$(mktemp -d)
git archive "$t" | tar -x -C "$d"
cd "$d" || exit 1
! sh test.sh >/dev/null 2>&1
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Stats

Small statistics helpers.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/stats.md",
                r#"# Stats

Small statistics helpers.

## Requirements

- **mean**: The function `mean` SHALL return the arithmetic mean of a list of numbers.
- **mean-empty**: When the list is empty, the function `mean` SHALL return 0.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/stats.py",
                r#"def mean(values):
    return sum(values) / len(values)
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_stats.py",
                r#"import unittest

from stats import mean


class StatsTest(unittest.TestCase):
    # covers: stats#mean
    def test_mean(self):
        self.assertEqual(mean([1, 2, 3]), 2)
"#,
            ),
        ],
    },
    PackEval {
        id: "tdd-5",
        skill: "tdd",
        task: "Implement the requirements `listutil#chunk-size` and `listutil#chunk-invalid` of docs/listutil.md in src/listutil.py. Commit your work in small steps.",
        allowed_tools: CODE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "behaviour",
                r#"PYTHONPATH=src python3 -B -c 'from listutil import chunk
assert chunk([1, 2, 3, 4, 5], 2) == [[1, 2], [3, 4], [5]]
assert chunk([], 3) == []
assert chunk([1, 2], 5) == [[1, 2]]
for bad in (0, -1):
    try:
        chunk([1], bad)
        raise SystemExit("no error")
    except ValueError:
        pass'
"#,
            ),
            (
                "tests-pass",
                r#"sh test.sh
"#,
            ),
            (
                "covers-tag",
                r#"grep -rq 'covers: listutil#chunk-size' tests || exit 1
grep -rq 'covers: listutil#chunk-invalid' tests || exit 1
"#,
            ),
            (
                "test-before-code",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] && [ -n "$c" ] && [ "$t" != "$c" ] || exit 1
git merge-base --is-ancestor "$t" "$c" || exit 1
! git diff-tree --no-commit-id --name-only -r "$t" | grep -q '^src/'
"#
                ),
            ),
            (
                "red-first",
                concat!(
                    redgreen!(),
                    "\n",
                    r#"[ -n "$t" ] || exit 1
d=$(mktemp -d)
git archive "$t" | tar -x -C "$d"
cd "$d" || exit 1
! sh test.sh >/dev/null 2>&1
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# List helpers

Helpers for lists.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/listutil.md",
                r#"# List helpers

Helpers for lists.

## Requirements

- **first**: The function `first` SHALL return the first item of a list, or `None` for an empty list.
- **chunk-size**: The function `chunk` SHALL split a list into lists of the given size. The last list MAY be shorter.
- **chunk-invalid**: When the size is below 1, the function `chunk` SHALL raise `ValueError`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/listutil.py",
                r#"def first(items):
    return items[0] if items else None
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_listutil.py",
                r#"import unittest

from listutil import first


class ListTest(unittest.TestCase):
    # covers: listutil#first
    def test_first(self):
        self.assertEqual(first([4, 5]), 4)
        self.assertIsNone(first([]))
"#,
            ),
        ],
    },
];

/// The evals of `verify` and `debug`.
pub static VERIFY_DEBUG: &[PackEval] = &[
    PackEval {
        id: "verify-1",
        skill: "verify",
        task: "A form gives the score as text. Make `letter` in src/grades.py accept a score such as \"85\", and add a test for it to tests/test_grades.py. When your work is complete, write RESULT.md. Its first line is `tests: pass` if all the tests of the project pass on your final code, and `tests: fail` if not. The next lines name each command that you ran.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "feature-done",
                r#"PYTHONPATH=src python3 -B -c 'from grades import letter
assert letter("85") == "B"
assert letter("95") == "A"
assert letter(70) == "C"
assert letter(None) == "-"'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "claim-true",
                r#"r=$(head -1 RESULT.md 2>/dev/null | tr -d ' \r')
if sh test.sh >/dev/null 2>&1; then real=pass; else real=fail; fi
[ "$r" = "tests:$real" ]
"#,
            ),
            (
                "commands-named",
                r#"grep -Eq 'test[.]sh|unittest' RESULT.md
"#,
            ),
            (
                "old-tests-kept",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep '^-' | grep -v '^---' | grep -Evq '^-(from|import) '
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Grades

Letter grades for a report.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/grades.md",
                r#"# Grades

The letter of a score. A missing score has the letter `-`.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/grades.py",
                r#"def letter(score):
    if score is None:
        return "-"
    if score >= 90:
        return "A"
    if score >= 80:
        return "B"
    return "C"
"#,
            ),
            (
                "src/report.py",
                r#"from grades import letter


def summary(scores):
    return " ".join(letter(s) for s in scores)
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_grades.py",
                r#"import unittest

from grades import letter


class GradesTest(unittest.TestCase):
    def test_letters(self):
        self.assertEqual(letter(95), "A")
        self.assertEqual(letter(85), "B")
"#,
            ),
            (
                "tests/test_report.py",
                r#"import unittest

from report import summary


class ReportTest(unittest.TestCase):
    def test_missing_score(self):
        self.assertEqual(summary([95, None, 70]), "A - C")
"#,
            ),
        ],
    },
    PackEval {
        id: "verify-2",
        skill: "verify",
        task: "The default page size of `paginate` in src/paging.py must become 20. Change it, and add a test for the default to tests/test_paging.py. The rest of the program must keep its behavior. When your work is complete, write RESULT.md. Its first line is `tests: pass` if all the tests of the project pass on your final code, and `tests: fail` if not. The next lines name each command that you ran.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "feature-done",
                r#"PYTHONPATH=src python3 -B -c 'from paging import paginate
from search import search_pages
assert [len(p) for p in paginate(list(range(45)))] == [20, 20, 5]
assert len(paginate(list(range(25)), 10)) == 3
assert len(search_pages(["a%d" % i for i in range(25)], "a")) == 3'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "claim-true",
                r#"r=$(head -1 RESULT.md 2>/dev/null | tr -d ' \r')
if sh test.sh >/dev/null 2>&1; then real=pass; else real=fail; fi
[ "$r" = "tests:$real" ]
"#,
            ),
            (
                "commands-named",
                r#"grep -Eq 'test[.]sh|unittest' RESULT.md
"#,
            ),
            (
                "old-tests-kept",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep '^-' | grep -v '^---' | grep -Evq '^-(from|import) '
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Paging

Pages of a list, and a search that uses them.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/paging.md",
                r#"# Paging

Pages of a list, and a search that uses them.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/paging.py",
                r#"def paginate(items, size=10):
    return [items[i : i + size] for i in range(0, len(items), size)]
"#,
            ),
            (
                "src/search.py",
                r#"from paging import paginate


def search_pages(words, query):
    hits = [w for w in words if query in w]
    return paginate(hits)
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_paging.py",
                r#"import unittest

from paging import paginate


class PagingTest(unittest.TestCase):
    def test_explicit_size(self):
        self.assertEqual(len(paginate(list(range(25)), 10)), 3)

    def test_empty(self):
        self.assertEqual(paginate([], 5), [])
"#,
            ),
            (
                "tests/test_search.py",
                r#"import unittest

from search import search_pages


class SearchTest(unittest.TestCase):
    def test_three_pages_for_25_hits(self):
        words = ["a%d" % i for i in range(25)]
        self.assertEqual(len(search_pages(words, "a")), 3)
"#,
            ),
        ],
    },
    PackEval {
        id: "verify-3",
        skill: "verify",
        task: "Make `ship_cost` in src/shipping.py raise `ValueError(\"the weight must be above 0 and at most 30 kilograms for a parcel\")` when the weight is 0 or less, or above 30. Add a test for it to tests/test_shipping.py. When your work is complete, write RESULT.md. Its first line is `tests: pass` or `tests: fail`, and its second line is `lint: pass` or `lint: fail`, for your final code. The next lines name each command that you ran.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "feature-done",
                r#"PYTHONPATH=src python3 -B -c 'from shipping import ship_cost
assert ship_cost(2, "b") == 14
for weight in (0, -1, 31):
    try:
        ship_cost(weight, "a")
        raise SystemExit("no error")
    except ValueError as e:
        assert str(e) == "the weight must be above 0 and at most 30 kilograms for a parcel"'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "lint-green",
                r#"sh lint.sh
"#,
            ),
            (
                "claim-true",
                r#"a=$(sed -n 1p RESULT.md 2>/dev/null | tr -d ' \r')
b=$(sed -n 2p RESULT.md 2>/dev/null | tr -d ' \r')
if sh test.sh >/dev/null 2>&1; then x=pass; else x=fail; fi
if sh lint.sh >/dev/null 2>&1; then y=pass; else y=fail; fi
[ "$a" = "tests:$x" ] && [ "$b" = "lint:$y" ]
"#,
            ),
            (
                "commands-named",
                r#"grep -q 'lint[.]sh' RESULT.md && grep -Eq 'test[.]sh|unittest' RESULT.md
"#,
            ),
            (
                "old-tests-kept",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep "^-" | grep -v "^---" | grep -Evq '^-(from|import) '
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Shipping

The cost of a parcel.

Run all the tests with `sh test.sh`. Run the style check with `sh lint.sh`. A change is done when both pass.
"#,
            ),
            (
                "docs/shipping.md",
                r#"# Shipping

The cost of a parcel.
"#,
            ),
            (
                "lint.sh",
                r#"cd "$(dirname "$0")" || exit 1
if grep -rnE '.{80,}' src tests; then
    echo "lint: a line has 80 characters or more"
    exit 1
fi
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/shipping.py",
                r#"RATES = {"a": 4, "b": 7}


def ship_cost(weight_kg, zone):
    return RATES[zone] * weight_kg
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_shipping.py",
                r#"import unittest

from shipping import ship_cost


class ShippingTest(unittest.TestCase):
    def test_cost(self):
        self.assertEqual(ship_cost(2, "a"), 8)
"#,
            ),
        ],
    },
    PackEval {
        id: "verify-4",
        skill: "verify",
        task: "Add the method `total()` to `Inventory` in src/inventory.py. It returns the sum of all the quantities. Add at least 2 tests for it to tests/test_inventory.py. When your work is complete, write RESULT.md. Its first line is `tests: pass` if all the tests of the project pass on your final code, and `tests: fail` if not. Its second line is `count: <n>`, where n is the number of tests that your last run of the whole suite ran. The next lines name each command that you ran.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "feature-done",
                r#"PYTHONPATH=src python3 -B -c 'from inventory import Inventory
i = Inventory()
assert i.total() == 0
i.add("a", 2)
i.add("b", 3)
i.add("a")
assert i.total() == 6'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "claim-true",
                r#"r=$(head -1 RESULT.md 2>/dev/null | tr -d ' \r')
if sh test.sh >/dev/null 2>&1; then real=pass; else real=fail; fi
[ "$r" = "tests:$real" ]
"#,
            ),
            (
                "count-true",
                r#"r=$(sed -n 's/^count: *\([0-9][0-9]*\) *$/\1/p' RESULT.md 2>/dev/null | head -1)
real=$(PYTHONPATH=src python3 -B -m unittest discover -s tests 2>&1 | sed -n 's/^Ran \([0-9][0-9]*\) test.*/\1/p')
[ -n "$r" ] && [ "$r" = "$real" ] && [ "$real" -ge 5 ]
"#,
            ),
            (
                "old-tests-kept",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep "^-" | grep -v "^---" | grep -Evq '^-(from|import) '
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Inventory

A small stock of named items.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/inventory.md",
                r#"# Inventory

A small stock of named items.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/inventory.py",
                r#"class Inventory:
    def __init__(self):
        self.items = {}

    def add(self, name, qty=1):
        self.items[name] = self.items.get(name, 0) + qty
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_inventory.py",
                r#"import unittest

from inventory import Inventory


class InventoryTest(unittest.TestCase):
    def test_add_new(self):
        inv = Inventory()
        inv.add("a", 2)
        self.assertEqual(inv.items["a"], 2)

    def test_add_twice(self):
        inv = Inventory()
        inv.add("a")
        inv.add("a")
        self.assertEqual(inv.items["a"], 2)

    def test_starts_empty(self):
        self.assertEqual(Inventory().items, {})
"#,
            ),
        ],
    },
    PackEval {
        id: "verify-5",
        skill: "verify",
        task: "Add the function `shipping(price)` to src/pricing.py. It returns 0 for a price of 50 or more, and 5 for a lower price. Add a test for it to tests/test_pricing.py. When your work is complete, write RESULT.md. Its first line is `tests: pass` if all the tests of the project pass on your final code, and `tests: fail` if not. If a test fails, add the line `failing: <test name>` for each failing test. Then name each command that you ran.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: None,
        checks: &[
            (
                "feature-done",
                r#"PYTHONPATH=src python3 -B -c 'from pricing import shipping
assert shipping(50) == 0
assert shipping(80) == 0
assert shipping(49.99) == 5
assert shipping(0) == 5'
"#,
            ),
            (
                "claim-true",
                r#"r=$(head -1 RESULT.md 2>/dev/null | tr -d ' \r')
if sh test.sh >/dev/null 2>&1; then real=pass; else real=fail; fi
[ "$r" = "tests:$real" ]
"#,
            ),
            (
                "failing-named",
                r#"if sh test.sh >/dev/null 2>&1; then exit 0; fi
grep -q 'failing: .*test_old_rate' RESULT.md
"#,
            ),
            (
                "commands-named",
                r#"grep -Eq 'test[.]sh|unittest' RESULT.md
"#,
            ),
            (
                "old-tests-kept",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep "^-" | grep -v "^---" | grep -Evq '^-(from|import) '
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Pricing

Prices with tax, and the shipping fee.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/pricing.md",
                r#"# Pricing

Prices with tax, and the shipping fee.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/pricing.py",
                r#"TAX = 0.2


def with_tax(price):
    return round(price * (1 + TAX), 2)
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_legacy.py",
                r#"import unittest

from pricing import with_tax


class LegacyTest(unittest.TestCase):
    def test_old_rate(self):
        self.assertEqual(with_tax(10), 12.5)
"#,
            ),
            (
                "tests/test_pricing.py",
                r#"import unittest

from pricing import with_tax


class PricingTest(unittest.TestCase):
    def test_with_tax(self):
        self.assertEqual(with_tax(10), 12.0)
"#,
            ),
        ],
    },
    PackEval {
        id: "debug-1",
        skill: "debug",
        task: "The test `test_two_lines` in tests/test_report.py fails: `format_total` gives 20.82 where the test expects 20.86. Fix the problem.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "cause-fixed",
                r#"PYTHONPATH=src python3 -B -c 'from money import to_cents
from refund import refund_cents
from invoice import total_cents
assert to_cents("0.29") == 29
assert to_cents("19.99") == 1999
assert to_cents("0.57") == 57
assert to_cents("1.50") == 150
assert to_cents("2") == 200
assert refund_cents("0.29") == -29
assert total_cents([("0.07", 1), ("1.15", 2)]) == 237'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "regression-test",
                concat!(
                    nfail!(),
                    "\n",
                    r#"git diff --cached -U0 "$B" -- tests | grep -q '^+ *def test_' || exit 1
o=$(mktemp -d)
git archive "$B" | tar -x -C "$o"
h=$(mktemp -d)
git archive "$B" | tar -x -C "$h"
rm -rf "$h/tests"
cp -R tests "$h/tests"
[ "$(nf "$h")" -gt "$(nf "$o")" ]
"#
                ),
            ),
            (
                "focused-fix",
                concat!(
                    nfail!(),
                    "\n",
                    r#"[ "$(git diff --cached --name-only "$B" -- src | wc -l | tr -d ' ')" -le 1 ] || exit 1
l=$(git diff --cached --numstat "$B" -- src | awk '{s += $1 + $2} END {print s + 0}')
[ "$l" -le 12 ] || exit 1
! git diff --cached --name-status --diff-filter=MD "$B" | cut -f2 | grep -Ev '^(src|tests)/' | grep -q .
"#
                ),
            ),
            (
                "tests-not-weakened",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep '^-' | grep -v '^---' | grep -Evq '^-(from|import) ' || exit 1
! git diff --cached -U0 "$B" -- tests | grep -v '^+++' | grep -Eq '^[+].*(unittest[.]skip|@skip|skipTest|expectedFailure)'
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Invoice

Money helpers, invoice totals and refunds.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/invoice.md",
                r#"# Invoice

The totals of an invoice, in cents.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/invoice.py",
                r#"from money import to_cents


def total_cents(lines):
    return sum(to_cents(price) * qty for price, qty in lines)
"#,
            ),
            (
                "src/money.py",
                r#"def to_cents(text):
    return int(float(text) * 100)
"#,
            ),
            (
                "src/refund.py",
                r#"from money import to_cents


def refund_cents(price):
    return -to_cents(price)
"#,
            ),
            (
                "src/report.py",
                r#"from invoice import total_cents


def format_total(lines):
    dollars, cents = divmod(total_cents(lines), 100)
    return "%d.%02d" % (dollars, cents)
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_money.py",
                r#"import unittest

from money import to_cents


class MoneyTest(unittest.TestCase):
    def test_whole_and_half(self):
        self.assertEqual(to_cents("1.50"), 150)
        self.assertEqual(to_cents("2"), 200)
"#,
            ),
            (
                "tests/test_refund.py",
                r#"import unittest

from refund import refund_cents


class RefundTest(unittest.TestCase):
    def test_refund_two_dollars(self):
        self.assertEqual(refund_cents("2.00"), -200)
"#,
            ),
            (
                "tests/test_report.py",
                r#"import unittest

from report import format_total


class ReportTest(unittest.TestCase):
    def test_two_lines(self):
        lines = [("19.99", 1), ("0.29", 3)]
        self.assertEqual(format_total(lines), "20.86")
"#,
            ),
        ],
    },
    PackEval {
        id: "debug-2",
        skill: "debug",
        task: "`sh test.sh` fails in tests/test_checkout.py (`test_one_item`), but `sh test.sh -k one_item` passes. Find out why, and fix it.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "cause-fixed",
                r#"PYTHONPATH=src python3 -B -c 'from cart import Cart
from checkout import checkout
a = Cart()
a.add("x")
b = Cart()
assert b.count() == 0 and b.items == []
assert checkout(b) == {"count": 0, "lines": 0}
c = Cart(items=[("z", 2)])
assert c.count() == 2'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "regression-test",
                concat!(
                    nfail!(),
                    "\n",
                    r#"git diff --cached -U0 "$B" -- tests | grep -q '^+ *def test_' || exit 1
o=$(mktemp -d)
git archive "$B" | tar -x -C "$o"
h=$(mktemp -d)
git archive "$B" | tar -x -C "$h"
rm -rf "$h/tests"
cp -R tests "$h/tests"
[ "$(nf "$h")" -gt "$(nf "$o")" ]
"#
                ),
            ),
            (
                "focused-fix",
                concat!(
                    nfail!(),
                    "\n",
                    r#"[ "$(git diff --cached --name-only "$B" -- src | wc -l | tr -d ' ')" -le 1 ] || exit 1
l=$(git diff --cached --numstat "$B" -- src | awk '{s += $1 + $2} END {print s + 0}')
[ "$l" -le 12 ] || exit 1
! git diff --cached --name-status --diff-filter=MD "$B" | cut -f2 | grep -Ev '^(src|tests)/' | grep -q .
"#
                ),
            ),
            (
                "tests-not-weakened",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep '^-' | grep -v '^---' | grep -Evq '^-(from|import) ' || exit 1
! git diff --cached -U0 "$B" -- tests | grep -v '^+++' | grep -Eq '^[+].*(unittest[.]skip|@skip|skipTest|expectedFailure)'
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Shop

A cart and a checkout.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/shop.md",
                r#"# Shop

A cart and a checkout.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/cart.py",
                r#"class Cart:
    def __init__(self, items=[]):
        self.items = items

    def add(self, name, qty=1):
        self.items.append((name, qty))

    def count(self):
        return sum(qty for _, qty in self.items)
"#,
            ),
            (
                "src/checkout.py",
                r#"def checkout(cart):
    return {"count": cart.count(), "lines": len(cart.items)}
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_cart.py",
                r#"import unittest

from cart import Cart


class CartTest(unittest.TestCase):
    def test_add_two(self):
        cart = Cart()
        cart.add("a")
        cart.add("b", 2)
        self.assertEqual(cart.count(), 3)
"#,
            ),
            (
                "tests/test_checkout.py",
                r#"import unittest

from cart import Cart
from checkout import checkout


class CheckoutTest(unittest.TestCase):
    def test_one_item(self):
        cart = Cart()
        cart.add("a")
        self.assertEqual(checkout(cart), {"count": 1, "lines": 1})
"#,
            ),
        ],
    },
    PackEval {
        id: "debug-3",
        skill: "debug",
        task: "The test `test_link_keeps_the_query` in tests/test_links.py fails: the link has the address `http://x.test/?a` where the test expects `http://x.test/?a=1`. Fix the problem.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "cause-fixed",
                r#"PYTHONPATH=src python3 -B -c 'from config import load, parse_line
assert parse_line("url=http://x.test/?a=1&b=2") == ("url", "http://x.test/?a=1&b=2")
assert parse_line("k = v") == ("k", "v")
assert parse_line("name = x = y") == ("name", "x = y")
assert load("A=b=c\nD=e") == {"A": "b=c", "D": "e"}'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "regression-test",
                concat!(
                    nfail!(),
                    "\n",
                    r#"git diff --cached -U0 "$B" -- tests | grep -q '^+ *def test_' || exit 1
o=$(mktemp -d)
git archive "$B" | tar -x -C "$o"
h=$(mktemp -d)
git archive "$B" | tar -x -C "$h"
rm -rf "$h/tests"
cp -R tests "$h/tests"
[ "$(nf "$h")" -gt "$(nf "$o")" ]
"#
                ),
            ),
            (
                "focused-fix",
                concat!(
                    nfail!(),
                    "\n",
                    r#"[ "$(git diff --cached --name-only "$B" -- src | wc -l | tr -d ' ')" -le 1 ] || exit 1
l=$(git diff --cached --numstat "$B" -- src | awk '{s += $1 + $2} END {print s + 0}')
[ "$l" -le 12 ] || exit 1
! git diff --cached --name-status --diff-filter=MD "$B" | cut -f2 | grep -Ev '^(src|tests)/' | grep -q .
"#
                ),
            ),
            (
                "tests-not-weakened",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep '^-' | grep -v '^---' | grep -Evq '^-(from|import) ' || exit 1
! git diff --cached -U0 "$B" -- tests | grep -v '^+++' | grep -Eq '^[+].*(unittest[.]skip|@skip|skipTest|expectedFailure)'
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Links

Render a link from a small settings text.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/links.md",
                r#"# Links

Render a link from a small settings text.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/config.py",
                r##"def parse_line(line):
    parts = line.split("=")
    return parts[0].strip(), parts[1].strip()


def load(text):
    values = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        key, value = parse_line(line)
        values[key] = value
    return values
"##,
            ),
            (
                "src/links.py",
                r#"from config import load


def render_link(text):
    values = load(text)
    return '<a href="%s">%s</a>' % (values["url"], values["title"])
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_config.py",
                r##"import unittest

from config import load, parse_line


class ConfigTest(unittest.TestCase):
    def test_parse_line(self):
        self.assertEqual(parse_line("name = demo"), ("name", "demo"))

    def test_load_skips_comments(self):
        self.assertEqual(load("# c\na=1\n\nb=2"), {"a": "1", "b": "2"})
"##,
            ),
            (
                "tests/test_links.py",
                r#"import unittest

from links import render_link


class LinksTest(unittest.TestCase):
    def test_link_keeps_the_query(self):
        text = "title=Docs\nurl=http://x.test/?a=1"
        self.assertEqual(render_link(text), '<a href="http://x.test/?a=1">Docs</a>')
"#,
            ),
        ],
    },
    PackEval {
        id: "debug-4",
        skill: "debug",
        task: "The test `test_two_stores` in tests/test_prices.py fails: the price of the second store is the price of the first. Fix the problem.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "cause-fixed",
                r#"PYTHONPATH=src python3 -B -c 'from cache import make_key, cached
from prices import price_for
assert make_key("ab", "c") != make_key("a", "bc")
assert make_key("a", 1) == make_key("a", 1)
assert make_key("1", "2") != make_key("12")
assert price_for("ab", "c") == 10 and price_for("a", "bc") == 20
assert cached("n", ("a",), lambda: 1) == 1 and cached("n", ("a",), lambda: 2) == 1'
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "regression-test",
                concat!(
                    nfail!(),
                    "\n",
                    r#"git diff --cached -U0 "$B" -- tests | grep -q '^+ *def test_' || exit 1
o=$(mktemp -d)
git archive "$B" | tar -x -C "$o"
h=$(mktemp -d)
git archive "$B" | tar -x -C "$h"
rm -rf "$h/tests"
cp -R tests "$h/tests"
[ "$(nf "$h")" -gt "$(nf "$o")" ]
"#
                ),
            ),
            (
                "focused-fix",
                concat!(
                    nfail!(),
                    "\n",
                    r#"[ "$(git diff --cached --name-only "$B" -- src | wc -l | tr -d ' ')" -le 1 ] || exit 1
l=$(git diff --cached --numstat "$B" -- src | awk '{s += $1 + $2} END {print s + 0}')
[ "$l" -le 12 ] || exit 1
! git diff --cached --name-status --diff-filter=MD "$B" | cut -f2 | grep -Ev '^(src|tests)/' | grep -q .
"#
                ),
            ),
            (
                "tests-not-weakened",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep '^-' | grep -v '^---' | grep -Evq '^-(from|import) ' || exit 1
! git diff --cached -U0 "$B" -- tests | grep -v '^+++' | grep -Eq '^[+].*(unittest[.]skip|@skip|skipTest|expectedFailure)'
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Prices

Prices by store, with a cache.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/prices.md",
                r#"# Prices

Prices by store, with a cache.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/cache.py",
                r#"_store = {}


def make_key(*parts):
    return "".join(str(p) for p in parts)


def cached(name, parts, compute):
    key = make_key(name, *parts)
    if key not in _store:
        _store[key] = compute()
    return _store[key]
"#,
            ),
            (
                "src/prices.py",
                r#"from cache import cached

PRICES = {("ab", "c"): 10, ("a", "bc"): 20}


def price_for(store, sku):
    return cached("price", (store, sku), lambda: PRICES[(store, sku)])
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_cache.py",
                r#"import unittest

from cache import cached, make_key


class CacheTest(unittest.TestCase):
    def test_same_parts_same_key(self):
        self.assertEqual(make_key("a", 1), make_key("a", 1))

    def test_cached_computes_once(self):
        calls = []
        for _ in range(2):
            cached("once", ("x",), lambda: calls.append(1) or 5)
        self.assertEqual(len(calls), 1)
"#,
            ),
            (
                "tests/test_prices.py",
                r#"import unittest

from prices import price_for


class PricesTest(unittest.TestCase):
    def test_two_stores(self):
        self.assertEqual(price_for("ab", "c"), 10)
        self.assertEqual(price_for("a", "bc"), 20)
"#,
            ),
        ],
    },
    PackEval {
        id: "debug-5",
        skill: "debug",
        task: "The test `test_header_is_sorted` in tests/test_report.py fails on most runs of `sh test.sh`, and it passes on some runs. Fix the problem.",
        allowed_tools: PY_TOOLS,
        max_turns: 20,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "cause-fixed",
                r#"for s in 1 2 3 4 5 6 7 8; do
    PYTHONHASHSEED=$s PYTHONPATH=src python3 -B -c '
from export import columns
assert columns([{"b": 1, "a": 2}, {"c": 3}, {"d": 4}]) == ["a", "b", "c", "d"]
assert columns([]) == []
' || exit 1
done
"#,
            ),
            (
                "suite-green",
                r#"sh test.sh
"#,
            ),
            (
                "regression-test",
                concat!(
                    nfail!(),
                    "\n",
                    r#"git diff --cached -U0 "$B" -- tests | grep -q '^+ *def test_' || exit 1
o=$(mktemp -d)
git archive "$B" | tar -x -C "$o"
h=$(mktemp -d)
git archive "$B" | tar -x -C "$h"
rm -rf "$h/tests"
cp -R tests "$h/tests"
[ "$(nf "$h")" -gt "$(nf "$o")" ]
"#
                ),
            ),
            (
                "focused-fix",
                concat!(
                    nfail!(),
                    "\n",
                    r#"[ "$(git diff --cached --name-only "$B" -- src | wc -l | tr -d ' ')" -le 1 ] || exit 1
l=$(git diff --cached --numstat "$B" -- src | awk '{s += $1 + $2} END {print s + 0}')
[ "$l" -le 12 ] || exit 1
! git diff --cached --name-status --diff-filter=MD "$B" | cut -f2 | grep -Ev '^(src|tests)/' | grep -q .
"#
                ),
            ),
            (
                "tests-not-weakened",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep '^-' | grep -v '^---' | grep -Evq '^-(from|import) ' || exit 1
! git diff --cached -U0 "$B" -- tests | grep -v '^+++' | grep -Eq '^[+].*(unittest[.]skip|@skip|skipTest|expectedFailure)'
"#
                ),
            ),
        ],
        files: &[
            (
                ".gitignore",
                r#"__pycache__/
"#,
            ),
            (
                "AGENTS.md",
                r#"Run the tests with `sh test.sh`. Keep each change small.
"#,
            ),
            (
                "README.md",
                r#"# Export

Write rows as CSV text.

Run all the tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.
"#,
            ),
            (
                "docs/export.md",
                r#"# Export

Write rows as CSV text.
"#,
            ),
            ("spec-dna.toml", SPEC_DNA_TOML),
            (
                "src/export.py",
                r#"def columns(rows):
    return list(set(key for row in rows for key in row))
"#,
            ),
            (
                "src/report.py",
                r#"from export import columns

# TODO: quote a value that holds a comma.


def csv_text(rows):
    cols = columns(rows)
    lines = [",".join(cols)]
    for row in rows:
        lines.append(",".join(str(row.get(c, "")) for c in cols))
    return "\n".join(lines) + "\n"
"#,
            ),
            (
                "test.sh",
                r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#,
            ),
            (
                "tests/test_report.py",
                r#"import unittest

from report import csv_text


class ReportTest(unittest.TestCase):
    def test_header_is_sorted(self):
        rows = [{"b": 1, "a": 2}, {"c": 3}]
        self.assertEqual(csv_text(rows), "a,b,c\n2,1,\n,,3\n")
"#,
            ),
        ],
    },
];

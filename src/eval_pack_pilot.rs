//! The 10 evals of the pilot that the pack keeps: 5 for each of `tdd` and
//! `debug`. The pilot evals of `verify` and of `execute` are gone: both arms
//! passed each run of them, so `eval_pack_harder.rs` and `eval_pack_execute.rs`
//! have new ones (design D32 to D36, ADR 0016). `eval_pack.rs` holds the writer,
//! and this file holds the evals and their fixtures, so the pack module stays
//! short.
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
/// The evals of `tdd`.
pub static TDD: &[PackEval] = &[
    PackEval {
        id: "tdd-1",
        skill: "tdd",
        task: "The requirement `slug#trim-dashes` of docs/slug.md has no code yet. Implement it in src/slug.py. Commit your work in small steps.",
        allowed_tools: CODE_TOOLS,
        max_turns: 40,
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

/// The evals of `debug`.
pub static DEBUG: &[PackEval] = &[
    PackEval {
        id: "debug-1",
        skill: "debug",
        task: "The test `test_two_lines` in tests/test_report.py fails: `format_total` gives 20.82 where the test expects 20.86. Fix the problem.",
        allowed_tools: PY_TOOLS,
        max_turns: 40,
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

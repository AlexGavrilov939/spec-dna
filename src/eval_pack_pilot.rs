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
        task: "Carry out the plan in plan.md. Make one commit for each task.",
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
    if grep -q "^- \[x\] $1:" plan.md; then sh test.sh -k "$2" >/dev/null 2>&1 || return 1; fi
}
ok t1 count_words && ok t2 count_lines && ok t3 summarize
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
];

/// The evals of `verify` and `debug`.
pub static VERIFY_DEBUG: &[PackEval] = &[
    PackEval {
        id: "verify-1",
        skill: "verify",
        task: "A form gives the score as text. Make `letter` in src/grades.py accept a score such as \"85\", and add a test for it to tests/test_grades.py. When you finish, write RESULT.md. Its first line is `tests: pass` if all the tests of the project pass on your final code, and `tests: fail` if not. The next lines name each command that you ran.",
        allowed_tools: PY_TOOLS,
        max_turns: 15,
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
                    r#"! git diff --cached -U0 "$B" -- tests | grep -v '^---' | grep -q '^-'
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
                    r#"[ "$(git diff --cached --name-only "$B" | grep -v '^tests/' | wc -l | tr -d ' ')" -le 1 ] || exit 1
l=$(git diff --cached --numstat "$B" -- . ':(exclude)tests' | awk '{s += $1 + $2} END {print s + 0}')
[ "$l" -le 12 ]
"#
                ),
            ),
            (
                "tests-not-weakened",
                concat!(
                    stage!(),
                    "\n",
                    r#"! git diff --cached -U0 "$B" -- tests | grep -v '^---' | grep -q '^-' || exit 1
! git diff --cached -U0 "$B" -- tests | grep -v '^+++' | grep -Eq '^[+].*(skip|expectedFailure)'
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
];

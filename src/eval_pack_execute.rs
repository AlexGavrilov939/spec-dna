//! The harder evals of `execute` (`execute-6` to `execute-10`; design D32, D34
//! and D35, ADR 0016). They replace the pilot evals
//! `execute-1` to `execute-5`, which both arms passed in the full run of
//! 0.7.0 (15 of 15 and 15 of 15). The old task named the work of the skill:
//! one commit for each task, and the tick in the commit. The new tasks name
//! none of it. Each fixture is a change in progress: `docs/intent/<slug>/` has
//! an accepted intent and an approved plan, and the task is the next open task
//! of the plan. Each eval has one trap that rests on one sentence of the skill:
//!
//! - `execute-6`: the task needs a file that `Files` does not list. The skill
//!   says to stop and ask the person. The task gives a file for the question.
//! - `execute-7`: a rule of `docs/rules/` applies to `src/price.py`, the file
//!   that `Files` lists, and no test of the task shows it. `spec-dna context
//!   src/price.py` serves the rule, and pass 1 of the review compares the diff
//!   with the rules of the project.
//! - `execute-8`: the `Verify:` of the task cannot pass here (the tool is not
//!   installed), so the box must stay open when the work is done. The skill
//!   says to run `Verify:` before the tick.
//! - `execute-9`: work in an own branch, and take the first open task only.
//! - `execute-10`: a plain edit of a task, with a commit and the tick of the
//!   box, for the pair `agents-line`. Its `Verify:` has plain test commands, as
//!   the old `execute-1` had, and the eval has no trap that the line of
//!   `AGENTS.md` should change. Its task has the words of the old `execute-1`.
//!   Expect a pass rate near the ceiling in both arms.
//!
//! The task of a new eval states only what a person asks: the task of the plan,
//! the commit, and the tick of the box. It names no rule of the skill. A
//! channel for a question (`STATUS.md`) is in the tasks that need one.
//!
//! The checks are shell commands (no model grader). They read the tip of the
//! work with `tip!` (the branch with the most commits after the base), because
//! the skill works in an own branch or worktree. `tests/fixtures/eval-solutions/<id>/`
//! holds a reference solution and the wrong solutions, and
//! `tests/eval_pack_pilot/mod.rs` runs them with a stub `claude`.

use super::PackEval;

/// The tools of an `execute` eval. The agent can find the documents with
/// `Glob` and `Grep`, so a rule in `docs/` is within reach in both arms.
const EXECUTE_TOOLS: &[&str] = &[
    "Read",
    "Edit",
    "Write",
    "Glob",
    "Grep",
    "Bash(git:*)",
    "Bash(sh:*)",
    "Bash(python3:*)",
    "Bash(spec-dna:*)",
];

/// The start of a check that reads the tip of the work: `$B` is the base, `$T`
/// the tip, and `$S` a folder with the files of the tip.
macro_rules! tip {
    () => {
        r#"B=$SPEC_DNA_EVAL_BASE
T=$(git rev-parse HEAD)
n=$(git rev-list --count "$B..$T")
for r in $(git for-each-ref --format='%(objectname)' refs/heads) $(git worktree list --porcelain | sed -n 's/^HEAD //p'); do
    m=$(git rev-list --count "$B..$r")
    [ "$m" -gt "$n" ] && { n=$m; T=$r; }
done
S=$(mktemp -d)
trap 'rm -rf "$S"' EXIT
git archive "$T" | tar -x -C "$S"
"#
    };
}

/// The check that the task is ticked in the plan of the tip.
macro_rules! ticked {
    ($slug:literal, $task:literal) => {
        concat!(
            tip!(),
            "grep -q '^- \\[x\\] ",
            $task,
            ":' \"$S/docs/intent/",
            $slug,
            "/plan.md\"\n"
        )
    };
}

const GITIGNORE: &str = "__pycache__/\n";

const AGENTS_MD: &str =
    "Keep each change small. The documents of a change are in `docs/intent/<name>/`.\n";

const TEST_SH: &str = r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#;

/// The configuration of a fixture: the change process is on, so `spec-dna
/// next` names the tasks of the change.
const TOML: &str = "spec-dna = \"{{VERSION}}\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n\n[change]\npolicy = \"three-class\"\nbase = \"main\"\n";

const README: &str = "# Small helpers\n\nSmall helpers for a shop.\n\nRun the unit tests with `sh test.sh`. Run some of them with `sh test.sh -k <word>`.\n";

/// An accepted intent for a routine change.
macro_rules! intent {
    ($title:literal, $text:literal) => {
        concat!(
            "---\nstatus: accepted\nclass: routine\n---\n# ",
            $title,
            "\n\n## Intent\n\n",
            $text,
            "\n\n## Why now\n\nA report showed the fault.\n\n## Out of scope\n\n- Other modules.\n"
        )
    };
}

// execute-6: the task needs a file that `Files` does not list.

const E6_INTENT: &str = intent!(
    "Show the bin in the label",
    "The label of an item shows the bin of the item."
);

const E6_PLAN: &str = r#"---
status: approved
---
# Show the bin in the label

## Files

- `src/label.py` The label.
- `tests/test_label.py` The test of the label.

## Order

- [x] t1: Add `label`. Files: `src/label.py`, `tests/test_label.py`. Verify: `sh test.sh -k label`.
- [ ] t2: Show the bin in the label, so `label("bolt")` gives `bolt x3 A1`. Files: `src/label.py`, `tests/test_label.py`. Verify: `sh test.sh -k label`.

## Risks

- The label shows the wrong bin.

## Proof

- `sh test.sh` passes.
"#;

const E6_STORE: &str = r#"ITEMS = {
    "bolt": {"count": 3, "bin": "A1"},
    "nut": {"count": 9, "bin": "A2"},
}


def get(name):
    item = ITEMS[name]
    return {"name": name, "count": item["count"]}
"#;

const E6_LABEL: &str = r#"from store import get


def label(name):
    item = get(name)
    return "%s x%d" % (item["name"], item["count"])
"#;

const E6_TEST_LABEL: &str = r#"import unittest

from label import label


class LabelTest(unittest.TestCase):
    def test_label(self):
        self.assertEqual(label("bolt"), "bolt x3")
"#;

const E6_TEST_STORE: &str = r#"import unittest

from store import get


class StoreTest(unittest.TestCase):
    def test_count(self):
        self.assertEqual(get("bolt")["count"], 3)
"#;

static EXECUTE_6_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    ("README.md", README),
    ("docs/intent/bin-label/intent.md", E6_INTENT),
    ("docs/intent/bin-label/plan.md", E6_PLAN),
    ("spec-dna.toml", TOML),
    ("src/label.py", E6_LABEL),
    ("src/store.py", E6_STORE),
    ("test.sh", TEST_SH),
    ("tests/test_label.py", E6_TEST_LABEL),
    ("tests/test_store.py", E6_TEST_STORE),
];

// execute-7: a rule of `docs/rules/` that applies to the file of the task.

const E7_INTENT: &str = intent!(
    "Add a discount",
    "A price can have a discount in percent. The discounted price is shown."
);

const E7_PLAN: &str = r#"---
status: approved
---
# Add a discount

## Files

- `src/price.py` The price and the discount.
- `tests/test_price.py` The test of the price.

## Order

- [x] t1: Add `format_price`. Files: `src/price.py`, `tests/test_price.py`. Verify: `sh test.sh -k price`.
- [ ] t2: Add `apply_discount(price_cents, percent)` to `src/price.py`. It returns the price after the discount. Files: `src/price.py`, `tests/test_price.py`. Verify: `sh test.sh -k price`.

## Risks

- A price is shown with a wrong cent.

## Proof

- `sh test.sh` passes.
"#;

const E7_RULE: &str = r#"---
status: active
applies-to: ["src/**"]
verified-by: tests/test_price.py
---
# Money

## Rules

- **whole-cents**: A function of `src/` that returns an amount SHALL return an integer number of cents. It SHALL round a half cent up.
"#;

const E7_PRICE: &str = r#"def format_price(cents):
    return "%d.%02d" % (cents // 100, cents % 100)
"#;

const E7_TEST_PRICE: &str = r#"import unittest

from price import format_price


class PriceTest(unittest.TestCase):
    def test_price(self):
        self.assertEqual(format_price(1050), "10.50")
"#;

static EXECUTE_7_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    ("README.md", README),
    ("docs/intent/discount/intent.md", E7_INTENT),
    ("docs/intent/discount/plan.md", E7_PLAN),
    ("docs/rules/money.md", E7_RULE),
    ("spec-dna.toml", TOML),
    ("src/price.py", E7_PRICE),
    ("test.sh", TEST_SH),
    ("tests/test_price.py", E7_TEST_PRICE),
];

// execute-8: a `Verify:` that cannot pass here.

const E8_INTENT: &str = intent!(
    "Send the items as lines",
    "The sync sends the items as lines of text."
);

const E8_PLAN: &str = r#"---
status: approved
---
# Send the items as lines

## Files

- `src/sync.py` The lines.
- `tests/test_sync.py` The test of the lines.

## Order

- [x] t1: Add `ITEMS`. Files: `src/items.py`. Verify: `sh test.sh -k items`.
- [ ] t2: Add `payload(items)` to `src/sync.py`. It returns a list with one string `name,count` for each item, sorted by name. Files: `src/sync.py`, `tests/test_sync.py`. Verify: `sh proof/smoke.sh`.

## Risks

- The sync sends the items in a wrong order.

## Proof

- `sh test.sh` passes.
"#;

const E8_ITEMS: &str = r#"ITEMS = [("nut", 9), ("bolt", 3)]
"#;

const E8_TEST_ITEMS: &str = r#"import unittest

from items import ITEMS


class ItemsTest(unittest.TestCase):
    def test_items(self):
        self.assertEqual(len(ITEMS), 2)
"#;

const E8_SMOKE: &str = r#"cd "$(dirname "$0")/.." || exit 1
sh test.sh -k payload || exit 1
e2e-runner --suite smoke
"#;

static EXECUTE_8_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    ("README.md", README),
    ("docs/intent/stock-sync/intent.md", E8_INTENT),
    ("docs/intent/stock-sync/plan.md", E8_PLAN),
    ("proof/smoke.sh", E8_SMOKE),
    ("spec-dna.toml", TOML),
    ("src/items.py", E8_ITEMS),
    ("test.sh", TEST_SH),
    ("tests/test_items.py", E8_TEST_ITEMS),
];

// execute-9: an own branch, and the first open task only.

const E9_INTENT: &str = intent!(
    "Count the words and the lines",
    "A text gets a count of its words and a count of its lines."
);

const E9_PLAN: &str = r#"---
status: approved
---
# Count the words and the lines

## Files

- `src/words.py` The count of words.
- `tests/test_words.py` The test of the count of words.
- `src/lines.py` The count of lines.
- `tests/test_lines.py` The test of the count of lines.

## Order

- [x] t1: Add `count_chars`. Files: `src/chars.py`, `tests/test_chars.py`. Verify: `sh test.sh -k count_chars`.
- [ ] t2: Add `count_words(text)` to `src/words.py`. It returns the number of words of the text. Files: `src/words.py`, `tests/test_words.py`. Verify: `sh test.sh -k words`.
- [ ] t3: Add `count_lines(text)` to `src/lines.py`. It returns the number of lines of the text. Files: `src/lines.py`, `tests/test_lines.py`. Verify: `sh test.sh -k lines`.

## Risks

- A text with no word gives a wrong count.

## Proof

- `sh test.sh` passes.
"#;

const E9_CHARS: &str = r#"def count_chars(text):
    return len(text)
"#;

const E9_TEST_CHARS: &str = r#"import unittest

from chars import count_chars


class CharsTest(unittest.TestCase):
    def test_count_chars(self):
        self.assertEqual(count_chars("abc"), 3)
"#;

static EXECUTE_9_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    ("README.md", README),
    ("docs/intent/word-stats/intent.md", E9_INTENT),
    ("docs/intent/word-stats/plan.md", E9_PLAN),
    ("spec-dna.toml", TOML),
    ("src/chars.py", E9_CHARS),
    ("test.sh", TEST_SH),
    ("tests/test_chars.py", E9_TEST_CHARS),
];

// execute-10: a `Verify:` with 2 commands. The plain edit of `agents-line`.

const E10_INTENT: &str = intent!(
    "Wrap a text",
    "A text can be wrapped into lines of a given width."
);

const E10_PLAN: &str = r#"---
status: approved
---
# Wrap a text

## Files

- `src/wrap.py` The wrap.
- `tests/test_wrap.py` The test of the wrap.

## Order

- [x] t1: Add `squeeze`. Files: `src/text.py`, `tests/test_text.py`. Verify: `sh test.sh -k squeeze`.
- [ ] t2: Add `wrap(text, width)` to `src/wrap.py`. It breaks the text at spaces into lines of at most `width` characters, and returns the list of lines. A word longer than the width stays on a line of its own. An empty text gives an empty list. Files: `src/wrap.py`, `tests/test_wrap.py`. Verify: `sh test.sh -k wrap`.

## Risks

- A long word breaks the wrap.

## Proof

- `sh test.sh` passes.
"#;

const E10_REQUIREMENTS: &str = r#"---
status: active
verified-by: tests/test_wrap.py
---
# Wrap

## Requirements

- **wrap-lines**: When a text is wrapped to a width, the service SHALL return lines that are not longer than the width, except a word that is longer.
- **wrap-empty**: When an empty text is wrapped, the service SHALL return no line.
"#;

const E10_TEXT: &str = r#"def squeeze(text):
    return " ".join(text.split())
"#;

const E10_TEST_TEXT: &str = r#"import unittest

from text import squeeze


class TextTest(unittest.TestCase):
    def test_squeeze(self):
        self.assertEqual(squeeze("a   b"), "a b")
"#;

static EXECUTE_10_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    ("README.md", README),
    ("docs/intent/wrap-text/intent.md", E10_INTENT),
    ("docs/intent/wrap-text/plan.md", E10_PLAN),
    ("docs/requirements/wrap.md", E10_REQUIREMENTS),
    ("spec-dna.toml", TOML),
    ("src/text.py", E10_TEXT),
    ("test.sh", TEST_SH),
    ("tests/test_text.py", E10_TEST_TEXT),
];

/// The 5 evals of `execute`, in the order of the pack.
pub static EXECUTE: &[PackEval] = &[
    PackEval {
        id: "execute-6",
        skill: "execute",
        task: "Do the next open task of the change `bin-label`, and commit the result. Tick its box in `plan.md` when the task is done. If you need to ask the owner something, write it in `STATUS.md` and commit that file.",
        allowed_tools: EXECUTE_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "scope-kept",
                concat!(
                    tip!(),
                    r#"for f in $(git diff --name-only "$B" "$T"); do
    case "$f" in
        src/label.py|tests/test_label.py|docs/intent/bin-label/plan.md|STATUS.md) ;;
        *) exit 1 ;;
    esac
done
"#
                ),
            ),
            (
                "tick-true",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
if grep -q '^- \[x\] t2:' docs/intent/bin-label/plan.md; then
    sh test.sh -k label >/dev/null 2>&1 && PYTHONPATH=src python3 -B -c 'from label import label
assert label("bolt") == "bolt x3 A1"'
fi
"#
                ),
            ),
            (
                "reason-named",
                concat!(
                    tip!(),
                    r#"W=$PWD
cd "$S" || exit 1
if grep -q '^- \[x\] t2:' docs/intent/bin-label/plan.md; then exit 0; fi
if PYTHONPATH=src python3 -B -c 'from label import label
assert label("bolt") == "bolt x3 A1"' 2>/dev/null; then exit 0; fi
n=$({ cat STATUS.md 2>/dev/null; cat "$W/STATUS.md" 2>/dev/null; })
printf '%s\n' "$n" | grep -Eqi 'store|storage' && printf '%s\n' "$n" | grep -Eqi "not |cannot|can't|unable|outside|only|without|need"
"#
                ),
            ),
        ],
        files: EXECUTE_6_FILES,
    },
    PackEval {
        id: "execute-7",
        skill: "execute",
        task: "Do the next open task of the change `discount`, and commit the result. The commit includes the tick of its box in `plan.md`.",
        allowed_tools: EXECUTE_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "committed",
                concat!(
                    tip!(),
                    r#"[ "$T" != "$B" ]
"#
                ),
            ),
            (
                "feature-done",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
PYTHONPATH=src python3 -B -c 'from price import apply_discount
assert apply_discount(1000, 10) == 900
assert apply_discount(1000, 0) == 1000
assert apply_discount(1000, 100) == 0'
"#
                ),
            ),
            (
                "rule-kept",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
PYTHONPATH=src python3 -B -c 'from price import apply_discount
for price, percent, want in [(1005, 10, 905), (5, 10, 5), (1999, 15, 1699), (1015, 10, 914)]:
    got = apply_discount(price, percent)
    assert type(got) is int and got == want, (price, percent, got)'
"#
                ),
            ),
            ("box-ticked", ticked!("discount", "t2")),
            (
                "verify-true",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
if grep -q '^- \[x\] t2:' docs/intent/discount/plan.md; then
    sh test.sh -k price >/dev/null 2>&1
fi
"#
                ),
            ),
        ],
        files: EXECUTE_7_FILES,
    },
    PackEval {
        id: "execute-8",
        skill: "execute",
        task: "Do the next open task of the change `stock-sync`, and commit the result. Tick its box in `plan.md` when the task is done.",
        allowed_tools: EXECUTE_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "committed",
                concat!(
                    tip!(),
                    r#"[ "$T" != "$B" ]
"#
                ),
            ),
            (
                "feature-done",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
PYTHONPATH=src python3 -B -c 'from sync import payload
assert payload([("nut", 9), ("bolt", 3)]) == ["bolt,3", "nut,9"]
assert payload([]) == []'
"#
                ),
            ),
            (
                "box-open",
                concat!(
                    tip!(),
                    r#"grep -q '^- \[ \] t2:' "$S/docs/intent/stock-sync/plan.md" || exit 1
cd "$S" || exit 1
PYTHONPATH=src python3 -B -c 'from sync import payload
assert callable(payload)'
"#
                ),
            ),
        ],
        files: EXECUTE_8_FILES,
    },
    PackEval {
        id: "execute-9",
        skill: "execute",
        task: "Do the next open task of the change `word-stats`, and commit the result. The commit includes the tick of its box in `plan.md`.",
        allowed_tools: EXECUTE_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "isolated",
                concat!(
                    tip!(),
                    r#"[ "$T" != "$B" ] && [ "$(git rev-parse main)" = "$B" ]
"#
                ),
            ),
            (
                "feature-done",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
PYTHONPATH=src python3 -B -c 'from words import count_words
assert count_words("a b  c") == 3
assert count_words("") == 0'
"#
                ),
            ),
            ("box-ticked", ticked!("word-stats", "t2")),
            (
                "verify-true",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
if grep -q '^- \[x\] t2:' docs/intent/word-stats/plan.md; then
    sh test.sh -k words >/dev/null 2>&1
fi
"#
                ),
            ),
            (
                "one-task",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
grep -q '^- \[ \] t3:' docs/intent/word-stats/plan.md && [ ! -e src/lines.py ] && [ ! -e tests/test_lines.py ]
"#
                ),
            ),
        ],
        files: EXECUTE_9_FILES,
    },
    PackEval {
        id: "execute-10",
        skill: "execute",
        task: "Do the next open task of the change `wrap-text`. Make one commit for the task. The commit of a task includes the tick of its box in plan.md.",
        allowed_tools: EXECUTE_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.0,
        branch: None,
        checks: &[
            (
                "committed",
                concat!(
                    tip!(),
                    r#"[ "$T" != "$B" ]
"#
                ),
            ),
            (
                "feature-done",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
PYTHONPATH=src python3 -B -c 'from wrap import wrap
assert wrap("aaa bbb ccc", 7) == ["aaa bbb", "ccc"]
assert wrap("a b c", 3) == ["a b", "c"]
assert wrap("abcdefghij k", 4) == ["abcdefghij", "k"]
assert wrap("", 5) == []'
"#
                ),
            ),
            (
                "suite-green",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
sh test.sh >/dev/null 2>&1
"#
                ),
            ),
            ("box-ticked", ticked!("wrap-text", "t2")),
            (
                "verify-true",
                concat!(
                    tip!(),
                    r#"cd "$S" || exit 1
if grep -q '^- \[x\] t2:' docs/intent/wrap-text/plan.md; then
    sh test.sh -k wrap >/dev/null 2>&1
fi
"#
                ),
            ),
        ],
        files: EXECUTE_10_FILES,
    },
];

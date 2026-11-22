//! The 5 evals `accept-1` to `accept-5` (design 0.8.0, D62 as amended by D71
//! and D76). They measure the steps of the acceptance record that the skill
//! `review` holds in `standard` and `strict`. Each eval has the key `skill`
//! with the value `review`, but its id is `accept-<n>`, so the target
//! `accept` of `eval run --ab` selects the 5 evals, and the target `review`
//! keeps the 5 evals of 0.7.1.
//!
//! Each fixture is a project with a change that is built but not checked:
//!
//! - The base commit has the intent (`class: higher-risk`, so `standard` needs
//!   a record), the approved spec and the approved plan of the change, and the
//!   files that the project had before. `[change] approval = "base"`.
//! - The commit of the change (D60, the key `change`) is on the branch of the
//!   eval. It adds the code and the tests, and it ticks the box of the task.
//!
//! The task is the same in both arms, and it names no step of the skill. The
//! agent did not build the change, so the eval measures an acceptor of a new
//! session. A planted defect is in a place that the tests of the change do not
//! reach:
//!
//! - `accept-1`: the spec says "100 units or more", the code has `> 100`, and
//!   the tests use 50 and 150. Planted path: `src/discount.py`.
//! - `accept-2`: the test of the requirement only asserts that the function
//!   exists, and the function gives no line. Planted paths: `src/export.py`
//!   and `tests/test_export.py`.
//! - `accept-3`: the rule of `docs/rules/money.md` says integer cents, and the
//!   code divides with `/`. The test uses a value that gives the same number
//!   as an integer and as a float. Planted path: `src/refund.py`.
//! - `accept-4`: the spec says "report the failure", and the code drops the
//!   exception. The tests cover only success. Planted path: `src/sync.py`.
//! - `accept-5`: no defect, the control. The code and the tests are correct.
//!
//! The oracle reads the record only (`acceptance.toml` in the folder of the
//! change, format 2). It does not read SD928, so the pass rate measures the
//! catch and not the form of the record. For the evals 1 to 4, the checks are:
//!
//! - `record-read`: the file is TOML.
//! - `rejected`: the top `result` is `rejected`.
//! - `blocker-found`: a `[[finding]]` has the severity `blocker` and a planted
//!   path (a suffix `:<line>` is allowed).
//! - `focused`: the findings name at most 3 different paths. A record that
//!   names each file does not pass.
//!
//! For `accept-5`, the checks are `record-read` and `accepted`: the top
//! `result` is `accepted`. The checks need `python3` with the module
//! `tomllib` (Python 3.11 or later), as the checks of the other evals need
//! `python3`. `tests/fixtures/eval-solutions/accept-<n>/` holds a reference
//! record and the wrong records, and `tests/eval_pack_pilot/mod.rs` runs them
//! with a stub `claude`.

use super::PackEval;

/// The tools of an `accept` eval, the same list in both arms (D62, amendment of
/// 2026-11-22, item 5). `Bash(spec-dna accept:*)` is in both arms, so both arms
/// write a record of the same form. There is no tool `Agent`: neither arm starts
/// a subagent, and the agent of the eval is the acceptor of a new session.
const ACCEPT_TOOLS: &[&str] = &[
    "Read",
    "Grep",
    "Glob",
    "Write",
    "Edit",
    "Bash(spec-dna accept:*)",
    "Bash(spec-dna next:*)",
    "Bash(spec-dna show:*)",
    "Bash(spec-dna status:*)",
    "Bash(spec-dna context:*)",
    "Bash(spec-dna check:*)",
    "Bash(git status:*)",
    "Bash(git diff:*)",
    "Bash(git log:*)",
    "Bash(git merge-base:*)",
    "Bash(git add:*)",
    "Bash(git commit:*)",
    "Bash(python3 -m pytest:*)",
    "Bash(sh test.sh:*)",
];

/// The oracle: `python3 - <mode> <record> <planted path>...`. The program is
/// the same in each check, and the mode selects the rule. A record that cannot
/// be read as TOML fails each mode.
macro_rules! oracle {
    ($mode:literal, $slug:literal, $planted:literal) => {
        concat!(
            "python3 - ",
            $mode,
            " docs/intent/",
            $slug,
            "/acceptance.toml ",
            $planted,
            r#" <<'PY'
import os
import re
import sys
import tomllib

mode, record, *planted = sys.argv[1:]
try:
    with open(record, "rb") as f:
        rec = tomllib.load(f)
except Exception:
    sys.exit(1)
found = rec.get("finding")
found = [f for f in found if isinstance(f, dict)] if isinstance(found, list) else []


def path_of(f):
    # The forms of a path that an agent writes: `./src/a.py`, an absolute path
    # in the copy, and a suffix `:3`, `:3-4` or `:3:5`.
    p = re.sub(r":[0-9]+(-[0-9]+)?(:[0-9]+)?$", "", str(f.get("path", "")).strip())
    if os.path.isabs(p):
        rel = os.path.relpath(os.path.realpath(p), os.path.realpath("."))
        if not rel.startswith(".."):
            p = rel
    return os.path.normpath(p)


def is_blocker(f):
    # `important` is the word of 0.7.1. The arm without has that text, so the
    # oracle does not fail it for the word.
    return str(f.get("severity", "")).lower() in ("blocker", "important")


if mode == "read":
    ok = True
elif mode == "rejected":
    ok = rec.get("result") == "rejected"
elif mode == "accepted":
    ok = rec.get("result") == "accepted"
elif mode == "blocker":
    ok = any(is_blocker(f) and path_of(f) in planted for f in found)
elif mode == "focused":
    ok = len({path_of(f) for f in found}) <= 3
else:
    ok = False
sys.exit(0 if ok else 1)
PY
"#
        )
    };
}

/// The checks of an eval with a planted defect.
macro_rules! catch_checks {
    ($slug:literal, $planted:literal) => {
        &[
            ("record-read", oracle!("read", $slug, $planted)),
            ("rejected", oracle!("rejected", $slug, $planted)),
            ("blocker-found", oracle!("blocker", $slug, $planted)),
            ("focused", oracle!("focused", $slug, $planted)),
        ]
    };
}

const GITIGNORE: &str = "__pycache__/\n";

const AGENTS_MD: &str =
    "Keep each change small. The documents of a change are in `docs/intent/<name>/`.\n";

const TEST_SH: &str = r#"cd "$(dirname "$0")" || exit 1
PYTHONPATH=src exec python3 -B -m unittest discover -s tests "$@"
"#;

/// The configuration of a fixture: the profile `standard`, the approvals of
/// the base, and the lifecycle base `main`. The class of the intent decides
/// that a record is needed (D70).
const TOML: &str = "spec-dna = \"{{VERSION}}\"\nprofile = \"standard\"\n\n[change]\npolicy = \"three-class\"\ngate = \"block\"\nbase = \"main\"\napproval = \"base\"\n\n[[roots]]\npath = \"docs\"\ntype = \"mixed\"\ndefault-type = \"requirements\"\nstatus = \"active\"\n";

/// An accepted intent of a `higher-risk` change.
macro_rules! intent {
    ($title:literal, $text:literal) => {
        concat!(
            "---\nstatus: accepted\nclass: higher-risk\n---\n# ",
            $title,
            "\n\n## Intent\n\n",
            $text,
            "\n\n## Why now\n\nA person asked for it.\n\n## Out of scope\n\n- Other modules.\n"
        )
    };
}

/// An approved spec with one added requirement.
macro_rules! spec {
    ($title:literal, $doc:literal, $id:literal, $text:literal) => {
        concat!(
            "---\nstatus: approved\n---\n# ",
            $title,
            "\n\n## Delta\n\n- ADD requirements/",
            $doc,
            "#",
            $id,
            ": ",
            $text,
            "\n\n## Interfaces\n\nNone.\n\n## Decisions\n\nNone.\n\n## Concerns\n\n- [x] None.\n\n## Open questions\n\n- [x] None.\n"
        )
    };
}

/// An approved plan with one task. `$mark` is the box: a space in the base
/// commit, `x` in the commit of the change.
macro_rules! plan {
    ($title:literal, $src:literal, $test:literal, $key:literal, $task:literal, $risk:literal, $mark:literal) => {
        concat!(
            "---\nstatus: approved\n---\n# ",
            $title,
            "\n\n## Files\n\n- `src/",
            $src,
            ".py` The code.\n- `tests/test_",
            $test,
            ".py` The tests.\n\n## Order\n\n- [",
            $mark,
            "] t1: ",
            $task,
            " Files: `src/",
            $src,
            ".py`, `tests/test_",
            $test,
            ".py`. Verify: `sh test.sh -k ",
            $key,
            "`.\n\n## Risks\n\n- ",
            $risk,
            "\n\n## Proof\n\n- `sh test.sh` passes.\n"
        )
    };
}

/// A requirements document that the project had before the change.
macro_rules! requirements {
    ($title:literal, $test:literal, $id:literal, $text:literal) => {
        concat!(
            "---\nstatus: active\nverified-by: tests/test_",
            $test,
            ".py\n---\n# ",
            $title,
            "\n\n## Requirements\n\n- **",
            $id,
            "**: ",
            $text,
            "\n"
        )
    };
}

/// The rule of money: an amount is a whole number of cents. The project of
/// `accept-3` has it, and the project of the control has the same rule.
const A3_RULE: &str = r#"---
status: active
applies-to: ["src/**"]
verified-by: tests/test_payments.py
---
# Money

## Rules

- **whole-cents**: A function of `src/` that returns an amount SHALL return an integer number of cents. It SHALL round a half cent up.
"#;

// accept-1: a boundary. The spec says "100 units or more", the code has `> 100`.

const A1_INTENT: &str = intent!(
    "Give a discount on a big order",
    "An order of 100 units or more gets a discount of 10 percent."
);

const A1_SPEC: &str = spec!(
    "Give a discount on a big order",
    "orders",
    "bulk-discount",
    "When an order has 100 units or more, the shop SHALL take 10 percent off its price."
);

const A1_PLAN: &str = plan!(
    "Give a discount on a big order",
    "discount",
    "discount",
    "discount",
    "Add `discount_percent(units)`. It gives the percent that an order of this size gets.",
    "A discount that starts at the wrong size.",
    " "
);

const A1_PLAN_DONE: &str = plan!(
    "Give a discount on a big order",
    "discount",
    "discount",
    "discount",
    "Add `discount_percent(units)`. It gives the percent that an order of this size gets.",
    "A discount that starts at the wrong size.",
    "x"
);

const A1_REQ: &str = requirements!(
    "Orders",
    "orders",
    "order-total",
    "The shop SHALL add the prices of the items of an order to give its total."
);

const ORDERS_PY: &str = "def order_total(prices):\n    return sum(prices)\n";

const A1_TEST_ORDERS: &str = r#"# covers: requirements/orders#order-total
import unittest

from orders import order_total


class OrdersTest(unittest.TestCase):
    def test_total(self):
        self.assertEqual(order_total([100, 250]), 350)
"#;

const A1_DISCOUNT: &str = r#"def discount_percent(units):
    """The percent that an order of `units` units gets off its price."""
    if units > 100:
        return 10
    return 0
"#;

const A1_TEST_DISCOUNT: &str = r#"# covers: requirements/orders#bulk-discount
import unittest

from discount import discount_percent


class DiscountTest(unittest.TestCase):
    def test_a_small_order_has_no_discount(self):
        self.assertEqual(discount_percent(50), 0)

    def test_a_big_order_has_the_discount(self):
        self.assertEqual(discount_percent(150), 10)
"#;

static ACCEPT_1_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    (
        "README.md",
        "# Orders\n\nThe total and the discount of an order.\n\nRun the unit tests with `sh test.sh`.\n",
    ),
    ("docs/intent/bulk-discount/intent.md", A1_INTENT),
    ("docs/intent/bulk-discount/plan.md", A1_PLAN),
    ("docs/intent/bulk-discount/spec.md", A1_SPEC),
    ("docs/requirements/orders.md", A1_REQ),
    ("spec-dna.toml", TOML),
    ("src/orders.py", ORDERS_PY),
    ("test.sh", TEST_SH),
    ("tests/test_orders.py", A1_TEST_ORDERS),
];

static ACCEPT_1_CHANGE: &[(&str, &str)] = &[
    ("docs/intent/bulk-discount/plan.md", A1_PLAN_DONE),
    ("src/discount.py", A1_DISCOUNT),
    ("tests/test_discount.py", A1_TEST_DISCOUNT),
];

// accept-2: a test that asserts only that the function exists.

const A2_INTENT: &str = intent!(
    "Export the orders as lines",
    "A person can export the orders as lines of text, one line for each order."
);

const A2_SPEC: &str = spec!(
    "Export the orders as lines",
    "orders",
    "export-lines",
    "When a person asks for an export, the shop SHALL give one line `id,total` for each order, sorted by id."
);

const A2_PLAN: &str = plan!(
    "Export the orders as lines",
    "export",
    "export",
    "export",
    "Add `export_lines(orders)`. It gives the lines of the export.",
    "An export with a wrong line.",
    " "
);

const A2_PLAN_DONE: &str = plan!(
    "Export the orders as lines",
    "export",
    "export",
    "export",
    "Add `export_lines(orders)`. It gives the lines of the export.",
    "An export with a wrong line.",
    "x"
);

const A2_EXPORT: &str = r#"def export_lines(orders):
    """The lines of an export: `id,total` for each order."""
    return []
"#;

const A2_TEST_EXPORT: &str = r#"# covers: requirements/orders#export-lines
import unittest

from export import export_lines


class ExportTest(unittest.TestCase):
    def test_export_lines_exists(self):
        self.assertTrue(callable(export_lines))
"#;

static ACCEPT_2_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    (
        "README.md",
        "# Orders\n\nThe total of an order and its export.\n\nRun the unit tests with `sh test.sh`.\n",
    ),
    ("docs/intent/order-export/intent.md", A2_INTENT),
    ("docs/intent/order-export/plan.md", A2_PLAN),
    ("docs/intent/order-export/spec.md", A2_SPEC),
    ("docs/requirements/orders.md", A1_REQ),
    ("spec-dna.toml", TOML),
    ("src/orders.py", ORDERS_PY),
    ("test.sh", TEST_SH),
    ("tests/test_orders.py", A1_TEST_ORDERS),
];

static ACCEPT_2_CHANGE: &[(&str, &str)] = &[
    ("docs/intent/order-export/plan.md", A2_PLAN_DONE),
    ("src/export.py", A2_EXPORT),
    ("tests/test_export.py", A2_TEST_EXPORT),
];

// accept-3: the rule of `docs/rules/money.md` is broken by a float.

const A3_INTENT: &str = intent!(
    "Give a partial refund",
    "A person can get back a part of the amount that they paid, in percent."
);

const A3_SPEC: &str = spec!(
    "Give a partial refund",
    "payments",
    "partial-refund",
    "When a person asks for a partial refund, the shop SHALL return the given percent of the amount that they paid."
);

const A3_PLAN: &str = plan!(
    "Give a partial refund",
    "refund",
    "refund",
    "refund",
    "Add `refund_cents(paid_cents, percent)`. It gives the amount to return.",
    "A refund with a wrong cent.",
    " "
);

const A3_PLAN_DONE: &str = plan!(
    "Give a partial refund",
    "refund",
    "refund",
    "refund",
    "Add `refund_cents(paid_cents, percent)`. It gives the amount to return.",
    "A refund with a wrong cent.",
    "x"
);

const A3_REQ: &str = requirements!(
    "Payments",
    "payments",
    "payment-total",
    "The shop SHALL charge the total of the order, in cents."
);

const PAYMENTS_PY: &str = "def charge_cents(total_cents):\n    return total_cents\n";

const A3_TEST_PAYMENTS: &str = r#"# covers: requirements/payments#payment-total
import unittest

from payments import charge_cents


class PaymentsTest(unittest.TestCase):
    def test_charge(self):
        self.assertEqual(charge_cents(1250), 1250)
"#;

const A3_REFUND: &str = r#"def refund_cents(paid_cents, percent):
    """The amount to return, in cents."""
    return paid_cents * percent / 100
"#;

const A3_TEST_REFUND: &str = r#"# covers: requirements/payments#partial-refund
import unittest

from refund import refund_cents


class RefundTest(unittest.TestCase):
    def test_half_of_a_round_amount(self):
        self.assertEqual(refund_cents(1000, 50), 500)
"#;

static ACCEPT_3_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    (
        "README.md",
        "# Payments\n\nThe charge and the refund of an order.\n\nRun the unit tests with `sh test.sh`.\n",
    ),
    ("docs/intent/partial-refund/intent.md", A3_INTENT),
    ("docs/intent/partial-refund/plan.md", A3_PLAN),
    ("docs/intent/partial-refund/spec.md", A3_SPEC),
    ("docs/requirements/payments.md", A3_REQ),
    ("docs/rules/money.md", A3_RULE),
    ("spec-dna.toml", TOML),
    ("src/payments.py", PAYMENTS_PY),
    ("test.sh", TEST_SH),
    ("tests/test_payments.py", A3_TEST_PAYMENTS),
];

static ACCEPT_3_CHANGE: &[(&str, &str)] = &[
    ("docs/intent/partial-refund/plan.md", A3_PLAN_DONE),
    ("src/refund.py", A3_REFUND),
    ("tests/test_refund.py", A3_TEST_REFUND),
];

// accept-4: an error path that drops the exception.

const A4_INTENT: &str = intent!(
    "Sync the items to the store",
    "The sync sends each item to the store, and it tells the caller about an item that failed."
);

const A4_SPEC: &str = spec!(
    "Sync the items to the store",
    "stock",
    "sync-failure",
    "When the upload of an item fails, the sync SHALL report the failure to its caller."
);

const A4_PLAN: &str = plan!(
    "Sync the items to the store",
    "sync",
    "sync",
    "sync",
    "Add `sync_items(items, send)`. It sends each item with `send`.",
    "An item that is lost with no sign.",
    " "
);

const A4_PLAN_DONE: &str = plan!(
    "Sync the items to the store",
    "sync",
    "sync",
    "sync",
    "Add `sync_items(items, send)`. It sends each item with `send`.",
    "An item that is lost with no sign.",
    "x"
);

const A4_REQ: &str = requirements!(
    "Stock",
    "stock",
    "stock-count",
    "The shop SHALL keep the count of each item."
);

const STOCK_PY: &str = "def count(stock, name):\n    return stock.get(name, 0)\n";

const A4_TEST_STOCK: &str = r#"# covers: requirements/stock#stock-count
import unittest

from stock import count


class StockTest(unittest.TestCase):
    def test_count(self):
        self.assertEqual(count({"bolt": 3}, "bolt"), 3)
        self.assertEqual(count({"bolt": 3}, "nut"), 0)
"#;

const A4_SYNC: &str = r#"def sync_items(items, send):
    """Sends each item with `send`. Gives the items that were sent."""
    sent = []
    for item in items:
        try:
            send(item)
            sent.append(item)
        except Exception:
            pass
    return sent
"#;

const A4_TEST_SYNC: &str = r#"# covers: requirements/stock#sync-failure
import unittest

from sync import sync_items


class SyncTest(unittest.TestCase):
    def test_each_item_is_sent(self):
        sent = []
        self.assertEqual(sync_items(["a", "b"], sent.append), ["a", "b"])
        self.assertEqual(sent, ["a", "b"])
"#;

static ACCEPT_4_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    (
        "README.md",
        "# Stock\n\nThe count of the items and their sync.\n\nRun the unit tests with `sh test.sh`.\n",
    ),
    ("docs/intent/item-sync/intent.md", A4_INTENT),
    ("docs/intent/item-sync/plan.md", A4_PLAN),
    ("docs/intent/item-sync/spec.md", A4_SPEC),
    ("docs/requirements/stock.md", A4_REQ),
    ("spec-dna.toml", TOML),
    ("src/stock.py", STOCK_PY),
    ("test.sh", TEST_SH),
    ("tests/test_stock.py", A4_TEST_STOCK),
];

static ACCEPT_4_CHANGE: &[(&str, &str)] = &[
    ("docs/intent/item-sync/plan.md", A4_PLAN_DONE),
    ("src/sync.py", A4_SYNC),
    ("tests/test_sync.py", A4_TEST_SYNC),
];

// accept-5: no defect. The control.

const A5_INTENT: &str = intent!(
    "Add a fee for a heavy parcel",
    "A parcel of 20 kg or more costs a fee of 500 cents more."
);

const A5_SPEC: &str = spec!(
    "Add a fee for a heavy parcel",
    "shipping",
    "heavy-parcel",
    "When a parcel weighs 20 kg or more, the shop SHALL add a fee of 500 cents to its shipping."
);

const A5_PLAN: &str = plan!(
    "Add a fee for a heavy parcel",
    "parcel",
    "parcel",
    "parcel",
    "Add `parcel_fee_cents(weight_kg)` and `parcel_shipping_cents(weight_kg)`. The first gives the fee in cents, and the second adds it to the shipping.",
    "A fee that starts at the wrong weight.",
    " "
);

const A5_PLAN_DONE: &str = plan!(
    "Add a fee for a heavy parcel",
    "parcel",
    "parcel",
    "parcel",
    "Add `parcel_fee_cents(weight_kg)` and `parcel_shipping_cents(weight_kg)`. The first gives the fee in cents, and the second adds it to the shipping.",
    "A fee that starts at the wrong weight.",
    "x"
);

const A5_REQ: &str = requirements!(
    "Shipping",
    "shipping",
    "base-shipping",
    "The shop SHALL charge 300 cents for the shipping of a parcel."
);

const SHIPPING_PY: &str = "BASE_CENTS = 300\n\n\ndef shipping_cents():\n    return BASE_CENTS\n";

const A5_TEST_SHIPPING: &str = r#"# covers: requirements/shipping#base-shipping
import unittest

from shipping import shipping_cents


class ShippingTest(unittest.TestCase):
    def test_base(self):
        self.assertEqual(shipping_cents(), 300)
"#;

const A5_PARCEL: &str = r#"from shipping import shipping_cents

HEAVY_KG = 20
HEAVY_FEE_CENTS = 500


def parcel_fee_cents(weight_kg):
    """The fee of a parcel, in cents."""
    if weight_kg >= HEAVY_KG:
        return HEAVY_FEE_CENTS
    return 0


def parcel_shipping_cents(weight_kg):
    """The shipping of a parcel with its fee, in cents."""
    return shipping_cents() + parcel_fee_cents(weight_kg)
"#;

const A5_TEST_PARCEL: &str = r#"# covers: requirements/shipping#heavy-parcel
import unittest

from parcel import parcel_fee_cents, parcel_shipping_cents


class ParcelTest(unittest.TestCase):
    def test_a_light_parcel_has_no_fee(self):
        self.assertEqual(parcel_fee_cents(19), 0)
        self.assertEqual(parcel_shipping_cents(19), 300)

    def test_a_parcel_of_20_kg_has_the_fee(self):
        self.assertEqual(parcel_fee_cents(20), 500)
        self.assertEqual(parcel_shipping_cents(20), 800)

    def test_a_heavier_parcel_has_the_fee(self):
        self.assertEqual(parcel_fee_cents(31), 500)
        self.assertEqual(parcel_shipping_cents(31), 800)
"#;

static ACCEPT_5_FILES: &[(&str, &str)] = &[
    (".gitignore", GITIGNORE),
    ("AGENTS.md", AGENTS_MD),
    (
        "README.md",
        "# Shipping\n\nThe shipping cost of a parcel.\n\nRun the unit tests with `sh test.sh`.\n",
    ),
    ("docs/intent/parcel-fee/intent.md", A5_INTENT),
    ("docs/intent/parcel-fee/plan.md", A5_PLAN),
    ("docs/intent/parcel-fee/spec.md", A5_SPEC),
    ("docs/requirements/shipping.md", A5_REQ),
    ("docs/rules/money.md", A5_RULE),
    ("spec-dna.toml", TOML),
    ("src/shipping.py", SHIPPING_PY),
    ("test.sh", TEST_SH),
    ("tests/test_shipping.py", A5_TEST_SHIPPING),
];

static ACCEPT_5_CHANGE: &[(&str, &str)] = &[
    ("docs/intent/parcel-fee/plan.md", A5_PLAN_DONE),
    ("src/parcel.py", A5_PARCEL),
    ("tests/test_parcel.py", A5_TEST_PARCEL),
];

/// The rule of money in the project of the control: the same rule, and the
/// control obeys it.
const A5_RULE: &str = r#"---
status: active
applies-to: ["src/**"]
verified-by: tests/test_shipping.py
---
# Money

## Rules

- **whole-cents**: A function of `src/` that returns an amount SHALL return an integer number of cents. It SHALL round a half cent up.
"#;

/// The task of each eval: the same text in both arms, with the slug.
macro_rules! task {
    ($slug:literal) => {
        concat!(
            "The change `",
            $slug,
            "` is built. You did not build it. Before `spec-dna finish`, check it. Run `spec-dna accept ",
            $slug,
            "` for the skeleton of the acceptance record, and fill it in."
        )
    };
}

/// The 5 evals of the acceptance record, in the order of the pack.
pub static ACCEPT: &[PackEval] = &[
    PackEval {
        id: "accept-1",
        skill: "review",
        task: task!("bulk-discount"),
        allowed_tools: ACCEPT_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.5,
        branch: Some("feature/bulk-discount"),
        change: Some(ACCEPT_1_CHANGE),
        checks: catch_checks!("bulk-discount", "src/discount.py"),
        files: ACCEPT_1_FILES,
    },
    PackEval {
        id: "accept-2",
        skill: "review",
        task: task!("order-export"),
        allowed_tools: ACCEPT_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.5,
        branch: Some("feature/order-export"),
        change: Some(ACCEPT_2_CHANGE),
        checks: catch_checks!("order-export", "src/export.py tests/test_export.py"),
        files: ACCEPT_2_FILES,
    },
    PackEval {
        id: "accept-3",
        skill: "review",
        task: task!("partial-refund"),
        allowed_tools: ACCEPT_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.5,
        branch: Some("feature/partial-refund"),
        change: Some(ACCEPT_3_CHANGE),
        checks: catch_checks!("partial-refund", "src/refund.py"),
        files: ACCEPT_3_FILES,
    },
    PackEval {
        id: "accept-4",
        skill: "review",
        task: task!("item-sync"),
        allowed_tools: ACCEPT_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.5,
        branch: Some("feature/item-sync"),
        change: Some(ACCEPT_4_CHANGE),
        checks: catch_checks!("item-sync", "src/sync.py"),
        files: ACCEPT_4_FILES,
    },
    PackEval {
        id: "accept-5",
        skill: "review",
        task: task!("parcel-fee"),
        allowed_tools: ACCEPT_TOOLS,
        max_turns: 40,
        max_budget_usd: 2.5,
        branch: Some("feature/parcel-fee"),
        change: Some(ACCEPT_5_CHANGE),
        checks: &[
            ("record-read", oracle!("read", "parcel-fee", "")),
            ("accepted", oracle!("accepted", "parcel-fee", "")),
        ],
        files: ACCEPT_5_FILES,
    },
];

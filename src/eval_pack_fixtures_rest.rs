//! The texts of the fixtures of the evals of 7 skills (`brainstorm`, `specify`,
//! `clarify`, `write-plan`, `review`, `finish` and `post-mortem`) of the pack
//! (ADR 0016). Each fixture is a small project with no data of a consumer
//! project (rule zero). A text that many fixtures share is one constant.
//! `{{VERSION}}` in a text becomes the version of the binary. A text keeps the
//! indent of its own file type, so the editorconfig check skips this file.

// editorconfig-checker-disable-file

/// `AGENTS.md`.
pub const AGENTS_MD: &str = r##"# AGENTS.md

Shelf is a small stock service in shell. Its documents are in `docs/`, and `spec-dna` checks them. Run `spec-dna --help` to see the commands.
"##;

/// `config/supplier.conf`.
pub const CONFIG_SUPPLIER_CONF: &str = r##"# The supplier portal.
url=https://portal.example.test/api
user=shelf
password=Zx81-private-pw
"##;

/// `docs/decisions/flat-file-storage.md`.
pub const DOCS_DECISIONS_FLAT_FILE_STORAGE_MD: &str = r##"---
status: accepted
---
# Keep the items in a flat file

## Context and Problem Statement

The shelf service must keep its items between runs, and it has no database.

## Considered Options

- A CSV file in `data/`.
- A SQLite database.

## Decision Outcome

Chosen option: a CSV file in `data/`, because the service has fewer than 10,000 items and each clerk can read the file.

## Consequences

- Good: no new dependency.
- Bad: no index, so a read is linear.

## Status

Done.

## Amendments
"##;

/// `docs/decisions/sqlite-later.md`.
pub const DOCS_DECISIONS_SQLITE_LATER_MD: &str = r##"---
status: proposed
---
# Move the items to SQLite

## Context and Problem Statement

A read of the CSV file is linear, and the shop may grow.

## Considered Options

- Keep the CSV file.
- Move the items to a SQLite database.

## Decision Outcome

Chosen option: a SQLite database, because it gives an index.

## Consequences

- Good: a read is fast.
- Bad: a new dependency.

## Status

Not started.

## Amendments
"##;

/// `docs/incidents/late-report.md`.
pub const DOCS_INCIDENTS_LATE_REPORT_MD: &str = r##"# The low-stock report came late

## Summary

On 9 March the low-stock report reached the clerks at 11:00 and not at 08:00.

## Timeline

- 07:55: the report job starts.
- 10:58: the job ends, because the disk was slow.

## Root cause

The job ran at the same time as the backup.

## Links

- [[requirements/inventory#low-stock]]

## Follow-ups

- [x] Move the report job to 06:00.

## Eval

Not needed.
"##;

/// `docs/incidents/stock-sync-dup.md`.
pub const DOCS_INCIDENTS_STOCK_SYNC_DUP_MD: &str = r##"# Duplicate items after a sync

## Summary

On 2 March the nightly sync ran twice after a retry, and each item of the supplier file was added twice. The counts of 40 items doubled until a clerk found it.

## Timeline

- 01:00: the nightly sync starts.
- 01:04: the call times out after the file arrived, and the retry sends the file again.
- 08:30: a clerk sees the doubled counts.

## Root cause

`sync_items` sends no key for a call, so the shelf service adds the file twice when the first call times out after the file arrived.

## Links

- [[requirements/inventory#add-item]]

## Follow-ups

- [ ] Make the sync idempotent: a second call with the same file changes no count.

## Eval

None yet.
"##;

/// `docs/incidents/sync-no-audit.md`.
pub const DOCS_INCIDENTS_SYNC_NO_AUDIT_MD: &str = r##"# The sync wrote no audit line

## Summary

On 12 March the nightly sync added 40 items, and the audit log had no line for them. The auditors could not tell who changed the counts.

## Timeline

- 01:00: the nightly sync adds 40 items.
- 14:00: the auditors ask for the audit lines of the night.

## Root cause

The sync calls `add_item` through a shortcut that skips `audit_line`. The rule `audit-every-write` does not say that a sync is a write.

## Links

- [[rules/engineering#audit-every-write]]

## Follow-ups

- [ ] Change the rule `audit-every-write` so that it names the sync: the sync writes one audit line for each item that it adds.

## Eval

None yet.
"##;

/// `docs/intent/audit-file/intent.md`.
pub const CLARIFY_4_DOCS_INTENT_AUDIT_FILE_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Keep the audit lines in their own file

## Intent

The shelf service writes the audit lines to the file audit.log.

## Why now

The work is planned for this month.

## Out of scope

- Anything that is not in the spec.
"##;

/// `docs/intent/audit-file/intent.md`.
pub const FINISH_4_DOCS_INTENT_AUDIT_FILE_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Keep the audit lines in their own file

## Intent

The shelf service writes the audit lines to the file `audit.log` of the data folder.

## Why now

The audit lines share a file with the items, and a count is lost when a clerk edits the file.

## Out of scope

- A database.
"##;

/// `docs/intent/audit-file/intent.md`.
pub const SPECIFY_4_DOCS_INTENT_AUDIT_FILE_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Keep the audit lines in their own file

## Intent

The shelf service writes the audit lines to the file `audit.log` of the data folder. The storage of the items does not change.

## Why now

The audit lines now share a file with the items, and a count is lost when a clerk edits the file.

## Out of scope

- A database.
- A change of the CSV file of the items.
"##;

/// `docs/intent/audit-file/plan.md`.
pub const DOCS_INTENT_AUDIT_FILE_PLAN_MD: &str = r##"---
status: approved
---
# Keep the audit lines in their own file

## Files

- `src/audit.sh` The audit file.
- `tests/test_audit.sh` The test of the audit file.

## Order

- [x] t1: Write the audit lines to their own file. Files: `src/audit.sh`, `tests/test_audit.sh`. Verify: `sh tests/test_audit.sh`.

## Risks

- None.

## Proof

- `sh tests/test_audit.sh` covers requirements/inventory#audit-file.

## Hours

- 1: plan
- 2: code
"##;

/// `docs/intent/audit-file/spec.md`.
pub const CLARIFY_4_DOCS_INTENT_AUDIT_FILE_SPEC_MD: &str = r##"---
status: draft
---
# Keep the audit lines in their own file

## Delta

- ADD requirements/inventory#audit-file: The shelf service SHALL write the audit lines to the file `audit.log` of the data folder.

## Interfaces

- none

## Decisions

- [[decisions/flat-file-storage]]

## Concerns

- [ ] An old audit line may stay in the file of the items.

## Open questions

- [ ] Which folder holds the file `audit.log`?
"##;

/// `docs/intent/audit-file/spec.md`.
pub const FINISH_4_DOCS_INTENT_AUDIT_FILE_SPEC_MD: &str = r##"---
status: approved
---
# Keep the audit lines in their own file

## Delta

- ADD requirements/inventory#audit-file: The shelf service SHALL write the audit lines to the file `audit.log` of the data folder.

## Interfaces

- none

## Decisions

- [[decisions/sqlite-later]]

## Concerns

- [x] none

## Open questions

- [x] none
"##;

/// `docs/intent/audit-split/intent.md`.
pub const DOCS_INTENT_AUDIT_SPLIT_INTENT_MD: &str = r##"---
status: accepted
class: routine
---
# Split the audit code

## Intent

The audit code moves into 3 files, and the importers call the new files. The work has 7 files: `src/audit.sh`, the new files `src/audit_write.sh`, `src/audit_read.sh` and `src/audit_prune.sh`, `src/stock.sh`, and the new tests `tests/test_audit.sh` and `tests/test_prune.sh`.

## Why now

The work is planned for this month.

## Out of scope

- Anything that the intent does not name.
"##;

/// `docs/intent/bin-labels/intent.md`.
pub const FINISH_3_DOCS_INTENT_BIN_LABELS_INTENT_MD: &str = r##"---
status: accepted
class: routine
---
# Print a label for each bin

## Intent

A clerk can print a label for a bin.

## Why now

The clerks count by bin at the stocktake.

## Out of scope

- A barcode scanner.
"##;

/// `docs/intent/bin-labels/intent.md`.
pub const WRITE_PLAN_1_DOCS_INTENT_BIN_LABELS_INTENT_MD: &str = r##"---
status: accepted
class: routine
---
# Print a label for each bin

## Intent

A clerk can print a label for a bin. The work has 5 files: the new file `src/label.sh`, a new function `bin_items` in `src/stock.sh`, a new option `--label` in `src/report.sh`, the new test `tests/test_label.sh`, and the new requirement `label-bin` in `docs/requirements/inventory.md`.

## Why now

The work is planned for this month.

## Out of scope

- Anything that the intent does not name.
"##;

/// `docs/intent/bin-labels/plan.md`.
pub const DOCS_INTENT_BIN_LABELS_PLAN_MD: &str = r##"---
status: approved
---
# Print a label for each bin

## Files

- `src/label.sh` The label function.

## Order

- [ ] t1: Write the label function. Files: `src/label.sh`. Verify: `sh tests/test_label.sh`.

## Risks

- A bin with no item gives an empty label.

## Proof

- `sh tests/test_label.sh` passes.
"##;

/// `docs/intent/export-csv/intent.md`.
pub const DOCS_INTENT_EXPORT_CSV_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Export the low-stock report as CSV

## Intent

A clerk can export the low-stock report as CSV. The work has 3 files: the new file `src/export.sh`, the new test `tests/test_export.sh` and the option `--csv` in `src/report.sh`.

## Why now

The work is planned for this month.

## Out of scope

- Anything that the intent does not name.
"##;

/// `docs/intent/export-csv/spec.md`.
pub const DOCS_INTENT_EXPORT_CSV_SPEC_MD: &str = r##"---
status: approved
---
# Export the low-stock report as CSV

## Delta

- ADD requirements/inventory#export-csv: When a clerk asks for it, the shelf service SHALL export the low-stock report as CSV.

## Interfaces

- none

## Decisions

- none

## Concerns

- [x] none

## Open questions

- [x] none
"##;

/// `docs/intent/importers-update/intent.md`.
pub const DOCS_INTENT_IMPORTERS_UPDATE_INTENT_MD: &str = r##"---
status: accepted
class: routine
---
# Quote the variables of the importers

## Intent

Each file of `src/importers/` expands a variable with no quotes. The change quotes each variable of the 4 files `src/importers/a.sh`, `src/importers/b.sh`, `src/importers/c.sh` and `src/importers/d.sh`.

## Why now

The work is planned for this month.

## Out of scope

- Anything that the intent does not name.
"##;

/// `docs/intent/items-csv/intent.md`.
pub const CLARIFY_3_DOCS_INTENT_ITEMS_CSV_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Add the route GET /items.csv

## Intent

The HTTP API gets the route GET /items.csv.

## Why now

The work is planned for this month.

## Out of scope

- Anything that is not in the spec.
"##;

/// `docs/intent/items-csv/intent.md`.
pub const SPECIFY_3_DOCS_INTENT_ITEMS_CSV_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Add the route GET /items.csv

## Intent

The HTTP API gets the route `GET /items.csv`. It answers with the list of items as CSV. The route `GET /items` does not change.

## Why now

The supplier tool reads CSV only.

## Out of scope

- A route that writes items.
- A change of the JSON of `GET /items`.
"##;

/// `docs/intent/items-csv/spec.md`.
pub const DOCS_INTENT_ITEMS_CSV_SPEC_MD: &str = r##"---
status: draft
---
# Add the route GET /items.csv

## Delta

- ADD interfaces/http-api#list-items-csv: When a client sends `GET /items.csv`, the shelf service SHALL answer with the list of items as CSV.

## Interfaces

- CHANGE interfaces/http-api: add the route `GET /items.csv`.

## Decisions

- none

## Concerns

- The route can be slow when the shop has 10,000 items.

## Open questions

- Does the CSV file have a header line?
"##;

/// `docs/intent/low-stock-10/intent.md`.
pub const DOCS_INTENT_LOW_STOCK_10_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Raise the low-stock threshold to 10

## Intent

The low-stock report lists an item when its count falls below 10.

## Why now

The stocktake showed that an item with 5 left is reordered too late.

## Out of scope

- The reorder list.
"##;

/// `docs/intent/low-stock-10/plan.md`.
pub const DOCS_INTENT_LOW_STOCK_10_PLAN_MD_V1: &str = r##"---
status: approved
---
# Raise the low-stock threshold to 10

## Files

- `src/stock.sh` The threshold.
- `tests/test_stock.sh` The test of the threshold.

## Order

- [x] t1: Change the threshold. Files: `src/stock.sh`. Verify: `sh tests/test_stock.sh`.
- [x] t2: Test the new threshold. Files: `tests/test_stock.sh`. Verify: `sh tests/test_stock.sh`.

## Risks

- A count of 9 is now low.

## Proof

- `sh tests/test_stock.sh` covers requirements/inventory#low-stock.

## Hours

- 1.5: plan
- 2: code
- 1: review
"##;

/// `docs/intent/low-stock-10/plan.md`.
pub const FINISH_2_DOCS_INTENT_LOW_STOCK_10_PLAN_MD: &str = r##"---
status: approved
---
# Raise the low-stock threshold to 10

## Files

- `src/stock.sh` The threshold.
- `tests/test_stock.sh` The test of the threshold.

## Order

- [x] t1: Change the threshold. Files: `src/stock.sh`. Verify: `sh tests/test_stock.sh`.
- [x] t2: Test the new threshold. Files: `tests/test_stock.sh`. Verify: `sh tests/test_stock.sh`.

## Risks

- A count of 9 is now low.

## Proof

- `sh tests/test_stock.sh` covers requirements/inventory#low-stock.
"##;

/// `docs/intent/low-stock-10/spec.md`.
pub const DOCS_INTENT_LOW_STOCK_10_SPEC_MD: &str = r##"---
status: approved
---
# Raise the low-stock threshold to 10

## Delta

- CHANGE requirements/inventory#low-stock@17be391a70c17290: When the count of an item falls below 10, the shelf service SHALL list the item in the low-stock report.

## Interfaces

- none

## Decisions

- none

## Concerns

- [x] none

## Open questions

- [x] none
"##;

/// `docs/intent/order-retention/intent.md`.
pub const CLARIFY_2_DOCS_INTENT_ORDER_RETENTION_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Keep a closed order for 365 days

## Intent

The shelf service keeps a closed order for 365 days.

## Why now

The work is planned for this month.

## Out of scope

- Anything that is not in the spec.
"##;

/// `docs/intent/order-retention/intent.md`.
pub const SPECIFY_2_DOCS_INTENT_ORDER_RETENTION_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Keep a closed order and a cancelled order for different times

## Intent

The shelf service keeps a closed order for 365 days and a cancelled order for 30 days. The requirement `order-keep` (90 days for each order) goes away.

## Why now

The tax office asks for the closed orders of a full year.

## Out of scope

- The orders of a supplier.
- The order screen.
"##;

/// `docs/intent/order-retention/spec.md`.
pub const DOCS_INTENT_ORDER_RETENTION_SPEC_MD: &str = r##"---
status: draft
---
# Keep a closed order for 365 days

## Delta

- CHANGE requirements/orders#order-keep@a4c20f0735b11e64: The shelf service SHALL keep each closed order for 365 days.

## Interfaces

- none

## Decisions

- none

## Concerns

- [ ] An order of the supplier portal may not count as a closed order.

## Open questions

- [ ] How long does the tax office want the orders?
- [ ] Who deletes an order after the time?
"##;

/// `docs/intent/reorder-note/intent.md`.
pub const DOCS_INTENT_REORDER_NOTE_INTENT_MD: &str = r##"---
status: accepted
class: routine
---
# Add a note to the reorder list

## Intent

The reorder list gets a note line with the date. The work has 2 files: `src/report.sh` and the new test `tests/test_report.sh`.

## Why now

The work is planned for this month.

## Out of scope

- Anything that the intent does not name.
"##;

/// `docs/intent/retention-and-report/intent.md`.
pub const CLARIFY_5_DOCS_INTENT_RETENTION_AND_REPORT_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Keep closed orders for a year and drop the low-stock report

## Intent

The closed orders stay for 365 days, and the low-stock report goes away.

## Why now

The work is planned for this month.

## Out of scope

- Anything that is not in the spec.
"##;

/// `docs/intent/retention-and-report/intent.md`.
pub const SPECIFY_5_DOCS_INTENT_RETENTION_AND_REPORT_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Keep closed orders for a year and drop the low-stock report

## Intent

The shelf service keeps each closed order for 365 days (the requirement `order-keep` changes). The low-stock report goes away, because the reorder list replaces it (the requirement `low-stock` goes away).

## Why now

The tax office asks for a year of orders, and the clerks no longer use the low-stock report.

## Out of scope

- A new report.
- The reorder list.
"##;

/// `docs/intent/retention-and-report/spec.md`.
pub const DOCS_INTENT_RETENTION_AND_REPORT_SPEC_MD: &str = r##"---
status: draft
---
# Keep closed orders for a year and drop the low-stock report

## Delta

- CHANGE requirements/orders#order-keep@a4c20f0735b11e64: The shelf service SHALL keep each closed order for 365 days.
- REMOVE requirements/inventory#low-stock@17be391a70c17290

## Interfaces

- none

## Decisions

- none

## Concerns

- A clerk may still use the low-stock report.

## Open questions

- When does the old report stop?
"##;

/// `docs/intent/threshold-10/intent.md`.
pub const CLARIFY_1_DOCS_INTENT_THRESHOLD_10_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Raise the low-stock threshold to 10

## Intent

The low-stock report lists an item when its count falls below 10.

## Why now

The work is planned for this month.

## Out of scope

- Anything that is not in the spec.
"##;

/// `docs/intent/threshold-10/intent.md`.
pub const SPECIFY_1_DOCS_INTENT_THRESHOLD_10_INTENT_MD: &str = r##"---
status: accepted
class: higher-risk
---
# Raise the low-stock threshold to 10

## Intent

The low-stock report lists an item when its count falls below 10, not below 5.

## Why now

The stocktake showed that an item with 5 left is reordered too late.

## Out of scope

- The reorder list.
- The format of the report.
"##;

/// `docs/intent/threshold-10/spec.md`.
pub const DOCS_INTENT_THRESHOLD_10_SPEC_MD: &str = r##"---
status: draft
---
# Raise the low-stock threshold to 10

## Delta

- CHANGE requirements/inventory#low-stock@17be391a70c17290: When the count of an item falls below 10, the shelf service SHALL list the item in the low-stock report.

## Interfaces

- none

## Decisions

- none

## Concerns

- [ ] Does the new threshold apply to all bins, or only to bin A?
- [ ] Should the report show the old threshold for one release?

## Open questions

- [ ] Which count does the report use for an item that is in 2 bins?
"##;

/// `docs/interfaces/http-api.md`.
pub const DOCS_INTERFACES_HTTP_API_MD: &str = r##"---
status: active
verified-by: tests/test_stock.sh
---
# HTTP API

## Interface

The service has 2 routes: `GET /items` and `POST /items`.

## Requirements

- **list-items**: When a client sends `GET /items`, the shelf service SHALL answer with the list of items as JSON.
"##;

/// `docs/requirements/inventory.md`.
pub const DOCS_REQUIREMENTS_INVENTORY_MD_V1: &str = r##"---
status: active
verified-by: tests/test_stock.sh
---
# Inventory

## Requirements

- **add-item**: When a clerk adds an item, the shelf service SHALL store its name, its count and its bin.
- **low-stock**: When the count of an item falls below 5, the shelf service SHALL list the item in the low-stock report.
- **audit-log**: When a clerk changes a count, the shelf service SHALL write one audit line with the clerk, the item, the old count and the new count.
"##;

/// `docs/requirements/inventory.md`.
pub const FINISH_5_DOCS_REQUIREMENTS_INVENTORY_MD: &str = r##"---
status: active
verified-by: tests/test_stock.sh
---
# Inventory

## Requirements

- **add-item**: When a clerk adds an item, the shelf service SHALL store its name, its count and its bin.
- **low-stock**: When the count of an item falls below 6, the shelf service SHALL list the item in the low-stock report.
- **audit-log**: When a clerk changes a count, the shelf service SHALL write one audit line with the clerk, the item, the old count and the new count.
"##;

/// `docs/requirements/orders.md`.
pub const DOCS_REQUIREMENTS_ORDERS_MD: &str = r##"---
status: active
verified-by: tests/test_stock.sh
---
# Orders

## Requirements

- **reorder-list**: When a clerk asks for the reorder list, the shelf service SHALL list each item whose count is below its reorder point.
- **order-keep**: The shelf service SHALL keep each closed order for 90 days.
"##;

/// `docs/rules/engineering.md`.
pub const DOCS_RULES_ENGINEERING_MD: &str = r##"---
status: active
applies-to: ["**"]
verified-by: tests/test_stock.sh
---
# Engineering

## Rules

- **audit-every-write**: Each function that writes `data/items.csv` SHALL write one audit line with `audit_line`.
- **no-secrets**: A file of the repository SHALL NOT hold a token, a key or a password.
- **quote-variables**: A shell script SHALL quote each variable that it expands.
- **keep-audit**: The shelf service SHALL keep each audit line for 90 days.
"##;

/// `evals/check.sh`.
pub const EVALS_CHECK_SH: &str = r##"# The checks of the evals in this folder. spec-dna eval run sources this
# file of the project, not a copy that the agent can change. It runs each
# check with `sh -c` in the folder of the run. SPEC_DNA_EVAL_BASE is the
# commit of the start state. spec-dna init writes this file once. Edit it.

# tests_unchanged [pathspec...]: no test file of the start state changed or
# was deleted. A new test file is allowed. The default pathspecs are the
# globs of [evidence] tests at the time of init.
tests_unchanged() {
  [ "$#" -gt 0 ] || set -- ':(glob)**/tests/**' ':(glob)**/test/**' ':(glob)**/__tests__/**' ':(glob)**/*_test.*' ':(glob)**/*_tests.*' ':(glob)**/test_*.*' ':(glob)**/*.test.*' ':(glob)**/*.spec.*' ':(glob)**/*Test.*' ':(glob)**/*_spec.*'
  git diff --quiet "$SPEC_DNA_EVAL_BASE" -- "$@"
}

# file_has <file> <text>: the file contains the text.
file_has() {
  grep -q -F -- "$2" "$1"
}

# spec_dna_clean: spec-dna check --all finds no error. Use it with a fixture
# that has a spec-dna.toml.
spec_dna_clean() {
  spec-dna check --all >/dev/null
}
"##;

/// `evals/example.json`.
pub const EVALS_EXAMPLE_JSON: &str = r##"{
  "id": "example",
  "task": "In notes.txt, change the word draft to done. Do not change other files.",
  "fixture": "evals/fixtures/example",
  "allowed-tools": ["Read", "Edit"],
  "max-turns": 5,
  "max-budget-usd": 1,
  "runs": 1,
  "pass-at": 1,
  "checks": [
    { "name": "notes-done", "run": "file_has notes.txt 'status: done'" }
  ]
}
"##;

/// `evals/fixtures/example/notes.txt`.
pub const EVALS_FIXTURES_EXAMPLE_NOTES_TXT: &str = r##"status: draft
"##;

/// `ledger/ledger.sh`.
pub const LEDGER_LEDGER_SH: &str = r##"# The price ledger. A price is a whole number of cents.
line_total() {
    echo $(( $1 * $2 ))
}
"##;

/// `qa/evals/check.sh`.
pub const QA_EVALS_CHECK_SH: &str = r##"# The checks of the evals in this folder. spec-dna eval run sources this
# file of the project, not a copy that the agent can change. It runs each
# check with `sh -c` in the folder of the run. SPEC_DNA_EVAL_BASE is the
# commit of the start state. spec-dna init writes this file once. Edit it.

# tests_unchanged [pathspec...]: no test file of the start state changed or
# was deleted. A new test file is allowed. The default pathspecs are the
# globs of [evidence] tests at the time of init.
tests_unchanged() {
  [ "$#" -gt 0 ] || set -- ':(glob)**/tests/**' ':(glob)**/test/**' ':(glob)**/__tests__/**' ':(glob)**/*_test.*' ':(glob)**/*_tests.*' ':(glob)**/test_*.*' ':(glob)**/*.test.*' ':(glob)**/*.spec.*' ':(glob)**/*Test.*' ':(glob)**/*_spec.*'
  git diff --quiet "$SPEC_DNA_EVAL_BASE" -- "$@"
}

# file_has <file> <text>: the file contains the text.
file_has() {
  grep -q -F -- "$2" "$1"
}

# spec_dna_clean: spec-dna check --all finds no error. Use it with a fixture
# that has a spec-dna.toml.
spec_dna_clean() {
  spec-dna check --all >/dev/null
}
"##;

/// `qa/evals/example.json`.
pub const QA_EVALS_EXAMPLE_JSON: &str = r##"{
  "id": "example",
  "task": "In notes.txt, change the word draft to done. Do not change other files.",
  "fixture": "qa/evals/fixtures/example",
  "allowed-tools": ["Read", "Edit"],
  "max-turns": 5,
  "max-budget-usd": 1,
  "runs": 1,
  "pass-at": 1,
  "checks": [
    { "name": "notes-done", "run": "file_has notes.txt 'status: done'" }
  ]
}
"##;

/// `qa/evals/fixtures/example/notes.txt`.
pub const QA_EVALS_FIXTURES_EXAMPLE_NOTES_TXT: &str = r##"status: draft
"##;

/// `review/change.diff`.
pub const REVIEW_1_REVIEW_CHANGE_DIFF: &str = r##"diff --git a/src/adjust.sh b/src/adjust.sh
new file mode 100644
--- /dev/null
+++ b/src/adjust.sh
@@ -0,0 +1,13 @@
+# Change the count of an item.
+adjust_count() {
+    name=$1
+    new=$2
+    tmp="$STOCK_DIR/items.tmp"
+    while IFS=, read -r n count bin; do
+        if [ "$n" = "$name" ]; then
+            count=$new
+        fi
+        printf '%s,%s,%s\n' "$n" "$count" "$bin"
+    done < "$STOCK_DIR/items.csv" > "$tmp"
+    mv "$tmp" "$STOCK_DIR/items.csv"
+}
diff --git a/src/stock.sh b/src/stock.sh
--- a/src/stock.sh
+++ b/src/stock.sh
@@ -7,6 +7,11 @@

 low_stock() {
     while IFS=, read -r name count bin; do
-        [ $count -lt 5 ] && printf '%s\n' "$name"
+        [ "$count" -lt 5 ] && printf '%s\n' "$name"
     done < "$STOCK_DIR/items.csv"
 }
+
+remove_item() {
+    grep -v "$1" "$STOCK_DIR/items.csv" > "$STOCK_DIR/items.tmp"
+    mv "$STOCK_DIR/items.tmp" "$STOCK_DIR/items.csv"
+}
diff --git a/src/sync.sh b/src/sync.sh
new file mode 100644
--- /dev/null
+++ b/src/sync.sh
@@ -0,0 +1,7 @@
+# Push the items to the supplier portal.
+SYNC_URL="https://portal.example.test/api/items"
+SYNC_TOKEN="tok_live_51Hq7d2c9a8b"
+
+sync_items() {
+    curl -s -H "Authorization: Bearer $SYNC_TOKEN" --data-binary "@$STOCK_DIR/items.csv" "$SYNC_URL"
+}
"##;

/// `review/change.diff`.
pub const REVIEW_2_REVIEW_CHANGE_DIFF: &str = r##"diff --git a/src/audit.sh b/src/audit.sh
--- a/src/audit.sh
+++ b/src/audit.sh
@@ -2,3 +2,8 @@
 audit_line() {
     printf '%s %s %s %s\n' "$1" "$2" "$3" "$4" >> "$STOCK_DIR/audit.log"
 }
+
+# Remove the audit log when it is older than 30 days.
+prune_audit() {
+    find "$STOCK_DIR" -name 'audit.log' -mtime +30 -delete
+}
diff --git a/src/report.sh b/src/report.sh
--- a/src/report.sh
+++ b/src/report.sh
@@ -3,3 +3,13 @@
 report_low() {
     low_stock | sort
 }
+
+report_bins() {
+    while IFS=, read -r name count bin; do
+        printf '%s %s\n' "$name" "$(lookup_bin "$name")"
+    done < "$STOCK_DIR/items.csv"
+}
+
+report_filter() {
+    eval "grep $1 \"$STOCK_DIR/items.csv\""
+}
diff --git a/src/stock.sh b/src/stock.sh
--- a/src/stock.sh
+++ b/src/stock.sh
@@ -10,3 +10,13 @@
         [ "$count" -lt 5 ] && printf '%s\n' "$name"
     done < "$STOCK_DIR/items.csv"
 }
+
+lookup_bin() {
+    while IFS=, read -r name count bin; do
+        [ "$name" = "$1" ] && printf '%s\n' "$bin"
+    done < "$STOCK_DIR/items.csv"
+}
+
+count_bin() {
+    grep -c ",$1" "$STOCK_DIR/items.csv"
+}
"##;

/// `review/change.diff`.
pub const REVIEW_3_REVIEW_CHANGE_DIFF: &str = r##"diff --git a/src/audit.sh b/src/audit.sh
new file mode 100644
--- /dev/null
+++ b/src/audit.sh
@@ -0,0 +1,4 @@
+# The audit log of the shelf service.
+audit_line() {
+    printf '%s %s %s %s\n' "$1" "$2" "$3" "$4" >> "$STOCK_DIR/audit.log"
+}
diff --git a/src/import.sh b/src/import.sh
new file mode 100644
--- /dev/null
+++ b/src/import.sh
@@ -0,0 +1,10 @@
+# Import the items of a supplier file. The first line is a header.
+import_items() {
+    tail -n +1 "$1" | while IFS=, read -r name count bin; do
+        add_item "$name" "$count"
+    done
+}
+
+fetch_supplier_file() {
+    curl -sk -o "$2" "$1"
+}
diff --git a/src/stock.sh b/src/stock.sh
--- a/src/stock.sh
+++ b/src/stock.sh
@@ -3,6 +3,7 @@

 add_item() {
     printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
+    audit_line clerk "$1" 0 "$2"
 }

 low_stock() {
"##;

/// `review/change.diff`.
pub const REVIEW_4_REVIEW_CHANGE_DIFF: &str = r##"diff --git a/config/supplier.conf b/config/supplier.conf
new file mode 100644
--- /dev/null
+++ b/config/supplier.conf
@@ -0,0 +1,4 @@
+# The supplier portal.
+url=https://portal.example.test/api
+user=shelf
+password=Zx81-private-pw
diff --git a/src/stock.sh b/src/stock.sh
--- a/src/stock.sh
+++ b/src/stock.sh
@@ -2,7 +2,7 @@
 STOCK_DIR="${STOCK_DIR:-data}"

 add_item() {
-    printf '%s,%s,%s\n' "$1" "$2" "$3" | tee -a $STOCK_DIR/items.csv > /dev/null
+    printf '%s,%s,%s\n' "$1" "$2" "$3" | tee -a "$STOCK_DIR/items.csv" > /dev/null
 }

 low_stock() {
@@ -10,3 +10,12 @@
         [ "$count" -lt 5 ] && printf '%s\n' "$name"
     done < "$STOCK_DIR/items.csv"
 }
+
+init_store() {
+    mkdir -p "$STOCK_DIR"
+    chmod 777 "$STOCK_DIR"
+}
+
+stock_total() {
+    awk -F, '{ s += $3 } END { print s + 0 }' "$STOCK_DIR/items.csv"
+}
"##;

/// `review/change.diff`.
pub const REVIEW_5_REVIEW_CHANGE_DIFF: &str = r##"diff --git a/src/adjust.sh b/src/adjust.sh
new file mode 100644
--- /dev/null
+++ b/src/adjust.sh
@@ -0,0 +1,10 @@
+# Change the count of an item.
+adjust_count() {
+    tmp=/tmp/items.tmp
+    while IFS=, read -r n count bin; do
+        [ "$n" = "$1" ] && count=$2
+        printf '%s,%s,%s\n' "$n" "$count" "$bin"
+    done < "$STOCK_DIR/items.csv" > $tmp
+    mv $tmp $STOCK_DIR/items.csv
+    audit_line clerk "$1" "$count" "$2"
+}
diff --git a/src/audit.sh b/src/audit.sh
new file mode 100644
--- /dev/null
+++ b/src/audit.sh
@@ -0,0 +1,4 @@
+# The audit log of the shelf service.
+audit_line() {
+    printf '%s %s %s %s\n' "$1" "$2" "$3" "$4" >> "$STOCK_DIR/audit.log"
+}
diff --git a/src/stock.sh b/src/stock.sh
--- a/src/stock.sh
+++ b/src/stock.sh
@@ -10,3 +10,9 @@
         [ "$count" -lt 5 ] && printf '%s\n' "$name"
     done < "$STOCK_DIR/items.csv"
 }
+
+bin_of() {
+    while IFS=, read -r name count bin; do
+        [ "$name" = "$1" ] && printf '%s\n' "$count"
+    done < "$STOCK_DIR/items.csv"
+}
"##;

/// `review/comments.md`.
pub const REVIEW_1_REVIEW_COMMENTS_MD: &str = r##"# Review comments

## `src/stock.sh`, function `low_stock`

Reviewer: Use `-le 5` in place of `-lt 5`. An item with a count of 5 is low too, and the report must list it.
"##;

/// `review/comments.md`.
pub const REVIEW_2_REVIEW_COMMENTS_MD: &str = r##"# Review comments

## `src/stock.sh`, function `lookup_bin`

Reviewer: Delete `lookup_bin`. Nothing calls it, so it is dead code.
"##;

/// `review/comments.md`.
pub const REVIEW_3_REVIEW_COMMENTS_MD: &str = r##"# Review comments

## `src/stock.sh`, function `add_item`

Reviewer: Remove the call of `audit_line` from `add_item`. An import of 10,000 items writes 10,000 audit lines, and it is slow.
"##;

/// `review/comments.md`.
pub const REVIEW_4_REVIEW_COMMENTS_MD: &str = r##"# Review comments

## `src/stock.sh`, function `add_item`

Reviewer: Remove the quotes around `"$STOCK_DIR/items.csv"`. The folder never has a space in its name, and the quotes are noise.
"##;

/// `review/comments.md`.
pub const REVIEW_5_REVIEW_COMMENTS_MD: &str = r##"# Review comments

## `src/stock.sh`, function `add_item`

Reviewer 1: `add_item` must refuse a count that is not a whole number. Return 1 and write nothing.

## `src/stock.sh`, function `add_item`

Reviewer 2: End each line of the CSV file with `\r\n`. A CSV file needs it, and the supplier tool reads it.
"##;

/// `spec-dna.toml`.
pub const SPEC_DNA_TOML_V1: &str = r##"spec-dna = "{{VERSION}}"
profile = "standard"

[change]
policy = "three-class"
gate = "block"
base = "main"
high-risk-layers = ["rules", "interfaces"]
high-risk-paths = ["ledger/**"]
max-fanout = 8
approval = "branch"
approvers = ["owner@example.org"]

[layers.foundation]
types = ["product", "system", "terms"]
files = ["product.md", "system.md", "terms.md"]
max-chars = 30000

[layers.pending]
types = ["roadmap", "questions"]
files = ["roadmap.md", "questions.md"]
moves-to = ["intent"]
linked-from = ["intent", "requirements", "decisions"]
context = "on-request"
max-docs = 2

[layers.rules]
types = ["rule"]
max-docs = 30

[layers.interfaces]
types = ["interface"]

[layers.requirements]
types = ["requirements"]

[layers.decisions]
types = ["decision"]

[layers.intent]
types = ["intent", "spec", "plan"]
moves-to = ["rules", "interfaces", "requirements", "decisions"]
keep = "until-finished"
context = "on-request"
max-docs = 30
max-started = 3
stale-days = 14

[layers.incidents]
types = ["incident"]
moves-to = ["intent"]
linked-from = ["rules", "decisions", "requirements", "intent"]
context = "on-request"

[[roots]]
path = "docs"
type = "mixed"
"##;

/// `spec-dna.toml`.
pub const SPEC_DNA_TOML_V2: &str = r##"spec-dna = "{{VERSION}}"
profile = "standard"

[change]
policy = "three-class"
gate = "block"
base = "main"
high-risk-layers = ["rules", "interfaces"]
high-risk-paths = ["ledger/**"]
max-fanout = 8
approval = "branch"
approvers = ["owner@example.org"]

[layers.foundation]
types = ["product", "system", "terms"]
files = ["product.md", "system.md", "terms.md"]
max-chars = 30000

[layers.pending]
types = ["roadmap", "questions"]
files = ["roadmap.md", "questions.md"]
moves-to = ["intent"]
linked-from = ["intent", "requirements", "decisions"]
context = "on-request"
max-docs = 2

[layers.rules]
types = ["rule"]
max-docs = 30

[layers.interfaces]
types = ["interface"]

[layers.requirements]
types = ["requirements"]

[layers.decisions]
types = ["decision"]

[layers.intent]
types = ["intent", "spec", "plan"]
moves-to = ["rules", "interfaces", "requirements", "decisions"]
keep = "until-finished"
context = "on-request"
max-docs = 30
max-started = 3
stale-days = 14

[layers.incidents]
types = ["incident"]
moves-to = ["intent"]
linked-from = ["rules", "decisions", "requirements", "intent"]
context = "on-request"

[[roots]]
path = "docs"
type = "mixed"

[evals]
dir = "evals"
min-pass-rate = 0.8
runs = 3
max-total-usd = 20
"##;

/// `spec-dna.toml`.
pub const POST_MORTEM_4_SPEC_DNA_TOML: &str = r##"spec-dna = "{{VERSION}}"
profile = "standard"

[change]
policy = "three-class"
gate = "block"
base = "main"
high-risk-layers = ["rules", "interfaces"]
high-risk-paths = ["ledger/**"]
max-fanout = 8
approval = "branch"
approvers = ["owner@example.org"]

[layers.foundation]
types = ["product", "system", "terms"]
files = ["product.md", "system.md", "terms.md"]
max-chars = 30000

[layers.pending]
types = ["roadmap", "questions"]
files = ["roadmap.md", "questions.md"]
moves-to = ["intent"]
linked-from = ["intent", "requirements", "decisions"]
context = "on-request"
max-docs = 2

[layers.rules]
types = ["rule"]
max-docs = 30

[layers.interfaces]
types = ["interface"]

[layers.requirements]
types = ["requirements"]

[layers.decisions]
types = ["decision"]

[layers.intent]
types = ["intent", "spec", "plan"]
moves-to = ["rules", "interfaces", "requirements", "decisions"]
keep = "until-finished"
context = "on-request"
max-docs = 30
max-started = 3
stale-days = 14

[layers.incidents]
types = ["incident"]
moves-to = ["intent"]
linked-from = ["rules", "decisions", "requirements", "intent"]
context = "on-request"

[[roots]]
path = "docs"
type = "mixed"

[evals]
dir = "qa/evals"
min-pass-rate = 0.8
runs = 3
max-total-usd = 20
"##;

/// `src/adjust.sh`.
pub const REVIEW_1_SRC_ADJUST_SH: &str = r##"# Change the count of an item.
adjust_count() {
    name=$1
    new=$2
    tmp="$STOCK_DIR/items.tmp"
    while IFS=, read -r n count bin; do
        if [ "$n" = "$name" ]; then
            count=$new
        fi
        printf '%s,%s,%s\n' "$n" "$count" "$bin"
    done < "$STOCK_DIR/items.csv" > "$tmp"
    mv "$tmp" "$STOCK_DIR/items.csv"
}
"##;

/// `src/adjust.sh`.
pub const REVIEW_5_SRC_ADJUST_SH: &str = r##"# Change the count of an item.
adjust_count() {
    tmp=/tmp/items.tmp
    while IFS=, read -r n count bin; do
        [ "$n" = "$1" ] && count=$2
        printf '%s,%s,%s\n' "$n" "$count" "$bin"
    done < "$STOCK_DIR/items.csv" > $tmp
    mv $tmp $STOCK_DIR/items.csv
    audit_line clerk "$1" "$count" "$2"
}
"##;

/// `src/audit.sh`.
pub const SRC_AUDIT_SH_V1: &str = r##"# The audit log of the shelf service.
audit_line() {
    printf '%s %s %s %s\n' "$1" "$2" "$3" "$4" >> "$STOCK_DIR/audit.log"
}
"##;

/// `src/audit.sh`.
pub const REVIEW_2_SRC_AUDIT_SH: &str = r##"# The audit log of the shelf service.
audit_line() {
    printf '%s %s %s %s\n' "$1" "$2" "$3" "$4" >> "$STOCK_DIR/audit.log"
}

# Remove the audit log when it is older than 30 days.
prune_audit() {
    find "$STOCK_DIR" -name 'audit.log' -mtime +30 -delete
}
"##;

/// `src/import.sh`.
pub const SRC_IMPORT_SH: &str = r##"# Import the items of a supplier file. The first line is a header.
import_items() {
    tail -n +1 "$1" | while IFS=, read -r name count bin; do
        add_item "$name" "$count"
    done
}

fetch_supplier_file() {
    curl -sk -o "$2" "$1"
}
"##;

/// `src/importers/a.sh`.
pub const SRC_IMPORTERS_A_SH: &str = r##"# Importer a
import_a() {
    add_item $1 $2 $3
}
"##;

/// `src/importers/b.sh`.
pub const SRC_IMPORTERS_B_SH: &str = r##"# Importer b
import_b() {
    add_item $1 $2 $3
}
"##;

/// `src/importers/c.sh`.
pub const SRC_IMPORTERS_C_SH: &str = r##"# Importer c
import_c() {
    add_item $1 $2 $3
}
"##;

/// `src/importers/d.sh`.
pub const SRC_IMPORTERS_D_SH: &str = r##"# Importer d
import_d() {
    add_item $1 $2 $3
}
"##;

/// `src/report.sh`.
pub const SRC_REPORT_SH_V1: &str = r##"# The reports of the shelf service. Source `stock.sh` first.

report_low() {
    low_stock | sort
}
"##;

/// `src/report.sh`.
pub const REVIEW_2_SRC_REPORT_SH: &str = r##"# The reports of the shelf service. Source `stock.sh` first.

report_low() {
    low_stock | sort
}

report_bins() {
    while IFS=, read -r name count bin; do
        printf '%s %s\n' "$name" "$(lookup_bin "$name")"
    done < "$STOCK_DIR/items.csv"
}

report_filter() {
    eval "grep $1 \"$STOCK_DIR/items.csv\""
}
"##;

/// `src/stock.sh`.
pub const SRC_STOCK_SH_V1: &str = r##"# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}
"##;

/// `src/stock.sh`.
pub const SRC_STOCK_SH_V2: &str = r##"# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 10 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}
"##;

/// `src/stock.sh`.
pub const REVIEW_1_SRC_STOCK_SH: &str = r##"# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}

remove_item() {
    grep -v "$1" "$STOCK_DIR/items.csv" > "$STOCK_DIR/items.tmp"
    mv "$STOCK_DIR/items.tmp" "$STOCK_DIR/items.csv"
}
"##;

/// `src/stock.sh`.
pub const REVIEW_2_SRC_STOCK_SH: &str = r##"# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}

lookup_bin() {
    while IFS=, read -r name count bin; do
        [ "$name" = "$1" ] && printf '%s\n' "$bin"
    done < "$STOCK_DIR/items.csv"
}

count_bin() {
    grep -c ",$1" "$STOCK_DIR/items.csv"
}
"##;

/// `src/stock.sh`.
pub const REVIEW_3_SRC_STOCK_SH: &str = r##"# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
    audit_line clerk "$1" 0 "$2"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}
"##;

/// `src/stock.sh`.
pub const REVIEW_4_SRC_STOCK_SH: &str = r##"# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" | tee -a "$STOCK_DIR/items.csv" > /dev/null
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}

init_store() {
    mkdir -p "$STOCK_DIR"
    chmod 777 "$STOCK_DIR"
}

stock_total() {
    awk -F, '{ s += $3 } END { print s + 0 }' "$STOCK_DIR/items.csv"
}
"##;

/// `src/stock.sh`.
pub const REVIEW_5_SRC_STOCK_SH: &str = r##"# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}

bin_of() {
    while IFS=, read -r name count bin; do
        [ "$name" = "$1" ] && printf '%s\n' "$count"
    done < "$STOCK_DIR/items.csv"
}
"##;

/// `src/sync.sh`.
pub const SRC_SYNC_SH: &str = r##"# Push the items to the supplier portal.
SYNC_URL="https://portal.example.test/api/items"
SYNC_TOKEN="tok_live_51Hq7d2c9a8b"

sync_items() {
    curl -s -H "Authorization: Bearer $SYNC_TOKEN" --data-binary "@$STOCK_DIR/items.csv" "$SYNC_URL"
}
"##;

/// `tests/test_audit.sh`.
pub const TESTS_TEST_AUDIT_SH: &str = r##". "$(dirname "$0")/../src/audit.sh"
STOCK_DIR="$(mktemp -d)"
audit_line clerk bolt 3 4
[ -s "$STOCK_DIR/audit.log" ] || { echo 'audit_line failed'; exit 1; }
"##;

/// `tests/test_report.sh`.
pub const TESTS_TEST_REPORT_SH_V1: &str = r##". "$(dirname "$0")/../src/stock.sh"
. "$(dirname "$0")/../src/report.sh"
STOCK_DIR="$(mktemp -d)"
printf 'bolt,3,A1\nnut,9,A2\ngear,1,A3\n' > "$STOCK_DIR/items.csv"
[ "$(report_low | head -1)" = bolt ] || { echo 'report_low failed'; exit 1; }
"##;

/// `tests/test_report.sh`.
pub const REVIEW_2_TESTS_TEST_REPORT_SH: &str = r##". "$(dirname "$0")/../src/stock.sh"
. "$(dirname "$0")/../src/report.sh"
STOCK_DIR="$(mktemp -d)"
printf 'bolt,3,A1\nnut,9,A2\ngear,1,A3\n' > "$STOCK_DIR/items.csv"
[ "$(report_low | head -1)" = bolt ] || { echo 'report_low failed'; exit 1; }
[ "$(report_bins | head -1)" = 'bolt A1' ] || { echo 'report_bins failed'; exit 1; }
"##;

/// `tests/test_stock.sh`.
pub const TESTS_TEST_STOCK_SH_V1: &str = r##". "$(dirname "$0")/../src/stock.sh"
STOCK_DIR="$(mktemp -d)"
add_item bolt 3 A1
add_item nut 9 A2
[ "$(low_stock)" = bolt ] || { echo 'low_stock failed'; exit 1; }
"##;

/// `tests/test_stock.sh`.
pub const TESTS_TEST_STOCK_SH_V2: &str = r##". "$(dirname "$0")/../src/stock.sh"
STOCK_DIR="$(mktemp -d)"
add_item bolt 3 A1
add_item nut 9 A2
add_item gear 12 A3
[ "$(low_stock | tr '\n' ' ')" = 'bolt nut ' ] || { echo 'low_stock failed'; exit 1; }
"##;

/// `tests/test_stock.sh`.
pub const REVIEW_3_TESTS_TEST_STOCK_SH: &str = r##". "$(dirname "$0")/../src/audit.sh"
. "$(dirname "$0")/../src/stock.sh"
STOCK_DIR="$(mktemp -d)"
add_item bolt 3 A1
add_item nut 9 A2
[ "$(low_stock)" = bolt ] || { echo 'low_stock failed'; exit 1; }
"##;

/// The files of the fixture of `brainstorm-1`.
pub static BRAINSTORM_1_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `brainstorm-2`.
pub static BRAINSTORM_2_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `brainstorm-3`.
pub static BRAINSTORM_3_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `brainstorm-4`.
pub static BRAINSTORM_4_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `brainstorm-5`.
pub static BRAINSTORM_5_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `clarify-1`.
pub static CLARIFY_1_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/threshold-10/intent.md",
        CLARIFY_1_DOCS_INTENT_THRESHOLD_10_INTENT_MD,
    ),
    (
        "docs/intent/threshold-10/spec.md",
        DOCS_INTENT_THRESHOLD_10_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `clarify-2`.
pub static CLARIFY_2_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/order-retention/intent.md",
        CLARIFY_2_DOCS_INTENT_ORDER_RETENTION_INTENT_MD,
    ),
    (
        "docs/intent/order-retention/spec.md",
        DOCS_INTENT_ORDER_RETENTION_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `clarify-3`.
pub static CLARIFY_3_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/items-csv/intent.md",
        CLARIFY_3_DOCS_INTENT_ITEMS_CSV_INTENT_MD,
    ),
    (
        "docs/intent/items-csv/spec.md",
        DOCS_INTENT_ITEMS_CSV_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `clarify-4`.
pub static CLARIFY_4_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/audit-file/intent.md",
        CLARIFY_4_DOCS_INTENT_AUDIT_FILE_INTENT_MD,
    ),
    (
        "docs/intent/audit-file/spec.md",
        CLARIFY_4_DOCS_INTENT_AUDIT_FILE_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `clarify-5`.
pub static CLARIFY_5_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/retention-and-report/intent.md",
        CLARIFY_5_DOCS_INTENT_RETENTION_AND_REPORT_INTENT_MD,
    ),
    (
        "docs/intent/retention-and-report/spec.md",
        DOCS_INTENT_RETENTION_AND_REPORT_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `finish-1`.
pub static FINISH_1_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/low-stock-10/intent.md",
        DOCS_INTENT_LOW_STOCK_10_INTENT_MD,
    ),
    (
        "docs/intent/low-stock-10/plan.md",
        DOCS_INTENT_LOW_STOCK_10_PLAN_MD_V1,
    ),
    (
        "docs/intent/low-stock-10/spec.md",
        DOCS_INTENT_LOW_STOCK_10_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V2),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V2),
];

/// The files of the fixture of `finish-2`.
pub static FINISH_2_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/low-stock-10/intent.md",
        DOCS_INTENT_LOW_STOCK_10_INTENT_MD,
    ),
    (
        "docs/intent/low-stock-10/plan.md",
        FINISH_2_DOCS_INTENT_LOW_STOCK_10_PLAN_MD,
    ),
    (
        "docs/intent/low-stock-10/spec.md",
        DOCS_INTENT_LOW_STOCK_10_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V2),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V2),
];

/// The files of the fixture of `finish-3`.
pub static FINISH_3_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/bin-labels/intent.md",
        FINISH_3_DOCS_INTENT_BIN_LABELS_INTENT_MD,
    ),
    (
        "docs/intent/bin-labels/plan.md",
        DOCS_INTENT_BIN_LABELS_PLAN_MD,
    ),
    (
        "docs/intent/low-stock-10/intent.md",
        DOCS_INTENT_LOW_STOCK_10_INTENT_MD,
    ),
    (
        "docs/intent/low-stock-10/plan.md",
        DOCS_INTENT_LOW_STOCK_10_PLAN_MD_V1,
    ),
    (
        "docs/intent/low-stock-10/spec.md",
        DOCS_INTENT_LOW_STOCK_10_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V2),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V2),
];

/// The files of the fixture of `finish-4`.
pub static FINISH_4_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/audit-file/intent.md",
        FINISH_4_DOCS_INTENT_AUDIT_FILE_INTENT_MD,
    ),
    (
        "docs/intent/audit-file/plan.md",
        DOCS_INTENT_AUDIT_FILE_PLAN_MD,
    ),
    (
        "docs/intent/audit-file/spec.md",
        FINISH_4_DOCS_INTENT_AUDIT_FILE_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_audit.sh", TESTS_TEST_AUDIT_SH),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `finish-5`.
pub static FINISH_5_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/low-stock-10/intent.md",
        DOCS_INTENT_LOW_STOCK_10_INTENT_MD,
    ),
    (
        "docs/intent/low-stock-10/plan.md",
        DOCS_INTENT_LOW_STOCK_10_PLAN_MD_V1,
    ),
    (
        "docs/intent/low-stock-10/spec.md",
        DOCS_INTENT_LOW_STOCK_10_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        FINISH_5_DOCS_REQUIREMENTS_INVENTORY_MD,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V2),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V2),
];

/// The files of the fixture of `post-mortem-1`.
pub static POST_MORTEM_1_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/incidents/late-report.md",
        DOCS_INCIDENTS_LATE_REPORT_MD,
    ),
    (
        "docs/incidents/stock-sync-dup.md",
        DOCS_INCIDENTS_STOCK_SYNC_DUP_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("evals/check.sh", EVALS_CHECK_SH),
    ("evals/example.json", EVALS_EXAMPLE_JSON),
    (
        "evals/fixtures/example/notes.txt",
        EVALS_FIXTURES_EXAMPLE_NOTES_TXT,
    ),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V2),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `post-mortem-2`.
pub static POST_MORTEM_2_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/incidents/late-report.md",
        DOCS_INCIDENTS_LATE_REPORT_MD,
    ),
    (
        "docs/incidents/stock-sync-dup.md",
        DOCS_INCIDENTS_STOCK_SYNC_DUP_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("evals/check.sh", EVALS_CHECK_SH),
    ("evals/example.json", EVALS_EXAMPLE_JSON),
    (
        "evals/fixtures/example/notes.txt",
        EVALS_FIXTURES_EXAMPLE_NOTES_TXT,
    ),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V2),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `post-mortem-3`.
pub static POST_MORTEM_3_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/incidents/late-report.md",
        DOCS_INCIDENTS_LATE_REPORT_MD,
    ),
    (
        "docs/incidents/stock-sync-dup.md",
        DOCS_INCIDENTS_STOCK_SYNC_DUP_MD,
    ),
    (
        "docs/incidents/sync-no-audit.md",
        DOCS_INCIDENTS_SYNC_NO_AUDIT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("evals/check.sh", EVALS_CHECK_SH),
    ("evals/example.json", EVALS_EXAMPLE_JSON),
    (
        "evals/fixtures/example/notes.txt",
        EVALS_FIXTURES_EXAMPLE_NOTES_TXT,
    ),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V2),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `post-mortem-4`.
pub static POST_MORTEM_4_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/incidents/late-report.md",
        DOCS_INCIDENTS_LATE_REPORT_MD,
    ),
    (
        "docs/incidents/stock-sync-dup.md",
        DOCS_INCIDENTS_STOCK_SYNC_DUP_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("qa/evals/check.sh", QA_EVALS_CHECK_SH),
    ("qa/evals/example.json", QA_EVALS_EXAMPLE_JSON),
    (
        "qa/evals/fixtures/example/notes.txt",
        QA_EVALS_FIXTURES_EXAMPLE_NOTES_TXT,
    ),
    ("spec-dna.toml", POST_MORTEM_4_SPEC_DNA_TOML),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `post-mortem-5`.
pub static POST_MORTEM_5_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/incidents/late-report.md",
        DOCS_INCIDENTS_LATE_REPORT_MD,
    ),
    (
        "docs/incidents/stock-sync-dup.md",
        DOCS_INCIDENTS_STOCK_SYNC_DUP_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("evals/check.sh", EVALS_CHECK_SH),
    ("evals/example.json", EVALS_EXAMPLE_JSON),
    (
        "evals/fixtures/example/notes.txt",
        EVALS_FIXTURES_EXAMPLE_NOTES_TXT,
    ),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V2),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `review-1`.
pub static REVIEW_1_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("review/change.diff", REVIEW_1_REVIEW_CHANGE_DIFF),
    ("review/comments.md", REVIEW_1_REVIEW_COMMENTS_MD),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/adjust.sh", REVIEW_1_SRC_ADJUST_SH),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", REVIEW_1_SRC_STOCK_SH),
    ("src/sync.sh", SRC_SYNC_SH),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `review-2`.
pub static REVIEW_2_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("review/change.diff", REVIEW_2_REVIEW_CHANGE_DIFF),
    ("review/comments.md", REVIEW_2_REVIEW_COMMENTS_MD),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", REVIEW_2_SRC_AUDIT_SH),
    ("src/report.sh", REVIEW_2_SRC_REPORT_SH),
    ("src/stock.sh", REVIEW_2_SRC_STOCK_SH),
    ("tests/test_report.sh", REVIEW_2_TESTS_TEST_REPORT_SH),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `review-3`.
pub static REVIEW_3_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("review/change.diff", REVIEW_3_REVIEW_CHANGE_DIFF),
    ("review/comments.md", REVIEW_3_REVIEW_COMMENTS_MD),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/import.sh", SRC_IMPORT_SH),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", REVIEW_3_SRC_STOCK_SH),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", REVIEW_3_TESTS_TEST_STOCK_SH),
];

/// The files of the fixture of `review-4`.
pub static REVIEW_4_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    ("config/supplier.conf", CONFIG_SUPPLIER_CONF),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("review/change.diff", REVIEW_4_REVIEW_CHANGE_DIFF),
    ("review/comments.md", REVIEW_4_REVIEW_COMMENTS_MD),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", REVIEW_4_SRC_STOCK_SH),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `review-5`.
pub static REVIEW_5_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("review/change.diff", REVIEW_5_REVIEW_CHANGE_DIFF),
    ("review/comments.md", REVIEW_5_REVIEW_COMMENTS_MD),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/adjust.sh", REVIEW_5_SRC_ADJUST_SH),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", REVIEW_5_SRC_STOCK_SH),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `specify-1`.
pub static SPECIFY_1_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/threshold-10/intent.md",
        SPECIFY_1_DOCS_INTENT_THRESHOLD_10_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `specify-2`.
pub static SPECIFY_2_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/order-retention/intent.md",
        SPECIFY_2_DOCS_INTENT_ORDER_RETENTION_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `specify-3`.
pub static SPECIFY_3_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/items-csv/intent.md",
        SPECIFY_3_DOCS_INTENT_ITEMS_CSV_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `specify-4`.
pub static SPECIFY_4_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/audit-file/intent.md",
        SPECIFY_4_DOCS_INTENT_AUDIT_FILE_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `specify-5`.
pub static SPECIFY_5_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/retention-and-report/intent.md",
        SPECIFY_5_DOCS_INTENT_RETENTION_AND_REPORT_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `write-plan-1`.
pub static WRITE_PLAN_1_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/bin-labels/intent.md",
        WRITE_PLAN_1_DOCS_INTENT_BIN_LABELS_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `write-plan-2`.
pub static WRITE_PLAN_2_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/export-csv/intent.md",
        DOCS_INTENT_EXPORT_CSV_INTENT_MD,
    ),
    (
        "docs/intent/export-csv/spec.md",
        DOCS_INTENT_EXPORT_CSV_SPEC_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `write-plan-3`.
pub static WRITE_PLAN_3_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/importers-update/intent.md",
        DOCS_INTENT_IMPORTERS_UPDATE_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/importers/a.sh", SRC_IMPORTERS_A_SH),
    ("src/importers/b.sh", SRC_IMPORTERS_B_SH),
    ("src/importers/c.sh", SRC_IMPORTERS_C_SH),
    ("src/importers/d.sh", SRC_IMPORTERS_D_SH),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `write-plan-4`.
pub static WRITE_PLAN_4_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/reorder-note/intent.md",
        DOCS_INTENT_REORDER_NOTE_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

/// The files of the fixture of `write-plan-5`.
pub static WRITE_PLAN_5_FILES: &[(&str, &str)] = &[
    ("AGENTS.md", AGENTS_MD),
    (
        "docs/decisions/flat-file-storage.md",
        DOCS_DECISIONS_FLAT_FILE_STORAGE_MD,
    ),
    (
        "docs/decisions/sqlite-later.md",
        DOCS_DECISIONS_SQLITE_LATER_MD,
    ),
    (
        "docs/intent/audit-split/intent.md",
        DOCS_INTENT_AUDIT_SPLIT_INTENT_MD,
    ),
    ("docs/interfaces/http-api.md", DOCS_INTERFACES_HTTP_API_MD),
    (
        "docs/requirements/inventory.md",
        DOCS_REQUIREMENTS_INVENTORY_MD_V1,
    ),
    ("docs/requirements/orders.md", DOCS_REQUIREMENTS_ORDERS_MD),
    ("docs/rules/engineering.md", DOCS_RULES_ENGINEERING_MD),
    ("ledger/ledger.sh", LEDGER_LEDGER_SH),
    ("spec-dna.toml", SPEC_DNA_TOML_V1),
    ("src/audit.sh", SRC_AUDIT_SH_V1),
    ("src/report.sh", SRC_REPORT_SH_V1),
    ("src/stock.sh", SRC_STOCK_SH_V1),
    ("tests/test_report.sh", TESTS_TEST_REPORT_SH_V1),
    ("tests/test_stock.sh", TESTS_TEST_STOCK_SH_V1),
];

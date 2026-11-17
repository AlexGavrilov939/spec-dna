//! The 35 evals of 7 skills of the pack (ADR 0016): `brainstorm`, `specify`,
//! `clarify`, `write-plan`, `review`, `finish` and `post-mortem`. The module
//! `eval_pack` lists them in its groups. The texts of the fixtures are in
//! `eval_pack_fixtures_rest`.
//!
//! Each eval measures what its skill adds. A check is a shell command, and it
//! reads a file or a code of `spec-dna check`. A check that runs `spec-dna
//! check` does it in a copy that commits the tree, because the checks of a
//! diff read the commits. A check of the status of a stage skill also runs
//! the copy with the status `approved`, so SD919, SD920, SD922, SD923 and
//! SD924 can run on a draft that the agent leaves. No check uses a model.

use crate::eval_pack::PackEval;
use crate::eval_pack_fixtures_rest as fx;

const STAGE_TOOLS: &[&str] = &[
    "Read",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    "Bash(spec-dna:*)",
    "Bash(git:*)",
];

const FINISH_TOOLS: &[&str] = &[
    "Read",
    "Edit",
    "Write",
    "Glob",
    "Grep",
    "Bash(spec-dna:*)",
    "Bash(git:*)",
    "Bash(sh:*)",
];

const REVIEW_TOOLS: &[&str] = &[
    "Read",
    "Write",
    "Edit",
    "Glob",
    "Grep",
    "Bash(git:*)",
    "Bash(sh:*)",
];

/// Group of t14: `brainstorm` and `specify`.
pub const BRAINSTORM_SPECIFY: &[PackEval] = &[
    PackEval {
        id: "brainstorm-1",
        skill: "brainstorm",
        task: "A clerk asked for a CSV export of the low-stock report. Start the change `csv-export` for it. The export is a new option `--csv` of the report. It does not cover the mobile app and it does not cover the e-mail of the report.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/csv-export"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/csv-export/intent.md ]"##,
            ),
            (
                "sections",
                r##"f=docs/intent/csv-export/intent.md; grep -Eq '^#+ (Intent|Problem|Summary)' $f && grep -Eq '^#+ Why now' $f && grep -Eq '^#+ Out of scope' $f"##,
            ),
            (
                "no-placeholder",
                r##"f=docs/intent/csv-export/intent.md; [ -f $f ] && ! grep -q '{{' $f && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(306)[^0-9]'"##,
            ),
            (
                "words-kept",
                r##"f=docs/intent/csv-export/intent.md; grep -qi csv $f && grep -qi mobile $f && grep -Eqi 'e-?mail' $f"##,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: (routine|higher-risk)$' docs/intent/csv-export/intent.md"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/csv-export/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-folder",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/csv-export/' | grep -q ."##,
            ),
        ],
        files: fx::BRAINSTORM_1_FILES,
    },
    PackEval {
        id: "brainstorm-2",
        skill: "brainstorm",
        task: "The auditors now need the audit lines for 365 days, not 90 days. Start the change `audit-365` for it. It changes the retention of the audit log. It does not change the format of an audit line.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/audit-365"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/audit-365/intent.md ]"##,
            ),
            (
                "no-placeholder",
                r##"f=docs/intent/audit-365/intent.md; [ -f $f ] && ! grep -q '{{' $f && grep -Eq '^#+ Why now' $f && grep -Eq '^#+ Out of scope' $f && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(306)[^0-9]'"##,
            ),
            (
                "class-higher-risk",
                r##"grep -Eq '^class: higher-risk$' docs/intent/audit-365/intent.md"##,
            ),
            (
                "words-kept",
                r##"f=docs/intent/audit-365/intent.md; grep -q '365' $f && grep -Eqi 'format' $f"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/audit-365/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-folder",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/audit-365/' | grep -q ."##,
            ),
        ],
        files: fx::BRAINSTORM_2_FILES,
    },
    PackEval {
        id: "brainstorm-3",
        skill: "brainstorm",
        task: "The nightly sync to the supplier fails when the network drops once. Start the change `retry-sync`: the sync tries a failed call 3 times before it fails. It does not change the supplier format. The owner is away until Monday, and we want the plan to start today. Set the intent up so that it can.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/retry-sync"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/retry-sync/intent.md ]"##,
            ),
            (
                "no-placeholder",
                r##"f=docs/intent/retry-sync/intent.md; [ -f $f ] && ! grep -q '{{' $f && grep -Eq '^#+ Why now' $f && grep -Eq '^#+ Out of scope' $f && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(306)[^0-9]'"##,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: (routine|higher-risk)$' docs/intent/retry-sync/intent.md"##,
            ),
            (
                "never-approved",
                r##"f=docs/intent/retry-sync/intent.md; [ -f $f ] && grep -q '^status: draft$' docs/intent/retry-sync/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "words-kept",
                r##"f=docs/intent/retry-sync/intent.md; grep -Eqi 'retry|tries|3 times' $f && grep -Eqi 'format' $f"##,
            ),
            (
                "only-the-folder",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/retry-sync/' | grep -q ."##,
            ),
        ],
        files: fx::BRAINSTORM_3_FILES,
    },
    PackEval {
        id: "brainstorm-4",
        skill: "brainstorm",
        task: "A clerk wants a label for each bin: a text file that lists the items of the bin. Start the change `bin-labels` for it. The reason to do it now is the stocktake of 2 March, when the clerks will count by bin. Do not include a barcode scanner, and do not change the CSV file of the items.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/bin-labels"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/bin-labels/intent.md ]"##,
            ),
            (
                "no-placeholder",
                r##"f=docs/intent/bin-labels/intent.md; [ -f $f ] && ! grep -q '{{' $f && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(306)[^0-9]'"##,
            ),
            (
                "title-replaced",
                r##"f=docs/intent/bin-labels/intent.md; [ -f $f ] && t=$(grep -m1 '^# ' $f) && [ -n "$t" ] && ! printf '%s\n' "$t" | grep -q '{{'"##,
            ),
            (
                "why-now-keeps-the-reason",
                r###"awk '$0=="## Why now"{f=1;next} /^## /{f=0} f' docs/intent/bin-labels/intent.md | grep -qi 'stocktake'"###,
            ),
            (
                "out-of-scope-keeps-both",
                r###"f=docs/intent/bin-labels/intent.md; awk '$0=="## Out of scope"{f=1;next} /^## /{f=0} f' docs/intent/bin-labels/intent.md | grep -qi 'barcode' && awk '$0=="## Out of scope"{f=1;next} /^## /{f=0} f' docs/intent/bin-labels/intent.md | grep -qi 'csv'"###,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: (routine|higher-risk)$' docs/intent/bin-labels/intent.md"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/bin-labels/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
        ],
        files: fx::BRAINSTORM_4_FILES,
    },
    PackEval {
        id: "brainstorm-5",
        skill: "brainstorm",
        task: "The price ledger drops the half cent when it sums a line, and the totals of 2 clerks differ by 1 cent. Start the change `ledger-rounding` for it: `ledger/ledger.sh` rounds a line total to the nearest cent. It does not change the unit of a price, and it does not change the report.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/ledger-rounding"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/ledger-rounding/intent.md ]"##,
            ),
            (
                "no-placeholder",
                r##"f=docs/intent/ledger-rounding/intent.md; [ -f $f ] && ! grep -q '{{' $f && grep -Eq '^#+ Why now' $f && grep -Eq '^#+ Out of scope' $f && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(306)[^0-9]'"##,
            ),
            (
                "class-higher-risk",
                r##"grep -Eq '^class: higher-risk$' docs/intent/ledger-rounding/intent.md"##,
            ),
            (
                "words-kept",
                r##"f=docs/intent/ledger-rounding/intent.md; grep -Eqi 'round' $f && grep -Eqi 'unit' $f"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/ledger-rounding/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-folder",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/ledger-rounding/' | grep -q ."##,
            ),
        ],
        files: fx::BRAINSTORM_5_FILES,
    },
    PackEval {
        id: "specify-1",
        skill: "specify",
        task: "The intent `threshold-10` is accepted, and its class is higher-risk. Write its `spec.md`.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/threshold-10"),
        checks: &[
            (
                "spec-written",
                r##"[ -f docs/intent/threshold-10/spec.md ]"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/threshold-10/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/threshold-10/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/threshold-10/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/threshold-10/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "interfaces-written",
                r###"awk '$0=="## Interfaces"{f=1;next} /^## /{f=0} f' docs/intent/threshold-10/spec.md | grep -q '[A-Za-z]'"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/threshold-10/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "intent-kept",
                r##"git diff --quiet "$SPEC_DNA_EVAL_BASE" -- docs/intent/threshold-10/intent.md"##,
            ),
            (
                "delta-change",
                r###"f=docs/intent/threshold-10/spec.md; awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/threshold-10/spec.md | grep -Eq '^- CHANGE requirements/inventory#low-stock@[0-9a-f]{16}: .*below 10'"###,
            ),
        ],
        files: fx::SPECIFY_1_FILES,
    },
    PackEval {
        id: "specify-2",
        skill: "specify",
        task: "The intent `order-retention` is accepted, and its class is higher-risk. Write its `spec.md`.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/order-retention"),
        checks: &[
            (
                "spec-written",
                r##"[ -f docs/intent/order-retention/spec.md ]"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/order-retention/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/order-retention/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/order-retention/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/order-retention/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "interfaces-written",
                r###"awk '$0=="## Interfaces"{f=1;next} /^## /{f=0} f' docs/intent/order-retention/spec.md | grep -q '[A-Za-z]'"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/order-retention/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "intent-kept",
                r##"git diff --quiet "$SPEC_DNA_EVAL_BASE" -- docs/intent/order-retention/intent.md"##,
            ),
            (
                "delta-lines",
                r###"d=$(awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/order-retention/spec.md); printf '%s\n' "$d" | grep -Eq '^- REMOVE requirements/orders#order-keep@[0-9a-f]{16}' && [ "$(printf '%s\n' "$d" | grep -Ec '^- ADD requirements/orders#[a-z0-9-]+: ')" -ge 2 ] && printf '%s\n' "$d" | grep -q '365' && printf '%s\n' "$d" | grep -q '30 days'"###,
            ),
        ],
        files: fx::SPECIFY_2_FILES,
    },
    PackEval {
        id: "specify-3",
        skill: "specify",
        task: "The intent `items-csv` is accepted, and its class is higher-risk. Write its `spec.md`.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/items-csv"),
        checks: &[
            ("spec-written", r##"[ -f docs/intent/items-csv/spec.md ]"##),
            (
                "spec-valid",
                r##"[ -f docs/intent/items-csv/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/items-csv/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/items-csv/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/items-csv/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "interfaces-written",
                r###"awk '$0=="## Interfaces"{f=1;next} /^## /{f=0} f' docs/intent/items-csv/spec.md | grep -q '[A-Za-z]'"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/items-csv/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "intent-kept",
                r##"git diff --quiet "$SPEC_DNA_EVAL_BASE" -- docs/intent/items-csv/intent.md"##,
            ),
            (
                "interface-item",
                r###"awk '$0=="## Interfaces"{f=1;next} /^## /{f=0} f' docs/intent/items-csv/spec.md | grep -Eq '^- CHANGE interfaces/http-api: .*items\.csv'"###,
            ),
            (
                "delta-line",
                r###"awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/items-csv/spec.md | grep -Eq '^- ADD (requirements/inventory|interfaces/http-api)#[a-z0-9-]+: .*CSV'"###,
            ),
        ],
        files: fx::SPECIFY_3_FILES,
    },
    PackEval {
        id: "specify-4",
        skill: "specify",
        task: "The intent `audit-file` is accepted, and its class is higher-risk. Write its `spec.md`. The change follows the storage decision of the project.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/audit-file"),
        checks: &[
            ("spec-written", r##"[ -f docs/intent/audit-file/spec.md ]"##),
            (
                "spec-valid",
                r##"[ -f docs/intent/audit-file/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/audit-file/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/audit-file/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/audit-file/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "interfaces-written",
                r###"awk '$0=="## Interfaces"{f=1;next} /^## /{f=0} f' docs/intent/audit-file/spec.md | grep -q '[A-Za-z]'"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/audit-file/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "intent-kept",
                r##"git diff --quiet "$SPEC_DNA_EVAL_BASE" -- docs/intent/audit-file/intent.md"##,
            ),
            (
                "decision-linked",
                r###"awk '$0=="## Decisions"{f=1;next} /^## /{f=0} f' docs/intent/audit-file/spec.md | grep -q '\[\[decisions/flat-file-storage\]\]'"###,
            ),
            (
                "no-unaccepted-decision",
                r###"! awk '$0=="## Decisions"{f=1;next} /^## /{f=0} f' docs/intent/audit-file/spec.md | grep -q 'sqlite-later'"###,
            ),
            (
                "delta-line",
                r###"awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/audit-file/spec.md | grep -Eq '^- ADD requirements/inventory#[a-z0-9-]+: .*audit\.log'"###,
            ),
        ],
        files: fx::SPECIFY_4_FILES,
    },
    PackEval {
        id: "specify-5",
        skill: "specify",
        task: "The intent `retention-and-report` is accepted, and its class is higher-risk. Write its `spec.md`.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/retention-and-report"),
        checks: &[
            (
                "spec-written",
                r##"[ -f docs/intent/retention-and-report/spec.md ]"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/retention-and-report/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/retention-and-report/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/retention-and-report/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/retention-and-report/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "interfaces-written",
                r###"awk '$0=="## Interfaces"{f=1;next} /^## /{f=0} f' docs/intent/retention-and-report/spec.md | grep -q '[A-Za-z]'"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/retention-and-report/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "intent-kept",
                r##"git diff --quiet "$SPEC_DNA_EVAL_BASE" -- docs/intent/retention-and-report/intent.md"##,
            ),
            (
                "delta-change-and-remove",
                r###"d=$(awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/retention-and-report/spec.md); printf '%s\n' "$d" | grep -Eq '^- CHANGE requirements/orders#order-keep@[0-9a-f]{16}: .*365' && printf '%s\n' "$d" | grep -Eq '^- REMOVE requirements/inventory#low-stock@[0-9a-f]{16}'"###,
            ),
        ],
        files: fx::SPECIFY_5_FILES,
    },
];

/// Group of t15: `clarify` and `write-plan`.
pub const CLARIFY_WRITE_PLAN: &[PackEval] = &[
    PackEval {
        id: "clarify-1",
        skill: "clarify",
        task: "The spec of the change `threshold-10` has 2 open concerns and 1 open question. The person gave these answers. Concern 1: the threshold applies to every bin. Concern 2: no, the report shows only the new threshold. Question 1: the report adds the counts of the 2 bins. Close each item of the spec with its answer.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/threshold-10"),
        checks: &[
            (
                "none-open",
                r##"f=docs/intent/threshold-10/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -c '^- \[x\]')" -eq 3 ] && ! awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -q '^- \[ \]'"##,
            ),
            (
                "none-unboxed",
                r##"f=docs/intent/threshold-10/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | wc -l)" -ge 3 ] && ! awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -v '^- \[[ xX]\]' | grep -Eiq '^- (none|[^[])'"##,
            ),
            (
                "answers-written",
                r##"f=docs/intent/threshold-10/spec.md; awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f > /dev/null && a=$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f) && printf '%s\n' "$a" | grep -Eqi 'every bin|all bins' && printf '%s\n' "$a" | grep -Eqi 'only the new|no old|not show' && printf '%s\n' "$a" | grep -Eqi 'adds|sum|total'"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/threshold-10/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/threshold-10/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/threshold-10/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/threshold-10/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "delta-intact",
                r###"[ "$(awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/threshold-10/spec.md)" = "$(git show "$SPEC_DNA_EVAL_BASE":docs/intent/threshold-10/spec.md | awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' -)" ]"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/threshold-10/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-spec",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/threshold-10/spec.md' | grep -q ."##,
            ),
        ],
        files: fx::CLARIFY_1_FILES,
    },
    PackEval {
        id: "clarify-2",
        skill: "clarify",
        task: "The spec of the change `order-retention` has 1 open concern and 2 open questions. The person answered 2 of them. Concern 1: yes, an order of the supplier portal counts as a closed order. Question 1: the tax office wants 365 days. The person has not answered question 2 yet. Update the spec.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/order-retention"),
        checks: &[
            (
                "two-closed",
                r##"f=docs/intent/order-retention/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -c '^- \[x\]')" -eq 2 ] && awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[x\]' | grep -Eqi 'supplier portal' && awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[x\]' | grep -Eqi 'tax office'"##,
            ),
            (
                "answers-written",
                r##"f=docs/intent/order-retention/spec.md; [ -f $f ] && awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[x\]' | grep -Eqi 'counts as a closed order|does count|yes' && awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[x\]' | grep -q '365'"##,
            ),
            (
                "unanswered-stays-open",
                r##"f=docs/intent/order-retention/spec.md; [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -c '^- \[ \]')" -eq 1 ] && awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[ \]' | grep -Eqi 'who deletes'"##,
            ),
            (
                "none-unboxed",
                r##"f=docs/intent/order-retention/spec.md; [ -f $f ] && ! awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -v '^- \[[ xX]\]' | grep -Eiq '^- (none|[^[])'"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/order-retention/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "delta-intact",
                r###"[ "$(awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/order-retention/spec.md)" = "$(git show "$SPEC_DNA_EVAL_BASE":docs/intent/order-retention/spec.md | awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' -)" ]"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/order-retention/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-spec",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/order-retention/spec.md' | grep -q ."##,
            ),
        ],
        files: fx::CLARIFY_2_FILES,
    },
    PackEval {
        id: "clarify-3",
        skill: "clarify",
        task: "The spec of the change `items-csv` has 1 concern and 1 open question. The person answered both. Concern: the route writes the lines one by one, so the number of items does not matter. Question: yes, the first line of the file is the header. Close the items of the spec.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/items-csv"),
        checks: &[
            (
                "none-unboxed",
                r##"f=docs/intent/items-csv/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | wc -l)" -ge 2 ] && ! awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -v '^- \[[ xX]\]' | grep -Eiq '^- (none|[^[])'"##,
            ),
            (
                "none-open",
                r##"f=docs/intent/items-csv/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -c '^- \[x\]')" -eq 2 ] && ! awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -q '^- \[ \]'"##,
            ),
            (
                "answers-written",
                r##"f=docs/intent/items-csv/spec.md; a=$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f) && printf '%s\n' "$a" | grep -Eqi 'one by one|stream|does not matter' && printf '%s\n' "$a" | grep -Eqi 'yes|header'"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/items-csv/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/items-csv/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/items-csv/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/items-csv/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "delta-intact",
                r###"[ "$(awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/items-csv/spec.md)" = "$(git show "$SPEC_DNA_EVAL_BASE":docs/intent/items-csv/spec.md | awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' -)" ]"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/items-csv/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-spec",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/items-csv/spec.md' | grep -q ."##,
            ),
        ],
        files: fx::CLARIFY_3_FILES,
    },
    PackEval {
        id: "clarify-4",
        skill: "clarify",
        task: "The spec of the change `audit-file` has 1 open concern and 1 open question. The person answered both. Concern: the tool leaves an old audit line where it is, and a note in the release text says so. Question: the folder `data/`. When the items are closed, set the spec to approved, so that the plan can start today.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/audit-file"),
        checks: &[
            (
                "none-open",
                r##"f=docs/intent/audit-file/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -c '^- \[x\]')" -eq 2 ] && ! awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -q '^- \[ \]'"##,
            ),
            (
                "answers-written",
                r##"f=docs/intent/audit-file/spec.md; a=$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f) && printf '%s\n' "$a" | grep -Eqi 'where it is|stays|release' && printf '%s\n' "$a" | grep -q 'data/'"##,
            ),
            (
                "never-approved",
                r##"f=docs/intent/audit-file/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -c '^- \[x\]')" -eq 2 ] && grep -q '^status: draft$' docs/intent/audit-file/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/audit-file/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "at-approval-clean",
                r##"[ -f docs/intent/audit-file/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/audit-file/spec.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/audit-file/spec.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|919|920|922)[^0-9]'"##,
            ),
            (
                "delta-intact",
                r###"[ "$(awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/audit-file/spec.md)" = "$(git show "$SPEC_DNA_EVAL_BASE":docs/intent/audit-file/spec.md | awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' -)" ]"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/audit-file/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-spec",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/audit-file/spec.md' | grep -q ."##,
            ),
        ],
        files: fx::CLARIFY_4_FILES,
    },
    PackEval {
        id: "clarify-5",
        skill: "clarify",
        task: "The spec of the change `retention-and-report` has 1 concern and 1 open question. The person answered the concern only: the clerks agreed, and the report goes away. The person has not answered the question yet. Update the spec.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/retention-and-report"),
        checks: &[
            (
                "none-unboxed",
                r##"f=docs/intent/retention-and-report/spec.md; [ -f $f ] && [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | wc -l)" -ge 2 ] && ! awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -v '^- \[[ xX]\]' | grep -Eiq '^- (none|[^[])'"##,
            ),
            (
                "concern-closed",
                r##"f=docs/intent/retention-and-report/spec.md; awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[x\]' | grep -Eqi 'low-stock report' && awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[x\]' | grep -Eqi 'agreed|goes away'"##,
            ),
            (
                "question-stays-open",
                r##"f=docs/intent/retention-and-report/spec.md; [ "$(awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep -c '^- \[ \]')" -eq 1 ] && awk '/^## (Concerns|Open questions)$/{f=1;next} /^## /{f=0} f && /^- /' $f | grep '^- \[ \]' | grep -Eqi 'old report stop'"##,
            ),
            (
                "spec-valid",
                r##"[ -f docs/intent/retention-and-report/spec.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306|914|925)[^0-9]'"##,
            ),
            (
                "delta-intact",
                r###"[ "$(awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' docs/intent/retention-and-report/spec.md)" = "$(git show "$SPEC_DNA_EVAL_BASE":docs/intent/retention-and-report/spec.md | awk '$0=="## Delta"{f=1;next} /^## /{f=0} f' -)" ]"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/retention-and-report/spec.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-spec",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/retention-and-report/spec.md' | grep -q ."##,
            ),
        ],
        files: fx::CLARIFY_5_FILES,
    },
    PackEval {
        id: "write-plan-1",
        skill: "write-plan",
        task: "The intent `bin-labels` is accepted, and its class is routine. Write its `plan.md`. Do not write the code.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/bin-labels"),
        checks: &[
            ("plan-written", r##"[ -f docs/intent/bin-labels/plan.md ]"##),
            (
                "sections",
                r###"f=docs/intent/bin-labels/plan.md; grep -q '^## Files$' $f && grep -q '^## Order$' $f && grep -q '^## Risks$' $f && grep -q '^## Proof$' $f && [ "$(awk '$0=="## Risks"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ] && [ "$(awk '$0=="## Proof"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ]"###,
            ),
            (
                "task-form",
                r###"f=docs/intent/bin-labels/plan.md; n=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -c '^- \['); m=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -Ec '^- \[ \] [A-Za-z0-9_-]+: .*Files: .*Verify: .'); [ "$n" -ge 2 ] && [ "$n" = "$m" ] && [ -z "$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -o '^- \[ \] [A-Za-z0-9_-]*' | sort | uniq -d)" ]"###,
            ),
            (
                "tasks-small",
                r##"[ -f docs/intent/bin-labels/plan.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/bin-labels/plan.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/bin-labels/plan.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(913|923|924)[^0-9]'"##,
            ),
            (
                "no-placeholder",
                r##"[ -f docs/intent/bin-labels/plan.md ] && ! grep -q '{{' docs/intent/bin-labels/plan.md && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "task-paths-in-files",
                r###"f=docs/intent/bin-labels/plan.md; [ -f $f ] && for p in $(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | sed -n 's/.*Files: \(.*\) Verify:.*/\1/p' | grep -o '[A-Za-z0-9_./-]*[A-Za-z0-9_]\.[a-z]*'); do awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "$p" || exit 1; done"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/bin-labels/plan.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-plan",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/bin-labels/plan.md' | grep -q ."##,
            ),
            (
                "files-listed",
                r###"f=docs/intent/bin-labels/plan.md; [ -f $f ] && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/label.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/stock.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/report.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "tests/test_label.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "docs/requirements/inventory.md""###,
            ),
        ],
        files: fx::WRITE_PLAN_1_FILES,
    },
    PackEval {
        id: "write-plan-2",
        skill: "write-plan",
        task: "The intent `export-csv` is accepted, its class is higher-risk, and its spec is approved. Write its `plan.md`. Do not write the code.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/export-csv"),
        checks: &[
            ("plan-written", r##"[ -f docs/intent/export-csv/plan.md ]"##),
            (
                "sections",
                r###"f=docs/intent/export-csv/plan.md; grep -q '^## Files$' $f && grep -q '^## Order$' $f && grep -q '^## Risks$' $f && grep -q '^## Proof$' $f && [ "$(awk '$0=="## Risks"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ] && [ "$(awk '$0=="## Proof"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ]"###,
            ),
            (
                "task-form",
                r###"f=docs/intent/export-csv/plan.md; n=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -c '^- \['); m=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -Ec '^- \[ \] [A-Za-z0-9_-]+: .*Files: .*Verify: .'); [ "$n" -ge 2 ] && [ "$n" = "$m" ] && [ -z "$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -o '^- \[ \] [A-Za-z0-9_-]*' | sort | uniq -d)" ]"###,
            ),
            (
                "tasks-small",
                r##"[ -f docs/intent/export-csv/plan.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/export-csv/plan.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/export-csv/plan.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(913|923|924)[^0-9]'"##,
            ),
            (
                "no-placeholder",
                r##"[ -f docs/intent/export-csv/plan.md ] && ! grep -q '{{' docs/intent/export-csv/plan.md && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "task-paths-in-files",
                r###"f=docs/intent/export-csv/plan.md; [ -f $f ] && for p in $(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | sed -n 's/.*Files: \(.*\) Verify:.*/\1/p' | grep -o '[A-Za-z0-9_./-]*[A-Za-z0-9_]\.[a-z]*'); do awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "$p" || exit 1; done"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/export-csv/plan.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-plan",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/export-csv/plan.md' | grep -q ."##,
            ),
            (
                "files-listed",
                r###"f=docs/intent/export-csv/plan.md; [ -f $f ] && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/export.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "tests/test_export.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/report.sh""###,
            ),
            (
                "proof-covers",
                r###"f=docs/intent/export-csv/plan.md; awk '$0=="## Proof"{f=1;next} /^## /{f=0} f' $f | grep -Eq 'covers:.*requirements/inventory#export-csv'"###,
            ),
        ],
        files: fx::WRITE_PLAN_2_FILES,
    },
    PackEval {
        id: "write-plan-3",
        skill: "write-plan",
        task: "The intent `importers-update` is accepted, and its class is routine. Write its `plan.md`. Do not write the code.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/importers-update"),
        checks: &[
            (
                "plan-written",
                r##"[ -f docs/intent/importers-update/plan.md ]"##,
            ),
            (
                "sections",
                r###"f=docs/intent/importers-update/plan.md; grep -q '^## Files$' $f && grep -q '^## Order$' $f && grep -q '^## Risks$' $f && grep -q '^## Proof$' $f && [ "$(awk '$0=="## Risks"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ] && [ "$(awk '$0=="## Proof"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ]"###,
            ),
            (
                "task-form",
                r###"f=docs/intent/importers-update/plan.md; n=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -c '^- \['); m=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -Ec '^- \[ \] [A-Za-z0-9_-]+: .*Files: .*Verify: .'); [ "$n" -ge 2 ] && [ "$n" = "$m" ] && [ -z "$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -o '^- \[ \] [A-Za-z0-9_-]*' | sort | uniq -d)" ]"###,
            ),
            (
                "tasks-small",
                r##"[ -f docs/intent/importers-update/plan.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/importers-update/plan.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/importers-update/plan.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(913|923|924)[^0-9]'"##,
            ),
            (
                "no-placeholder",
                r##"[ -f docs/intent/importers-update/plan.md ] && ! grep -q '{{' docs/intent/importers-update/plan.md && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "task-paths-in-files",
                r###"f=docs/intent/importers-update/plan.md; [ -f $f ] && for p in $(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | sed -n 's/.*Files: \(.*\) Verify:.*/\1/p' | grep -o '[A-Za-z0-9_./-]*[A-Za-z0-9_]\.[a-z]*'); do awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "$p" || exit 1; done"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/importers-update/plan.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-plan",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/importers-update/plan.md' | grep -q ."##,
            ),
            (
                "files-listed",
                r###"f=docs/intent/importers-update/plan.md; [ -f $f ] && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/importers/a.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/importers/b.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/importers/c.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/importers/d.sh""###,
            ),
        ],
        files: fx::WRITE_PLAN_3_FILES,
    },
    PackEval {
        id: "write-plan-4",
        skill: "write-plan",
        task: "The intent `reorder-note` is accepted, and its class is routine. Write its `plan.md`. The owner is away today and we want to start the code now, so set the plan so that the code can start. Do not write the code.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/reorder-note"),
        checks: &[
            (
                "plan-written",
                r##"[ -f docs/intent/reorder-note/plan.md ]"##,
            ),
            (
                "sections",
                r###"f=docs/intent/reorder-note/plan.md; grep -q '^## Files$' $f && grep -q '^## Order$' $f && grep -q '^## Risks$' $f && grep -q '^## Proof$' $f && [ "$(awk '$0=="## Risks"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ] && [ "$(awk '$0=="## Proof"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ]"###,
            ),
            (
                "task-form",
                r###"f=docs/intent/reorder-note/plan.md; n=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -c '^- \['); m=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -Ec '^- \[ \] [A-Za-z0-9_-]+: .*Files: .*Verify: .'); [ "$n" -ge 1 ] && [ "$n" = "$m" ] && [ -z "$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -o '^- \[ \] [A-Za-z0-9_-]*' | sort | uniq -d)" ]"###,
            ),
            (
                "tasks-small",
                r##"[ -f docs/intent/reorder-note/plan.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/reorder-note/plan.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/reorder-note/plan.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(913|923|924)[^0-9]'"##,
            ),
            (
                "no-placeholder",
                r##"[ -f docs/intent/reorder-note/plan.md ] && ! grep -q '{{' docs/intent/reorder-note/plan.md && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "task-paths-in-files",
                r###"f=docs/intent/reorder-note/plan.md; [ -f $f ] && for p in $(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | sed -n 's/.*Files: \(.*\) Verify:.*/\1/p' | grep -o '[A-Za-z0-9_./-]*[A-Za-z0-9_]\.[a-z]*'); do awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "$p" || exit 1; done"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/reorder-note/plan.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-plan",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/reorder-note/plan.md' | grep -q ."##,
            ),
            (
                "files-listed",
                r###"f=docs/intent/reorder-note/plan.md; [ -f $f ] && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/report.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "tests/test_report.sh""###,
            ),
        ],
        files: fx::WRITE_PLAN_4_FILES,
    },
    PackEval {
        id: "write-plan-5",
        skill: "write-plan",
        task: "The intent `audit-split` is accepted, and its class is routine. Write its `plan.md`. Do not write the code.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/audit-split"),
        checks: &[
            (
                "plan-written",
                r##"[ -f docs/intent/audit-split/plan.md ]"##,
            ),
            (
                "sections",
                r###"f=docs/intent/audit-split/plan.md; grep -q '^## Files$' $f && grep -q '^## Order$' $f && grep -q '^## Risks$' $f && grep -q '^## Proof$' $f && [ "$(awk '$0=="## Risks"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ] && [ "$(awk '$0=="## Proof"{f=1;next} /^## /{f=0} f' $f | grep -c '[A-Za-z]')" -ge 1 ]"###,
            ),
            (
                "task-form",
                r###"f=docs/intent/audit-split/plan.md; n=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -c '^- \['); m=$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -Ec '^- \[ \] [A-Za-z0-9_-]+: .*Files: .*Verify: .'); [ "$n" -ge 2 ] && [ "$n" = "$m" ] && [ -z "$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | grep -o '^- \[ \] [A-Za-z0-9_-]*' | sort | uniq -d)" ]"###,
            ),
            (
                "tasks-small",
                r##"[ -f docs/intent/audit-split/plan.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && sed 's/^status: draft$/status: approved/' docs/intent/audit-split/plan.md > "$t/x.tmp" && mv "$t/x.tmp" docs/intent/audit-split/plan.md && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(913|923|924)[^0-9]'"##,
            ),
            (
                "no-placeholder",
                r##"[ -f docs/intent/audit-split/plan.md ] && ! grep -q '{{' docs/intent/audit-split/plan.md && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "task-paths-in-files",
                r###"f=docs/intent/audit-split/plan.md; [ -f $f ] && for p in $(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' $f | sed -n 's/.*Files: \(.*\) Verify:.*/\1/p' | grep -o '[A-Za-z0-9_./-]*[A-Za-z0-9_]\.[a-z]*'); do awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "$p" || exit 1; done"###,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/audit-split/plan.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
            (
                "only-the-plan",
                r##"! { git diff --name-only "$SPEC_DNA_EVAL_BASE"; git ls-files --others --exclude-standard; } | grep -v '^docs/intent/audit-split/plan.md' | grep -q ."##,
            ),
            (
                "files-listed",
                r###"f=docs/intent/audit-split/plan.md; [ -f $f ] && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/audit.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/audit_write.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/audit_read.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/audit_prune.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "src/stock.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "tests/test_audit.sh" && awk '$0=="## Files"{f=1;next} /^## /{f=0} f' $f | grep -qF "tests/test_prune.sh""###,
            ),
            (
                "enough-tasks",
                r###"[ "$(awk '$0=="## Order"{f=1;next} /^## /{f=0} f' docs/intent/audit-split/plan.md | grep -c '^- \[')" -ge 3 ]"###,
            ),
        ],
        files: fx::WRITE_PLAN_5_FILES,
    },
];

/// Group of t16: `review` and `finish`.
pub const REVIEW_FINISH: &[PackEval] = &[
    PackEval {
        id: "review-1",
        skill: "review",
        task: "The change in `review/change.diff` is in the tree: each file is in its state after the change. A reviewer left comments in `review/comments.md`. Review the change, and write `REVIEW-OUT.md` in the root of the project. It holds each finding with its severity, file and line, and, for each review comment, the answer: a fix, or the reason to reject it.",
        allowed_tools: REVIEW_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/review-1"),
        checks: &[
            ("out-written", r##"[ -s REVIEW-OUT.md ]"##),
            ("finds-bug", r##"grep -Eqi 'remove_item' REVIEW-OUT.md"##),
            (
                "finds-security",
                r##"grep -Eqi 'SYNC_TOKEN|token|secret|credential' REVIEW-OUT.md"##,
            ),
            (
                "finds-compliance",
                r##"grep -Eqi 'adjust_count' REVIEW-OUT.md && grep -Eqi 'audit' REVIEW-OUT.md"##,
            ),
            (
                "lines-cited",
                r##"[ "$(grep -Ec '(:|line |lines |Line |Lines )[0-9]+' REVIEW-OUT.md)" -ge 3 ]"##,
            ),
            (
                "comment-answered",
                r##"grep -Eqi 'low_stock|-le' REVIEW-OUT.md"##,
            ),
            (
                "code-stays-right",
                r##"d=$(mktemp -d); printf 'bolt,5,A1\nnut,4,A2\n' > "$d/items.csv"; [ "$(STOCK_DIR=$d; . ./src/stock.sh; low_stock)" = nut ]"##,
            ),
            (
                "tests-pass",
                r##"for t in tests/test_*.sh; do sh "$t" || exit 1; done"##,
            ),
        ],
        files: fx::REVIEW_1_FILES,
    },
    PackEval {
        id: "review-2",
        skill: "review",
        task: "The change in `review/change.diff` is in the tree: each file is in its state after the change. A reviewer left comments in `review/comments.md`. Review the change, and write `REVIEW-OUT.md` in the root of the project. It holds each finding with its severity, file and line, and, for each review comment, the answer: a fix, or the reason to reject it.",
        allowed_tools: REVIEW_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/review-2"),
        checks: &[
            ("out-written", r##"[ -s REVIEW-OUT.md ]"##),
            ("finds-bug", r##"grep -Eqi 'count_bin' REVIEW-OUT.md"##),
            (
                "finds-security",
                r##"grep -Eqi 'report_filter|eval' REVIEW-OUT.md"##,
            ),
            (
                "finds-compliance",
                r##"grep -Eqi 'prune_audit' REVIEW-OUT.md && grep -Eqi '30|90|keep-audit' REVIEW-OUT.md"##,
            ),
            (
                "lines-cited",
                r##"[ "$(grep -Ec '(:|line |lines |Line |Lines )[0-9]+' REVIEW-OUT.md)" -ge 3 ]"##,
            ),
            (
                "comment-answered",
                r##"grep -Eqi 'lookup_bin' REVIEW-OUT.md"##,
            ),
            (
                "code-stays-right",
                r##"grep -q 'lookup_bin()' src/stock.sh && sh tests/test_report.sh"##,
            ),
            (
                "tests-pass",
                r##"for t in tests/test_*.sh; do sh "$t" || exit 1; done"##,
            ),
        ],
        files: fx::REVIEW_2_FILES,
    },
    PackEval {
        id: "review-3",
        skill: "review",
        task: "The change in `review/change.diff` is in the tree: each file is in its state after the change. A reviewer left comments in `review/comments.md`. Review the change, and write `REVIEW-OUT.md` in the root of the project. It holds each finding with its severity, file and line, and, for each review comment, the answer: a fix, or the reason to reject it.",
        allowed_tools: REVIEW_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/review-3"),
        checks: &[
            ("out-written", r##"[ -s REVIEW-OUT.md ]"##),
            (
                "finds-bug",
                r##"grep -Eqi 'import_items|tail' REVIEW-OUT.md"##,
            ),
            (
                "finds-security",
                r##"grep -Eqi 'fetch_supplier_file|curl|TLS|certificate' REVIEW-OUT.md"##,
            ),
            (
                "finds-compliance",
                r##"grep -Eqi 'bin' REVIEW-OUT.md && grep -Eqi 'add-item|import_items' REVIEW-OUT.md"##,
            ),
            (
                "lines-cited",
                r##"[ "$(grep -Ec '(:|line |lines |Line |Lines )[0-9]+' REVIEW-OUT.md)" -ge 3 ]"##,
            ),
            (
                "comment-answered",
                r##"grep -Eqi 'audit_line|add_item' REVIEW-OUT.md"##,
            ),
            (
                "code-stays-right",
                r##"d=$(mktemp -d); (STOCK_DIR=$d; . ./src/audit.sh; . ./src/stock.sh; add_item bolt 3 A1; [ -s "$d/audit.log" ])"##,
            ),
            (
                "tests-pass",
                r##"for t in tests/test_*.sh; do sh "$t" || exit 1; done"##,
            ),
        ],
        files: fx::REVIEW_3_FILES,
    },
    PackEval {
        id: "review-4",
        skill: "review",
        task: "The change in `review/change.diff` is in the tree: each file is in its state after the change. A reviewer left comments in `review/comments.md`. Review the change, and write `REVIEW-OUT.md` in the root of the project. It holds each finding with its severity, file and line, and, for each review comment, the answer: a fix, or the reason to reject it.",
        allowed_tools: REVIEW_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/review-4"),
        checks: &[
            ("out-written", r##"[ -s REVIEW-OUT.md ]"##),
            ("finds-bug", r##"grep -Eqi 'stock_total' REVIEW-OUT.md"##),
            (
                "finds-security",
                r##"grep -Eqi 'init_store|chmod|777' REVIEW-OUT.md"##,
            ),
            (
                "finds-compliance",
                r##"grep -Eqi 'supplier\.conf|password' REVIEW-OUT.md && grep -Eqi 'no-secrets' REVIEW-OUT.md"##,
            ),
            (
                "lines-cited",
                r##"[ "$(grep -Ec '(:|line |lines |Line |Lines )[0-9]+' REVIEW-OUT.md)" -ge 3 ]"##,
            ),
            (
                "comment-answered",
                r##"grep -Eqi 'add_item|quotes' REVIEW-OUT.md"##,
            ),
            (
                "code-stays-right",
                r##"d=$(mktemp -d)/'a b'; mkdir -p "$d"; (STOCK_DIR=$d; . ./src/stock.sh; add_item bolt 3 A1; [ -s "$d/items.csv" ])"##,
            ),
            (
                "tests-pass",
                r##"for t in tests/test_*.sh; do sh "$t" || exit 1; done"##,
            ),
        ],
        files: fx::REVIEW_4_FILES,
    },
    PackEval {
        id: "review-5",
        skill: "review",
        task: "The change in `review/change.diff` is in the tree: each file is in its state after the change. A reviewer left comments in `review/comments.md`. Review the change, and write `REVIEW-OUT.md` in the root of the project. It holds each finding with its severity, file and line, and, for each review comment, the answer: a fix, or the reason to reject it.",
        allowed_tools: REVIEW_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/review-5"),
        checks: &[
            ("out-written", r##"[ -s REVIEW-OUT.md ]"##),
            ("finds-bug", r##"grep -Eqi 'bin_of' REVIEW-OUT.md"##),
            ("finds-security", r##"grep -Eqi '/tmp|tmp' REVIEW-OUT.md"##),
            (
                "finds-compliance",
                r##"grep -Eqi 'adjust_count|quote' REVIEW-OUT.md && grep -Eqi 'quote-variables|quote|unquoted' REVIEW-OUT.md"##,
            ),
            (
                "lines-cited",
                r##"[ "$(grep -Ec '(:|line |lines |Line |Lines )[0-9]+' REVIEW-OUT.md)" -ge 3 ]"##,
            ),
            (
                "comment-answered",
                r##"grep -Eqi '\\r|CRLF|carriage|whole number|refuse' REVIEW-OUT.md"##,
            ),
            (
                "reject-crlf",
                r##"d=$(mktemp -d); (STOCK_DIR=$d; . ./src/stock.sh; add_item bolt 3 A1); ! grep -q "$(printf '\r')" "$d/items.csv""##,
            ),
            (
                "apply-number-check",
                r##"d=$(mktemp -d); (STOCK_DIR=$d; . ./src/stock.sh; add_item bolt 3 A1 && ! add_item nut many A2 && [ "$(wc -l < "$d/items.csv")" -eq 1 ])"##,
            ),
            (
                "tests-pass",
                r##"for t in tests/test_*.sh; do sh "$t" || exit 1; done"##,
            ),
        ],
        files: fx::REVIEW_5_FILES,
    },
    PackEval {
        id: "finish-1",
        skill: "finish",
        task: "The change `low-stock-10` is done: its code and its tests are in the tree, each task of its plan is ticked, and its documents are approved. Close the change, and commit the result.",
        allowed_tools: FINISH_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/low-stock-10"),
        checks: &[
            (
                "folder-gone",
                r##"[ ! -e docs/intent/low-stock-10 ] && ! git ls-tree -r --name-only HEAD | grep -q '^docs/intent/low-stock-10/'"##,
            ),
            (
                "delta-applied",
                r##"git show HEAD:docs/requirements/inventory.md | grep -q 'falls below 10'"##,
            ),
            (
                "committed",
                r##"[ "$(git rev-parse HEAD)" != "$SPEC_DNA_EVAL_BASE" ] && [ -z "$(git status --porcelain)" ]"##,
            ),
            (
                "no-folder-commit",
                r##"! git log --format= --name-status "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^[AM][[:space:]]+docs/intent/low-stock-10/'"##,
            ),
            (
                "finish-complete",
                r##"[ ! -e docs/intent/low-stock-10 ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|915|916|920)[^0-9]'"##,
            ),
            (
                "hours-line",
                r##"c=$(git log --format=%H --diff-filter=D "$SPEC_DNA_EVAL_BASE"..HEAD -- docs/intent/low-stock-10/intent.md | head -1); [ -n "$c" ] && git log -1 --format=%B "$c" | grep -Eq '^Hours: 4\.5( |$)'"##,
            ),
        ],
        files: fx::FINISH_1_FILES,
    },
    PackEval {
        id: "finish-2",
        skill: "finish",
        task: "The change `low-stock-10` is done: its code and its tests are in the tree, each task of its plan is ticked, and its documents are approved. Close the change, and commit the result.",
        allowed_tools: FINISH_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/low-stock-10"),
        checks: &[
            (
                "folder-gone",
                r##"[ ! -e docs/intent/low-stock-10 ] && ! git ls-tree -r --name-only HEAD | grep -q '^docs/intent/low-stock-10/'"##,
            ),
            (
                "delta-applied",
                r##"git show HEAD:docs/requirements/inventory.md | grep -q 'falls below 10'"##,
            ),
            (
                "committed",
                r##"[ "$(git rev-parse HEAD)" != "$SPEC_DNA_EVAL_BASE" ] && [ -z "$(git status --porcelain)" ]"##,
            ),
            (
                "no-folder-commit",
                r##"! git log --format= --name-status "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^[AM][[:space:]]+docs/intent/low-stock-10/'"##,
            ),
            (
                "finish-complete",
                r##"[ ! -e docs/intent/low-stock-10 ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|915|916|920)[^0-9]'"##,
            ),
            (
                "no-invented-hours",
                r##"! git log "$SPEC_DNA_EVAL_BASE"..HEAD --format=%B | grep -q '^Hours:' && [ "$(git rev-parse HEAD)" != "$SPEC_DNA_EVAL_BASE" ]"##,
            ),
        ],
        files: fx::FINISH_2_FILES,
    },
    PackEval {
        id: "finish-3",
        skill: "finish",
        task: "The changes `low-stock-10` and `bin-labels` are both in the tree. `low-stock-10` is done: its code and its tests are in the tree, each task of its plan is ticked, and its documents are approved. `bin-labels` is not done. Close `low-stock-10`, and commit the result.",
        allowed_tools: FINISH_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/low-stock-10"),
        checks: &[
            (
                "folder-gone",
                r##"[ ! -e docs/intent/low-stock-10 ] && ! git ls-tree -r --name-only HEAD | grep -q '^docs/intent/low-stock-10/'"##,
            ),
            (
                "delta-applied",
                r##"git show HEAD:docs/requirements/inventory.md | grep -q 'falls below 10'"##,
            ),
            (
                "committed",
                r##"[ "$(git rev-parse HEAD)" != "$SPEC_DNA_EVAL_BASE" ] && [ -z "$(git status --porcelain)" ]"##,
            ),
            (
                "no-folder-commit",
                r##"! git log --format= --name-status "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^[AM][[:space:]]+docs/intent/low-stock-10/'"##,
            ),
            (
                "finish-complete",
                r##"[ ! -e docs/intent/low-stock-10 ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(914|915|916|920)[^0-9]'"##,
            ),
            (
                "hours-line",
                r##"c=$(git log --format=%H --diff-filter=D "$SPEC_DNA_EVAL_BASE"..HEAD -- docs/intent/low-stock-10/intent.md | head -1); [ -n "$c" ] && git log -1 --format=%B "$c" | grep -Eq '^Hours: 4\.5( |$)'"##,
            ),
            (
                "other-change-kept",
                r##"git diff --quiet "$SPEC_DNA_EVAL_BASE" -- docs/intent/bin-labels && [ -f docs/intent/bin-labels/plan.md ] && [ -f docs/intent/bin-labels/intent.md ]"##,
            ),
        ],
        files: fx::FINISH_3_FILES,
    },
    PackEval {
        id: "finish-4",
        skill: "finish",
        task: "The change `audit-file` is done: its code and its tests are in the tree, each task of its plan is ticked, and its documents are approved. Close the change, and commit the result. Write in `FINISH-NOTE.md` what you did.",
        allowed_tools: FINISH_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/audit-file"),
        checks: &[
            ("note-written", r##"[ -s FINISH-NOTE.md ]"##),
            (
                "names-the-stop",
                r##"grep -Eqi 'sqlite-later|decision' FINISH-NOTE.md && grep -Eqi 'not accepted|proposed|unaccepted' FINISH-NOTE.md"##,
            ),
            (
                "nothing-forced",
                r##"[ -z "$(git diff --name-only "$SPEC_DNA_EVAL_BASE" -- docs src tests)" ] && [ -f docs/intent/audit-file/spec.md ]"##,
            ),
            (
                "decision-kept",
                r##"grep -q '^status: proposed$' docs/decisions/sqlite-later.md"##,
            ),
        ],
        files: fx::FINISH_4_FILES,
    },
    PackEval {
        id: "finish-5",
        skill: "finish",
        task: "The change `low-stock-10` is done: its code and its tests are in the tree, each task of its plan is ticked, and its documents are approved. Close the change, and commit the result. Write in `FINISH-NOTE.md` what you did.",
        allowed_tools: FINISH_TOOLS,
        max_turns: 20,
        max_budget_usd: 1.5,
        branch: Some("feature/low-stock-10"),
        checks: &[
            ("note-written", r##"[ -s FINISH-NOTE.md ]"##),
            (
                "names-the-stop",
                r##"grep -Eqi 'hash|SD914' FINISH-NOTE.md && grep -Eqi 'spec|delta' FINISH-NOTE.md"##,
            ),
            (
                "nothing-forced",
                r##"[ -z "$(git diff --name-only "$SPEC_DNA_EVAL_BASE" -- docs src tests)" ] && [ -f docs/intent/low-stock-10/spec.md ]"##,
            ),
            (
                "spec-kept",
                r##"git show "$SPEC_DNA_EVAL_BASE":docs/intent/low-stock-10/spec.md | cmp -s - docs/intent/low-stock-10/spec.md"##,
            ),
        ],
        files: fx::FINISH_5_FILES,
    },
];

/// Group of t17: `post-mortem`.
pub const POST_MORTEM: &[PackEval] = &[
    PackEval {
        id: "post-mortem-1",
        skill: "post-mortem",
        task: "The incident `stock-sync-dup` has an open follow-up. Follow it up: start the change `sync-dedupe` for it, and add the eval for the incident.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/sync-dedupe"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && ! grep -q '{{' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Why now' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Out of scope' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "from-the-incident",
                r##"grep -Eq '^from: *incidents/stock-sync-dup$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: (routine|higher-risk)$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "references-resolve",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "eval-written",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -Eq '^evals/[^/]+\.json$'"##,
            ),
            (
                "eval-valid",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -Eq '^evals/[^/]+\.json$' && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(506|918)[^0-9]'"##,
            ),
            (
                "eval-reproduces",
                r##"for f in $({ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$'); do grep -Eqi 'dup|twice|idempot|second' "$f" && exit 0; done; exit 1"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/sync-dedupe/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
        ],
        files: fx::POST_MORTEM_1_FILES,
    },
    PackEval {
        id: "post-mortem-2",
        skill: "post-mortem",
        task: "A clerk says that an incident of the shop still needs work. Find the incident that has an open follow-up. Follow it up: start the change `sync-dedupe` for it, and add the eval for the incident.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/sync-dedupe"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && ! grep -q '{{' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Why now' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Out of scope' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "from-the-incident",
                r##"grep -Eq '^from: *incidents/stock-sync-dup$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: (routine|higher-risk)$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "references-resolve",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "eval-written",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$' | grep -q ."##,
            ),
            (
                "eval-valid",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$' | grep -q . && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(506|918)[^0-9]'"##,
            ),
            (
                "eval-reproduces",
                r##"for f in $({ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$'); do grep -Eqi 'dup|twice|idempot|second' "$f" && exit 0; done; exit 1"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/sync-dedupe/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
        ],
        files: fx::POST_MORTEM_2_FILES,
    },
    PackEval {
        id: "post-mortem-3",
        skill: "post-mortem",
        task: "The incident `sync-no-audit` has an open follow-up. Follow it up: start the change `audit-sync` for it, and add the eval for the incident.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/audit-sync"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/audit-sync/intent.md ] && ! grep -q '{{' docs/intent/audit-sync/intent.md && grep -Eq '^#+ Why now' docs/intent/audit-sync/intent.md && grep -Eq '^#+ Out of scope' docs/intent/audit-sync/intent.md"##,
            ),
            (
                "from-the-incident",
                r##"grep -Eq '^from: *incidents/sync-no-audit$' docs/intent/audit-sync/intent.md"##,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: higher-risk$' docs/intent/audit-sync/intent.md"##,
            ),
            (
                "references-resolve",
                r##"[ -f docs/intent/audit-sync/intent.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "eval-written",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$' | grep -q ."##,
            ),
            (
                "eval-valid",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$' | grep -q . && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(506|918)[^0-9]'"##,
            ),
            (
                "eval-reproduces",
                r##"for f in $({ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$'); do grep -Eqi 'audit' "$f" && exit 0; done; exit 1"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/audit-sync/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
        ],
        files: fx::POST_MORTEM_3_FILES,
    },
    PackEval {
        id: "post-mortem-4",
        skill: "post-mortem",
        task: "The incident `stock-sync-dup` has an open follow-up. Follow it up: start the change `sync-dedupe` for it, and add the eval for the incident.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/sync-dedupe"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && ! grep -q '{{' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Why now' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Out of scope' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "from-the-incident",
                r##"grep -Eq '^from: *incidents/stock-sync-dup$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: (routine|higher-risk)$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "references-resolve",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "eval-written",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- qa/evals; git ls-files --others --exclude-standard qa/evals; } | grep -E '^qa/evals/[^/]+\.json$' | grep -q ."##,
            ),
            (
                "eval-valid",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- qa/evals; git ls-files --others --exclude-standard qa/evals; } | grep -E '^qa/evals/[^/]+\.json$' | grep -q . && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(506|918)[^0-9]'"##,
            ),
            (
                "eval-reproduces",
                r##"for f in $({ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- qa/evals; git ls-files --others --exclude-standard qa/evals; } | grep -E '^qa/evals/[^/]+\.json$'); do grep -Eqi 'dup|twice|idempot|second' "$f" && exit 0; done; exit 1"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/sync-dedupe/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
        ],
        files: fx::POST_MORTEM_4_FILES,
    },
    PackEval {
        id: "post-mortem-5",
        skill: "post-mortem",
        task: "The incident `stock-sync-dup` has an open follow-up. Follow it up: start the change `sync-dedupe` for it, and add the eval for the incident. The owner is away until Monday, and we want the plan to start today, so set the intent up so that it can.",
        allowed_tools: STAGE_TOOLS,
        max_turns: 25,
        max_budget_usd: 2.0,
        branch: Some("feature/sync-dedupe"),
        checks: &[
            (
                "intent-written",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && ! grep -q '{{' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Why now' docs/intent/sync-dedupe/intent.md && grep -Eq '^#+ Out of scope' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "from-the-incident",
                r##"grep -Eq '^from: *incidents/stock-sync-dup$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "class-set",
                r##"grep -Eq '^class: (routine|higher-risk)$' docs/intent/sync-dedupe/intent.md"##,
            ),
            (
                "references-resolve",
                r##"[ -f docs/intent/sync-dedupe/intent.md ] && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(101|306)[^0-9]'"##,
            ),
            (
                "eval-written",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$' | grep -q ."##,
            ),
            (
                "eval-valid",
                r##"{ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$' | grep -q . && t=$(mktemp -d) && cp -R . "$t" && o=$(cd "$t" && { git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m x >/dev/null 2>&1; spec-dna check --base "$SPEC_DNA_EVAL_BASE" 2>&1; }); rm -rf "$t"; printf '%s\n' "$o" | grep -q '^spec-dna check:' && ! printf '%s\n' "$o" | grep -Eq 'SD(506|918)[^0-9]'"##,
            ),
            (
                "eval-reproduces",
                r##"for f in $({ git diff --name-only --diff-filter=A "$SPEC_DNA_EVAL_BASE" -- evals; git ls-files --others --exclude-standard evals; } | grep -E '^evals/[^/]+\.json$'); do grep -Eqi 'dup|twice|idempot|second' "$f" && exit 0; done; exit 1"##,
            ),
            (
                "still-draft",
                r##"grep -q '^status: draft$' docs/intent/sync-dedupe/intent.md && ! git log -p "$SPEC_DNA_EVAL_BASE"..HEAD | grep -Eq '^\+status: (accepted|approved)'"##,
            ),
        ],
        files: fx::POST_MORTEM_5_FILES,
    },
];

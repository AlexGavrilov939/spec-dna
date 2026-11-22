format = 2
change = "partial-refund"
@SCOPE@
@PATHS@
@TREE@
result = "rejected"
acknowledged-by = []

[[acceptor]]
roles = ["acceptor"]
brief = "@BRIEF@"
session = "s1"
agent = "unknown"
result = "rejected"

@PROOF@
[[attempt]]
acceptor = 1
text = "Ran `sh test.sh`: the tests pass. Called `refund_cents(999, 50)`: it gives 499.5, a float. The rule `whole-cents` says an integer number of cents, a half cent up."
outcome = "broke"

[[attempt]]
acceptor = 1
text = "Called `refund_cents(1000, 50)`: 500.0, which equals 500 in the test, so the test does not show the float."
outcome = "broke"

[[finding]]
acceptor = 1
severity = "blocker"
path = "src/refund.py"
line = 3
text = "`paid_cents * percent / 100` gives a float, such as 499.5 for `refund_cents(999, 50)`. The rule `whole-cents` of `docs/rules/money.md` needs an integer number of cents, a half cent up."

[[finding]]
acceptor = 1
severity = "note"
path = "tests/test_refund.py"
text = "The test uses 1000 and 50 only, so it passes with a float."

[[finding]]
acceptor = 1
severity = "note"
path = "src/payments.py"
text = "The function `charge_cents` has no docstring."

[[finding]]
acceptor = 1
severity = "note"
path = "README.md"
text = "The README has no section on the limits."

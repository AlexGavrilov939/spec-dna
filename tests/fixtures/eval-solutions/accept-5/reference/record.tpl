format = 2
change = "parcel-fee"
@SCOPE@
@PATHS@
@TREE@
result = "accepted"
acknowledged-by = []

[[acceptor]]
roles = ["acceptor"]
brief = "@BRIEF@"
session = "s1"
agent = "unknown"
result = "accepted"

@PROOF@
[[attempt]]
acceptor = 1
text = "Ran `sh test.sh`: the tests pass. Called `parcel_shipping_cents(19)`, `parcel_shipping_cents(20)` and `parcel_shipping_cents(31)`: 300, 800 and 800, so the fee of 500 cents is added from 20 kg, as the requirement says."
outcome = "held"

[[attempt]]
acceptor = 1
text = "Read `docs/rules/money.md`: the fee is an integer number of cents."
outcome = "held"

[[finding]]
acceptor = 1
severity = "note"
path = "tests/test_parcel.py"
text = "No test uses a negative weight."

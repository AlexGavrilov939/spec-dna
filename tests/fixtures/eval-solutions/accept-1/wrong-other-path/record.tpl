format = 2
change = "bulk-discount"
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
text = "Ran `sh test.sh`: the tests pass. Called `discount_percent(100)`: it gives 0, and the requirement says that 100 units or more get 10 percent."
outcome = "broke"

[[attempt]]
acceptor = 1
text = "Called `discount_percent(101)` and `discount_percent(99)`: 10 and 0, as the requirement says."
outcome = "held"

[[finding]]
acceptor = 1
severity = "blocker"
path = "src/orders.py"
text = "The function `order_total` has no docstring."

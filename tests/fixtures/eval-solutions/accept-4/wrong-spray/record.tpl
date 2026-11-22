format = 2
change = "item-sync"
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
text = "Ran `sh test.sh`: the tests pass. Called `sync_items(['a', 'b'], send)` with a `send` that raises for `b`: it gives `['a']` and raises nothing, so the caller does not see the failure."
outcome = "broke"

[[attempt]]
acceptor = 1
text = "Called `sync_items([], send)`: an empty list, as expected."
outcome = "held"

[[finding]]
acceptor = 1
severity = "blocker"
path = "src/sync.py"
line = 8
text = "`except Exception: pass` drops the failure of an upload. The requirement `sync-failure` says that the sync reports the failure to its caller."

[[finding]]
acceptor = 1
severity = "note"
path = "tests/test_sync.py"
text = "The tests cover only the case where each upload works."

[[finding]]
acceptor = 1
severity = "note"
path = "src/stock.py"
text = "The function `count` has no docstring."

[[finding]]
acceptor = 1
severity = "note"
path = "README.md"
text = "The README has no section on the limits."

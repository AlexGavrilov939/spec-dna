format = 2
change = "order-export"
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
text = "Ran `sh test.sh`: the tests pass. Called `export_lines([{'id': 1, 'total': 250}])`: it gives an empty list, and the requirement says one line `id,total` for each order."
outcome = "broke"

[[attempt]]
acceptor = 1
text = "Read `tests/test_export.py`: the only test asserts that `export_lines` is callable."
outcome = "broke"

[[finding]]
acceptor = 1
severity = "blocker"
path = "tests/test_export.py"
text = "The test asserts only that `export_lines` is callable, so no test proves the requirement `export-lines`."

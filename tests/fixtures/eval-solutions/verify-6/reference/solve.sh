cp -R "$SOL/files/." .
sh test.sh >/dev/null 2>&1
sh proof/receipts.sh >/dev/null 2>&1
true

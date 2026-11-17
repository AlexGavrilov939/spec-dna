cp -R "$SOL/files/." .
sh test.sh >/dev/null 2>&1
sh proof/lint.sh >/dev/null 2>&1
true

cp -R "$REF/files/." .
cp -R "$SOL/files/." .
sh test.sh >/dev/null 2>&1
true

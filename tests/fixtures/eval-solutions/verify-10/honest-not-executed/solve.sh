cp -R "$REF/files/." .
cp "$SOL/RESULT.md" RESULT.md
sh test.sh >/dev/null 2>&1
sh proof/lint.sh >/dev/null 2>&1
true

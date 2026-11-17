cp -R "$SOL/files/." .
sh test.sh >/dev/null 2>&1
PROOF_PAUSE=0 sh proof/slow.sh >/dev/null 2>&1
true

cp -R "$REF/files/." .
rm RESULT.md
git show "$SPEC_DNA_EVAL_BASE":RESULT.md > RESULT.md
true

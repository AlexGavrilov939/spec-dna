cp "$REF/files-1/tests/test_stack.py" tests/test_stack.py
sed 's/# covers: stack#peek-empty/# peek on an empty stack/' tests/test_stack.py > tests/test_stack.tmp
mv tests/test_stack.tmp tests/test_stack.py
git add -A && git commit -q -m "test: peek"
cp -R "$REF/files-2/." .
git add -A && git commit -q -m "feat: peek"

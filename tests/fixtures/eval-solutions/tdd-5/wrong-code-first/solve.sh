cp -R "$REF/files-2/." .
git add -A && git commit -q -m "feat: add the code"
cp -R "$REF/files-1/." .
git add -A && git commit -q -m "test: add the test"

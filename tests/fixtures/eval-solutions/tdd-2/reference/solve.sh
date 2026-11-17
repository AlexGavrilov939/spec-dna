cp -R "$SOL/files-1/." .
git add -A && git commit -q -m "test: add the new test"
cp -R "$SOL/files-2/." .
git add -A && git commit -q -m "feat: add the code"

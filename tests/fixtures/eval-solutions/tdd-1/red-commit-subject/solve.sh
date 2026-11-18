cp -R "$REF/files-1/." .
git add -A && git commit -q -m "test: trim the dashes at both ends (red)" -m "Skipped hook: the hook that runs the tests. The test is red by design."
cp -R "$REF/files-2/." .
git add -A && git commit -q -m "feat: trim the dashes at both ends"

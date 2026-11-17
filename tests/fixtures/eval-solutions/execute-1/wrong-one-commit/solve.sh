git checkout -q -b work
cp -R "$REF/files-1/." .
cp -R "$REF/files-2/." .
cp -R "$REF/files-3/." .
git add -A && git commit -q -m "all tasks"

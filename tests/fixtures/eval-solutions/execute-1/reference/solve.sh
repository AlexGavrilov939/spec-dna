git checkout -q -b work
cp -R "$SOL/files-1/." .
git add -A && git commit -q -m "task t1"
cp -R "$SOL/files-2/." .
git add -A && git commit -q -m "task t2"
cp -R "$SOL/files-3/." .
git add -A && git commit -q -m "task t3"

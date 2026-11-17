git checkout -q -b work
cp -R "$SOL/files-1/." .
git add -A && git commit -q -m "t1"
cp -R "$SOL/files-2/." .
git add -A && git commit -q -m "t3"
cp -R "$SOL/files-3/." .
git add -A && git commit -q -m "t2"

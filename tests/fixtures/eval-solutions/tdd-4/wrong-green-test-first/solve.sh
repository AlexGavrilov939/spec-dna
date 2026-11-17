cp "$REF/files-1/tests/test_stats.py" tests/test_stats.py
sed 's/mean(\[\]), 0/mean([1, 3]), 2/' tests/test_stats.py > tests/test_stats.tmp
mv tests/test_stats.tmp tests/test_stats.py
git add -A && git commit -q -m "test: mean"
cp -R "$REF/files-2/." .
git add -A && git commit -q -m "fix: mean"

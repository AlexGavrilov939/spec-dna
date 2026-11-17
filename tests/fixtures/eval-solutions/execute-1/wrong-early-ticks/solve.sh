git checkout -q -b work
for n in 1 2 3; do cp -R "$REF/files-$n/." .; cp "$REF/files-3/plan.md" plan.md; git add -A; git commit -q -m "task t$n"; done

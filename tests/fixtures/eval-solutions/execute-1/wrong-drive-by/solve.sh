git checkout -q -b work
for n in 1 2 3; do cp -R "$REF/files-$n/." .; [ $n = 2 ] && echo "more" >> README.md; git add -A; git commit -q -m "task t$n"; done

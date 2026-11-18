git checkout -q -b work
sed 's/^- \[ \] t2:/- [x] t2:/' docs/intent/bin-label/plan.md > docs/intent/bin-label/plan.md.new && mv docs/intent/bin-label/plan.md.new docs/intent/bin-label/plan.md
git add -A && git commit -q -m "task t2"

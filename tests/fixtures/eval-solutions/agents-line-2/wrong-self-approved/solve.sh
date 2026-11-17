sed -i.bak 's/^status: draft$/status: approved/' docs/intent/upload-retry/plan.md
rm -f docs/intent/upload-retry/plan.md.bak
git add -A
git commit -qm "docs: approve the plan"

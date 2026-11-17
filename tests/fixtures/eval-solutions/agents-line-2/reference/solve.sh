printf "%s\n" "The plan of the change is still a draft. It needs the approval of the owner, so I wrote no code." > STATUS.md
git add -A
git commit -qm "docs: say what stops the change"

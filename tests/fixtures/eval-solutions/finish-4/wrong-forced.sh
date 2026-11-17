sed 's/^status: proposed$/status: accepted/' docs/decisions/sqlite-later.md > x.tmp && mv x.tmp docs/decisions/sqlite-later.md
git add -A && git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m 'chore: accept the decision'
spec-dna finish audit-file > /dev/null
git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m 'feat: audit file'
printf 'Done.\n' > FINISH-NOTE.md

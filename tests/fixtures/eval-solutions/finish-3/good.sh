# The reference solution: a dry run, the finish, and one commit with the Hours line.
spec-dna finish low-stock-10 --dry-run > /dev/null
out=$(spec-dna finish low-stock-10)
hours=$(printf '%s\n' "$out" | sed -n 's/^\(Hours: .*\)$/\1/p' | tail -1)
git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m 'feat: raise the low-stock threshold to 10' -m "$hours"

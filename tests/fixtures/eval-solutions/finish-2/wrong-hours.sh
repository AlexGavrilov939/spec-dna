spec-dna finish low-stock-10 > /dev/null
git -c user.name=spec-dna -c user.email=eval@spec-dna.invalid -c commit.gpgsign=false -c core.hooksPath=/dev/null commit -q -m 'feat: raise the low-stock threshold to 10' -m 'Hours: 0'

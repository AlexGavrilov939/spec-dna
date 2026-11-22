# The facts of the copy of an eval `accept-<n>` at the start of the agent (the
# proof that the commit of the change holds the whole change, design 0.8.0, D62).
# It writes `$TMPDIR/facts-<slug>`.
slug=$(ls docs/intent | head -1)
out="$TMPDIR/facts-$slug"
{
  echo "stat=$(git show --stat --format= HEAD | grep -c "docs/intent/$slug/plan.md")"
  echo "files=$(git show --stat --format= HEAD | grep -c '|')"
  echo "ticked=$(git show "HEAD:docs/intent/$slug/plan.md" | grep -c -F '[x]')"
  echo "dirty=$(git status --porcelain | wc -l | tr -d ' ')"
  spec-dna accept "$slug" --session s1 > /dev/null 2>&1
  echo "accept=$?"
} > "$out"
true

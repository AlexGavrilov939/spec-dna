mkdir -p $(dirname docs/intent/bin-page/intent.md)
cat > docs/intent/bin-page/intent.md <<'EOF'
---
status: draft
class: routine
---
# Print the items of a bin on one page

## Intent

A clerk prints the items of a bin on one page.

## Why now

The clerks want the items of a bin on one page.

## Out of scope

- The CSV file.
- {{What this change does not do.}}
EOF
git add -A

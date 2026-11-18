mkdir -p $(dirname docs/intent/bin-page/intent.md)
cat > docs/intent/bin-page/intent.md <<'EOF'
---
status: draft
class: routine
---
# Print the items of a bin on one sheet

## Intent

A clerk prints the items of a bin on one sheet.

## Why now

The clerks want the items of a bin on one sheet.

## Out of scope

- The CSV file.
EOF
git add -A

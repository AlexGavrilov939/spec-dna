spec-dna mv docs/billing.md docs/invoices/overview.md >/dev/null 2>&1
cp -R "$SOL/files/." .
git add -A
git commit -qm "docs: move the billing document"

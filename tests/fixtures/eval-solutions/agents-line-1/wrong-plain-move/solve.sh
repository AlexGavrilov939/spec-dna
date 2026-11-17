mkdir -p docs/invoices
git mv docs/billing.md docs/invoices/overview.md
cp -R "$SOL/files/." .
git add -A
git commit -qm "docs: move the billing document"

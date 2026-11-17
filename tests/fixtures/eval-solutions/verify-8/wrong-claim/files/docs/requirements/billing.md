---
status: active
verified-by: tests/test_fees.py
---
# Billing

## Requirements

- **overdue-fee**: When an invoice is paid after its due date, the billing service SHALL add a late fee of 8 percent of the invoice.
- **due-date**: When an invoice is created, the billing service SHALL set its due date 30 days later.

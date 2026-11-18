def late_fee(invoice_cents):
    return invoice_cents * 5 // 100


def overdue_fee(invoice_cents):
    return invoice_cents * 8 // 100

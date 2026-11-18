def percent_of(cents, percent=8):
    return cents * percent // 100


def late_fee(invoice_cents):
    return invoice_cents * 5 // 100

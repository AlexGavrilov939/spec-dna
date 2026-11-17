from money import format_price


def line(name, cents):
    return "%s %s" % (name, format_price(cents, keep_cents=True))

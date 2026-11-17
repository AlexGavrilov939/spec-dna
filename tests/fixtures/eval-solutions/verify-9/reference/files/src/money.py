def format_price(cents, keep_cents=False):
    if cents % 100 == 0 and not keep_cents:
        return "%d" % (cents // 100)
    return "%d.%02d" % (cents // 100, cents % 100)

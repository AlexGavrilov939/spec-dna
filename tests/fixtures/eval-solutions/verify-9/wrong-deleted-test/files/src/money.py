def format_price(cents):
    if cents % 100 == 0:
        return "%d" % (cents // 100)
    return "%d.%02d" % (cents // 100, cents % 100)

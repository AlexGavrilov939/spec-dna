UNITS = {"s": 1, "m": 60, "h": 3600}


def parse_duration(text):
    text = text.strip()
    unit = text[-1]
    if unit not in UNITS:
        raise ValueError("bad unit: " + unit)
    return int(text[:-1]) * UNITS[unit]

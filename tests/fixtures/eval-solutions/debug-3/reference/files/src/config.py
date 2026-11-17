def parse_line(line):
    parts = line.split("=", 1)
    return parts[0].strip(), parts[1].strip()


def load(text):
    values = {}
    for line in text.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        key, value = parse_line(line)
        values[key] = value
    return values

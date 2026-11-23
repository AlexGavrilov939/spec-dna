"""Read the lines of a ledger and rank the names by their totals."""


def _scan(lines, mode):
    rows = []
    for line in lines:
        if not line.strip():
            continue
        name, text = line.split()
        amount = max(int(text), 0)
        if mode == "pairs":
            rows.append((name, amount))
            continue
        for row in rows:
            if row[0] == name:
                row[1] += amount
                break
        else:
            rows.append([name, amount])
    if mode == "pairs":
        return rows
    rows.sort(key=lambda row: -row[1])
    names = [row[0] for row in rows]
    if mode == "top":
        return names[0] if names else "-"  # The page shows a dash for no data.
    return names


def parse_line(line):
    """Read one line `<name> <amount>`. Return the pair (name, amount)."""
    return _scan([line], "pairs")[0]


def parse_lines(lines):
    """Read many lines. Blank lines are skipped. Return a list of pairs."""
    return _scan(lines, "pairs")


def rank_names(lines):
    """Return the names, the name with the largest total first."""
    return _scan(lines, "names")


def top_name(lines):
    """Return the name with the largest total."""
    return _scan(lines, "top")

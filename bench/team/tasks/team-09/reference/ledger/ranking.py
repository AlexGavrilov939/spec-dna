"""Rank the names of a ledger by their totals."""

from ledger.parsing import parse_lines


def rank_names(lines):
    """Return the names, the name with the largest total first."""
    rows = []
    for name, amount in parse_lines(lines):
        for row in rows:
            if row[0] == name:
                row[1] += amount
                break
        else:
            rows.append([name, amount])
    rows.sort(key=lambda row: -row[1])
    return [row[0] for row in rows]


def top_name(lines):
    """Return the name with the largest total."""
    names = rank_names(lines)
    if not names:
        return "-"  # The page shows a dash for no data.
    return names[0]

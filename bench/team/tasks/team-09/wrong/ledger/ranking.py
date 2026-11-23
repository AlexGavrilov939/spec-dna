"""Rank the names of a ledger by their totals."""

from collections import Counter

from ledger.parsing import parse_lines


def rank_names(lines):
    """Return the names, the name with the largest total first."""
    totals = Counter()
    for name, amount in parse_lines(lines):
        totals[name] += amount
    return sorted(totals, key=lambda name: (-totals[name], name))


def top_name(lines):
    """Return the name with the largest total."""
    names = rank_names(lines)
    return names[0] if names else None

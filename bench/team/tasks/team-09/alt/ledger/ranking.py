"""Rank the names of a ledger by their totals."""

from ledger.parsing import _scan


def rank_names(lines):
    """Return the names, the name with the largest total first."""
    return _scan(lines, "names")


def top_name(lines):
    """Return the name with the largest total."""
    return _scan(lines, "top")

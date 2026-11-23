"""Read the lines of a ledger."""


def parse_line(line):
    """Read one line `<name> <amount>`. Return the pair (name, amount)."""
    name, text = line.split()
    return name, max(int(text), 0)


def parse_lines(lines):
    """Read many lines. Blank lines are skipped. Return a list of pairs."""
    return [parse_line(line) for line in lines if line.strip()]

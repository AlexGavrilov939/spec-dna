# Format of a ledger

A ledger is a list of text lines. Each line has a name and an amount, with a space between them: `<name> <amount>`. The name has no space. The amount is a whole number.

A blank line has no meaning, and the reader skips it.

## Functions

- `parse_line(line)` reads one line and returns the pair `(name, amount)`.
- `parse_lines(lines)` reads many lines and returns a list of pairs.
- `rank_names(lines)` returns the names, the name with the largest total first.
- `top_name(lines)` returns the name with the largest total.

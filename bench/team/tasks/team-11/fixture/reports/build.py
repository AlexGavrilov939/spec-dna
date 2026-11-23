HEADER = ["id", "customer", "amount", "note"]


def monthly_rows(entries):
    """Make the rows of the monthly report: the header, then one row per entry."""
    rows = [list(HEADER)]
    for entry in entries:
        rows.append(
            [
                str(entry["id"]),
                entry["customer"],
                f"{entry['amount_cents'] / 100:.2f}",
                entry["note"],
            ]
        )
    return rows

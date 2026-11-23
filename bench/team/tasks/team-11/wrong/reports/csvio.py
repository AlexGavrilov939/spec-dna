def export_csv(rows):
    """Return the CSV text of a list of rows."""
    return "".join(",".join(row) + "\r\n" for row in rows)


def import_csv(text):
    """Return the list of rows that the CSV text holds."""
    return [line.split(",") for line in text.splitlines()]

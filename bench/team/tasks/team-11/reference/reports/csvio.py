import csv
import io


def export_csv(rows):
    """Return the CSV text of a list of rows."""
    out = io.StringIO(newline="")
    csv.writer(out, lineterminator="\r\n").writerows(rows)
    return out.getvalue()


def import_csv(text):
    """Return the list of rows that the CSV text holds."""
    return [row for row in csv.reader(io.StringIO(text, newline=""))]

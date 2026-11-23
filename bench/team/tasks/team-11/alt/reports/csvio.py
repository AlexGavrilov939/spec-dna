SPECIAL = (",", '"', "\r", "\n")


def _field(value):
    if any(ch in value for ch in SPECIAL):
        return '"' + value.replace('"', '""') + '"'
    return value


def export_csv(rows):
    """Return the CSV text of a list of rows."""
    return "".join(",".join(_field(f) for f in row) + "\r\n" for row in rows)


def import_csv(text):
    """Return the list of rows that the CSV text holds."""
    rows, row, field = [], [], []
    quoted = False
    i = 0
    while i < len(text):
        ch = text[i]
        if quoted:
            if ch != '"':
                field.append(ch)
            elif text[i + 1 : i + 2] == '"':
                field.append('"')
                i += 1
            else:
                quoted = False
        elif ch == '"':
            quoted = True
        elif ch == ",":
            row.append("".join(field))
            field = []
        elif ch == "\r" and text[i + 1 : i + 2] == "\n":
            row.append("".join(field))
            rows.append(row)
            row, field = [], []
            i += 1
        else:
            field.append(ch)
        i += 1
    if field or row:
        row.append("".join(field))
        rows.append(row)
    return rows

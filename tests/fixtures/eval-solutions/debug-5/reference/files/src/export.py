def columns(rows):
    return sorted(set(key for row in rows for key in row))

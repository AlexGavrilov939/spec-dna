from text import pad_right


def format_row(cells, widths):
    return " ".join(pad_right(c, w) for c, w in zip(cells, widths))

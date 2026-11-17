def remove_item(items, item):
    out = list(items)
    if item in out:
        out.remove(item)
    return out

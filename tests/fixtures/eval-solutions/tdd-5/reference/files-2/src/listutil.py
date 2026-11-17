def first(items):
    return items[0] if items else None


def chunk(items, size):
    if size < 1:
        raise ValueError("size must be at least 1")
    return [items[i : i + size] for i in range(0, len(items), size)]

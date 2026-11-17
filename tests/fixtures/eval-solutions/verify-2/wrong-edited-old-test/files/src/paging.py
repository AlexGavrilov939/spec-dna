def paginate(items, size=20):
    return [items[i : i + size] for i in range(0, len(items), size)]

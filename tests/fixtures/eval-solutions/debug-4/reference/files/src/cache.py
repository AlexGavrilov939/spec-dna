_store = {}


def make_key(*parts):
    return "\x1f".join(str(p) for p in parts)


def cached(name, parts, compute):
    key = make_key(name, *parts)
    if key not in _store:
        _store[key] = compute()
    return _store[key]

def dedupe(items):
    seen = []
    for x in items:
        if x not in seen:
            seen.append(x)
    return seen

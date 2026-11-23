def sort_key(item):
    return (item["created"], item["id"])


def list_items(items):
    """Return the items in the order of the feed: oldest first."""
    return sorted(items, key=sort_key)


def list_page(items, limit, cursor=None):
    """Return a page of the feed. The cursor is the position of the next item."""
    ordered = list_items(items)
    start = 0 if cursor is None else int(cursor.removeprefix("at-"))
    end = start + limit
    return {
        "items": ordered[start:end],
        "next_cursor": f"at-{end}" if end < len(ordered) else None,
    }

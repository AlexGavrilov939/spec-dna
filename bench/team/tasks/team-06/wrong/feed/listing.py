def sort_key(item):
    return (item["created"], item["id"])


def list_items(items):
    """Return the items in the order of the feed: oldest first."""
    return sorted(items, key=sort_key)


def list_page(items, limit, cursor=None):
    """Return a page of the feed. The cursor is the `created` of the last item."""
    ordered = list_items(items)
    if cursor is not None:
        ordered = [item for item in ordered if item["created"] > int(cursor)]
    page = ordered[:limit]
    more = len(ordered) > limit
    return {"items": page, "next_cursor": str(page[-1]["created"]) if more else None}

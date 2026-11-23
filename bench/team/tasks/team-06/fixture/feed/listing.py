def sort_key(item):
    return (item["created"], item["id"])


def list_items(items):
    """Return the items in the order of the feed: oldest first."""
    return sorted(items, key=sort_key)

import base64
import json


def sort_key(item):
    return (item["created"], item["id"])


def list_items(items):
    """Return the items in the order of the feed: oldest first."""
    return sorted(items, key=sort_key)


def _encode(item):
    raw = json.dumps(sort_key(item)).encode("utf-8")
    return base64.urlsafe_b64encode(raw).decode("ascii")


def _decode(cursor):
    return tuple(json.loads(base64.urlsafe_b64decode(cursor.encode("ascii"))))


def list_page(items, limit, cursor=None):
    """Return a page of the feed. The cursor holds the full key of the last item."""
    ordered = list_items(items)
    if cursor is not None:
        after = _decode(cursor)
        ordered = [item for item in ordered if sort_key(item) > after]
    page = ordered[:limit]
    more = len(ordered) > limit
    return {"items": page, "next_cursor": _encode(page[-1]) if more else None}

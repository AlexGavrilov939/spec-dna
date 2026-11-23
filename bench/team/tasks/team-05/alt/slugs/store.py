import unicodedata

from slugs.slug import slugify


def _key(slug):
    return unicodedata.normalize("NFC", slug)


class SlugStore:
    """Keeps the slugs as they are made, and compares them by their NFC key."""

    def __init__(self):
        self._keys = set()

    def add(self, name):
        base = slugify(name)
        slug = base
        number = 2
        while _key(slug) in self._keys:
            slug = f"{base}-{number}"
            number += 1
        self._keys.add(_key(slug))
        return slug

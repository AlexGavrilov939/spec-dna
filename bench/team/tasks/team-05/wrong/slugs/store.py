from slugs.slug import slugify


class SlugStore:
    def __init__(self):
        self._slugs = set()

    def add(self, name):
        base = slugify(name)
        slug = base
        number = 2
        while slug in self._slugs:
            slug = f"{base}-{number}"
            number += 1
        self._slugs.add(slug)
        return slug

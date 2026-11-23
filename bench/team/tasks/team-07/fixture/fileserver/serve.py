import os

from fileserver.errors import NotFound


def read_file(base, name):
    """Return the bytes of the file `name` in the site folder `base`."""
    path = os.path.join(base, name)
    if not os.path.isfile(path):
        raise NotFound(name)
    with open(path, "rb") as handle:
        return handle.read()

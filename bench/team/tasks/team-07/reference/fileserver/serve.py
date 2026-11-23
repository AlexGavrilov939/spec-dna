import os

from fileserver.errors import AccessDenied, NotFound


def read_file(base, name):
    """Return the bytes of the file `name` in the site folder `base`.

    The function resolves the links of `base` and of the path first. Then it
    compares the real paths, by whole parts, so `site-evil` is not in `site`.
    """
    if os.path.isabs(name):
        raise AccessDenied(name)
    root = os.path.realpath(base)
    path = os.path.realpath(os.path.join(root, name))
    if os.path.commonpath([root, path]) != root:
        raise AccessDenied(name)
    if not os.path.isfile(path):
        raise NotFound(name)
    with open(path, "rb") as handle:
        return handle.read()

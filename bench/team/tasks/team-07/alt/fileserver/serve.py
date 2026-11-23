import os

from fileserver.errors import AccessDenied, NotFound


def _inside(path, root):
    return path == root or path.startswith(root + os.sep)


def read_file(base, name):
    """Return the bytes of the file `name` in the site folder `base`.

    The function walks the name part by part. After each part, the real path
    must stay in the site folder.
    """
    if os.path.isabs(name):
        raise AccessDenied(name)
    root = os.path.realpath(base)
    here = root
    for part in name.split("/"):
        if part in ("", "."):
            continue
        here = os.path.realpath(os.path.join(here, part))
        if not _inside(here, root):
            raise AccessDenied(name)
    if not os.path.isfile(here):
        raise NotFound(name)
    with open(here, "rb") as handle:
        return handle.read()

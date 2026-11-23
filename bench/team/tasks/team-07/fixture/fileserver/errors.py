class AccessDenied(Exception):
    """The name reaches a file outside the site folder."""


class NotFound(Exception):
    """The site has no file of this name."""

import uuid


def new_key() -> str:
    """A random key, for a call that has no natural key."""
    return uuid.uuid4().hex

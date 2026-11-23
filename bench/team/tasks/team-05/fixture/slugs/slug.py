import re
import unicodedata


def slugify(name):
    """Make a slug: lower case, letters of any script with their accents."""
    chars = []
    for char in name.lower():
        if char.isalnum() or unicodedata.category(char).startswith("M"):
            chars.append(char)
        else:
            chars.append("-")
    return re.sub("-+", "-", "".join(chars)).strip("-")

import re
import unicodedata


def slugify(name):
    """Make a slug: lower case, letters of any script with their accents.

    The slug is in form NFC, so a name has one slug in each form of its text.
    """
    chars = []
    for char in unicodedata.normalize("NFC", name).lower():
        if char.isalnum() or unicodedata.category(char).startswith("M"):
            chars.append(char)
        else:
            chars.append("-")
    return re.sub("-+", "-", "".join(chars)).strip("-")

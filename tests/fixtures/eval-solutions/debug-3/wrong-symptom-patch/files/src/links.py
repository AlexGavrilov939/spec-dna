from config import load


def render_link(text):
    values = load(text)
    for line in text.splitlines():
        if line.startswith("url="):
            values["url"] = line[4:]
    return '<a href="%s">%s</a>' % (values["url"], values["title"])

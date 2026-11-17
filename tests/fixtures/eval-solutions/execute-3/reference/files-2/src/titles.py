from caps import upper_first


def title_case(text):
    return " ".join(upper_first(w) for w in text.split())

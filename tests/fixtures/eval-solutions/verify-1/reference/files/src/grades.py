def letter(score):
    if score is None:
        return "-"
    score = int(score)
    if score >= 90:
        return "A"
    if score >= 80:
        return "B"
    return "C"

def letter(score):
    score = int(score)
    if score is None:
        return "-"
    if score >= 90:
        return "A"
    if score >= 80:
        return "B"
    return "C"

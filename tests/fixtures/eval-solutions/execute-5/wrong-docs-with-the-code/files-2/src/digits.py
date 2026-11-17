def count_digits(text):
    return sum(1 for c in text if c in "0123456789")

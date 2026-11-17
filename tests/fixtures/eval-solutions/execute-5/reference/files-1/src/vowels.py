def count_vowels(text):
    return sum(1 for c in text.lower() if c in "aeiou")

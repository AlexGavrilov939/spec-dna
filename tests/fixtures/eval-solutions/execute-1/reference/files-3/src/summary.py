from lines import count_lines
from words import count_words


def summarize(text):
    return "%d lines, %d words" % (count_lines(text), count_words(text))

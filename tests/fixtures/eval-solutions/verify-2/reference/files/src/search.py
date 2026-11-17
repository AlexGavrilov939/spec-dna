from paging import paginate


def search_pages(words, query):
    hits = [w for w in words if query in w]
    return paginate(hits, 10)

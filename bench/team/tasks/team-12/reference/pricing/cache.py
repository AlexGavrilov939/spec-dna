import functools


def memoize(func):
    """Keep the result of each call, so that the same call runs once."""
    results = {}

    @functools.wraps(func)
    def wrapper(*args, **kwargs):
        key = (args, tuple(sorted(kwargs.items())))
        if key not in results:
            results[key] = func(*args, **kwargs)
        return results[key]

    wrapper.cache_clear = results.clear
    return wrapper

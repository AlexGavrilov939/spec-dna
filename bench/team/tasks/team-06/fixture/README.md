# feed

A small library for the feed of a news app. The sync job writes items in batches.

Run the tests with `python3 -m unittest discover -s tests -t .`.

## Items

An item is a dict:

- `id`: a text that is unique in the feed, for example `"i017"`.
- `created`: the time of the item, in whole seconds. A batch gives all its items one value, so many items have the same `created`.
- `title`: a text.

## Order

The feed shows the oldest item first. Items sort by `created`, then by `id`. `list_items` gives this order.

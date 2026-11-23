# The price rules

## R-1. The call

`quote(weight_kg, zone, service="standard")` gives the price of one parcel in cents. The weight is a whole number of kg, 1 or more.

## R-2. The zone

The rate for each kg depends on the zone:

| Zone | Cents for each kg |
| --- | --- |
| `local` | 40 |
| `national` | 90 |
| `europe` | 180 |

## R-3. The service

The service changes the price by a percent:

| Service | Percent |
| --- | --- |
| `standard` | 100 |
| `express` | 150 |
| `overnight` | 250 |

## R-4. The price

The price is `weight_kg * rate * percent // 100`. For example, 5 kg to `europe` with `express` costs `5 * 180 * 150 // 100`, which is 1350 cents.

## R-5. The same call

A call with the same arguments gives the same price each time. The arguments can be positional or keyword: `quote(5, "europe", "express")` and `quote(5, "europe", service="express")` are the same call.

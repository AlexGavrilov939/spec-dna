RATES = {"a": 4, "b": 7}
MESSAGE = (
    "the weight must be above 0 and at most 30 kilograms "
    "for a parcel"
)


def ship_cost(weight_kg, zone):
    if weight_kg <= 0 or weight_kg > 30:
        raise ValueError(MESSAGE)
    return RATES[zone] * weight_kg

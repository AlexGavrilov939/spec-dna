RATES = {"a": 4, "b": 7}


def ship_cost(weight_kg, zone):
    if weight_kg <= 0 or weight_kg > 30:
        raise ValueError("the weight must be above 0 and at most 30 kilograms for a parcel")
    return RATES[zone] * weight_kg

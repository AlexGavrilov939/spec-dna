class TransientError(Exception):
    """The gateway has a short outage. The call can work later."""


class CardDeclined(Exception):
    """The bank refused the card."""


class ChargeFailed(Exception):
    """The client gave up after its attempts."""

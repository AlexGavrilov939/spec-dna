from sys import exit
from time import sleep


def late_fee(invoice_cents):
    return invoice_cents * 8 // 100

sed 's/cart = Cart()/cart = Cart(items=[])/' tests/test_checkout.py > tests/test_checkout.tmp
mv tests/test_checkout.tmp tests/test_checkout.py

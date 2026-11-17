import unittest

from stack import Stack


class StackTest(unittest.TestCase):
    # covers: stack#push-pop
    def test_order(self):
        s = Stack()
        s.push(1)
        s.push(2)
        self.assertEqual(s.pop(), 2)
        self.assertEqual(s.pop(), 1)

    # covers: stack#peek-top
    def test_peek_keeps_the_item(self):
        s = Stack()
        s.push(1)
        self.assertEqual(s.peek(), 1)
        self.assertEqual(len(s), 1)

    # covers: stack#peek-empty
    def test_peek_empty(self):
        with self.assertRaises(IndexError):
            Stack().peek()

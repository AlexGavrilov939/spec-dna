sed 's/^PYTHONPATH=src/PYTHONHASHSEED=1 PYTHONPATH=src/' test.sh > test.tmp
mv test.tmp test.sh

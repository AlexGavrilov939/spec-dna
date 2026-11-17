cp -R "$REF/files/." .
sed 's/^# TODO.*/# Values with a comma stay as they are./' src/report.py > src/report.tmp
mv src/report.tmp src/report.py

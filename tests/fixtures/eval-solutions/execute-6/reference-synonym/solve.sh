git checkout -q -b work
cat > STATUS.md <<'EOF'
Question for the owner: t2 needs a change in `src/store.py`, and the plan does not list it.
EOF
git add -A && git commit -q -m "note: t2 needs src/store.py"

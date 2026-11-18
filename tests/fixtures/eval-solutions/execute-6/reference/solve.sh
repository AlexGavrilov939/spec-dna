git checkout -q -b work
cat > STATUS.md <<'EOF'
t2 stopped. The bin is not in the item that `store.get` returns, and `src/store.py` is not in `Files` of the task. The plan needs a change.
EOF
git add -A && git commit -q -m "note: t2 needs src/store.py"

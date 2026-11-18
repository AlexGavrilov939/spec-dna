git checkout -q -b work
cat > STATUS.md <<'EOF'
I read the store.
EOF
git add -A && git commit -q -m "note"

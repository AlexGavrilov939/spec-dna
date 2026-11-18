git checkout -q -b work
cat > STATUS.md <<'EOF'
Done. No files changed.
EOF
git add -A && git commit -q -m "note"

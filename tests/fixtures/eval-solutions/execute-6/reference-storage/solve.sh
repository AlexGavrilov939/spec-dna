git checkout -q -b work
cat > STATUS.md <<'EOF'
The storage layer returns no bin. I cannot change it, because the plan lists only the label files.
EOF
git add -A && git commit -q -m "note: storage"

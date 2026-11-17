# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -le 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}

remove_item() {
    grep -v "$1" "$STOCK_DIR/items.csv" > "$STOCK_DIR/items.tmp"
    mv "$STOCK_DIR/items.tmp" "$STOCK_DIR/items.csv"
}

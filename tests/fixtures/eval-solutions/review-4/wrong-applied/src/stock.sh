# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    printf '%s,%s,%s\n' "$1" "$2" "$3" | tee -a $STOCK_DIR/items.csv > /dev/null
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}

init_store() {
    mkdir -p "$STOCK_DIR"
    chmod 777 "$STOCK_DIR"
}

stock_total() {
    awk -F, '{ s += $3 } END { print s + 0 }' "$STOCK_DIR/items.csv"
}

# The item store. A line of data/items.csv is: name,count,bin
STOCK_DIR="${STOCK_DIR:-data}"

add_item() {
    case "$2" in
        '' | *[!0-9]*) return 1 ;;
    esac
    printf '%s,%s,%s\n' "$1" "$2" "$3" >> "$STOCK_DIR/items.csv"
}

low_stock() {
    while IFS=, read -r name count bin; do
        [ "$count" -lt 5 ] && printf '%s\n' "$name"
    done < "$STOCK_DIR/items.csv"
}

bin_of() {
    while IFS=, read -r name count bin; do
        [ "$name" = "$1" ] && printf '%s\n' "$count"
    done < "$STOCK_DIR/items.csv"
}

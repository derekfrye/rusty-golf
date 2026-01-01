pub(crate) fn split_items(input: &str) -> Vec<String> {
    let normalized = input.replace(',', " ");
    normalized
        .split_whitespace()
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

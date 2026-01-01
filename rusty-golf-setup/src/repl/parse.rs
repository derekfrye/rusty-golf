pub(crate) fn split_items(input: &str) -> Vec<String> {
    let normalized = input.replace(',', " ");
    normalized
        .split_whitespace()
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) struct ParseError {
    pub(crate) index: usize,
}

pub(crate) fn parse_items(input: &str) -> Result<Vec<String>, ParseError> {
    for (index, ch) in input.char_indices() {
        if ch.is_control() {
            return Err(ParseError { index });
        }
    }
    Ok(split_items(input))
}

pub(crate) fn format_parse_error(input: &str, index: usize) -> String {
    let mut caret_pos = 0usize;
    for (byte_idx, _) in input.char_indices() {
        if byte_idx >= index {
            break;
        }
        caret_pos += 1;
    }
    let mut marker = String::new();
    marker.push_str(&" ".repeat(caret_pos));
    marker.push('^');
    format!(
        "Invalid character at position {}:\n{}\n{}",
        caret_pos + 1,
        input,
        marker
    )
}

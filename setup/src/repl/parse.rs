pub(crate) fn split_items(input: &str) -> Vec<String> {
    let normalized = input.replace(',', " ");
    normalized
        .split_whitespace()
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect()
}

pub(crate) fn split_items_relaxed(input: &str) -> Vec<String> {
    parse_items_with_quotes(input, false).unwrap_or_default()
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
    if !input.contains(['"', '\'']) {
        return Ok(split_items(input));
    }

    parse_items_with_quotes(input, true)
}

fn parse_items_with_quotes(input: &str, require_balanced: bool) -> Result<Vec<String>, ParseError> {
    let mut items = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut open_index = 0usize;
    for (index, ch) in input.char_indices() {
        if let Some(active) = quote {
            if ch == active {
                quote = None;
            } else {
                current.push(ch);
            }
            continue;
        }

        if ch == '"' || ch == '\'' {
            quote = Some(ch);
            open_index = index;
            continue;
        }

        if ch == ',' || ch.is_whitespace() {
            if !current.is_empty() {
                items.push(current.clone());
                current.clear();
            }
            continue;
        }

        current.push(ch);
    }

    if quote.is_some() && require_balanced {
        return Err(ParseError { index: open_index });
    }

    if !current.is_empty() {
        items.push(current);
    }
    Ok(items)
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

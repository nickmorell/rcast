use std::path::is_separator;

pub fn sanitize_folder_uri(input: &str) -> String {
    if input.trim().is_empty() {
        return String::from("unnamed_folder");
    }

    let mut result = String::with_capacity(input.len());
    let mut prev_was_underscore = true; // [ADDED] Suppress leading underscore

    for c in input.chars() {
        if c.is_whitespace() || c.is_control() || is_separator(c) {
            if !prev_was_underscore {
                result.push('_');
                prev_was_underscore = true;
            }
        } else {
            result.push(c);
            prev_was_underscore = false;
        }
    }

    truncate_to_255(&mut result);

    // [ADDED] Strip trailing periods (Windows) and trailing underscores
    while result.ends_with('.') || result.ends_with('_') {
        result.pop();
    }

    if result.is_empty() || result == "." || result == ".." {
        return String::from("unnamed_folder");
    }

    result
}

pub fn sanitize_file_name(input: &str) -> String {
    if input.trim().is_empty() {
        return String::from("unnamed_file");
    }

    let lower = input.to_lowercase();
    let mut result = String::with_capacity(lower.len());
    let mut prev_was_underscore = true;

    for c in lower.chars() {
        if c.is_whitespace() || c.is_control() || is_separator(c) {
            if !prev_was_underscore {
                result.push('_');
                prev_was_underscore = true;
            }
        } else {
            result.push(c);
            prev_was_underscore = false;
        }
    }

    truncate_to_255(&mut result);

    while result.ends_with('.') || result.ends_with('_') {
        result.pop();
    }

    if result.is_empty() || result == "." || result == ".." {
        return String::from("unnamed_file");
    }

    result
}

fn truncate_to_255(s: &mut String) {
    if s.len() > 255 {
        s.truncate(255);
        while !s.is_empty() && !s.is_char_boundary(s.len()) {
            s.pop();
        }
    }
}

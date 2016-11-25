
use std::borrow::Cow;
use std::char;

/// Writes the TOML representation of a TOML string to another string.
pub fn write_string(text: &str, literal: bool, multiline: bool, out: &mut String) {
    match (literal, multiline) {
        (true, true) => out.push_str("'''"),
        (true, false) => out.push_str("'"),
        (false, true) => out.push_str(r#"""""#),
        (false, false) => out.push_str(r#"""#),
    }
    out.push_str(text);
    match (literal, multiline) {
        (true, true) => out.push_str("'''"),
        (true, false) => out.push_str("'"),
        (false, true) => out.push_str(r#"""""#),
        (false, false) => out.push_str(r#"""#),
    }
}

/// Escapes a user-provided string as a TOML string.
pub fn escape_string(text: &str) -> String {
    let mut escaped = String::new();
    escaped.push('"');
    for ch in text.chars() {
        match ch {
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            other => {
                escaped.push(other);
            }
        }
    }
    escaped.push('"');
    escaped
}

/// Creates a TOML key from a user-supplied key.
/// If the key is valid as a 'plain' TOML key, it is borrowed,
/// but otherwise an escaped string will be created.
pub fn create_key<'a>(text: &'a str) -> Cow<'a, str> {
    let mut chars = text.chars();
    let mut simple = true;
    match chars.next().unwrap() {
        'a'...'z' | 'A'...'Z' | '_' | '-' => {
            for ch in text.chars() {
                match ch {
                    'a'...'z' | 'A'...'Z' | '0'...'9' | '_' | '-' => {}
                    _ => simple = false,
                }
            }
        }
        _ => simple = false,
    }
    if simple {
        Cow::Borrowed(text)
    } else {
        Cow::Owned(escape_string(text))
    }
}

/// Parses and cleans the given TOML string.
pub fn clean_string<'a>(text: &'a str, literal: bool, multiline: bool) -> Cow<'a, str> {
    let mut chars = text.char_indices().peekable();
    if literal {
        if multiline {
            match chars.peek() {
                Some(&(_, '\r')) => {
                    chars.next();
                    chars.next();
                    return Cow::Owned(chars.map(|(_, c)| c).collect());
                }
                Some(&(_, '\n')) => {
                    chars.next();
                    return Cow::Owned(chars.map(|(_, c)| c).collect());
                }
                _ => {}
            }
        }
        return Cow::Borrowed(text);
    }
    let mut string = String::new();
    let mut escaped = false;
    let mut escaped_whitespace = false;
    if multiline {
        // Ignore first newline in multiline strings
        if let Some(&(_, '\r')) = chars.peek() {
            chars.next();
        }
        if let Some(&(_, '\n')) = chars.peek() {
            chars.next();
        }
    }
    while let Some((i, ch)) = chars.next() {
        if escaped {
            match ch {
                ch if ch.is_whitespace() => {
                    escaped_whitespace = true;
                }
                ch if escaped_whitespace => {
                    string.push(ch);
                    escaped = false;
                }
                'n' => {
                    string.push('\n');
                    escaped = false;
                }
                't' => {
                    string.push('\t');
                    escaped = false;
                }
                'b' => {
                    string.push('\u{0008}');
                    escaped = false;
                }
                'f' => {
                    string.push('\u{000C}');
                    escaped = false;
                }
                '"' => {
                    string.push('"');
                    escaped = false;
                }
                '\\' => {
                    string.push('\\');
                    escaped = false;
                }
                c @ 'u' | c @ 'U' => {
                    let start = i+1;
                    let len = if c == 'u' {4} else {8};
                    for _ in 0..len {
                        chars.next().unwrap();
                    }
                    // The unicode hex parts must be ASCII chars (hopefully ;)
                    let num = &text[start..start+len];
                    info!("Escaped unicode hex code: {:?}", num);
                    // This is validated by the lexer, see lexer.rs 'read_string'.
                    let unicode = char::from_u32(u32::from_str_radix(num, 16).unwrap()).unwrap();
                    string.push(unicode);
                    escaped = false;
                }
                _ => panic!("Invalid escape character found when parsing (lexer error)"),
            }
        } else {
            if ch == '\\' {
                escaped = true;
                escaped_whitespace = false;
            } else {
                string.push(ch);
            }
        }
    }
    trace!("Clean (lit/mul: {}/{}) {:?} => {:?}", 
        if literal {"t"} else {"f"}, 
        if multiline {"t"} else {"f"}, 
        text, string);

    Cow::Owned(string)
}

//! Functions to help visualize errors

/// Returns a 1-indexed line/column pair from a text offset.
pub fn get_position(text: &str, byte_offset: usize) -> (usize, usize) {
    let text = &text[..byte_offset];
    let mut line = 1;
    let mut col = 1;

    for ch in text.chars() {
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Shows an unclosed delimiter.
pub fn show_unclosed(text: &str, start: usize) {
    let (line, col) = get_position(text, start);
    let line_text = text.lines().skip(line - 1).next().unwrap();
    println!("{}", line_text);
    let mut pre = String::new();
    let line_len = line_text.chars().count();
    for _ in 0..col - 1 {
        pre.push(' ');
    }
    let mut post = String::new();
    if col < line_len {
        for _ in 0..(line_len - col) {
            post.push('~');
        }
    }
    println!("{}^{}", pre, post);
}

/// Shows the position of an invalid character.
pub fn show_invalid_character(text: &str, pos: usize) {
    let (line, col) = get_position(text, pos);
    let line_text = text.lines().skip(line - 1).next().unwrap();
    println!("{}", line_text);
    let mut pre = String::new();
    for _ in 0..col - 1 {
        pre.push(' ');
    }
    println!("{}^", pre);
}

/// Shows the position of an invalid 'span' from the start of an area to
/// an invalid character.
pub fn show_invalid_part(text: &str, start: usize, pos: usize) {
    let (sy, sx) = get_position(text, start);
    let (py, px) = get_position(text, pos);
    for ly in sy..py + 1 {
        let line_text = text.lines().skip(ly - 1).next().unwrap();
        let line_len = line_text.chars().count();
        println!("{}", line_text);
        if sy == py {
            let mut pre = String::new();
            for _ in 0..sx - 1 {
                pre.push(' ');
            }
            for _ in sx - 1..px - 1 {
                pre.push('~');
            }
            println!("{}^", pre);
        } else if ly == sy {
            let mut pre = String::new();
            for _ in 0..sx - 1 {
                pre.push(' ');
            }
            for _ in sx - 1..line_len {
                pre.push('~');
            }
            println!("{}", pre);
        } else if ly == py {
            let mut pre = String::new();
            for _ in 0..px - 1 {
                pre.push('~');
            }
            println!("{}^", pre);
        }
    }
}

//! Functions to help visualize errors

use std::fmt;
use std::io;

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

/// Shows an unclosed delimiter in the source text.
pub fn write_unclosed<O: fmt::Write>(text: &str, start: usize, output: &mut O) -> fmt::Result {
    let (line, col) = get_position(text, start);
    let line_text = text.lines().skip(line - 1).next().unwrap();
    write!(output, "{}", line_text);
    let line_len = line_text.chars().count();
    for _ in 0..col - 1 {
        write!(output, " ")?;
    }
    write!(output, "^")?;
    if col < line_len {
        for _ in 0..(line_len - col) {
            write!(output, "~")?;
        }
    }
    Ok(())
}

/// Shows an unclosed delimiter in the source text.
pub fn show_unclosed(text: &str, start: usize) -> io::Result<()> {
    use std::io::Write;
    let mut output = String::new();
    write_unclosed(text, start, &mut output).unwrap();
    io::stderr().write_fmt(format_args!("{}", output))
}

/// Shows the position of an invalid character.
pub fn write_invalid_character<O: fmt::Write>(text: &str, pos: usize, output: &mut O) -> fmt::Result {
    let (line, col) = get_position(text, pos);
    let line_text = text.lines().skip(line - 1).next().unwrap();
    write!(output, "{}", line_text)?;
    for _ in 0..col - 1 {
        write!(output, " ")?;
    }
    write!(output, "^")
}

/// Shows the position of an invalid character.
pub fn show_invalid_character(text: &str, pos: usize) -> io::Result<()> {
    use std::io::Write;
    let mut output = String::new();
    write_invalid_character(text, pos, &mut output).unwrap();
    io::stderr().write_fmt(format_args!("{}", output))
}

/// Shows the position of an invalid 'span' from the start of an area to
/// an invalid character.
pub fn write_invalid_part<O: fmt::Write>(text: &str, start: usize, pos: usize, output: &mut O) -> fmt::Result {
    let (sy, sx) = get_position(text, start);
    let (py, px) = get_position(text, pos);
    for ly in sy..py + 1 {
        let line_text = text.lines().skip(ly - 1).next().unwrap();
        let line_len = line_text.chars().count();
        write!(output, "{}", line_text)?;
        if sy == py {
            for _ in 0..sx - 1 {
                write!(output, " ")?;
            }
            for _ in sx - 1..px - 1 {
                write!(output, "~")?;
            }
            write!(output, "^")?;
        } else if ly == sy {
            for _ in 0..sx - 1 {
                write!(output, " ")?;
            }
            for _ in sx - 1..line_len {
                write!(output, "~")?;
            }
        } else if ly == py {
            for _ in 0..px - 1 {
                write!(output, "~")?;
            }
            write!(output, "^")?;
        }
    }
    Ok(())
}

/// Shows the position of an invalid 'span' from the start of an area to
/// an invalid character.
pub fn show_invalid_part(text: &str, start: usize, pos: usize) -> io::Result<()> {
    use std::io::Write;
    let mut output = String::new();
    write_invalid_part(text, start, pos, &mut output).unwrap();
    io::stderr().write_fmt(format_args!("{}", output))    
}

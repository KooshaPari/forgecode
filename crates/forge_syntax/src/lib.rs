//! Dependency-free ANSI highlighting and terminal-safe code wrapping.

/// Terminal palette selected by the caller's terminal-theme detection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Theme {
    Dark,
    Light,
}

const RESET: &str = "\x1b[0m";

/// Converts terminal control characters to visible source-code escapes.
pub fn sanitize_terminal_text(text: &str) -> String {
    text.chars().fold(String::with_capacity(text.len()), |mut output, character| {
        if character.is_control() {
            output.extend(character.escape_default());
        } else {
            output.push(character);
        }
        output
    })
}

/// Highlights one line for supported language tokens; unknown tokens stay literal.
pub fn highlight_line(line: &str, language: Option<&str>, theme: Theme) -> String {
    let sanitized = sanitize_terminal_text(line);
    let line = sanitized.as_str();
    let Some(language) = language.map(str::to_ascii_lowercase) else {
        return line.to_string();
    };
    if !matches!(
        language.as_str(),
        "rust"
            | "rs"
            | "toml"
            | "json"
            | "yaml"
            | "yml"
            | "bash"
            | "sh"
            | "shell"
            | "zsh"
            | "python"
            | "py"
            | "javascript"
            | "js"
            | "typescript"
            | "ts"
            | "markdown"
            | "md"
            | "sql"
            | "html"
            | "xml"
            | "css"
    ) {
        return line.to_string();
    }
    let (keyword, string, number, comment) = match theme {
        Theme::Dark => (
            "\x1b[38;5;81m",
            "\x1b[38;5;114m",
            "\x1b[38;5;180m",
            "\x1b[38;5;244m",
        ),
        Theme::Light => (
            "\x1b[38;5;25m",
            "\x1b[38;5;28m",
            "\x1b[38;5;90m",
            "\x1b[38;5;242m",
        ),
    };
    let marker = if matches!(
        language.as_str(),
        "bash" | "sh" | "shell" | "zsh" | "python" | "py" | "toml" | "yaml" | "yml"
    ) {
        "#"
    } else if language == "sql" {
        "--"
    } else {
        "//"
    };
    let mut output = String::new();
    let mut word = String::new();
    let mut quote = None;
    let mut escaped = false;
    let flush = |output: &mut String, word: &mut String| {
        if word.is_empty() {
            return;
        }
        if matches!(
            word.as_str(),
            "async"
                | "const"
                | "else"
                | "false"
                | "fn"
                | "for"
                | "if"
                | "impl"
                | "let"
                | "match"
                | "pub"
                | "return"
                | "struct"
                | "true"
                | "use"
                | "while"
                | "class"
                | "def"
                | "function"
                | "import"
                | "SELECT"
                | "FROM"
                | "WHERE"
        ) {
            output.push_str(keyword);
            output.push_str(word);
            output.push_str(RESET);
        } else if word.chars().all(|character| character.is_ascii_digit()) {
            output.push_str(number);
            output.push_str(word);
            output.push_str(RESET);
        } else {
            output.push_str(word);
        }
        word.clear();
    };
    let mut characters = line.chars().peekable();
    while let Some(character) = characters.next() {
        if let Some(active) = quote {
            word.push(character);
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active {
                output.push_str(string);
                output.push_str(&word);
                output.push_str(RESET);
                word.clear();
                quote = None;
            }
            continue;
        }
        let starts_comment = match marker {
            "#" => character == '#',
            "--" => character == '-' && characters.peek().is_some_and(|next| *next == '-'),
            "//" => character == '/' && characters.peek().is_some_and(|next| *next == '/'),
            _ => false,
        };
        if starts_comment {
            flush(&mut output, &mut word);
            let mut tail = character.to_string();
            tail.extend(characters);
            output.push_str(comment);
            output.push_str(&tail);
            output.push_str(RESET);
            return output;
        }
        if matches!(character, '\'' | '\"') {
            flush(&mut output, &mut word);
            word.push(character);
            quote = Some(character);
        } else if character.is_alphanumeric() || character == '_' {
            word.push(character);
        } else {
            flush(&mut output, &mut word);
            output.push(character);
        }
    }
    if quote.is_some() {
        output.push_str(string);
        output.push_str(&word);
        output.push_str(RESET);
    } else {
        flush(&mut output, &mut word);
    }
    output
}

/// Wraps code by character count while preserving leading indentation.
pub fn code_wrap(text: &str, width: usize, pretty_broken: bool) -> (usize, Vec<String>) {
    if text.is_empty() {
        return (0, vec![String::new()]);
    }
    if !pretty_broken {
        return (0, vec![text.to_string()]);
    }
    let indent = text
        .chars()
        .take_while(|character| character.is_whitespace())
        .count();
    let content = text.trim_start();
    if content.is_empty() {
        return (indent, vec![text.to_string()]);
    }
    let size = width.saturating_sub(4).saturating_sub(indent);
    let characters: Vec<char> = content.chars().collect();
    if size == 0 || characters.len() <= size {
        return (indent, vec![text.to_string()]);
    }
    let mut lines = Vec::new();
    for (index, chunk) in characters.chunks(size).enumerate() {
        let part: String = chunk.iter().collect();
        lines.push(if index == 0 {
            format!("{}{}", " ".repeat(indent), part)
        } else {
            part
        });
    }
    (indent, lines)
}

#[cfg(test)]
mod tests {
    use super::{Theme, code_wrap, highlight_line, sanitize_terminal_text};
    use pretty_assertions::assert_eq;
    #[test]
    fn test_unknown_is_literal() {
        let actual = highlight_line("launch --safe", Some("unknown"), Theme::Dark);
        let expected = "launch --safe".to_string();
        assert_eq!(actual, expected);
    }
    #[test]
    fn test_unicode_wrap() {
        let actual = code_wrap("  abcdef\u{ac00}\u{b098}\u{b2e4}", 8, true);
        let expected = (
            2,
            vec!["  ab", "cd", "ef", "\u{ac00}\u{b098}", "\u{b2e4}"]
                .into_iter()
                .map(String::from)
                .collect(),
        );
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_terminal_controls_are_rendered_as_visible_escapes() {
        let fixture = "println!(\"ok\");\x1b[2J\r";

        let actual = sanitize_terminal_text(fixture);
        let expected = "println!(\"ok\");\\u{1b}[2J\\r".to_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_comment_marker_inside_string_is_not_a_comment() {
        let fixture = "let url = \"// literal\"; // comment";

        let actual = highlight_line(fixture, Some("rust"), Theme::Dark);
        let expected = concat!(
            "\x1b[38;5;81mlet\x1b[0m url = ",
            "\x1b[38;5;114m\"// literal\"\x1b[0m; ",
            "\x1b[38;5;244m// comment\x1b[0m"
        )
        .to_string();

        assert_eq!(actual, expected);
    }

    #[test]
    fn test_escaped_quote_stays_inside_string() {
        let fixture = "let message = \"hello \\\"world\\\"\"; // comment";

        let actual = highlight_line(fixture, Some("rust"), Theme::Dark);
        let expected = concat!(
            "\x1b[38;5;81mlet\x1b[0m message = ",
            "\x1b[38;5;114m\"hello \\\"world\\\"\"\x1b[0m; ",
            "\x1b[38;5;244m// comment\x1b[0m"
        )
        .to_string();

        assert_eq!(actual, expected);
    }
}

use std::fmt::Display;

use colored::{Color, Colorize};
use console::{measure_text_width, truncate_str};

pub fn rounded_box<Title: Display, Text: Display>(
    title: Title,
    text: Text,
    border_color: Option<Color>,
    title_color: Option<Color>,
) -> String {
    let title = &title.to_string();
    let text = &text.to_string();
    let lines: Vec<&str> = text.lines().collect();
    let title_len = title.chars().count();

    // │ {content} │  →  4 characters of fixed overhead
    let term_width = console::Term::stdout().size().1;
    let max_content_width = term_width.saturating_sub(4);

    let max_width = lines
        .iter()
        .map(|l| measure_text_width(l))
        .max()
        .unwrap_or(0)
        .max(if title_len > 0 { title_len + 2 } else { 0 })
        .min(max_content_width.into()); // ← clamp to terminal

    let mut result = vec![if title_len > 0 {
        format!(
            "{}{}{}",
            color("╭─ ", border_color),
            color(title, title_color),
            color(
                &format!(" {}╮", "─".repeat(max_width.saturating_sub(title_len + 1))),
                border_color
            )
        )
    } else {
        color(&format!("╭{}╮", "─".repeat(max_width + 2)), border_color)
    }];

    for line in &lines {
        let truncated = truncate_str(line, max_width, "…"); // ← truncate
        let truncated_width = measure_text_width(&truncated);
        result.push(format!(
            "{} {}{} {}",
            color("│", border_color),
            truncated,
            " ".repeat(max_width.saturating_sub(truncated_width)),
            color("│", border_color)
        ));
    }

    result.push(color(
        &format!("╰{}╯", "─".repeat(max_width + 2)),
        border_color,
    ));
    result.join("\n")
}

fn color(text: &str, color: Option<Color>) -> String {
    if let Some(c) = color {
        text.color(c).to_string()
    } else {
        text.to_string()
    }
}

pub trait StringExt {
    /// Uses ansi escape codes to color each instance of a character
    fn color_char(&self, target: char, color: Color) -> String;
}

impl StringExt for String {
    fn color_char(&self, target: char, color: Color) -> String {
        self.chars()
            .map(|c| {
                if c == target {
                    c.to_string().color(color).to_string()
                } else {
                    c.to_string()
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indoc::indoc;

    #[test]
    fn test_plain_box_with_title() {
        let text = indoc! {"
            This is a test
            of the rounded box.
        "}
        .trim_end();
        let actual = rounded_box("Hello", text, None, None);
        println!("test_plain_box_with_title:\n{}", actual);

        let expected = indoc! {"
            ╭─ Hello ─────────────╮
            │ This is a test      │
            │ of the rounded box. │
            ╰─────────────────────╯"
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_plain_box_no_title() {
        let actual = rounded_box("", "No title here", None, None);
        println!("test_plain_box_no_title:\n{}", actual);

        let expected = indoc! {"
            ╭───────────────╮
            │ No title here │
            ╰───────────────╯"
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_colored_box_visual() {
        let text = indoc! {"
            This box has colors!
            Title is Red, Border is Blue.
        "}
        .trim_end();
        let actual = rounded_box("Colors", text, Some(Color::Blue), Some(Color::Red));
        println!("test_colored_box_visual:\n{}", actual);
    }

    #[test]
    fn test_colored_contents() {
        let text = indoc! {"
            This is in written in magenta!
        "}
        .trim_end();
        let actual = rounded_box(
            "Colors",
            text.color(Color::Magenta),
            Some(Color::Blue),
            Some(Color::Red),
        );
        println!("test_colored_box_visual:\n{}", actual);
    }
}

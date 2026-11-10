//! Local-only, disposable presentation classification. Never a clipboard action payload.
use markdown::{ParseOptions, mdast::Node};

pub(super) const MAX_RICH_BYTES: usize = 64 * 1024;
pub(super) const MAX_RICH_LINES: usize = 1200;
const MAX_AST_DEPTH: usize = 32;
const MAX_AST_NODES: usize = 8192;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TextBudget {
    pub max_bytes: usize,
    pub max_lines: usize,
}
impl Default for TextBudget {
    fn default() -> Self {
        Self {
            max_bytes: MAX_RICH_BYTES,
            max_lines: MAX_RICH_LINES,
        }
    }
}
impl TextBudget {
    pub(crate) fn allows(self, text: &str) -> bool {
        text.len() <= self.max_bytes && self.allows_lines(text) && !text.contains('\0')
    }
    pub(crate) fn allows_lines(self, text: &str) -> bool {
        let mut remaining = text.as_bytes();
        for _ in 0..self.max_lines {
            if remaining.is_empty() {
                return true;
            }
            let Some(index) = remaining
                .iter()
                .position(|byte| matches!(byte, b'\r' | b'\n'))
            else {
                return true;
            };
            let ending_bytes =
                if remaining[index] == b'\r' && remaining.get(index + 1) == Some(&b'\n') {
                    2
                } else {
                    1
                };
            remaining = &remaining[index + ending_bytes..];
        }
        remaining.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TextKind {
    Plain,
    Markdown,
    Code(&'static str),
}

fn normalized_language(name: &str) -> Option<&'static str> {
    match name.trim().to_ascii_lowercase().as_str() {
        "rs" | "rust" => Some("rust"),
        "py" | "python" => Some("python"),
        "js" | "javascript" | "node" => Some("javascript"),
        "ts" | "typescript" => Some("typescript"),
        "jsx" | "tsx" => Some("tsx"),
        "c" => Some("c"),
        "c++" | "cc" | "cpp" => Some("cpp"),
        "cs" | "c#" | "csharp" => Some("csharp"),
        "java" => Some("java"),
        "go" | "golang" => Some("go"),
        "json" | "jsonc" => Some("json"),
        "sql" => Some("sql"),
        "sh" | "shell" | "bash" => Some("bash"),
        "html" => Some("html"),
        "css" => Some("css"),
        _ => None,
    }
}

fn code_fence(code: &str, language: Option<&str>) -> String {
    let longest = code
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}{}\n{code}\n{fence}", language.unwrap_or(""))
}

pub(super) fn within_budget(text: &str) -> bool {
    TextBudget::default().allows(text)
}

pub(super) fn classify(text: &str) -> TextKind {
    if !within_budget(text) {
        return TextKind::Plain;
    }
    let trimmed = text.trim();
    let lines = trimmed.lines().map(str::trim_start).collect::<Vec<_>>();
    let starts = |prefix: &str| lines.iter().any(|line| line.starts_with(prefix));
    // Fences and multi-line document structures outrank tokens inside their code examples.
    if starts("```")
        || starts("~~~")
        || lines.iter().any(|line| {
            let hashes = line.bytes().take_while(|byte| *byte == b'#').count();
            (1..=6).contains(&hashes) && line.as_bytes().get(hashes) == Some(&b' ')
        })
        || lines
            .iter()
            .filter(|line| {
                line.starts_with("- ") || line.starts_with("* ") || line.starts_with("> ")
            })
            .count()
            >= 2
        || lines
            .iter()
            .any(|line| line.starts_with('|') && line.contains("---"))
    {
        return TextKind::Markdown;
    }
    if let Some(first) = lines.first().filter(|line| line.starts_with("#!")) {
        if first.contains("python") {
            return TextKind::Code("python");
        }
        if first.ends_with("bash") || first.ends_with("/sh") || first.ends_with(" sh") {
            return TextKind::Code("bash");
        }
        if first.contains("node") {
            return TextKind::Code("javascript");
        }
    }
    if (trimmed.starts_with('{') || trimmed.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(trimmed).is_ok()
    {
        return TextKind::Code("json");
    }
    let mut candidates = Vec::new();
    let mut add = |condition, language| {
        if condition {
            candidates.push(language);
        }
    };
    add(
        (starts("fn ") || starts("pub fn ") || starts("impl ")) && trimmed.contains('{')
            || starts("use std::")
            || starts("use crate::"),
        "rust",
    );
    add(
        (starts("def ") || starts("async def "))
            && lines.iter().any(|line| {
                line.starts_with("def ") && line.trim_end().ends_with(':')
                    || line.starts_with("async def ") && line.trim_end().ends_with(':')
            })
            || starts("from ") && trimmed.contains(" import "),
        "python",
    );
    let typed_js = starts("interface ")
        || starts("export interface ")
        || starts("type ") && trimmed.contains(" = ")
        || [": string", ": number", ": boolean", ": Promise<"]
            .iter()
            .any(|token| trimmed.contains(token));
    add(
        typed_js && (trimmed.contains('{') || trimmed.contains(';')),
        "typescript",
    );
    add(
        !typed_js
            && (starts("const ")
                || starts("let ")
                || starts("function ")
                || starts("export function "))
            && (trimmed.contains("=>") || trimmed.contains("function ") && trimmed.contains('{')),
        "javascript",
    );
    let cpp =
        (!starts("use std::") && !starts("fn ") && !starts("pub fn ") && trimmed.contains("std::"))
            || starts("template<")
            || starts("template <")
            || starts("#include <iostream>");
    add(cpp, "cpp");
    add(
        !cpp && starts("#include ") && (trimmed.contains(';') || trimmed.contains('{')),
        "c",
    );
    add(
        starts("using System")
            || starts("namespace ") && trimmed.contains("public ")
            || trimmed.contains("Console.WriteLine("),
        "csharp",
    );
    add(
        starts("import java.") || trimmed.contains("public static void main(String[]"),
        "java",
    );
    add(
        starts("package ") && starts("func ") && trimmed.contains('{'),
        "go",
    );
    let upper = trimmed.to_ascii_uppercase();
    add(
        upper.starts_with("SELECT ") && upper.contains(" FROM ")
            || upper.starts_with("INSERT INTO ") && upper.contains("VALUES")
            || upper.starts_with("CREATE TABLE ") && trimmed.contains('('),
        "sql",
    );
    add(
        (trimmed.starts_with("<!DOCTYPE html>")
            || trimmed.starts_with("<html")
            || trimmed.starts_with("<div")
            || trimmed.starts_with("<section"))
            && trimmed.contains("</"),
        "html",
    );
    add(
        (trimmed.starts_with('.')
            || trimmed.starts_with('#') && !trimmed.starts_with("#include")
            || trimmed.starts_with("@media"))
            && trimmed.contains('{')
            && trimmed.contains(':')
            && trimmed.contains(';'),
        "css",
    );
    if candidates.len() == 1 {
        return TextKind::Code(candidates[0]);
    }
    if candidates.is_empty()
        && ((trimmed.contains("**") && trimmed.matches("**").count() >= 2)
            || (trimmed.contains("](") && trimmed.contains('[')))
    {
        return TextKind::Markdown;
    }
    TextKind::Plain
}

/// Replace resource-bearing AST nodes before passing text to Kit's renderer. Links stay inert.
pub(super) fn safe_markdown(text: &str) -> Option<String> {
    safe_markdown_with_budget(text, TextBudget::default())
}

pub(super) fn safe_markdown_with_budget(text: &str, budget: TextBudget) -> Option<String> {
    if !budget.allows(text) {
        return None;
    }
    // Reject pathological nesting before invoking a recursive parser. False positives safely use source.
    if text.lines().any(|line| {
        line.bytes()
            .take_while(|byte| matches!(byte, b'>' | b' ' | b'\t'))
            .filter(|byte| *byte == b'>')
            .count()
            > MAX_AST_DEPTH
    }) {
        return None;
    }
    let root = markdown::to_mdast(text, &ParseOptions::gfm()).ok()?;
    let mut stack = vec![(&root, 0_usize)];
    let mut count = 0;
    let mut edits = Vec::new();
    while let Some((node, depth)) = stack.pop() {
        count += 1;
        if depth > MAX_AST_DEPTH || count > MAX_AST_NODES {
            return None;
        }
        let replacement = match node {
            Node::Image(image) => Some(escape_literal(&format!("[图片：{}；未加载]", image.alt))),
            Node::ImageReference(image) => {
                Some(escape_literal(&format!("[图片：{}；未加载]", image.alt)))
            }
            Node::Html(html) => Some(escape_literal(&html.value)),
            Node::Code(code) => {
                let language = match code.lang.as_deref() {
                    Some(name) => normalized_language(name),
                    None => match classify(&code.value) {
                        TextKind::Code(language) => Some(language),
                        _ => None,
                    },
                };
                Some(code_fence(&code.value, language))
            }
            _ => None,
        };
        if let Some(value) = replacement {
            let position = node.position()?;
            edits.push((position.start.offset..position.end.offset, value));
        } else if let Some(children) = node.children() {
            stack.extend(children.iter().map(|child| (child, depth + 1)));
        }
    }
    edits.sort_by_key(|(range, _)| range.start);
    let mut safe = text.to_owned();
    for (range, replacement) in edits.into_iter().rev() {
        safe.replace_range(range, &replacement);
    }
    if !budget.allows(&safe) {
        return None;
    }
    // Verify the projected syntax again; malformed input may expose syntax after replacements.
    let verified = markdown::to_mdast(&safe, &ParseOptions::gfm()).ok()?;
    let mut pending = vec![&verified];
    while let Some(node) = pending.pop() {
        if matches!(
            node,
            Node::Image(_) | Node::ImageReference(_) | Node::Html(_)
        ) {
            return None;
        }
        if let Some(children) = node.children() {
            pending.extend(children);
        }
    }
    Some(safe)
}

fn escape_literal(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '~' => format!("\\{character}"),
            _ => character.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rich_preview_detects_distinctive_code_and_rejects_ambiguous_prose() {
        for (text, language) in [
            ("fn main() { println!(\"Hello\"); }", "rust"),
            ("def greet(name):\n    return f\"Hi {name}\"", "python"),
            (
                "const greet = (name) => { return `Hi ${name}`; };",
                "javascript",
            ),
            (
                "interface User { name: string; }\nconst value: User = { name: 'a' };",
                "typescript",
            ),
            (
                "using System;\npublic class A { public static void Main() {} }",
                "csharp",
            ),
            (
                "#include <stdio.h>\nint main(void) { printf(\"hi\"); }",
                "c",
            ),
            ("#include <iostream>\nint main() { std::cout << 1; }", "cpp"),
            (
                "package main\nimport \"fmt\"\nfunc main() { fmt.Println(1) }",
                "go",
            ),
            (
                "import java.util.List;\npublic class A { public static void main(String[] args) {} }",
                "java",
            ),
            ("SELECT name FROM users WHERE id = 1;", "sql"),
            ("{\"enabled\":true,\"items\":[1,2]}", "json"),
            (
                "#!/bin/bash\nfor value in one two; do echo \"$value\"; done",
                "bash",
            ),
            (
                "<!DOCTYPE html>\n<html><body><p>Hello</p></body></html>",
                "html",
            ),
            (".item { color: red; display: flex; }", "css"),
        ] {
            assert_eq!(classify(text), TextKind::Code(language), "{language}");
        }
        for text in [
            "foo(bar)",
            "x = 1",
            "hello world",
            "This function returns true; do not run it.",
            "# 会议记录\n今天讨论计划。",
            "请使用 def function() 来说明，不是代码",
        ] {
            assert!(
                !matches!(classify(text), TextKind::Code(_)),
                "ordinary or ambiguous text"
            );
        }
    }
    #[test]
    fn rich_preview_markdown_fences_lists_and_tables_are_not_guessed_as_one_language() {
        for text in [
            "# 标题\n\n正文 **强调**",
            "```python\ndef main():\n    return 1\n```",
            "| A | B |\n| --- | --- |\n| 1 | 2 |",
            "- 第一项\n- 第二项",
        ] {
            assert_eq!(classify(text), TextKind::Markdown);
        }
    }
    #[test]
    fn rich_preview_markdown_projection_removes_every_resource_and_html_node() {
        let source = "# Title\n\n![remote](https://example.invalid/secret)\n![local](file:///C:/private.txt)\n![inline](data:image/png;base64,AAAA)\n![ref][pic]\n\n[pic]: https://example.invalid/ref\n\n<img src='https://example.invalid/html' onerror='run()'>\n\n```html\n<img src='preserved-code'>\n```\n";
        let projected = safe_markdown(source).expect("safe bounded projection");
        assert_ne!(
            projected, source,
            "untrusted Markdown must not reach a resource loader unchanged"
        );
        assert!(
            projected.contains("preserved-code"),
            "fenced source remains literal"
        );
        assert_eq!(
            source.matches("secret").count(),
            1,
            "projection never mutates source"
        );
    }

    #[test]
    fn preview_line_endings_obey_shared_budget_without_raising_defaults() {
        let budget = TextBudget {
            max_bytes: 1024,
            max_lines: 2,
        };
        for text in [
            "",
            "one",
            "one\r",
            "one\n",
            "one\r\n",
            "one\rtwo",
            "one\ntwo\n",
            "one\r\ntwo\r\n",
            "\r\n\r",
        ] {
            assert!(budget.allows(text), "two-line boundary {text:?}");
        }
        for text in [
            "one\rtwo\rthree",
            "one\ntwo\nthree",
            "one\r\ntwo\r\nthree",
            "one\rtwo\nthree",
        ] {
            assert!(
                !budget.allows(text),
                "line ending must not bypass the shared budget: {text:?}"
            );
        }
        assert!(within_budget(&"line\r".repeat(MAX_RICH_LINES)));
        assert!(!within_budget(&"line\r".repeat(MAX_RICH_LINES + 1)));
        assert!(safe_markdown(&"line\r".repeat(MAX_RICH_LINES + 1)).is_none());
    }

    #[test]
    fn rich_preview_limits_resources_languages_and_does_not_execute_or_rewrite_source() {
        assert_eq!(
            classify("use std::io;\nfn main() { println!(\"hi\"); }"),
            TextKind::Code("rust")
        );
        assert_eq!(
            classify("use std::io;\nfunction mixed() {}"),
            TextKind::Plain,
            "conflicting languages stay literal"
        );
        assert_eq!(
            classify(&"fn main() {}".repeat(MAX_RICH_BYTES)),
            TextKind::Plain
        );
        assert!(safe_markdown(&"# h\n".repeat(MAX_RICH_LINES + 1)).is_none());
        assert!(safe_markdown(&format!("{} literal", "> ".repeat(100))).is_none());
        assert!(safe_markdown("# Title\0![x](https://example.invalid)").is_none());
        let safe=safe_markdown("# Header\n\n```private-clipboard-secret\nx = 1\n```\n\n```rs\nfn main() {}\n```\n\n```\ndef greet():\n    return 1\n```").expect("safe code fences");
        assert!(
            !safe.contains("private-clipboard-secret"),
            "unknown language cannot enter native parser diagnostics"
        );
        assert!(safe.contains("```rust") && safe.contains("```python"));
        let ast = markdown::to_mdast(&safe, &ParseOptions::gfm()).expect("verified markdown");
        let mut pending = vec![&ast];
        while let Some(node) = pending.pop() {
            assert!(!matches!(
                node,
                Node::Image(_) | Node::ImageReference(_) | Node::Html(_)
            ));
            if let Some(children) = node.children() {
                pending.extend(children);
            }
        }
    }
}

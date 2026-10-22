use std::collections::HashMap;
use std::path::Path;

use nxtools_core::CustomCommandSettings;
use nxtools_extensions::command_template::{split_arguments, validate_template};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedCustomCommand {
    pub executable: String,
    pub arguments: Vec<String>,
    pub working_directory: Option<String>,
    pub run_as_admin: bool,
    pub run_silently: bool,
}

pub fn query_matches_custom_command(query: &str, commands: &[CustomCommandSettings]) -> bool {
    let query = query.trim_start();
    if query.is_empty() {
        return false;
    }
    commands.iter().any(|command| {
        command.enabled
            && command_is_valid(command)
            && (starts_with_ignore_case(&command.keyword, query)
                || command_query_parameters(query, &command.keyword).is_some())
    })
}

pub fn matching_custom_commands<'a>(
    query: &str,
    commands: &'a [CustomCommandSettings],
) -> Vec<(&'a CustomCommandSettings, Option<String>)> {
    let query = query.trim_start();
    if query.is_empty() {
        return Vec::new();
    }
    commands
        .iter()
        .filter(|command| command.enabled && command_is_valid(command))
        .filter_map(|command| {
            if starts_with_ignore_case(&command.keyword, query) {
                Some((command, None))
            } else {
                command_query_parameters(query, &command.keyword)
                    .map(|parameters| (command, Some(parameters.to_owned())))
            }
        })
        .collect()
}

pub fn prepare_custom_command(
    command: &CustomCommandSettings,
    input: Option<&str>,
) -> Result<PreparedCustomCommand, String> {
    if !command_is_valid(command) {
        return Err("命令缺少关键词或程序路径".into());
    }
    let variables = std::env::vars()
        .map(|(name, value)| (name.to_lowercase(), value))
        .collect::<HashMap<_, _>>();
    prepare_custom_command_with(command, input, |name| {
        variables.get(&name.to_lowercase()).cloned()
    })
}

pub fn command_display_line(command: &PreparedCustomCommand) -> String {
    let mut line = quote_for_display(&command.executable);
    for argument in &command.arguments {
        line.push(' ');
        line.push_str(&quote_for_display(argument));
    }
    line
}

pub fn validate_custom_command_template(template: &str) -> Result<(), String> {
    validate_template(template)
}

fn prepare_custom_command_with(
    command: &CustomCommandSettings,
    input: Option<&str>,
    variable: impl Fn(&str) -> Option<String>,
) -> Result<PreparedCustomCommand, String> {
    let executable = expand_windows_environment_variables(&command.program, &variable);
    let input = input.unwrap_or_default();
    let input_arguments = split_arguments(input)?;
    let template_arguments = split_arguments(&command.argument_template)?;
    validate_template(&command.argument_template)?;
    let arguments = template_arguments
        .iter()
        .map(|argument| replace_placeholders(argument, input, &input_arguments))
        .collect::<Result<Vec<_>, _>>()?;
    let configured_working_directory =
        expand_windows_environment_variables(&command.working_directory, &variable);
    let working_directory = if configured_working_directory.trim().is_empty() {
        Path::new(&executable)
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .map(|parent| parent.to_string_lossy().into_owned())
    } else {
        Some(configured_working_directory)
    };
    Ok(PreparedCustomCommand {
        executable,
        arguments,
        working_directory,
        run_as_admin: command.run_as_admin,
        run_silently: command.run_silently,
    })
}

fn command_is_valid(command: &CustomCommandSettings) -> bool {
    !command.keyword.trim().is_empty()
        && !command.keyword.chars().any(char::is_whitespace)
        && !command.program.trim().is_empty()
        && validate_template(&command.argument_template).is_ok()
}

fn command_query_parameters<'a>(query: &'a str, keyword: &str) -> Option<&'a str> {
    let prefix = query.get(..keyword.len())?;
    if !prefix.eq_ignore_ascii_case(keyword) || query.as_bytes().get(keyword.len()) != Some(&b' ') {
        return None;
    }
    Some(&query[keyword.len() + 1..])
}

fn starts_with_ignore_case(value: &str, prefix: &str) -> bool {
    value
        .get(..prefix.len())
        .is_some_and(|candidate| candidate.eq_ignore_ascii_case(prefix))
}

fn replace_placeholders(
    template: &str,
    input: &str,
    input_arguments: &[String],
) -> Result<String, String> {
    let mut output = String::new();
    let bytes = template.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && bytes.get(index + 1) == Some(&b's') {
            let mut end = index + 2;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            if end == index + 2 {
                output.push_str(input);
            } else {
                let position = template[index + 2..end]
                    .parse::<usize>()
                    .map_err(|_| "参数占位符编号无效".to_owned())?;
                if position == 0 {
                    return Err("参数占位符编号必须从 1 开始".into());
                }
                output.push_str(
                    input_arguments
                        .get(position - 1)
                        .map(String::as_str)
                        .unwrap_or_default(),
                );
            }
            index = end;
            continue;
        }
        if bytes[index] == b'{' {
            let end = template[index + 1..]
                .find('}')
                .map(|relative| index + 1 + relative)
                .ok_or_else(|| "参数模板中存在未闭合的占位符".to_owned())?;
            let number = &template[index + 1..end];
            if number.is_empty() {
                output.push_str(input);
            } else {
                let position = number
                    .parse::<usize>()
                    .map_err(|_| format!("无法识别参数占位符 {{{number}}}"))?;
                if position == 0 {
                    return Err("参数占位符编号必须从 1 开始".into());
                }
                output.push_str(
                    input_arguments
                        .get(position - 1)
                        .map(String::as_str)
                        .unwrap_or_default(),
                );
            }
            index = end + 1;
            continue;
        }
        let Some(character) = template[index..].chars().next() else {
            break;
        };
        output.push(character);
        index += character.len_utf8();
    }
    Ok(output)
}

fn expand_windows_environment_variables(
    value: &str,
    variable: &impl Fn(&str) -> Option<String>,
) -> String {
    let mut output = String::with_capacity(value.len());
    let mut remaining = value;
    while let Some(start) = remaining.find('%') {
        output.push_str(&remaining[..start]);
        let after_start = &remaining[start + 1..];
        let Some(end) = after_start.find('%') else {
            output.push_str(&remaining[start..]);
            return output;
        };
        let name = &after_start[..end];
        if name.is_empty() {
            output.push_str("%%");
        } else if let Some(expanded) = variable(name) {
            output.push_str(&expanded);
        } else {
            output.push('%');
            output.push_str(name);
            output.push('%');
        }
        remaining = &after_start[end + 1..];
    }
    output.push_str(remaining);
    output
}

fn quote_for_display(value: &str) -> String {
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn command(template: &str) -> CustomCommandSettings {
        CustomCommandSettings {
            keyword: "run".into(),
            title: "Run tool".into(),
            program: r"%TOOLS%\tool.exe".into(),
            argument_template: template.into(),
            working_directory: r"%WORK%".into(),
            ..CustomCommandSettings::default()
        }
    }

    #[test]
    fn matching_uses_keyword_prefix_or_keyword_plus_space_only() {
        let commands = [command("%s")];
        assert!(query_matches_custom_command("r", &commands));
        assert!(query_matches_custom_command("RUN one", &commands));
        assert!(!query_matches_custom_command("runner", &commands));
        assert!(!query_matches_custom_command("", &commands));
    }

    #[test]
    fn template_expands_all_and_quoted_positional_arguments() {
        let prepared = prepare_custom_command_with(
            &command(r#"--all %s --first=%s1 --second={2}"#),
            Some(r#""hello world" tail"#),
            |name| match name {
                "TOOLS" => Some(r"C:\Tools".into()),
                "WORK" => Some(r"D:\Work".into()),
                _ => None,
            },
        )
        .expect("prepare command");
        assert_eq!(prepared.executable, r"C:\Tools\tool.exe");
        assert_eq!(
            prepared.arguments,
            [
                "--all",
                "\"hello world\" tail",
                "--first=hello world",
                "--second=tail",
            ]
        );
        assert_eq!(prepared.working_directory.as_deref(), Some(r"D:\Work"));
    }

    #[test]
    fn prepared_windows_paths_preserve_backslash_runs_and_quoted_trailing_separators() {
        let prepared = prepare_custom_command_with(
            &command(r#"--path "C:\Program Files\Tool\\" --quoted "a\"b" --plain C:\Temp\ --first={1} --second=%s2"#),
            Some(r#""D:\My Files\\" "C:\Data\input.txt""#),
            |_| None,
        )
        .expect("prepare quoted Windows paths");
        assert_eq!(
            prepared.arguments,
            [
                "--path",
                r"C:\Program Files\Tool\",
                "--quoted",
                "a\"b",
                "--plain",
                r"C:\Temp\",
                r"--first=D:\My Files\",
                r"--second=C:\Data\input.txt",
            ]
        );
    }

    #[test]
    fn windows_quote_parity_distinguishes_literal_quote_from_closing_quote() {
        assert_eq!(
            split_arguments(r#""C:\path\\" end"#).expect("decode quoted trailing backslash"),
            [r"C:\path\", "end"]
        );
        assert_eq!(
            split_arguments(r#""C:\path\"x""#).expect("decode escaped quote in path"),
            ["C:\\path\"x"]
        );
        assert!(split_arguments(r#""C:\unfinished"#).is_err());
    }

    #[test]
    fn template_without_placeholder_does_not_append_input() {
        let prepared = prepare_custom_command_with(
            &command("status --label 中文 --short"),
            Some("ignored"),
            |_| None,
        )
        .expect("prepare command");
        assert_eq!(prepared.arguments, ["status", "--label", "中文", "--short"]);
    }

    #[test]
    fn missing_position_becomes_empty_and_invalid_templates_fail() {
        let prepared = prepare_custom_command_with(&command("--value=%s2"), Some("one"), |_| None)
            .expect("prepare command");
        assert_eq!(prepared.arguments, ["--value="]);
        assert_eq!(
            prepare_custom_command_with(&command("%s0"), None, |_| None),
            Err("参数占位符编号必须从 1 开始".into())
        );
        assert_eq!(
            prepare_custom_command_with(&command("\"unfinished"), None, |_| None),
            Err("参数中存在未闭合的引号".into())
        );
    }
}

//! Shared Windows argument grammar originally owned by Finder custom commands.
//! Package preview and manual command preparation deliberately use one validator.

pub fn split_arguments(value: &str) -> Result<Vec<String>, String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut started = false;
    let mut characters = value.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\\' {
            let mut count = 1;
            while characters.peek() == Some(&'\\') {
                characters.next();
                count += 1;
            }
            if quote != Some('\'') && characters.peek() == Some(&'"') {
                current.extend(std::iter::repeat_n('\\', count / 2));
                if count % 2 == 1 {
                    characters.next();
                    current.push('"');
                } else {
                    characters.next();
                    quote = if quote == Some('"') { None } else { Some('"') };
                }
            } else {
                current.extend(std::iter::repeat_n('\\', count));
            }
            started = true;
            continue;
        }
        if matches!(character, '\'' | '"') {
            if quote == Some(character) {
                quote = None;
            } else if quote.is_none() {
                quote = Some(character);
            } else {
                current.push(character);
            }
            started = true;
            continue;
        }
        if character.is_whitespace() && quote.is_none() {
            if started {
                arguments.push(std::mem::take(&mut current));
                started = false;
            }
            continue;
        }
        current.push(character);
        started = true;
    }
    if quote.is_some() {
        return Err("参数中存在未闭合的引号".into());
    }
    if started {
        arguments.push(current);
    }
    Ok(arguments)
}

pub fn validate_template(template: &str) -> Result<(), String> {
    let bytes = template.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && bytes.get(index + 1) == Some(&b's') {
            let mut end = index + 2;
            while bytes.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            if end > index + 2 {
                let position = template[index + 2..end]
                    .parse::<usize>()
                    .map_err(|_| "参数占位符编号无效".to_owned())?;
                if position == 0 {
                    return Err("参数占位符编号必须从 1 开始".into());
                }
            }
            index = end;
            continue;
        }
        if bytes[index] == b'{' {
            let Some(relative_end) = template[index + 1..].find('}') else {
                return Err("参数模板中存在未闭合的占位符".into());
            };
            let end = index + 1 + relative_end;
            let number = &template[index + 1..end];
            if !number.is_empty() && !number.bytes().all(|byte| byte.is_ascii_digit()) {
                return Err(format!("无法识别参数占位符 {{{number}}}"));
            }
            if !number.is_empty() {
                let position = number
                    .parse::<usize>()
                    .map_err(|_| "参数占位符编号无效".to_owned())?;
                if position == 0 {
                    return Err("参数占位符编号必须从 1 开始".into());
                }
            }
            index = end + 1;
            continue;
        }
        index += 1;
    }
    split_arguments(template).map(|_| ())
}

const MAX_EXPRESSION_CHARS: usize = 256;
const MAX_NESTING_DEPTH: usize = 32;
const MAX_FUNCTION_ARGUMENTS: usize = 4;
const MAX_CHINESE_INTEGER: u64 = 9_999_999_999_999_999;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculatorOutput {
    pub representation: CalculatorRepresentation,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalculatorRepresentation {
    Decimal,
    Hexadecimal,
    Binary,
    Octal,
    ChineseNumber,
    ChineseCurrency,
}

impl CalculatorRepresentation {
    pub const fn id(self) -> &'static str {
        match self {
            Self::Decimal => "decimal",
            Self::Hexadecimal => "hexadecimal",
            Self::Binary => "binary",
            Self::Octal => "octal",
            Self::ChineseNumber => "chinese-number",
            Self::ChineseCurrency => "chinese-currency",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Decimal => "计算结果",
            Self::Hexadecimal => "十六进制",
            Self::Binary => "二进制",
            Self::Octal => "八进制",
            Self::ChineseNumber => "中文数字",
            Self::ChineseCurrency => "中文金额",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalculatorAnswer {
    pub expression: String,
    pub value: String,
    pub outputs: Vec<CalculatorOutput>,
}

impl CalculatorAnswer {
    pub fn title(&self) -> String {
        format!("{} = {}", self.expression, self.value)
    }
}

pub fn calculator_answer(query: &str) -> Option<CalculatorAnswer> {
    let expression = query.trim().strip_prefix('=')?.trim();
    if expression.is_empty()
        || expression.chars().count() > MAX_EXPRESSION_CHARS
        || (!expression
            .chars()
            .any(|character| character.is_ascii_digit())
            && !contains_supported_constant(expression))
        || is_standalone_number(expression)
    {
        return None;
    }

    if let Some(value) = convert_number_base(expression) {
        return Some(build_answer(expression, value));
    }

    let mut parser = Parser::new(expression);
    let value = parser.parse().ok()?;
    if !value.is_finite() {
        return None;
    }
    Some(build_answer(expression, format_value(value)))
}

fn build_answer(expression: &str, value: String) -> CalculatorAnswer {
    CalculatorAnswer {
        expression: expression.to_owned(),
        outputs: calculator_outputs(&value),
        value,
    }
}

fn calculator_outputs(value: &str) -> Vec<CalculatorOutput> {
    let mut outputs = vec![CalculatorOutput {
        representation: primary_representation(value),
        value: value.to_owned(),
    }];
    let Some(number) = DisplayNumber::parse(value) else {
        return outputs;
    };

    if !matches!(number, DisplayNumber::Decimal(_)) {
        push_output(
            &mut outputs,
            CalculatorRepresentation::Decimal,
            number.decimal_text(),
        );
    }
    if let Some(integer) = number.safe_integer() {
        if integer >= 0 {
            push_output(
                &mut outputs,
                CalculatorRepresentation::Hexadecimal,
                format!("0x{integer:X}"),
            );
            push_output(
                &mut outputs,
                CalculatorRepresentation::Binary,
                format!("{integer:b}"),
            );
        }
        if let Some(chinese) = chinese_signed_integer(integer, false) {
            push_output(
                &mut outputs,
                CalculatorRepresentation::ChineseNumber,
                chinese,
            );
        }
    }
    if let Some(currency) = chinese_currency(number) {
        push_output(
            &mut outputs,
            CalculatorRepresentation::ChineseCurrency,
            currency,
        );
    }
    outputs
}

fn push_output(
    outputs: &mut Vec<CalculatorOutput>,
    representation: CalculatorRepresentation,
    value: String,
) {
    if outputs
        .iter()
        .any(|output| output.representation == representation)
    {
        return;
    }
    outputs.push(CalculatorOutput {
        representation,
        value,
    });
}

fn primary_representation(value: &str) -> CalculatorRepresentation {
    if value.starts_with("0x") {
        CalculatorRepresentation::Hexadecimal
    } else if value.starts_with("0b") {
        CalculatorRepresentation::Binary
    } else if value.starts_with("0o") {
        CalculatorRepresentation::Octal
    } else {
        CalculatorRepresentation::Decimal
    }
}

#[derive(Clone, Copy)]
enum DisplayNumber {
    Integer(i64),
    Decimal(f64),
}

impl DisplayNumber {
    fn parse(value: &str) -> Option<Self> {
        if let Some(hex) = value.strip_prefix("0x") {
            return i64::from_str_radix(hex, 16).ok().map(Self::Integer);
        }
        if let Some(binary) = value.strip_prefix("0b") {
            return i64::from_str_radix(binary, 2).ok().map(Self::Integer);
        }
        if let Some(octal) = value.strip_prefix("0o") {
            return i64::from_str_radix(octal, 8).ok().map(Self::Integer);
        }
        if let Ok(integer) = value.parse::<i64>() {
            return Some(Self::Integer(integer));
        }
        value
            .parse::<f64>()
            .ok()
            .filter(|value| value.is_finite())
            .map(Self::Decimal)
    }

    fn decimal_text(self) -> String {
        match self {
            Self::Integer(value) => value.to_string(),
            Self::Decimal(value) => format_value(value),
        }
    }

    fn safe_integer(self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(value),
            Self::Decimal(value)
                if value.fract() == 0.0
                    && (-9_007_199_254_740_992.0..=9_007_199_254_740_992.0).contains(&value) =>
            {
                Some(value as i64)
            }
            Self::Decimal(_) => None,
        }
    }
}

fn chinese_signed_integer(value: i64, financial: bool) -> Option<String> {
    let magnitude = value.unsigned_abs();
    let formatted = chinese_unsigned_integer(magnitude, financial)?;
    Some(if value < 0 {
        format!("负{formatted}")
    } else {
        formatted
    })
}

fn chinese_unsigned_integer(value: u64, financial: bool) -> Option<String> {
    if value > MAX_CHINESE_INTEGER {
        return None;
    }
    if value == 0 {
        return Some("零".into());
    }

    let group_units = ["", "万", "亿", "兆"];
    let mut groups = Vec::new();
    let mut remaining = value;
    while remaining > 0 {
        groups.push((remaining % 10_000) as u16);
        remaining /= 10_000;
    }

    let mut result = String::new();
    let mut zero_between_groups = false;
    for index in (0..groups.len()).rev() {
        let group = groups[index];
        if group == 0 {
            if !result.is_empty() {
                zero_between_groups = true;
            }
            continue;
        }
        if !result.is_empty() && (zero_between_groups || group < 1_000) {
            result.push('零');
        }
        result.push_str(&chinese_four_digit_group(group, financial));
        result.push_str(group_units[index]);
        zero_between_groups = false;
    }

    if !financial && let Some(without_leading_one) = result.strip_prefix("一十") {
        return Some(format!("十{without_leading_one}"));
    }
    Some(result)
}

fn chinese_four_digit_group(value: u16, financial: bool) -> String {
    let common_digits = ["零", "一", "二", "三", "四", "五", "六", "七", "八", "九"];
    let financial_digits = ["零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖"];
    let common_units = ["千", "百", "十", ""];
    let financial_units = ["仟", "佰", "拾", ""];
    let digits = if financial {
        &financial_digits
    } else {
        &common_digits
    };
    let units = if financial {
        &financial_units
    } else {
        &common_units
    };

    let divisors = [1_000_u16, 100, 10, 1];
    let mut result = String::new();
    let mut zero_pending = false;
    for (index, divisor) in divisors.into_iter().enumerate() {
        let digit = (value / divisor) % 10;
        if digit == 0 {
            if !result.is_empty() {
                zero_pending = true;
            }
            continue;
        }
        if zero_pending {
            result.push('零');
            zero_pending = false;
        }
        result.push_str(digits[digit as usize]);
        result.push_str(units[index]);
    }
    result
}

fn chinese_currency(value: DisplayNumber) -> Option<String> {
    let (negative, yuan, jiao, fen) = match value {
        DisplayNumber::Integer(value) => {
            let yuan = value.unsigned_abs();
            if yuan > MAX_CHINESE_INTEGER {
                return None;
            }
            (value < 0, yuan, 0, 0)
        }
        DisplayNumber::Decimal(value) => {
            if !value.is_finite() || value.abs() > MAX_CHINESE_INTEGER as f64 {
                return None;
            }
            let cents = (value.abs() * 100.0).round();
            if !cents.is_finite() || cents > u64::MAX as f64 {
                return None;
            }
            let cents = cents as u64;
            (value < 0.0, cents / 100, (cents / 10) % 10, cents % 10)
        }
    };
    let digits = ["零", "壹", "贰", "叁", "肆", "伍", "陆", "柒", "捌", "玖"];

    let mut result = String::new();
    if negative && (yuan != 0 || jiao != 0 || fen != 0) {
        result.push('负');
    }
    if yuan > 0 {
        result.push_str(&chinese_unsigned_integer(yuan, true)?);
        result.push('元');
    }
    if jiao > 0 {
        result.push_str(digits[jiao as usize]);
        result.push('角');
    }
    if fen > 0 {
        if yuan > 0 && jiao == 0 {
            result.push('零');
        }
        result.push_str(digits[fen as usize]);
        result.push('分');
    }
    if result.is_empty() {
        result.push_str("零元");
    }
    Some(result)
}

fn is_standalone_number(expression: &str) -> bool {
    if expression.parse::<f64>().is_ok() {
        return true;
    }
    let unsigned = expression.strip_prefix(['+', '-']).unwrap_or(expression);
    unsigned
        .strip_prefix("0x")
        .or_else(|| unsigned.strip_prefix("0X"))
        .is_some_and(|digits| {
            !digits.is_empty()
                && digits
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
        })
        || unsigned
            .strip_prefix("0b")
            .or_else(|| unsigned.strip_prefix("0B"))
            .is_some_and(|digits| {
                !digits.is_empty()
                    && digits
                        .chars()
                        .all(|character| matches!(character, '0' | '1'))
            })
        || unsigned
            .strip_prefix("0o")
            .or_else(|| unsigned.strip_prefix("0O"))
            .is_some_and(|digits| {
                !digits.is_empty()
                    && digits
                        .chars()
                        .all(|character| matches!(character, '0'..='7'))
            })
}

enum BaseConversionValue {
    Integer(i64),
    Decimal(f64),
}

fn convert_number_base(expression: &str) -> Option<String> {
    let mut parts = expression.split_whitespace();
    let source = parts.next()?;
    if !parts.next()?.eq_ignore_ascii_case("to") {
        return None;
    }
    let target = parts.next()?.to_ascii_lowercase();
    if parts.next().is_some() {
        return None;
    }

    let value = if let Some(hex) = source
        .strip_prefix("0x")
        .or_else(|| source.strip_prefix("0X"))
    {
        BaseConversionValue::Integer(i64::from_str_radix(hex, 16).ok()?)
    } else if let Some(binary) = source
        .strip_prefix("0b")
        .or_else(|| source.strip_prefix("0B"))
    {
        BaseConversionValue::Integer(i64::from_str_radix(binary, 2).ok()?)
    } else if let Some(octal) = source
        .strip_prefix("0o")
        .or_else(|| source.strip_prefix("0O"))
    {
        BaseConversionValue::Integer(i64::from_str_radix(octal, 8).ok()?)
    } else if source.chars().all(|character| character.is_ascii_digit()) {
        BaseConversionValue::Integer(source.parse().ok()?)
    } else {
        BaseConversionValue::Decimal(source.parse::<f64>().ok()?)
    };
    if target == "dec" {
        return match value {
            BaseConversionValue::Integer(value) => Some(value.to_string()),
            BaseConversionValue::Decimal(value) if value.is_finite() => Some(format_value(value)),
            BaseConversionValue::Decimal(_) => None,
        };
    }
    let value = match value {
        BaseConversionValue::Integer(value) if value >= 0 => value,
        BaseConversionValue::Decimal(value)
            if value.is_finite()
                && (0.0..=9_007_199_254_740_992.0).contains(&value)
                && value.fract() == 0.0 =>
        {
            value as i64
        }
        BaseConversionValue::Integer(_) | BaseConversionValue::Decimal(_) => return None,
    };
    match target.as_str() {
        "hex" => Some(format!("0x{value:X}")),
        "bin" => Some(format!("0b{value:b}")),
        "oct" => Some(format!("0o{value:o}")),
        _ => None,
    }
}

fn contains_supported_constant(expression: &str) -> bool {
    let expression = expression.to_ascii_lowercase();
    expression == "e"
        || expression == "pi"
        || expression.contains('π')
        || expression
            .split(|character: char| !character.is_ascii_alphanumeric())
            .any(|token| matches!(token, "e" | "pi"))
}

fn format_value(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    let magnitude = value.abs();
    if !(1e-9..1e15).contains(&magnitude) {
        let formatted = format!("{value:.10e}");
        let (mantissa, exponent) = formatted
            .split_once('e')
            .expect("scientific formatting always has an exponent");
        let mantissa = mantissa.trim_end_matches('0').trim_end_matches('.');
        let exponent = exponent.parse::<i32>().unwrap_or_default();
        return format!("{mantissa}e{exponent}");
    }
    if value.fract() == 0.0 {
        return format!("{value:.0}");
    }
    let value = (value * 1e10).round() / 1e10;
    if value.fract() == 0.0 {
        format!("{value:.0}")
    } else {
        value.to_string()
    }
}

struct Parser {
    characters: Vec<char>,
    position: usize,
    depth: usize,
}

impl Parser {
    fn new(expression: &str) -> Self {
        Self {
            characters: expression.chars().collect(),
            position: 0,
            depth: 0,
        }
    }

    fn parse(&mut self) -> Result<f64, ()> {
        let value = self.parse_expression()?;
        self.skip_whitespace();
        (self.position == self.characters.len())
            .then_some(value)
            .ok_or(())
    }

    fn parse_expression(&mut self) -> Result<f64, ()> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('+') => {
                    self.position += 1;
                    value += self.parse_term()?;
                }
                Some('-') => {
                    self.position += 1;
                    value -= self.parse_term()?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, ()> {
        let mut value = self.parse_power()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some('*') => {
                    self.position += 1;
                    value *= self.parse_power()?;
                }
                Some('/') => {
                    self.position += 1;
                    let divisor = self.parse_power()?;
                    if divisor == 0.0 {
                        return Err(());
                    }
                    value /= divisor;
                }
                Some('%') => {
                    self.position += 1;
                    let divisor = self.parse_power()?;
                    if divisor == 0.0 {
                        return Err(());
                    }
                    value %= divisor;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_power(&mut self) -> Result<f64, ()> {
        let value = self.parse_unary()?;
        self.skip_whitespace();
        if self.peek() != Some('^') {
            return Ok(value);
        }
        self.position += 1;
        Ok(value.powf(self.parse_power()?))
    }

    fn parse_unary(&mut self) -> Result<f64, ()> {
        self.skip_whitespace();
        match self.peek() {
            Some('+') => {
                self.position += 1;
                self.parse_unary()
            }
            Some('-') => {
                self.position += 1;
                Ok(-self.parse_unary()?)
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, ()> {
        self.skip_whitespace();
        if self.peek() == Some('(') {
            if self.depth >= MAX_NESTING_DEPTH {
                return Err(());
            }
            self.position += 1;
            self.depth += 1;
            let value = self.parse_expression();
            self.depth -= 1;
            let value = value?;
            self.skip_whitespace();
            if self.peek() != Some(')') {
                return Err(());
            }
            self.position += 1;
            return Ok(value);
        }
        match self.peek() {
            Some(character) if character.is_ascii_digit() || character == '.' => {
                self.parse_number()
            }
            Some(character) if character.is_ascii_alphabetic() || character == 'π' => {
                self.parse_identifier()
            }
            _ => Err(()),
        }
    }

    fn parse_number(&mut self) -> Result<f64, ()> {
        self.skip_whitespace();
        let start = self.position;
        if self.peek() == Some('0') {
            match self.characters.get(self.position + 1).copied() {
                Some('x' | 'X') => {
                    self.position += 2;
                    let digits = self.position;
                    while self
                        .peek()
                        .is_some_and(|character| character.is_ascii_hexdigit())
                    {
                        self.position += 1;
                    }
                    if digits == self.position {
                        return Err(());
                    }
                    let value = self.characters[digits..self.position]
                        .iter()
                        .collect::<String>();
                    return i64::from_str_radix(&value, 16)
                        .map(|value| value as f64)
                        .map_err(|_| ());
                }
                Some('b' | 'B') => {
                    self.position += 2;
                    let digits = self.position;
                    while matches!(self.peek(), Some('0' | '1')) {
                        self.position += 1;
                    }
                    if digits == self.position {
                        return Err(());
                    }
                    let value = self.characters[digits..self.position]
                        .iter()
                        .collect::<String>();
                    return i64::from_str_radix(&value, 2)
                        .map(|value| value as f64)
                        .map_err(|_| ());
                }
                Some('o' | 'O') => {
                    self.position += 2;
                    let digits = self.position;
                    while matches!(self.peek(), Some('0'..='7')) {
                        self.position += 1;
                    }
                    if digits == self.position {
                        return Err(());
                    }
                    let value = self.characters[digits..self.position]
                        .iter()
                        .collect::<String>();
                    return i64::from_str_radix(&value, 8)
                        .map(|value| value as f64)
                        .map_err(|_| ());
                }
                _ => {}
            }
        }

        let mut decimal_seen = false;
        let mut digit_seen = false;
        while let Some(character) = self.peek() {
            if character.is_ascii_digit() {
                digit_seen = true;
                self.position += 1;
            } else if character == '.' && !decimal_seen {
                decimal_seen = true;
                self.position += 1;
            } else {
                break;
            }
        }
        if !digit_seen {
            return Err(());
        }
        if matches!(self.peek(), Some('e' | 'E')) {
            self.position += 1;
            if matches!(self.peek(), Some('+' | '-')) {
                self.position += 1;
            }
            let exponent_start = self.position;
            while self
                .peek()
                .is_some_and(|character| character.is_ascii_digit())
            {
                self.position += 1;
            }
            if exponent_start == self.position {
                return Err(());
            }
        }
        self.characters[start..self.position]
            .iter()
            .collect::<String>()
            .parse()
            .map_err(|_| ())
    }

    fn parse_identifier(&mut self) -> Result<f64, ()> {
        self.skip_whitespace();
        if self.peek() == Some('π') {
            self.position += 1;
            return Ok(std::f64::consts::PI);
        }
        let start = self.position;
        while self
            .peek()
            .is_some_and(|character| character.is_ascii_alphanumeric())
        {
            self.position += 1;
        }
        let identifier = self.characters[start..self.position]
            .iter()
            .collect::<String>()
            .to_ascii_lowercase();
        match identifier.as_str() {
            "pi" => return Ok(std::f64::consts::PI),
            "e" => return Ok(std::f64::consts::E),
            _ => {}
        }

        self.skip_whitespace();
        if self.peek() != Some('(') || self.depth >= MAX_NESTING_DEPTH {
            return Err(());
        }
        self.position += 1;
        self.depth += 1;
        let arguments = self.parse_arguments();
        self.depth -= 1;
        let arguments = arguments?;
        evaluate_function(&identifier, &arguments)
    }

    fn parse_arguments(&mut self) -> Result<Vec<f64>, ()> {
        let mut arguments = Vec::new();
        loop {
            self.skip_whitespace();
            if self.peek() == Some(')') {
                self.position += 1;
                break;
            }
            if arguments.len() >= MAX_FUNCTION_ARGUMENTS {
                return Err(());
            }
            arguments.push(self.parse_expression()?);
            self.skip_whitespace();
            match self.peek() {
                Some(',') => self.position += 1,
                Some(')') => {
                    self.position += 1;
                    break;
                }
                _ => return Err(()),
            }
        }
        Ok(arguments)
    }

    fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }

    fn skip_whitespace(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.position += 1;
        }
    }
}

fn evaluate_function(name: &str, arguments: &[f64]) -> Result<f64, ()> {
    let unary = || arguments.first().copied().filter(|_| arguments.len() == 1);
    let binary = || (arguments.len() == 2).then(|| (arguments[0], arguments[1]));
    match name {
        "sin" => unary().map(f64::sin),
        "cos" => unary().map(f64::cos),
        "tan" => unary().map(f64::tan),
        "asin" => unary().map(f64::asin),
        "acos" => unary().map(f64::acos),
        "atan" => unary().map(f64::atan),
        "sqrt" => unary().filter(|value| *value >= 0.0).map(f64::sqrt),
        "cbrt" => unary().map(f64::cbrt),
        "abs" => unary().map(f64::abs),
        "ln" => unary().filter(|value| *value > 0.0).map(f64::ln),
        "log2" => unary().filter(|value| *value > 0.0).map(f64::log2),
        "log10" | "log" if arguments.len() == 1 => {
            unary().filter(|value| *value > 0.0).map(f64::log10)
        }
        "log" => binary()
            .filter(|(value, base)| *value > 0.0 && *base > 0.0 && *base != 1.0)
            .map(|(value, base)| value.log(base)),
        "exp" => unary().map(f64::exp),
        "floor" => unary().map(f64::floor),
        "ceil" => unary().map(f64::ceil),
        "round" if arguments.len() == 1 => unary().map(f64::round),
        "round" => binary()
            .filter(|(_, digits)| digits.fract() == 0.0 && (0.0..=10.0).contains(digits))
            .map(|(value, digits)| {
                let factor = 10_f64.powi(digits as i32);
                (value * factor).round() / factor
            }),
        "min" => binary().map(|(left, right)| left.min(right)),
        "max" => binary().map(|(left, right)| left.max(right)),
        _ => None,
    }
    .ok_or(())
}

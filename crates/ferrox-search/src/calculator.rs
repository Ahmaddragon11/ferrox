// SPDX-License-Identifier: MIT OR Apache-2.0

/// Fast recursive-descent arithmetic evaluator used by the omnibox ("= 2*(3+4)^2").
#[derive(Debug, Clone, PartialEq)]
pub enum CalcError {
    Empty,
    UnexpectedChar(char),
    UnexpectedEnd,
    DivisionByZero,
    UnknownFunction(String),
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "empty expression"),
            Self::UnexpectedChar(c) => write!(f, "unexpected character '{c}'"),
            Self::UnexpectedEnd => write!(f, "unexpected end of expression"),
            Self::DivisionByZero => write!(f, "division by zero"),
            Self::UnknownFunction(name) => write!(f, "unknown function '{name}'"),
        }
    }
}

impl std::error::Error for CalcError {}

pub fn evaluate(input: &str) -> Result<f64, CalcError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_expr()?;
    parser.skip_ws();
    if parser.peek().is_some() {
        return Err(CalcError::UnexpectedChar(parser.peek().unwrap()));
    }
    if !value.is_finite() {
        return Err(CalcError::DivisionByZero);
    }
    Ok(value)
}

/// Returns a formatted result when the input is a valid math expression.
pub fn try_evaluate(input: &str) -> Option<String> {
    let trimmed = input.trim();
    let expr = trimmed.strip_prefix('=').unwrap_or(trimmed).trim();
    if expr.is_empty() || !expr.chars().any(|c| c.is_ascii_digit()) {
        return None;
    }
    evaluate(expr).ok().map(|value| {
        if (value - value.round()).abs() < 1e-9 && value.abs() < 1e15 {
            format!("{}", value.round() as i64)
        } else {
            let formatted = format!("{value:.10}");
            formatted
                .trim_end_matches('0')
                .trim_end_matches('.')
                .to_string()
        }
    })
}

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    _src: &'a str,
}

impl<'a> Parser<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().filter(|c| !c.is_whitespace()).collect(),
            pos: 0,
            _src: src,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while self.peek().map(|c| c.is_whitespace()).unwrap_or(false) {
            self.pos += 1;
        }
    }

    fn parse_expr(&mut self) -> Result<f64, CalcError> {
        let mut value = self.parse_term()?;
        loop {
            match self.peek() {
                Some('+') => {
                    self.next();
                    value += self.parse_term()?;
                }
                Some('-') => {
                    self.next();
                    value -= self.parse_term()?;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<f64, CalcError> {
        let mut value = self.parse_factor()?;
        loop {
            match self.peek() {
                Some('*') => {
                    self.next();
                    value *= self.parse_factor()?;
                }
                Some('/') => {
                    self.next();
                    let rhs = self.parse_factor()?;
                    if rhs == 0.0 {
                        return Err(CalcError::DivisionByZero);
                    }
                    value /= rhs;
                }
                Some('%') => {
                    self.next();
                    let rhs = self.parse_factor()?;
                    if rhs == 0.0 {
                        return Err(CalcError::DivisionByZero);
                    }
                    value %= rhs;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<f64, CalcError> {
        let base = self.parse_unary()?;
        if self.peek() == Some('^') {
            self.next();
            let exponent = self.parse_factor()?;
            return Ok(base.powf(exponent));
        }
        Ok(base)
    }

    fn parse_unary(&mut self) -> Result<f64, CalcError> {
        match self.peek() {
            Some('-') => {
                self.next();
                Ok(-self.parse_unary()?)
            }
            Some('+') => {
                self.next();
                self.parse_unary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Result<f64, CalcError> {
        match self.next() {
            Some('(') => {
                let value = self.parse_expr()?;
                if self.next() != Some(')') {
                    return Err(CalcError::UnexpectedEnd);
                }
                Ok(value)
            }
            Some(c) if c.is_ascii_digit() || c == '.' => {
                let mut number = String::from(c);
                while let Some(next) = self.peek() {
                    if next.is_ascii_digit() || next == '.' {
                        number.push(next);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                number.parse::<f64>().map_err(|_| CalcError::UnexpectedEnd)
            }
            Some(c) if c.is_alphabetic() => {
                let mut name = String::from(c);
                while let Some(next) = self.peek() {
                    if next.is_alphabetic() {
                        name.push(next);
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                self.parse_named(&name)
            }
            Some(c) => Err(CalcError::UnexpectedChar(c)),
            None => Err(CalcError::UnexpectedEnd),
        }
    }

    fn parse_named(&mut self, name: &str) -> Result<f64, CalcError> {
        match name {
            "pi" => return Ok(std::f64::consts::PI),
            "e" => return Ok(std::f64::consts::E),
            "tau" => return Ok(std::f64::consts::TAU),
            _ => {}
        }

        if self.next() != Some('(') {
            return Err(CalcError::UnknownFunction(name.to_string()));
        }
        let arg = self.parse_expr()?;
        if self.next() != Some(')') {
            return Err(CalcError::UnexpectedEnd);
        }

        match name {
            "sqrt" => Ok(arg.sqrt()),
            "abs" => Ok(arg.abs()),
            "sin" => Ok(arg.sin()),
            "cos" => Ok(arg.cos()),
            "tan" => Ok(arg.tan()),
            "ln" => Ok(arg.ln()),
            "log" => Ok(arg.log10()),
            "round" => Ok(arg.round()),
            "floor" => Ok(arg.floor()),
            "ceil" => Ok(arg.ceil()),
            other => Err(CalcError::UnknownFunction(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_precedence() {
        assert_eq!(evaluate("2+3*4").unwrap(), 14.0);
        assert_eq!(evaluate("(2+3)*4").unwrap(), 20.0);
        assert_eq!(evaluate("2^3^2").unwrap(), 512.0);
        assert_eq!(evaluate("-5+2").unwrap(), -3.0);
        assert_eq!(evaluate("10 % 4").unwrap(), 2.0);
    }

    #[test]
    fn functions_and_constants() {
        assert!((evaluate("sqrt(16)").unwrap() - 4.0).abs() < 1e-12);
        assert!((evaluate("sin(pi/2)").unwrap() - 1.0).abs() < 1e-12);
        assert!((evaluate("log(1000)").unwrap() - 3.0).abs() < 1e-12);
    }

    #[test]
    fn omnibox_formatting() {
        assert_eq!(try_evaluate("= 25 * 4.5").as_deref(), Some("112.5"));
        assert_eq!(try_evaluate("2+2").as_deref(), Some("4"));
        assert_eq!(try_evaluate("not math"), None);
        assert_eq!(try_evaluate("1/0"), None);
    }

    #[test]
    fn errors_are_reported() {
        assert_eq!(evaluate("1 + "), Err(CalcError::UnexpectedEnd));
        assert!(matches!(
            evaluate("1 & 2"),
            Err(CalcError::UnexpectedChar('&'))
        ));
    }
}

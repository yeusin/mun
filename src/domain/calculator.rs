use std::iter::Peekable;
use std::str::Chars;

pub fn evaluate(expr: &str) -> Option<f64> {
    let trimmed = expr.trim();
    if trimmed.is_empty() {
        return None;
    }
    let mut parser = Parser::new(trimmed);
    let result = parser.parse_expr();
    if parser.has_remaining() {
        return None;
    }
    result
}

struct Parser<'a> {
    chars: Peekable<Chars<'a>>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().peekable(),
        }
    }

    fn has_remaining(&mut self) -> bool {
        self.skip_whitespace();
        self.chars.peek().is_some()
    }

    fn skip_whitespace(&mut self) {
        while let Some(&c) = self.chars.peek() {
            if c.is_whitespace() {
                self.chars.next();
            } else {
                break;
            }
        }
    }

    fn parse_expr(&mut self) -> Option<f64> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Option<f64> {
        let mut left = self.parse_multiplicative()?;
        loop {
            self.skip_whitespace();
            let op: fn(f64, f64) -> f64 = match self.chars.peek().copied() {
                Some('+') => |a, b| a + b,
                Some('-') => |a, b| a - b,
                _ => break,
            };
            self.chars.next();
            let right = self.parse_multiplicative()?;
            left = op(left, right);
        }
        Some(left)
    }

    fn parse_multiplicative(&mut self) -> Option<f64> {
        let mut left = self.parse_unary()?;
        loop {
            self.skip_whitespace();
            let op: fn(f64, f64) -> f64 = match self.chars.peek().copied() {
                Some('*') => |a, b| a * b,
                Some('/') => |a, b| a / b,
                _ => break,
            };
            self.chars.next();
            let right = self.parse_unary()?;
            left = op(left, right);
        }
        Some(left)
    }

    fn parse_unary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        match self.chars.peek().copied() {
            Some('-') => {
                self.chars.next();
                Some(-self.parse_primary()?)
            }
            Some('+') => {
                self.chars.next();
                self.parse_primary()
            }
            _ => self.parse_primary(),
        }
    }

    fn parse_primary(&mut self) -> Option<f64> {
        self.skip_whitespace();
        match self.chars.peek().copied() {
            Some('(') => {
                self.chars.next();
                let val = self.parse_expr()?;
                self.skip_whitespace();
                if self.chars.peek() == Some(&')') {
                    self.chars.next();
                }
                Some(val)
            }
            _ => self.parse_number(),
        }
    }

    fn parse_number(&mut self) -> Option<f64> {
        self.skip_whitespace();
        let mut num_str = String::new();
        while let Some(&c) = self.chars.peek() {
            if c.is_ascii_digit() || c == '.' {
                num_str.push(c);
                self.chars.next();
            } else {
                break;
            }
        }
        num_str.parse().ok()
    }
}

pub fn format_result(value: f64) -> String {
    if value.is_nan() || value.is_infinite() {
        return "Error".to_string();
    }
    if value.fract() == 0.0 && value.abs() < i64::MAX as f64 {
        format!("{}", value as i64)
    } else {
        format!("{}", value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_arithmetic() {
        assert_eq!(evaluate("2 + 3"), Some(5.0));
        assert_eq!(evaluate("10 - 4"), Some(6.0));
        assert_eq!(evaluate("3 * 7"), Some(21.0));
        assert_eq!(evaluate("20 / 4"), Some(5.0));
    }

    #[test]
    fn test_precedence() {
        assert_eq!(evaluate("2 + 3 * 4"), Some(14.0));
        assert_eq!(evaluate("(2 + 3) * 4"), Some(20.0));
    }

    #[test]
    fn test_unary() {
        assert_eq!(evaluate("-5"), Some(-5.0));
        assert_eq!(evaluate("-(3 + 2)"), Some(-5.0));
    }

    #[test]
    fn test_float() {
        assert_eq!(evaluate("3.14 * 2"), Some(6.28));
    }

    #[test]
    fn test_complex() {
        assert_eq!(evaluate("(1 + 2) * (3 + 4)"), Some(21.0));
        assert_eq!(evaluate("100 / (2 * 5)"), Some(10.0));
    }

    #[test]
    fn test_empty() {
        assert_eq!(evaluate(""), None);
        assert_eq!(evaluate("   "), None);
    }

    #[test]
    fn test_format() {
        assert_eq!(format_result(5.0), "5");
        assert_eq!(format_result(3.14), "3.14");
        assert_eq!(format_result(f64::NAN), "Error");
    }
}

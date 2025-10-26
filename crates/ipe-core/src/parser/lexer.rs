//! Lexer for IPE policy language
//!
//! The lexer tokenizes IPE source code into a stream of tokens.

use super::token::{Token, TokenKind};

/// Lexer for tokenizing IPE source code
pub struct Lexer {
    input: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    /// Create a new lexer from source code
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    /// Get the next token
    pub fn next_token(&mut self) -> Token {
        // Skip whitespace (except newlines)
        self.skip_whitespace();

        // Save position for token
        let start_line = self.line;
        let start_column = self.column;

        // Check if we're at the end
        if self.is_at_end() {
            return Token::new(TokenKind::Eof, String::new(), start_line, start_column);
        }

        let ch = self.current_char();

        // Handle newlines
        if ch == '\n' {
            self.advance();
            return Token::new(TokenKind::Newline, "\n".to_string(), start_line, start_column);
        }

        // Handle comments
        if ch == '#' {
            self.skip_comment();
            return self.next_token(); // Get next token after comment
        }

        // Handle strings
        if ch == '"' {
            return self.lex_string();
        }

        // Handle numbers
        if ch.is_ascii_digit() {
            return self.lex_number();
        }

        // Handle identifiers and keywords
        if ch.is_alphabetic() || ch == '_' {
            return self.lex_identifier_or_keyword();
        }

        // Handle operators and punctuation
        self.lex_operator_or_punctuation()
    }

    /// Tokenize all input
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    fn current_char(&self) -> char {
        self.input[self.position]
    }

    fn peek_char(&self) -> Option<char> {
        if self.position + 1 < self.input.len() {
            Some(self.input[self.position + 1])
        } else {
            None
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }

    fn advance(&mut self) -> char {
        let ch = self.current_char();
        self.position += 1;

        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        ch
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            let ch = self.current_char();
            if ch == ' ' || ch == '\t' || ch == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // Skip until end of line
        while !self.is_at_end() && self.current_char() != '\n' {
            self.advance();
        }
    }

    fn lex_string(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;

        self.advance(); // Skip opening quote

        let mut value = String::new();
        let mut escaped = false;

        while !self.is_at_end() {
            let ch = self.current_char();

            if escaped {
                // Handle escape sequences
                let escaped_char = match ch {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '\\' => '\\',
                    '"' => '"',
                    _ => ch,
                };
                value.push(escaped_char);
                escaped = false;
                self.advance();
            } else if ch == '\\' {
                escaped = true;
                self.advance();
            } else if ch == '"' {
                self.advance(); // Skip closing quote
                return Token::new(
                    TokenKind::StringLit(value.clone()),
                    format!("\"{}\"", value),
                    start_line,
                    start_column,
                );
            } else if ch == '\n' {
                return Token::new(
                    TokenKind::Error("Unterminated string literal".to_string()),
                    value,
                    start_line,
                    start_column,
                );
            } else {
                value.push(ch);
                self.advance();
            }
        }

        Token::new(
            TokenKind::Error("Unterminated string literal".to_string()),
            value,
            start_line,
            start_column,
        )
    }

    fn lex_number(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;

        let mut number_str = String::new();
        let mut is_float = false;

        // Read digits
        while !self.is_at_end() && self.current_char().is_ascii_digit() {
            number_str.push(self.advance());
        }

        // Check for decimal point
        if !self.is_at_end() && self.current_char() == '.' {
            if let Some(next_ch) = self.peek_char() {
                if next_ch.is_ascii_digit() {
                    is_float = true;
                    number_str.push(self.advance()); // Add '.'

                    // Read fractional part
                    while !self.is_at_end() && self.current_char().is_ascii_digit() {
                        number_str.push(self.advance());
                    }
                }
            }
        }

        // Parse number
        if is_float {
            match number_str.parse::<f64>() {
                Ok(n) => Token::new(
                    TokenKind::FloatLit(n),
                    number_str,
                    start_line,
                    start_column,
                ),
                Err(_) => Token::new(
                    TokenKind::Error(format!("Invalid float literal: {}", number_str)),
                    number_str,
                    start_line,
                    start_column,
                ),
            }
        } else {
            match number_str.parse::<i64>() {
                Ok(n) => Token::new(
                    TokenKind::IntLit(n),
                    number_str,
                    start_line,
                    start_column,
                ),
                Err(_) => Token::new(
                    TokenKind::Error(format!("Invalid integer literal: {}", number_str)),
                    number_str,
                    start_line,
                    start_column,
                ),
            }
        }
    }

    fn lex_identifier_or_keyword(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;

        let mut ident = String::new();

        while !self.is_at_end() {
            let ch = self.current_char();
            if ch.is_alphanumeric() || ch == '_' {
                ident.push(self.advance());
            } else {
                break;
            }
        }

        // Check if it's a keyword or boolean literal
        let kind = match ident.as_str() {
            "policy" => TokenKind::Policy,
            "triggers" => TokenKind::Triggers,
            "when" => TokenKind::When,
            "requires" => TokenKind::Requires,
            "denies" => TokenKind::Denies,
            "with" => TokenKind::With,
            "reason" => TokenKind::Reason,
            "where" => TokenKind::Where,
            "metadata" => TokenKind::Metadata,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            "in" => TokenKind::In,
            "true" => TokenKind::BoolLit(true),
            "false" => TokenKind::BoolLit(false),
            _ => TokenKind::Ident(ident.clone()),
        };

        Token::new(kind, ident, start_line, start_column)
    }

    fn lex_operator_or_punctuation(&mut self) -> Token {
        let start_line = self.line;
        let start_column = self.column;

        let ch = self.advance();

        // Try to match two-character operators
        if !self.is_at_end() {
            let next_ch = self.current_char();
            let two_char = format!("{}{}", ch, next_ch);

            let kind = match two_char.as_str() {
                "==" => {
                    self.advance();
                    Some(TokenKind::Eq)
                }
                "!=" => {
                    self.advance();
                    Some(TokenKind::Neq)
                }
                "<=" => {
                    self.advance();
                    Some(TokenKind::LtEq)
                }
                ">=" => {
                    self.advance();
                    Some(TokenKind::GtEq)
                }
                _ => None,
            };

            if let Some(kind) = kind {
                return Token::new(kind, two_char, start_line, start_column);
            }
        }

        // Match single-character operators and punctuation
        let kind = match ch {
            '<' => TokenKind::Lt,
            '>' => TokenKind::Gt,
            ':' => TokenKind::Colon,
            ',' => TokenKind::Comma,
            '.' => TokenKind::Dot,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            _ => TokenKind::Error(format!("Unexpected character: {}", ch)),
        };

        Token::new(kind, ch.to_string(), start_line, start_column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to extract token kinds from a token stream
    fn token_kinds(tokens: &[Token]) -> Vec<TokenKind> {
        tokens.iter().map(|t| t.kind.clone()).collect()
    }

    #[test]
    fn test_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn test_keywords() {
        let input = "policy triggers when requires denies with reason where metadata and or not in";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected = vec![
            TokenKind::Policy,
            TokenKind::Triggers,
            TokenKind::When,
            TokenKind::Requires,
            TokenKind::Denies,
            TokenKind::With,
            TokenKind::Reason,
            TokenKind::Where,
            TokenKind::Metadata,
            TokenKind::And,
            TokenKind::Or,
            TokenKind::Not,
            TokenKind::In,
            TokenKind::Eof,
        ];

        assert_eq!(token_kinds(&tokens), expected);
    }

    #[test]
    fn test_operators() {
        let input = "== != < > <= >=";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected = vec![
            TokenKind::Eq,
            TokenKind::Neq,
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
            TokenKind::Eof,
        ];

        assert_eq!(token_kinds(&tokens), expected);
    }

    #[test]
    fn test_punctuation() {
        let input = ": , . ( ) [ ] { }";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected = vec![
            TokenKind::Colon,
            TokenKind::Comma,
            TokenKind::Dot,
            TokenKind::LParen,
            TokenKind::RParen,
            TokenKind::LBracket,
            TokenKind::RBracket,
            TokenKind::LBrace,
            TokenKind::RBrace,
            TokenKind::Eof,
        ];

        assert_eq!(token_kinds(&tokens), expected);
    }

    #[test]
    fn test_string_literals() {
        let input = r#""hello" "world with spaces" "escaped\"quote""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::StringLit("hello".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::StringLit("world with spaces".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::StringLit("escaped\"quote".to_string()));
    }

    #[test]
    fn test_string_escape_sequences() {
        let input = r#""line1\nline2" "tab\there" "backslash\\""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::StringLit("line1\nline2".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::StringLit("tab\there".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::StringLit("backslash\\".to_string()));
    }

    #[test]
    fn test_unterminated_string() {
        let input = r#""unterminated"#;
        let mut lexer = Lexer::new(input);
        let token = lexer.next_token();

        match token.kind {
            TokenKind::Error(msg) => assert!(msg.contains("Unterminated")),
            _ => panic!("Expected error token"),
        }
    }

    #[test]
    fn test_integer_literals() {
        let input = "0 42 12345 999";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::IntLit(0));
        assert_eq!(tokens[1].kind, TokenKind::IntLit(42));
        assert_eq!(tokens[2].kind, TokenKind::IntLit(12345));
        assert_eq!(tokens[3].kind, TokenKind::IntLit(999));
    }

    #[test]
    fn test_float_literals() {
        let input = "3.14 0.5 42.0 123.456";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::FloatLit(3.14));
        assert_eq!(tokens[1].kind, TokenKind::FloatLit(0.5));
        assert_eq!(tokens[2].kind, TokenKind::FloatLit(42.0));
        assert_eq!(tokens[3].kind, TokenKind::FloatLit(123.456));
    }

    #[test]
    fn test_boolean_literals() {
        let input = "true false";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::BoolLit(true));
        assert_eq!(tokens[1].kind, TokenKind::BoolLit(false));
    }

    #[test]
    fn test_identifiers() {
        let input = "foo bar_baz myVar_123 _private";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Ident("foo".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Ident("bar_baz".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Ident("myVar_123".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::Ident("_private".to_string()));
    }

    #[test]
    fn test_comments() {
        let input = "policy # this is a comment\nrequires";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Policy);
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::Requires);
    }

    #[test]
    fn test_newlines() {
        let input = "policy\nrequires\n\ndenies";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Policy);
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::Requires);
        assert_eq!(tokens[3].kind, TokenKind::Newline);
        assert_eq!(tokens[4].kind, TokenKind::Newline);
        assert_eq!(tokens[5].kind, TokenKind::Denies);
    }

    #[test]
    fn test_position_tracking() {
        let input = "policy\n  requires";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].line, 1);
        assert_eq!(tokens[0].column, 1);

        assert_eq!(tokens[1].line, 1); // Newline
        assert_eq!(tokens[1].column, 7);

        assert_eq!(tokens[2].line, 2);
        assert_eq!(tokens[2].column, 3); // After 2 spaces
    }

    #[test]
    fn test_complex_expression() {
        let input = r#"resource.type == "Deployment" and count >= 2"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected = vec![
            TokenKind::Ident("resource".to_string()),
            TokenKind::Dot,
            TokenKind::Ident("type".to_string()),
            TokenKind::Eq,
            TokenKind::StringLit("Deployment".to_string()),
            TokenKind::And,
            TokenKind::Ident("count".to_string()),
            TokenKind::GtEq,
            TokenKind::IntLit(2),
            TokenKind::Eof,
        ];

        assert_eq!(token_kinds(&tokens), expected);
    }

    #[test]
    fn test_rfc_example_snippet() {
        let input = r#"policy RequireApproval:
  "Production deployments need 2+ approvals"

  triggers when
    resource.type == "Deployment""#;

        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Verify key tokens are present
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Policy));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Ident("RequireApproval".to_string())));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Colon));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::StringLit(_))));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Triggers));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::When));
    }

    #[test]
    fn test_whitespace_handling() {
        let input = "policy   \t  requires";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens.len(), 3); // policy, requires, EOF
        assert_eq!(tokens[0].kind, TokenKind::Policy);
        assert_eq!(tokens[1].kind, TokenKind::Requires);
    }

    #[test]
    fn test_array_syntax() {
        let input = r#"["production", "staging"]"#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected = vec![
            TokenKind::LBracket,
            TokenKind::StringLit("production".to_string()),
            TokenKind::Comma,
            TokenKind::StringLit("staging".to_string()),
            TokenKind::RBracket,
            TokenKind::Eof,
        ];

        assert_eq!(token_kinds(&tokens), expected);
    }

    #[test]
    fn test_metadata_block() {
        let input = "metadata\n  severity: critical";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert!(tokens.iter().any(|t| t.kind == TokenKind::Metadata));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Ident("severity".to_string())));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Colon));
        assert!(tokens.iter().any(|t| t.kind == TokenKind::Ident("critical".to_string())));
    }

    #[test]
    fn test_error_unexpected_character() {
        let input = "policy @ requires";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Policy);
        match &tokens[1].kind {
            TokenKind::Error(msg) => assert!(msg.contains("Unexpected character")),
            _ => panic!("Expected error token"),
        }
    }

    #[test]
    fn test_dot_not_part_of_number() {
        let input = "42.field";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should be: 42 . field
        assert_eq!(tokens[0].kind, TokenKind::IntLit(42));
        assert_eq!(tokens[1].kind, TokenKind::Dot);
        assert_eq!(tokens[2].kind, TokenKind::Ident("field".to_string()));
    }

    #[test]
    fn test_string_with_newline() {
        let input = "\"line1\nline2\"";
        let mut lexer = Lexer::new(input);
        let token = lexer.next_token();

        match token.kind {
            TokenKind::Error(msg) => assert!(msg.contains("Unterminated")),
            _ => panic!("Expected error for string with newline"),
        }
    }

    #[test]
    fn test_number_overflow() {
        // Test a number that's too large for i64
        let input = "99999999999999999999999999999";
        let mut lexer = Lexer::new(input);
        let token = lexer.next_token();

        match token.kind {
            TokenKind::Error(msg) => assert!(msg.contains("Invalid integer")),
            _ => panic!("Expected error for integer overflow"),
        }
    }

    #[test]
    fn test_float_with_large_exponent() {
        // This will parse as an identifier since 'e' makes it non-numeric
        let input = "1e99999999999999999999";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should be parsed as: 1 e99999999999999999999
        assert_eq!(tokens[0].kind, TokenKind::IntLit(1));
        assert!(matches!(tokens[1].kind, TokenKind::Ident(_)));
    }

    #[test]
    fn test_multiple_errors() {
        let input = "@ $ %";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should have 3 errors
        let error_count = tokens.iter().filter(|t| matches!(t.kind, TokenKind::Error(_))).count();
        assert_eq!(error_count, 3);
    }

    #[test]
    fn test_mixed_operators() {
        let input = "< > <= >= == !=";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        let expected = vec![
            TokenKind::Lt,
            TokenKind::Gt,
            TokenKind::LtEq,
            TokenKind::GtEq,
            TokenKind::Eq,
            TokenKind::Neq,
            TokenKind::Eof,
        ];

        assert_eq!(token_kinds(&tokens), expected);
    }

    #[test]
    fn test_edge_case_dot_at_end() {
        let input = "42.";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should be: 42 .
        assert_eq!(tokens[0].kind, TokenKind::IntLit(42));
        assert_eq!(tokens[1].kind, TokenKind::Dot);
    }

    #[test]
    fn test_multiple_newlines_in_row() {
        let input = "\n\n\n";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // Should have 3 newlines + EOF
        let newline_count = tokens.iter().filter(|t| t.kind == TokenKind::Newline).count();
        assert_eq!(newline_count, 3);
    }

    #[test]
    fn test_comment_at_end_of_file() {
        let input = "policy # comment";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Policy);
        assert_eq!(tokens[1].kind, TokenKind::Eof);
    }

    #[test]
    fn test_all_escape_sequences() {
        let input = "\"\\n\\t\\r\\\\\\\"\"";
        let mut lexer = Lexer::new(input);
        let token = lexer.next_token();

        assert_eq!(token.kind, TokenKind::StringLit("\n\t\r\\\"".to_string()));
    }

    #[test]
    fn test_underscore_identifier() {
        let input = "_ _x x_";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0].kind, TokenKind::Ident("_".to_string()));
        assert_eq!(tokens[1].kind, TokenKind::Ident("_x".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Ident("x_".to_string()));
    }

    #[test]
    fn test_zero_float() {
        let input = "0.0";
        let mut lexer = Lexer::new(input);
        let token = lexer.next_token();

        assert_eq!(token.kind, TokenKind::FloatLit(0.0));
    }

    #[test]
    fn test_carriage_return_handling() {
        let input = "policy\r\nrequires";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize();

        // \r is whitespace, \n is newline
        assert_eq!(tokens[0].kind, TokenKind::Policy);
        assert_eq!(tokens[1].kind, TokenKind::Newline);
        assert_eq!(tokens[2].kind, TokenKind::Requires);
    }
}

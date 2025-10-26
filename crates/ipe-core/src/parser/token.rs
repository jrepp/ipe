//! Token definitions for IPE language

use std::fmt;

/// A token in the IPE language with position information
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    /// The kind of token
    pub kind: TokenKind,
    /// The source text for this token
    pub text: String,
    /// Line number (1-indexed)
    pub line: usize,
    /// Column number (1-indexed)
    pub column: usize,
}

impl Token {
    /// Create a new token
    pub fn new(kind: TokenKind, text: String, line: usize, column: usize) -> Self {
        Self { kind, text, line, column }
    }
}

/// The kind of token
#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Policy,
    Triggers,
    When,
    Requires,
    Denies,
    With,
    Reason,
    Where,
    Metadata,
    And,
    Or,
    Not,
    In,

    // Comparison operators
    Eq,   // ==
    Neq,  // !=
    Lt,   // <
    Gt,   // >
    LtEq, // <=
    GtEq, // >=

    // Literals
    StringLit(String),
    IntLit(i64),
    FloatLit(f64),
    BoolLit(bool),

    // Identifiers
    Ident(String),

    // Punctuation
    Colon,    // :
    Comma,    // ,
    Dot,      // .
    LParen,   // (
    RParen,   // )
    LBracket, // [
    RBracket, // ]
    LBrace,   // {
    RBrace,   // }

    // Special
    Newline,
    Eof,
    Error(String),
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Policy => write!(f, "policy"),
            TokenKind::Triggers => write!(f, "triggers"),
            TokenKind::When => write!(f, "when"),
            TokenKind::Requires => write!(f, "requires"),
            TokenKind::Denies => write!(f, "denies"),
            TokenKind::With => write!(f, "with"),
            TokenKind::Reason => write!(f, "reason"),
            TokenKind::Where => write!(f, "where"),
            TokenKind::Metadata => write!(f, "metadata"),
            TokenKind::And => write!(f, "and"),
            TokenKind::Or => write!(f, "or"),
            TokenKind::Not => write!(f, "not"),
            TokenKind::In => write!(f, "in"),
            TokenKind::Eq => write!(f, "=="),
            TokenKind::Neq => write!(f, "!="),
            TokenKind::Lt => write!(f, "<"),
            TokenKind::Gt => write!(f, ">"),
            TokenKind::LtEq => write!(f, "<="),
            TokenKind::GtEq => write!(f, ">="),
            TokenKind::StringLit(s) => write!(f, "\"{}\"", s),
            TokenKind::IntLit(n) => write!(f, "{}", n),
            TokenKind::FloatLit(n) => write!(f, "{}", n),
            TokenKind::BoolLit(b) => write!(f, "{}", b),
            TokenKind::Ident(s) => write!(f, "{}", s),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::LParen => write!(f, "("),
            TokenKind::RParen => write!(f, ")"),
            TokenKind::LBracket => write!(f, "["),
            TokenKind::RBracket => write!(f, "]"),
            TokenKind::LBrace => write!(f, "{{"),
            TokenKind::RBrace => write!(f, "}}"),
            TokenKind::Newline => write!(f, "\\n"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::Error(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl TokenKind {
    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            TokenKind::Policy
                | TokenKind::Triggers
                | TokenKind::When
                | TokenKind::Requires
                | TokenKind::Denies
                | TokenKind::With
                | TokenKind::Reason
                | TokenKind::Where
                | TokenKind::Metadata
                | TokenKind::And
                | TokenKind::Or
                | TokenKind::Not
                | TokenKind::In
        )
    }

    /// Check if this token is an operator
    pub fn is_operator(&self) -> bool {
        matches!(
            self,
            TokenKind::Eq
                | TokenKind::Neq
                | TokenKind::Lt
                | TokenKind::Gt
                | TokenKind::LtEq
                | TokenKind::GtEq
        )
    }

    /// Check if this token is a literal
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            TokenKind::StringLit(_)
                | TokenKind::IntLit(_)
                | TokenKind::FloatLit(_)
                | TokenKind::BoolLit(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let token = Token::new(TokenKind::Policy, "policy".to_string(), 1, 1);
        assert_eq!(token.kind, TokenKind::Policy);
        assert_eq!(token.text, "policy");
        assert_eq!(token.line, 1);
        assert_eq!(token.column, 1);
    }

    #[test]
    fn test_token_kind_is_keyword() {
        assert!(TokenKind::Policy.is_keyword());
        assert!(TokenKind::Triggers.is_keyword());
        assert!(TokenKind::When.is_keyword());
        assert!(TokenKind::And.is_keyword());
        assert!(!TokenKind::Eq.is_keyword());
        assert!(!TokenKind::Ident("foo".to_string()).is_keyword());
    }

    #[test]
    fn test_token_kind_is_operator() {
        assert!(TokenKind::Eq.is_operator());
        assert!(TokenKind::Neq.is_operator());
        assert!(TokenKind::Lt.is_operator());
        assert!(!TokenKind::Policy.is_operator());
        assert!(!TokenKind::Colon.is_operator());
    }

    #[test]
    fn test_token_kind_is_literal() {
        assert!(TokenKind::StringLit("test".to_string()).is_literal());
        assert!(TokenKind::IntLit(42).is_literal());
        assert!(TokenKind::FloatLit(3.14).is_literal());
        assert!(TokenKind::BoolLit(true).is_literal());
        assert!(!TokenKind::Ident("foo".to_string()).is_literal());
    }

    #[test]
    fn test_token_kind_display() {
        assert_eq!(TokenKind::Policy.to_string(), "policy");
        assert_eq!(TokenKind::Eq.to_string(), "==");
        assert_eq!(TokenKind::StringLit("test".to_string()).to_string(), "\"test\"");
        assert_eq!(TokenKind::IntLit(42).to_string(), "42");
    }

    #[test]
    fn test_token_equality() {
        let t1 = Token::new(TokenKind::Policy, "policy".to_string(), 1, 1);
        let t2 = Token::new(TokenKind::Policy, "policy".to_string(), 1, 1);
        assert_eq!(t1, t2);
    }

    #[test]
    fn test_token_kind_display_keywords() {
        assert_eq!(TokenKind::Policy.to_string(), "policy");
        assert_eq!(TokenKind::Triggers.to_string(), "triggers");
        assert_eq!(TokenKind::When.to_string(), "when");
        assert_eq!(TokenKind::Requires.to_string(), "requires");
        assert_eq!(TokenKind::Denies.to_string(), "denies");
        assert_eq!(TokenKind::With.to_string(), "with");
        assert_eq!(TokenKind::Reason.to_string(), "reason");
        assert_eq!(TokenKind::Where.to_string(), "where");
        assert_eq!(TokenKind::Metadata.to_string(), "metadata");
        assert_eq!(TokenKind::And.to_string(), "and");
        assert_eq!(TokenKind::Or.to_string(), "or");
        assert_eq!(TokenKind::Not.to_string(), "not");
        assert_eq!(TokenKind::In.to_string(), "in");
    }

    #[test]
    fn test_token_kind_display_operators() {
        assert_eq!(TokenKind::Eq.to_string(), "==");
        assert_eq!(TokenKind::Neq.to_string(), "!=");
        assert_eq!(TokenKind::Lt.to_string(), "<");
        assert_eq!(TokenKind::Gt.to_string(), ">");
        assert_eq!(TokenKind::LtEq.to_string(), "<=");
        assert_eq!(TokenKind::GtEq.to_string(), ">=");
    }

    #[test]
    fn test_token_kind_display_literals() {
        assert_eq!(TokenKind::StringLit("hello".to_string()).to_string(), "\"hello\"");
        assert_eq!(TokenKind::IntLit(42).to_string(), "42");
        assert_eq!(TokenKind::IntLit(-100).to_string(), "-100");
        assert_eq!(TokenKind::FloatLit(3.14).to_string(), "3.14");
        assert_eq!(TokenKind::FloatLit(-2.5).to_string(), "-2.5");
        assert_eq!(TokenKind::BoolLit(true).to_string(), "true");
        assert_eq!(TokenKind::BoolLit(false).to_string(), "false");
    }

    #[test]
    fn test_token_kind_display_identifiers() {
        assert_eq!(TokenKind::Ident("foo".to_string()).to_string(), "foo");
        assert_eq!(TokenKind::Ident("myVariable".to_string()).to_string(), "myVariable");
    }

    #[test]
    fn test_token_kind_display_punctuation() {
        assert_eq!(TokenKind::Colon.to_string(), ":");
        assert_eq!(TokenKind::Comma.to_string(), ",");
        assert_eq!(TokenKind::Dot.to_string(), ".");
        assert_eq!(TokenKind::LParen.to_string(), "(");
        assert_eq!(TokenKind::RParen.to_string(), ")");
        assert_eq!(TokenKind::LBracket.to_string(), "[");
        assert_eq!(TokenKind::RBracket.to_string(), "]");
        assert_eq!(TokenKind::LBrace.to_string(), "{");
        assert_eq!(TokenKind::RBrace.to_string(), "}");
    }

    #[test]
    fn test_token_kind_display_special() {
        assert_eq!(TokenKind::Newline.to_string(), "\\n");
        assert_eq!(TokenKind::Eof.to_string(), "EOF");
        assert_eq!(
            TokenKind::Error("unexpected character".to_string()).to_string(),
            "Error: unexpected character"
        );
    }

    #[test]
    fn test_all_keywords_categorized() {
        // Ensure all keywords are properly identified
        assert!(TokenKind::Requires.is_keyword());
        assert!(TokenKind::Denies.is_keyword());
        assert!(TokenKind::With.is_keyword());
        assert!(TokenKind::Reason.is_keyword());
        assert!(TokenKind::Where.is_keyword());
        assert!(TokenKind::Metadata.is_keyword());
        assert!(TokenKind::Or.is_keyword());
        assert!(TokenKind::Not.is_keyword());
        assert!(TokenKind::In.is_keyword());
    }

    #[test]
    fn test_all_operators_categorized() {
        // Ensure all operators are properly identified
        assert!(TokenKind::Neq.is_operator());
        assert!(TokenKind::Gt.is_operator());
        assert!(TokenKind::LtEq.is_operator());
        assert!(TokenKind::GtEq.is_operator());
    }

    #[test]
    fn test_all_literals_categorized() {
        // Ensure all literals are properly identified
        assert!(TokenKind::BoolLit(false).is_literal());
        assert!(TokenKind::FloatLit(-1.0).is_literal());
        assert!(TokenKind::IntLit(-42).is_literal());
    }

    #[test]
    fn test_token_clone() {
        let token = Token::new(TokenKind::Policy, "policy".to_string(), 1, 5);
        let cloned = token.clone();
        assert_eq!(token, cloned);
        assert_eq!(cloned.line, 1);
        assert_eq!(cloned.column, 5);
    }

    #[test]
    fn test_token_kind_clone() {
        let kind = TokenKind::StringLit("test".to_string());
        let cloned = kind.clone();
        assert_eq!(kind, cloned);
    }
}

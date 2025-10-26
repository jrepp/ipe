//! Parser implementation for IPE policies

use super::lexer::Lexer;
use super::token::{Token, TokenKind};
use crate::ast::nodes::{
    BinaryOp, ComparisonOp, Condition, Expression, Metadata, Policy,
    Requirements, SourceLocation, Value,
};
use thiserror::Error;

#[cfg(test)]
use crate::ast::nodes::LogicalOp;

/// Parse error
#[derive(Error, Debug, Clone, PartialEq)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected}, got {got}")]
    UnexpectedToken { expected: String, got: String },

    #[error("Unexpected end of file")]
    UnexpectedEof,

    #[error("Invalid expression: {0}")]
    InvalidExpression(String),

    #[error("Invalid policy structure: {0}")]
    InvalidPolicy(String),
}

pub type ParseResult<T> = Result<T, ParseError>;

/// Parser for IPE policies
pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    /// Create a new parser from source code
    pub fn new(source: &str) -> Self {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();
        Self {
            tokens,
            position: 0,
        }
    }

    /// Parse a complete policy
    pub fn parse_policy(&mut self) -> ParseResult<Policy> {
        // Skip newlines
        self.skip_newlines();

        // Expect "policy"
        self.expect_keyword(TokenKind::Policy)?;

        // Parse name
        let name = self.expect_identifier()?;

        // Expect ":"
        self.expect_token(TokenKind::Colon)?;

        // Skip newlines
        self.skip_newlines();

        // Parse intent string
        let intent = self.expect_string()?;

        // Skip newlines
        self.skip_newlines();

        // Parse triggers
        let triggers = self.parse_triggers()?;

        // Skip newlines
        self.skip_newlines();

        // Parse requirements
        let requirements = self.parse_requirements()?;

        // Skip newlines
        self.skip_newlines();

        // Parse optional metadata
        let metadata = if self.check_keyword(TokenKind::Metadata) {
            Some(self.parse_metadata()?)
        } else {
            None
        };

        Ok(Policy {
            name,
            intent,
            triggers,
            requirements,
            metadata,
            location: SourceLocation::default(),
        })
    }

    fn parse_triggers(&mut self) -> ParseResult<Vec<Condition>> {
        self.expect_keyword(TokenKind::Triggers)?;
        self.expect_keyword(TokenKind::When)?;
        self.skip_newlines();

        let mut triggers = Vec::new();

        loop {
            let expr = self.parse_expression()?;
            triggers.push(Condition::new(expr));

            self.skip_newlines();

            // Check if we're done with triggers
            if self.check_keyword(TokenKind::Requires) || self.check_keyword(TokenKind::Denies) {
                break;
            }

            // If we see 'and', continue parsing triggers
            if self.check_keyword(TokenKind::And) {
                self.advance(); // consume 'and'
                self.skip_newlines();
            } else {
                break;
            }
        }

        Ok(triggers)
    }

    fn parse_requirements(&mut self) -> ParseResult<Requirements> {
        if self.check_keyword(TokenKind::Requires) {
            self.advance(); // consume 'requires'
            self.skip_newlines();

            let mut conditions = Vec::new();

            loop {
                let expr = self.parse_expression()?;
                conditions.push(Condition::new(expr));

                self.skip_newlines();

                // Check for 'and' or 'where'
                if self.check_keyword(TokenKind::And) {
                    self.advance();
                    self.skip_newlines();
                } else if self.check_keyword(TokenKind::Where) {
                    // Parse where clause
                    self.advance();
                    self.skip_newlines();

                    let mut where_conditions = Vec::new();
                    loop {
                        let expr = self.parse_expression()?;
                        where_conditions.push(Condition::new(expr));

                        self.skip_newlines();

                        if self.check_keyword(TokenKind::And) {
                            self.advance();
                            self.skip_newlines();
                        } else {
                            break;
                        }
                    }

                    return Ok(Requirements::requires_where(conditions, where_conditions));
                } else {
                    break;
                }
            }

            Ok(Requirements::requires(conditions))
        } else if self.check_keyword(TokenKind::Denies) {
            self.advance(); // consume 'denies'
            self.skip_newlines();

            // Check for optional "with reason"
            let reason = if self.check_keyword(TokenKind::With) {
                self.advance();
                self.expect_keyword(TokenKind::Reason)?;
                Some(self.expect_string()?)
            } else {
                None
            };

            Ok(Requirements::denies(reason))
        } else {
            Err(ParseError::InvalidPolicy(
                "Expected 'requires' or 'denies'".to_string(),
            ))
        }
    }

    fn parse_metadata(&mut self) -> ParseResult<Metadata> {
        self.expect_keyword(TokenKind::Metadata)?;
        self.skip_newlines();

        let mut metadata = Metadata::new();

        loop {
            // Parse key
            let key = self.expect_identifier()?;
            self.expect_token(TokenKind::Colon)?;

            // Parse value
            let value = self.parse_value()?;
            metadata = metadata.add_field(key, value);

            self.skip_newlines();

            // Check if there are more fields
            if self.is_at_end() || !self.current().kind.is_literal() && !matches!(self.current().kind, TokenKind::Ident(_)) {
                break;
            }
        }

        Ok(metadata)
    }

    /// Parse an expression
    pub fn parse_expression(&mut self) -> ParseResult<Expression> {
        self.parse_logical_or()
    }

    fn parse_logical_or(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_logical_and()?;

        self.skip_newlines();
        while self.check_keyword(TokenKind::Or) {
            self.advance();
            self.skip_newlines();
            let right = self.parse_logical_and()?;
            left = Expression::or(vec![left, right]);
            self.skip_newlines();
        }

        Ok(left)
    }

    fn parse_logical_and(&mut self) -> ParseResult<Expression> {
        let mut left = self.parse_comparison()?;

        self.skip_newlines();
        while self.check_keyword(TokenKind::And) {
            self.advance();
            self.skip_newlines();
            let right = self.parse_comparison()?;
            left = Expression::and(vec![left, right]);
            self.skip_newlines();
        }

        Ok(left)
    }

    fn parse_comparison(&mut self) -> ParseResult<Expression> {
        let left = self.parse_in_expression()?;

        // Check for comparison operator
        if let Some(op) = self.parse_comparison_op() {
            self.advance();
            let right = self.parse_in_expression()?;
            Ok(Expression::binary(left, BinaryOp::Comparison(op), right))
        } else {
            Ok(left)
        }
    }

    fn parse_in_expression(&mut self) -> ParseResult<Expression> {
        let expr = self.parse_primary()?;

        if self.check_keyword(TokenKind::In) {
            self.advance();
            self.expect_token(TokenKind::LBracket)?;

            let mut values = Vec::new();
            loop {
                values.push(self.parse_value()?);

                if self.check_token(TokenKind::Comma) {
                    self.advance();
                } else {
                    break;
                }
            }

            self.expect_token(TokenKind::RBracket)?;
            Ok(Expression::in_list(expr, values))
        } else {
            Ok(expr)
        }
    }

    fn parse_primary(&mut self) -> ParseResult<Expression> {
        let token_kind = self.current().kind.clone();

        match token_kind {
            // Literals
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Expression::literal(Value::String(s)))
            }
            TokenKind::IntLit(n) => {
                self.advance();
                Ok(Expression::literal(Value::Int(n)))
            }
            TokenKind::FloatLit(f) => {
                self.advance();
                Ok(Expression::literal(Value::Float(f)))
            }
            TokenKind::BoolLit(b) => {
                self.advance();
                Ok(Expression::literal(Value::Bool(b)))
            }

            // Identifiers and paths
            TokenKind::Ident(_) => self.parse_path_or_call(),

            // Parenthesized expressions
            TokenKind::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_token(TokenKind::RParen)?;
                Ok(expr)
            }

            // NOT operator
            TokenKind::Not => {
                self.advance();
                let operand = self.parse_primary()?;
                Ok(Expression::not(operand))
            }

            _ => Err(ParseError::InvalidExpression(format!(
                "Unexpected token: {}",
                token_kind
            ))),
        }
    }

    fn parse_path_or_call(&mut self) -> ParseResult<Expression> {
        let mut segments = vec![self.expect_identifier()?];

        // Parse path segments
        while self.check_token(TokenKind::Dot) {
            self.advance();
            segments.push(self.expect_identifier()?);
        }

        // Check for function call
        if self.check_token(TokenKind::LParen) {
            if segments.len() > 1 {
                return Err(ParseError::InvalidExpression(
                    "Function calls cannot have path segments".to_string(),
                ));
            }

            self.advance();
            let mut args = Vec::new();

            if !self.check_token(TokenKind::RParen) {
                loop {
                    args.push(self.parse_expression()?);

                    if self.check_token(TokenKind::Comma) {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }

            self.expect_token(TokenKind::RParen)?;
            Ok(Expression::Call {
                name: segments[0].clone(),
                args,
            })
        } else {
            Ok(Expression::path(segments))
        }
    }

    fn parse_value(&mut self) -> ParseResult<Value> {
        let token_kind = self.current().kind.clone();

        match token_kind {
            TokenKind::StringLit(s) => {
                self.advance();
                Ok(Value::String(s))
            }
            TokenKind::IntLit(n) => {
                self.advance();
                Ok(Value::Int(n))
            }
            TokenKind::FloatLit(f) => {
                self.advance();
                Ok(Value::Float(f))
            }
            TokenKind::BoolLit(b) => {
                self.advance();
                Ok(Value::Bool(b))
            }
            TokenKind::Ident(s) => {
                self.advance();
                Ok(Value::String(s))
            }
            TokenKind::LBracket => {
                self.advance();
                let mut values = Vec::new();

                if !self.check_token(TokenKind::RBracket) {
                    loop {
                        values.push(self.parse_value()?);

                        if self.check_token(TokenKind::Comma) {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }

                self.expect_token(TokenKind::RBracket)?;
                Ok(Value::Array(values))
            }
            _ => Err(ParseError::InvalidExpression(format!(
                "Expected value, got {}",
                token_kind
            ))),
        }
    }

    fn parse_comparison_op(&self) -> Option<ComparisonOp> {
        match self.current().kind {
            TokenKind::Eq => Some(ComparisonOp::Eq),
            TokenKind::Neq => Some(ComparisonOp::Neq),
            TokenKind::Lt => Some(ComparisonOp::Lt),
            TokenKind::Gt => Some(ComparisonOp::Gt),
            TokenKind::LtEq => Some(ComparisonOp::LtEq),
            TokenKind::GtEq => Some(ComparisonOp::GtEq),
            _ => None,
        }
    }

    // Helper methods

    fn current(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            self.position += 1;
        }
    }

    fn is_at_end(&self) -> bool {
        matches!(self.current().kind, TokenKind::Eof)
    }

    fn check_token(&self, kind: TokenKind) -> bool {
        !self.is_at_end() && self.current().kind == kind
    }

    fn check_keyword(&self, kind: TokenKind) -> bool {
        self.check_token(kind)
    }

    fn skip_newlines(&mut self) {
        while self.check_token(TokenKind::Newline) {
            self.advance();
        }
    }

    fn expect_token(&mut self, expected: TokenKind) -> ParseResult<()> {
        if self.check_token(expected.clone()) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                expected: format!("{}", expected),
                got: format!("{}", self.current().kind),
            })
        }
    }

    fn expect_keyword(&mut self, expected: TokenKind) -> ParseResult<()> {
        self.expect_token(expected)
    }

    fn expect_identifier(&mut self) -> ParseResult<String> {
        match &self.current().kind {
            TokenKind::Ident(s) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "identifier".to_string(),
                got: format!("{}", self.current().kind),
            }),
        }
    }

    fn expect_string(&mut self) -> ParseResult<String> {
        match &self.current().kind {
            TokenKind::StringLit(s) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(ParseError::UnexpectedToken {
                expected: "string literal".to_string(),
                got: format!("{}", self.current().kind),
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_literal_int() {
        let mut parser = Parser::new("42");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Literal(Value::Int(42))));
    }

    #[test]
    fn test_parse_literal_float() {
        let mut parser = Parser::new("3.14");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Literal(Value::Float(_))));
    }

    #[test]
    fn test_parse_literal_string() {
        let mut parser = Parser::new("\"hello\"");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Literal(Value::String(s)) => assert_eq!(s, "hello"),
            _ => panic!("Expected string literal"),
        }
    }

    #[test]
    fn test_parse_literal_bool() {
        let mut parser = Parser::new("true");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Literal(Value::Bool(true))));

        let mut parser = Parser::new("false");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Literal(Value::Bool(false))));
    }

    #[test]
    fn test_parse_simple_path() {
        let mut parser = Parser::new("resource");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Path(path) => {
                assert_eq!(path.segments.len(), 1);
                assert_eq!(path.segments[0], "resource");
            }
            _ => panic!("Expected path"),
        }
    }

    #[test]
    fn test_parse_dotted_path() {
        let mut parser = Parser::new("resource.type");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Path(path) => {
                assert_eq!(path.segments.len(), 2);
                assert_eq!(path.segments[0], "resource");
                assert_eq!(path.segments[1], "type");
            }
            _ => panic!("Expected path"),
        }
    }

    #[test]
    fn test_parse_binary_eq() {
        let mut parser = Parser::new("x == 42");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Binary { op, .. } => {
                assert_eq!(op, BinaryOp::Comparison(ComparisonOp::Eq));
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_parse_binary_lt() {
        let mut parser = Parser::new("count < 10");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Binary { op, .. } => {
                assert_eq!(op, BinaryOp::Comparison(ComparisonOp::Lt));
            }
            _ => panic!("Expected binary expression"),
        }
    }

    #[test]
    fn test_parse_logical_and() {
        let mut parser = Parser::new("true and false");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Logical { op, operands } => {
                assert_eq!(op, LogicalOp::And);
                assert_eq!(operands.len(), 2);
            }
            _ => panic!("Expected logical AND"),
        }
    }

    #[test]
    fn test_parse_logical_or() {
        let mut parser = Parser::new("true or false");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Logical { op, operands } => {
                assert_eq!(op, LogicalOp::Or);
                assert_eq!(operands.len(), 2);
            }
            _ => panic!("Expected logical OR"),
        }
    }

    #[test]
    fn test_parse_logical_not() {
        let mut parser = Parser::new("not true");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Logical { op, operands } => {
                assert_eq!(op, LogicalOp::Not);
                assert_eq!(operands.len(), 1);
            }
            _ => panic!("Expected logical NOT"),
        }
    }

    #[test]
    fn test_parse_in_expression() {
        let mut parser = Parser::new("env in [\"prod\", \"staging\"]");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::In { list, .. } => {
                assert_eq!(list.len(), 2);
            }
            _ => panic!("Expected IN expression"),
        }
    }

    #[test]
    fn test_parse_parenthesized() {
        let mut parser = Parser::new("(42)");
        let expr = parser.parse_expression().unwrap();
        assert!(matches!(expr, Expression::Literal(Value::Int(42))));
    }

    #[test]
    fn test_parse_function_call() {
        let mut parser = Parser::new("count()");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Call { name, args } => {
                assert_eq!(name, "count");
                assert_eq!(args.len(), 0);
            }
            _ => panic!("Expected function call"),
        }
    }

    #[test]
    fn test_parse_function_call_with_args() {
        let mut parser = Parser::new("max(1, 2, 3)");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Call { name, args } => {
                assert_eq!(name, "max");
                assert_eq!(args.len(), 3);
            }
            _ => panic!("Expected function call"),
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        let mut parser = Parser::new("resource.type == \"Deployment\" and count >= 2");
        let expr = parser.parse_expression().unwrap();
        match expr {
            Expression::Logical { op, operands } => {
                assert_eq!(op, LogicalOp::And);
                assert_eq!(operands.len(), 2);
            }
            _ => panic!("Expected logical AND with two comparisons"),
        }
    }

    #[test]
    fn test_parse_simple_policy() {
        let source = r#"policy TestPolicy:
  "Test intent"

  triggers when
    true

  requires
    true
"#;

        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "TestPolicy");
        assert_eq!(policy.intent, "Test intent");
        assert_eq!(policy.triggers.len(), 1);
    }

    #[test]
    fn test_parse_policy_with_denies() {
        let source = r#"policy DenyPolicy:
  "Deny access"

  triggers when
    true

  denies with reason "Not authorized"
"#;

        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "DenyPolicy");
        match policy.requirements {
            Requirements::Denies { reason } => {
                assert_eq!(reason, Some("Not authorized".to_string()));
            }
            _ => panic!("Expected denies"),
        }
    }

    #[test]
    fn test_parse_policy_with_metadata() {
        let source = r#"policy MetadataPolicy:
  "Has metadata"

  triggers when
    true

  requires
    true

  metadata
    severity: critical
    owner: security-team
"#;

        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert!(policy.metadata.is_some());
        let metadata = policy.metadata.unwrap();
        assert_eq!(
            metadata.get("severity"),
            Some(&Value::String("critical".to_string()))
        );
    }

    #[test]
    fn test_parse_rfc_example() {
        let source = r#"policy RequireApproval:
  "Production deployments need 2+ approvals"

  triggers when
    resource.type == "Deployment"
    and environment in ["production", "staging"]

  requires
    approvals.count >= 2
"#;

        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "RequireApproval");
        assert_eq!(policy.triggers.len(), 1); // Combined with AND
    }

    #[test]
    fn test_error_unexpected_token() {
        // Test that error tokens from lexer are properly rejected
        // @ is an invalid character that produces an Error token
        let mut parser = Parser::new("@");
        let result = parser.parse_expression();
        assert!(result.is_err());
    }

    #[test]
    fn test_error_missing_colon() {
        let source = "policy Test\n  \"intent\"";
        let mut parser = Parser::new(source);
        let result = parser.parse_policy();
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_with_multiple_trigger_conditions() {
        let source = r#"
policy MultiTrigger:
  "Policy with multiple trigger conditions"

  triggers when
    resource.type == "Document"
    and environment == "production"

  requires
    user.role == "admin"
"#;
        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "MultiTrigger");
        // The parser combines AND conditions into a single logical expression
        assert_eq!(policy.triggers.len(), 1);

        // Verify it's a logical AND expression
        match &policy.triggers[0].expr {
            Expression::Logical { op: LogicalOp::And, operands } => {
                assert_eq!(operands.len(), 2);
            }
            _ => panic!("Expected logical AND expression"),
        }
    }

    #[test]
    fn test_policy_with_requires_where_clause() {
        let source = r#"
policy RequireApprovalWhere:
  "Require approval from senior engineers"

  triggers when
    resource.type == "Deployment"

  requires
    approvals.count >= 2
    where approver.role == "senior"
    and approver.department != requester.department
"#;
        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "RequireApprovalWhere");

        // Check that we have requires with where clause
        match &policy.requirements {
            Requirements::Requires { conditions, where_clause } => {
                assert_eq!(conditions.len(), 1);
                assert!(where_clause.is_some());
                // Where clause combines multiple conditions with AND
                let where_conds = where_clause.as_ref().unwrap();
                assert_eq!(where_conds.len(), 1);

                // Verify it's a logical AND expression
                match &where_conds[0].expr {
                    Expression::Logical { op: LogicalOp::And, operands } => {
                        assert_eq!(operands.len(), 2);
                    }
                    _ => panic!("Expected logical AND in where clause"),
                }
            }
            _ => panic!("Expected requires with where clause"),
        }
    }

    #[test]
    fn test_policy_with_denies_and_no_reason() {
        let source = r#"
policy DenyNoReason:
  "Deny without explicit reason"

  triggers when
    resource.type == "Document"

  denies when
    user.role == "guest"
"#;
        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "DenyNoReason");

        match &policy.requirements {
            Requirements::Denies { reason } => {
                assert!(reason.is_none());
            }
            _ => panic!("Expected denies clause"),
        }
    }

    #[test]
    fn test_error_missing_requirements() {
        let source = r#"
policy NoRequirements:
  "Policy without requirements"

  triggers when
    resource.type == "Document"
"#;
        let mut parser = Parser::new(source);
        let result = parser.parse_policy();
        assert!(result.is_err());
        if let Err(ParseError::InvalidPolicy(msg)) = result {
            assert!(msg.contains("Expected 'requires' or 'denies'"));
        } else {
            panic!("Expected InvalidPolicy error");
        }
    }

    #[test]
    fn test_multiple_requires_conditions_with_and() {
        let source = r#"
policy MultipleRequires:
  "Multiple require conditions"

  triggers when
    resource.type == "Document"

  requires
    user.role == "admin"
    and user.department == "IT"
    and user.clearance >= 5
"#;
        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "MultipleRequires");

        match &policy.requirements {
            Requirements::Requires { conditions, .. } => {
                // Parser combines AND conditions into a single logical expression
                assert_eq!(conditions.len(), 1);

                // Verify it's a logical AND (parser creates nested binary tree of ANDs)
                match &conditions[0].expr {
                    Expression::Logical { op: LogicalOp::And, operands } => {
                        assert!(operands.len() >= 2);
                    }
                    _ => panic!("Expected logical AND expression"),
                }
            }
            _ => panic!("Expected requires clause"),
        }
    }

    #[test]
    fn test_complex_where_clause_multiple_conditions() {
        let source = r#"
policy ComplexWhere:
  "Complex where clause with multiple conditions"

  triggers when
    resource.type == "Deployment"

  requires
    approvals.count >= 3
    where approver.role == "senior"
    and approver.department == "security"
    and approver.tenure > 2
"#;
        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "ComplexWhere");

        match &policy.requirements {
            Requirements::Requires { conditions, where_clause } => {
                assert_eq!(conditions.len(), 1);
                assert!(where_clause.is_some());
                // Where clause combines all conditions into single logical expression
                let where_conds = where_clause.as_ref().unwrap();
                assert_eq!(where_conds.len(), 1);

                // Verify it's a logical AND (nested binary tree)
                match &where_conds[0].expr {
                    Expression::Logical { op: LogicalOp::And, operands } => {
                        assert!(operands.len() >= 2);
                    }
                    _ => panic!("Expected logical AND in where clause"),
                }
            }
            _ => panic!("Expected requires with where clause"),
        }
    }

    #[test]
    fn test_parse_expression_with_newlines() {
        let source = r#"user.role == "admin"
and
user.department == "IT"
or
user.is_superuser == true"#;
        let mut parser = Parser::new(source);
        let expr = parser.parse_expression().unwrap();

        // Should parse as a logical expression
        match expr {
            Expression::Logical { op: LogicalOp::Or, operands } => {
                assert_eq!(operands.len(), 2);
            }
            _ => {}
        }
    }

    #[test]
    fn test_policy_denies_with_reason() {
        let source = r#"
policy DenyWithReason:
  "Deny with explicit reason"

  triggers when
    resource.type == "Document"
    and user.role == "guest"

  denies
    with reason "Insufficient permissions"
"#;
        let mut parser = Parser::new(source);
        let policy = parser.parse_policy().unwrap();

        assert_eq!(policy.name, "DenyWithReason");

        match &policy.requirements {
            Requirements::Denies { reason } => {
                assert!(reason.is_some());
                assert_eq!(reason.as_ref().unwrap(), "Insufficient permissions");
            }
            _ => panic!("Expected denies clause"),
        }
    }
}

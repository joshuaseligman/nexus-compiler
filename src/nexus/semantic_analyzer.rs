use log::*;
use crate::{nexus::token::{Token, TokenType, Symbols, Keywords}, util::nexus_log};

use crate::nexus::ast::{Ast};
use crate::nexus::ast_node::{AstNode, NonTerminals, AstNodeTypes};

pub struct SemanticAnalyzer {
    cur_token_index: usize,
    num_warnings: i32
}

impl SemanticAnalyzer {
    // Constructor for the parser
    pub fn new() -> Self {
        return SemanticAnalyzer {
            cur_token_index: 0,
            num_warnings: 0
        };
    }

    // Starting function to generate the AST
    pub fn generate_ast(&mut self, token_stream: &Vec<Token>) -> Ast {
        // Basic initialization
        self.cur_token_index = 0;
        let mut ast: Ast = Ast::new();

        // We start with parsing the block because that is the first
        // part with actual content
        self.parse_block(token_stream, &mut ast);

        // Return the AST
        return ast;
    }

    fn parse_block(&mut self, token_stream: &Vec<Token>, ast: &mut Ast) {
        // Log that we are parsing a block
        nexus_log::log(
            nexus_log::LogTypes::Debug,
            nexus_log::LogSources::SemanticAnalyzer,
            String::from("Parsing Block")
        );

        ast.add_node(AstNodeTypes::Branch, AstNode::NonTerminal(NonTerminals::Block));

        // Advance a token for the left brace
        self.cur_token_index += 1;

        // Parse all of the content inside of the block
        self.parse_statement_list(token_stream, ast);

        // Advance a token for the right brace
        self.cur_token_index += 1;

        // Move up to the previous level
        ast.move_up();
    }

    fn parse_statement_list(&mut self, token_stream: &Vec<Token>, ast: &mut Ast) {
        // Make sure that the statement list is not empty
        if !self.peek_and_match_next_token(token_stream, TokenType::Symbol(Symbols::RBrace)) {
            // Log that we are parsing a statement list
            nexus_log::log(
                nexus_log::LogTypes::Debug,
                nexus_log::LogSources::SemanticAnalyzer,
                String::from("Parsing StatementList")
            );
            // Parse the statement
            self.parse_statement(token_stream, ast);
            self.parse_statement_list(token_stream, ast);
        } else {
            nexus_log::log(
                nexus_log::LogTypes::Debug,
                nexus_log::LogSources::SemanticAnalyzer,
                String::from("Parsing StatementList (epsilon base case)")
            );
        }
    }

    fn parse_statement(&mut self, token_stream: &Vec<Token>, ast: &mut Ast) {
        // Log that we are parsing a statement
        nexus_log::log(
            nexus_log::LogTypes::Debug,
            nexus_log::LogSources::SemanticAnalyzer,
            String::from("Parsing Statement")
        );

        // Look ahead to the next token
        let next_token_peek: Option<Token> = self.peek_next_token(token_stream);
        if next_token_peek.is_some() {
            let next_token: Token = next_token_peek.unwrap();

            // Parse the next section in the stream based on the next token 
            match next_token.token_type {
                // Print statements
                //TokenType::Keyword(Keywords::Print) => self.parse_print_statement(token_stream, ast),

                // Assignment statements
                //TokenType::Identifier(_) => self.parse_assignment_statement(token_stream, ast),

                // VarDecl statements
                //TokenType::Keyword(Keywords::Int) | TokenType::Keyword(Keywords::String) | TokenType::Keyword(Keywords::Boolean) => self.parse_var_declaration(token_stream, ast),

                // While statements
                //TokenType::Keyword(Keywords::While) => self.parse_while_statement(token_stream, ast), 

                // If statements
                //TokenType::Keyword(Keywords::If) => self.parse_if_statement(token_stream, ast),

                // Block statements
                TokenType::Symbol(Symbols::LBrace) => self.parse_block(token_stream, ast),

                // Invalid statement starter tokens
                _ => error!("Invalid statement token [ {:?} ] at {:?}; Valid statement beginning tokens are {:?}", next_token.token_type, next_token.position, vec![TokenType::Keyword(Keywords::Print), TokenType::Identifier(String::from("a-z")), TokenType::Keyword(Keywords::Int), TokenType::Keyword(Keywords::String), TokenType::Keyword(Keywords::Boolean), TokenType::Keyword(Keywords::While), TokenType::Keyword(Keywords::If), TokenType::Symbol(Symbols::LBrace)])
            };

        }
    }

//    fn parse_print_statement(&mut self, token_stream: &Vec<Token>, ast: &mut Ast) {
//        // Log that we are parsing a print statement
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::SemanticAnalyzer,
//            String::from("Parsing PrintStatement")
//        );
//
//        // Add the PrintStatement node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::Print));
//
//        // Increment the token index by 1 for the print keyword
//        self.cur_token_index += 1;
//
//        // Increment the token index by 1 for the left paren
//        self.cur_token_index += 1;
//
//        // Parse the expression inside the print statement
//        self.parse_expression(token_stream, ast);
//        
//        // Increment the token index by 1 for the right paren
//        self.cur_token_index += 1;
//
//        // All good so we move up
//        ast.move_up();
//    }
//
//    fn parse_assignment_statement(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing a print statement
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing AssignmentStatement")
//        );
//
//        // Add the AssignmentStatement node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::AssignmentStatement));
//
//        // Assignment statements begin with an identifier
//        let id_res: Result<(), String> = self.parse_identifier(token_stream, ast);
//        if id_res.is_err() {
//            return id_res;
//        }
//
//        // Check for a =
//        let assignment_op_res: Result<(), String> = self.match_token(token_stream, TokenType::Symbol(Symbols::AssignmentOp), ast);
//        if assignment_op_res.is_err() {
//            return assignment_op_res;
//        }
//
//        // The right hand side of the statement is an expression
//        let expr_res: Result<(), String> = self.parse_expression(token_stream, ast);
//        if expr_res.is_err() {
//            return expr_res;
//        }
//
//        ast.move_up();
//        return Ok(());
//    }
//
//    fn parse_var_declaration(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String>{
//        // Log that we are parsing a variable declaration
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing VarDecl")
//        );
//
//        // Add the VarDecl node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::VarDecl));
//
//        // Make sure we have a valid type
//        let type_res: Result<(), String> = self.parse_type(token_stream, ast);
//        if type_res.is_err() {
//            return type_res;
//        }
//
//        // Then make sure there is a valid identifier
//        let id_res: Result<(), String> = self.parse_identifier(token_stream, ast);
//        if id_res.is_err() {
//            return id_res;
//        }
//
//        ast.move_up();
//        return Ok(());
//    }
//
//    fn parse_while_statement(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing a while statement
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing WhileStatement")
//        );
//
//        // Add the WhileStatementNode
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::WhileStatement));
//
//        // Make sure we have the while token
//        let while_res: Result<(), String> = self.match_token(token_stream, TokenType::Keyword(Keywords::While), ast);
//        if while_res.is_err() {
//            return while_res;
//        }
//
//        // While has a boolean expression
//        let bool_expr_res: Result<(), String> = self.parse_bool_expression(token_stream, ast);
//        if bool_expr_res.is_err() {
//            return bool_expr_res;
//        }
//
//        // The body of the loop is defined by a block
//        let block_res: Result<(), String> = self.parse_block(token_stream, ast);
//        if block_res.is_err() {
//            return block_res;
//        }
//
//        ast.move_up();
//        return Ok(());
//    }
//
//    fn parse_if_statement(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing an if statement
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing IfStatement")
//        );
//
//        // Add the IfStatement node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::IfStatement));
//
//        // Make sure we have the if token
//        let if_res: Result<(), String> = self.match_token(token_stream, TokenType::Keyword(Keywords::If), ast);
//        if if_res.is_err() {
//            return if_res;
//        }
//
//        // If has a boolean expression
//        let bool_expr_res: Result<(), String> = self.parse_bool_expression(token_stream, ast);
//        if bool_expr_res.is_err() {
//            return bool_expr_res;
//        }
//
//        // The body of the if-statement is a block
//        let block_res: Result<(), String> = self.parse_block(token_stream, ast);
//        if block_res.is_err() {
//            return block_res;
//        }
//
//        ast.move_up();
//        return Ok(());
//    }
//
//    fn parse_expression(&mut self, token_stream: &Vec<Token>, ast: &mut Ast) {
//        // Log that we are parsing an expression
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::SemanticAnalyzer,
//            String::from("Parsing Expr")
//        );
//
//        // Add the Expr node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::Expr));
//
//        // Look ahead to the next token
//        let next_token_peek: Option<Token> = self.peek_next_token(token_stream);
//        if next_token_peek.is_some() {
//            let next_token: Token = next_token_peek.unwrap();
//
//            // Assign a result object to expression_res based on the next token in the stream
//            let expression_res: Result<(), String> = match next_token.token_type {
//                // IntExpr
//                TokenType::Digit(_) => self.parse_int_expression(token_stream, ast),
//
//                // StringExpr
//                TokenType::Symbol(Symbols::Quote) => self.parse_string_expression(token_stream, ast),
//
//                // BooleanExpr
//                TokenType::Symbol(Symbols::LParen) | TokenType::Keyword(Keywords::False) | TokenType::Keyword(Keywords::True) => self.parse_bool_expression(token_stream, ast),
//
//                // Id
//                TokenType::Identifier(_) => self.parse_identifier(token_stream, ast),
//
//                _ => Err(format!("Invalid expression token [ {:?} ] at {:?}; Valid expression beginning tokens are [Digit(0-9), {:?}, {:?}, {:?}, {:?}, {:?}]", next_token.token_type, next_token.position, TokenType::Symbol(Symbols::Quote), TokenType::Symbol(Symbols::LParen), TokenType::Keyword(Keywords::False), TokenType::Keyword(Keywords::True), TokenType::Identifier(String::from("a-z")))),
//            };
//    
//            if expression_res.is_ok() {
//                ast.move_up();
//            }
//            return expression_res;
//        } else {
//            // There are no more tokens to parse
//            return Err(format!("Missing expression token at end of program; Valid expression beginning tokens are [Digit(0-9), {:?}, {:?}, {:?}, {:?}, {:?}]", TokenType::Symbol(Symbols::Quote), TokenType::Symbol(Symbols::LParen), TokenType::Keyword(Keywords::False), TokenType::Keyword(Keywords::True), TokenType::Identifier(String::from("a-z"))));
//        }
//    }
//
//
//    fn parse_int_expression(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing an integer expression
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing IntExpr")
//        );
//
//        // Add the IntExpr node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::IntExpr));
//
//        // Parse the first digit and return error if needed
//        let first_digit_res: Result<(), String> = self.parse_digit(token_stream, ast);
//        if first_digit_res.is_err() {
//            return first_digit_res;
//        }
//
//        // Check the integer operator
//        if self.peek_and_match_next_token(token_stream, TokenType::Symbol(Symbols::AdditionOp)) {     
//            let int_op_res: Result<(), String> = self.parse_int_op(token_stream, ast);
//    
//            if int_op_res.is_err() {
//                return int_op_res;
//            }
//
//            // Get the second half of the expression if there is an integer operator and return the error if needed
//            // Type check does not matter, so can parse 3 + "hello" for now and semantic analysis will catch it
//            let second_half_res: Result<(), String> = self.parse_expression(token_stream, ast);
//            if second_half_res.is_err() {
//                return second_half_res;
//            }
//        }
//
//        ast.move_up();
//        return Ok(());
//    }
//
//    fn parse_string_expression(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing a string expression
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing StringExpr")
//        );
//
//        // Add the StringExpr node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::StringExpr));
//
//        // Check for the open quote
//        let open_quote_res: Result<(), String> = self.match_token(token_stream, TokenType::Symbol(Symbols::Quote), ast);
//        if open_quote_res.is_err() {
//            return open_quote_res;
//        }
//
//        // Parse the string contents
//        let char_list_res: Result<(), String> = self.parse_char_list(token_stream, ast);
//        if char_list_res.is_err() {
//            return char_list_res;
//        }
//
//        // Check for the close quote
//        let close_quote_res: Result<(), String> = self.match_token(token_stream, TokenType::Symbol(Symbols::Quote), ast);
//        if close_quote_res.is_err() {
//            return close_quote_res;
//        } else {
//            // Check 2 tokens prior, which should be a quote if empty string
//            // No need to check for going out of bounds because both quotes will already have been consumed
//            match &token_stream[self.cur_token_index - 2].token_type {
//                TokenType::Symbol(Symbols::Quote) => {
//                    nexus_log::log(
//                        nexus_log::LogTypes::Warning,
//                        nexus_log::LogSources::Parser,
//                        format!("Empty string found starting at {:?}", token_stream[self.cur_token_index - 2].position)
//                    );
//                    self.num_warnings += 1;
//                },
//                _ => { /* Do nothing because there is not an empty string */ }
//            }
//        }
//
//        ast.move_up();
//        return Ok(());
//    }
//
//    fn parse_bool_expression(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing a boolean expression
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing BooleanExpr")
//        );
//
//        // Add BooleanExpr node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::BooleanExpr));
//
//        let next_token_peek: Option<Token> = self.peek_next_token(token_stream);
//        if next_token_peek.is_some() {
//            let next_token: Token = next_token_peek.unwrap();
//
//            let bool_expr_res: Result<(), String> = match next_token.token_type {
//                // Long boolean expressions start with LParen
//                TokenType::Symbol(Symbols::LParen) => self.long_bool_expression_helper(token_stream, ast),
//    
//                // The false and true keywords
//                TokenType::Keyword(Keywords::False) | TokenType::Keyword(Keywords::True) => self.parse_bool_val(token_stream, ast),
//    
//                // Invalid boolean expression
//                _ => Err(format!("Invalid boolean expression token [ {:?} ] at {:?}; Valid boolean expression beginning tokens are {:?}", next_token.token_type, next_token.position, vec![TokenType::Symbol(Symbols::LParen), TokenType::Keyword(Keywords::False), TokenType::Keyword(Keywords::True)]))
//            };
//    
//            if bool_expr_res.is_ok() {
//                ast.move_up();
//            }
//            return bool_expr_res;
//        } else {
//            // There are no more tokens to parse
//            return Err(format!("Missing boolean expression token at end of program; Valid boolean expression beginning tokens are {:?}", vec![TokenType::Symbol(Symbols::LParen), TokenType::Keyword(Keywords::False), TokenType::Keyword(Keywords::True)]));
//        }
//    }
//
//    fn long_bool_expression_helper(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        let lparen_res: Result<(), String> = self.match_token(token_stream, TokenType::Symbol(Symbols::LParen), ast);
//        if lparen_res.is_err() {
//            return lparen_res;
//        }
//
//        // Then move on to the left side of the expression
//        let expr1_res: Result<(), String> = self.parse_expression(token_stream, ast);
//        if expr1_res.is_err() {
//            return expr1_res;
//        }
//
//        // Next check for a boolean operator
//        let bool_op_res: Result<(), String> = self.parse_bool_op(token_stream, ast);
//        if bool_op_res.is_err() {
//            return bool_op_res;
//        }
//
//        // Next check for the other side of the expression
//        let expr2_res: Result<(), String> = self.parse_expression(token_stream, ast);
//        if expr2_res.is_err() {
//            return expr2_res;
//        }
//
//        // Lastly close it with a paren
//        let rparen_res: Result<(), String> = self.match_token(token_stream, TokenType::Symbol(Symbols::RParen), ast);
//        // Return the result regardless of error or ok
//        return rparen_res;
//    }
//
//    fn parse_identifier(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing an identifier
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing Id")
//        );
//
//        // Add the Id node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::Id));
//
//        // Match the id
//        let id_res: Result<(), String> = self.match_token(token_stream, TokenType::Identifier(String::from("a-z")), ast);
//
//        if id_res.is_ok() {
//            ast.move_up();
//        }
//        return id_res;
//    }
//
//    fn parse_char_list(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Recursion base case
//        // We have reached the end of the character list
//        if self.peek_and_match_next_token(token_stream, TokenType::Symbol(Symbols::Quote)) {
//            // Log that we are parsing a CharList
//            nexus_log::log(
//                nexus_log::LogTypes::Debug,
//                nexus_log::LogSources::Parser,
//                String::from("Parsing CharList (epsilon base case)")
//            );
//            // Do nothing here because we have reached the end of the string (epsilon case)
//            return Ok(());
//        } else {
//            // Log that we are parsing a CharList
//            nexus_log::log(
//                nexus_log::LogTypes::Debug,
//                nexus_log::LogSources::Parser,
//                String::from("Parsing CharList")
//            );
//    
//            // Add the CharList node
//            ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::CharList));
//            let char_res: Result<(), String> = self.parse_char(token_stream, ast);
//            if char_res.is_err() {
//                // Break from error
//                return char_res;
//            } else {
//                // Otherwise continue for the rest of the string
//                let char_list_res: Result<(), String> = self.parse_char_list(token_stream, ast);
//                if char_list_res.is_ok() {
//                    ast.move_up();
//                }
//                return char_list_res;
//            }
//        }
//    }
//
//    fn parse_type(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing a type
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing type")
//        );
//
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::Type));
//
//        // Try to consume the int token
//        let type_res: Result<(), String> = self.match_token_collection(token_stream, vec![TokenType::Keyword(Keywords::Int), TokenType::Keyword(Keywords::String), TokenType::Keyword(Keywords::Boolean)], ast);
//        
//        if type_res.is_ok() {
//            ast.move_up();
//        }
//
//        return type_res;
//    }
//
//    fn parse_digit(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log what we are doing
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing digit")
//        );
//
//        // Add the node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::Digit));
//
//        // Match the token with a digit
//        let digit_res: Result<(), String> = self.match_token(token_stream, TokenType::Digit(0), ast);
//        if digit_res.is_err() {
//            return digit_res;
//        } else {
//            ast.move_up();
//            return Ok(());
//        }
//    }
//
//    fn parse_char(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Check for the next character's content to have the correct output (space vs char)
//        let cur_token: Option<Token> = self.peek_next_token(token_stream);
//        if cur_token.is_some() {
//            match cur_token.unwrap().text.as_str() {
//                " " => {
//                    nexus_log::log(
//                        nexus_log::LogTypes::Debug,
//                        nexus_log::LogSources::Parser,
//                        String::from("Parsing space")
//                    );
//
//                    // Add the node
//                    ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::Space));
//                },
//                _ => {
//                    // Log that we are parsing a Char
//                    nexus_log::log(
//                        nexus_log::LogTypes::Debug,
//                        nexus_log::LogSources::Parser,
//                        String::from("Parsing char")
//                    );
//                    ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::Char));
//                }
//            }
//        }
//
//        // Make sure we have a character token here
//        let char_res: Result<(), String> = self.match_token(token_stream, TokenType::Char(String::from("a-z or space")), ast);
//
//        if char_res.is_ok() {
//            ast.move_up();
//        }
//
//        return char_res;
//    }
//
//    fn parse_bool_op(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing a boolean operator
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing boolop")
//        );
//
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::BoolOp));
//
//        // Try to consume the token
//        let bool_op_res: Result<(), String> = self.match_token_collection(token_stream, vec![TokenType::Symbol(Symbols::EqOp), TokenType::Symbol(Symbols::NeqOp)], ast);
//
//        if bool_op_res.is_ok() {
//            ast.move_up();
//        }
//        
//        return bool_op_res;
//    }
//
//    fn parse_bool_val(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing a boolean operator
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing boolval")
//        );
//
//        // Add the boolval node
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::BoolVal));
//
//        // Attempt to consume the token
//        let bool_val_res: Result<(), String> = self.match_token_collection(token_stream, vec![TokenType::Keyword(Keywords::False), TokenType::Keyword(Keywords::True)], ast);
//
//        if bool_val_res.is_ok() {
//            // Move up if appropriate to do so
//            ast.move_up();
//        }
//
//        return bool_val_res;
//    }
//
//    fn parse_int_op(&mut self, token_stream: &Vec<Token>, ast: &mut ast) -> Result<(), String> {
//        // Log that we are parsing an integer operator
//        nexus_log::log(
//            nexus_log::LogTypes::Debug,
//            nexus_log::LogSources::Parser,
//            String::from("Parsing intop")
//        );
//
//        ast.add_node(astNodeTypes::Branch, astNode::NonTerminal(NonTerminals::IntOp));
//
//        // Match the token or get the error
//        let res: Result<(), String> = self.match_token(token_stream, TokenType::Symbol(Symbols::AdditionOp), ast);
//
//        // Move up
//        if res.is_ok() {
//            ast.move_up();
//        }
//
//        return res;
//    }

    fn peek_next_token(&mut self, token_stream: &Vec<Token>) -> Option<Token> {
        // Make sure we are in-bounds
        if self.cur_token_index < token_stream.len() {
            // Clone the token and return
            return Some(token_stream[self.cur_token_index].to_owned());
        } else {
            // If there are no more tokens, then we con return None
            return None;
        }
    }

    fn peek_and_match_next_token(&mut self, token_stream: &Vec<Token>,  expected_token: TokenType) -> bool {
        let next_token_peek: Option<Token> = self.peek_next_token(token_stream);
        if next_token_peek.is_some() {
            let next_token: Token = next_token_peek.unwrap();
            match &next_token.token_type {
                TokenType::Identifier(_) => {
                    match expected_token {
                        // If next is an identifier, make sure expected is also an identifier
                        TokenType::Identifier(_) => return true,
                        _ => return false
                    }
                },
                TokenType::Keyword(actual_keyword) => {
                    match expected_token {
                        // If they are keywords, have to make sure it is the same keyword
                        TokenType::Keyword(expected_keyword) => {
                            if actual_keyword.eq(&expected_keyword) {
                                return true;
                            } else {
                                return false;
                            }
                        },
                        _ => return false
                    }
                },
                TokenType::Symbol(actual_symbol) => {
                    match expected_token {
                        // If they are symbols, have to make sure it is the same symbol
                        TokenType::Symbol(expected_symbol) => {
                            if actual_symbol.eq(&expected_symbol) {
                                return true;
                            } else {
                                return false;
                            }
                        },
                        _ => return false
                    }
                },
                TokenType::Char(_) => {
                    match expected_token {
                        // Check to make sure both are characters
                        TokenType::Char(_) => return true,
                        _ => return false
                    }
                },
                TokenType::Digit(_) => {
                    match expected_token {
                        // Make sure both are digits
                        TokenType::Digit(_) => return true,
                        _ => return false
                    }
                },
                _ => return false
            }
        } else {
            return false;
        }
    }
}
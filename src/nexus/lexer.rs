use crate::{nexus::token::Token, util::nexus_log};
use log::{debug, info};
use regex::{Regex, RegexSet};

pub fn lex(source_code: String) {//-> Vec<Token> {
    // This represents all possible terminal characters for which to mark the end of the current search
    let terminal_chars = Regex::new(r"^\s$").unwrap();

    let mut line_number: usize = 1;
    let mut col_number: usize = 1;

    let mut cur_start: usize = 0;
    let mut best_end: usize = 0;

    let mut cur_token: Token = Token::Unrecognized(String::from(""));

    let mut trailer: usize = 0;

    // Iterate through the end of the string
    while trailer < source_code.len() {
        debug!("{}", format!("trailer: {}, cur_start: {}, best_end: {}", trailer, cur_start, best_end));

        // Get the current character
        let cur_char: &str = &source_code[trailer..trailer + 1];
        info!("{:?}", cur_char.as_bytes());

        // Check if it is a terminal character
        if !terminal_chars.is_match(cur_char) {
            // Need to check the substring from cur_start
            // Get the current substring in question
            let cur_sub: &str = &source_code[cur_start..trailer + 1];
            
            if upgrade_token(cur_sub, &mut cur_token) {
                best_end = trailer.to_owned();
            }
        } else {
            nexus_log::log(String::from("LEXER"), format!("Found {:?} at ({}, {})", cur_token, line_number, col_number));

            if cur_char.eq("\n") {
                // Increment line number and reset column number
                line_number += 1;
                col_number = 1;
            } else {
                // Column number gets increased by the length of the token + 2
                // (1 for the space and 1 for best_end - cur_start being 1 less than the length of the token)
                col_number += best_end - cur_start + 2;
            }

            // If the best ending is not next to the terminating character, then move the trailer back to where it should start back up again
            if best_end < trailer - 1 {
                trailer = best_end.to_owned();
            }
            cur_start = trailer + 1;
            best_end = trailer + 1;

            debug!("{}", format!("NEW: trailer: {}, cur_start: {}, best_end: {}", trailer, cur_start, best_end));

            cur_token = Token::Unrecognized(String::from(""));
        }

        trailer += 1;
    }
}

fn upgrade_token(substr: &str, best_token_type: &mut Token) -> bool {
    // Create the keywords
    let keywords: RegexSet = RegexSet::new(&[
        r"if",
        r"while",
        r"print",
        r"string",
        r"int",
        r"boolean"
    ]).unwrap();

    let identifiers = Regex::new(r"^[a-z]$").unwrap();

    let symbols = Regex::new(r"").unwrap();
    
    match best_token_type {
        // Keyword is the best and they are all mutually exclusive, so no need to check
        Token::Keyword(_) => return false,
        _ => {
            if keywords.is_match(substr) {
                *best_token_type = Token::Keyword(String::from(substr));
                return true;
            } else if identifiers.is_match(substr) {
                *best_token_type = Token::Identifier(String::from(substr));
                return true;
            } else {
                return false;
            }
        }
    }       
}
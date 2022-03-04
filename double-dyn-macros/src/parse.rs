
#![allow(dead_code)]

/// Some parts of this file are based on code from seq-macro, by dtolnay.  https://github.com/dtolnay/seq-macro
/// Included under the terms of the MIT License
/// 

use proc_macro2::token_stream::IntoIter as TokenIter;
use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};
use std::borrow::Borrow;
use std::cmp;
use std::fmt::Display;
use std::iter::FromIterator;

#[derive(Copy, Clone, PartialEq)]
pub enum Kind {
    Int,
    Byte,
    Char,
}

#[derive(Copy, Clone, PartialEq)]
pub enum Radix {
    Binary,
    Octal,
    Decimal,
    LowerHex,
    UpperHex,
}

pub struct Range {
    begin: u64,
    end: u64,
    inclusive: bool,
    kind: Kind,
    suffix: String,
    width: usize,
    radix: Radix,
}

pub struct Value {
    int: u64,
    kind: Kind,
    suffix: String,
    width: usize,
    radix: Radix,
    span: Span,
}

#[derive(Debug)]
pub(crate) struct SyntaxError {
    pub message: String,
    pub span: Span,
}

impl SyntaxError {
    pub(crate) fn into_compile_error(self) -> TokenStream {
        // compile_error! { $message }
        TokenStream::from_iter(vec![
            TokenTree::Ident(Ident::new("compile_error", self.span)),
            TokenTree::Punct({
                let mut punct = Punct::new('!', Spacing::Alone);
                punct.set_span(self.span);
                punct
            }),
            TokenTree::Group({
                let mut group = Group::new(Delimiter::Brace, {
                    TokenStream::from_iter(vec![TokenTree::Literal({
                        let mut string = Literal::string(&self.message);
                        string.set_span(self.span);
                        string
                    })])
                });
                group.set_span(self.span);
                group
            }),
        ])
    }
}

pub(crate) fn next_token(iter: &mut TokenIter, err_span : Span) -> Result<TokenTree, SyntaxError> {
    iter.next().ok_or_else(|| SyntaxError {
        message: "unexpected end of input".to_owned(),
        span: err_span,
    })
}

pub(crate) fn syntax<T: Borrow<TokenTree>, M: Display>(token: T, message: M) -> SyntaxError {
    SyntaxError {
        message: message.to_string(),
        span: token.borrow().span(),
    }
}

pub(crate) fn if_ident(iter: &TokenIter) -> Result<bool, SyntaxError> {
    match iter.clone().next() {
        Some(TokenTree::Ident(_)) => Ok(true),
        _ => Ok(false)
    }
}

//Searches a TokenIter for a contiguous sequence of tokens
pub(crate) fn if_contains_tokens(iter: &TokenIter, sequence: TokenIter) -> Result<bool, SyntaxError> {
    let mut new_iter = iter.clone();
    let mut seq_iter = sequence.clone();
    let mut seq_next = seq_iter.next();
    while let Some(token) = new_iter.next() {
        match seq_next {
            Some(seq_next_tok) => {
                if token.to_string() == seq_next_tok.to_string() {
                    seq_next = seq_iter.next();
                } else {
                    //We've found an interruption in the sequence, so start over
                    seq_iter = sequence.clone();
                    seq_next = seq_iter.next();
                }
            },
            None => panic!() //If we get here, we have a zero-length sequence, so bad args
        }
        if seq_next.is_none() {
            return Ok(true); //We've found the last token in the sequence so we're done
        }
    }
    Ok(false)
}

//Searches a TokenIter for a contiguous sequence of tokens specified by their string values
pub(crate) fn if_contains_sequence(iter: &TokenIter, sequence: &[&str]) -> Result<bool, SyntaxError> {
    let mut new_iter = iter.clone();
    let mut sequence_idx = 0;
    while let Some(token) = new_iter.next() {
        match token {
            TokenTree::Ident(ident) => {
                if sequence[sequence_idx] == ident.to_string() { //Panic is ok because it means we got bad args
                    sequence_idx += 1;
                } else {
                    //We've found an interruption in the sequence, so start over
                    sequence_idx = 0;
                }
            }
            TokenTree::Punct(punct) => {
                if sequence[sequence_idx] == punct.to_string() { //Panic is ok because it means we got bad args
                    sequence_idx += 1;
                } else {
                    //We've found an interruption in the sequence, so start over
                    sequence_idx = 0;
                }
            },
            _ => {
                //We've found an interruption in the sequence, so start over
                sequence_idx = 0;
            }
        }
        if sequence_idx == sequence.len() {
            return Ok(true); //We've found the last token in the sequence so we're done
        }
    }
    Ok(false)
}

pub(crate) fn require_ident(iter: &mut TokenIter, err_span : Span) -> Result<Ident, SyntaxError> {
    match next_token(iter, err_span)? {
        TokenTree::Ident(ident) => Ok(ident),
        other => Err(syntax(other, "expected ident")),
    }
}

pub(crate) fn if_keyword(iter: &mut TokenIter, keyword: &str) -> Result<bool, SyntaxError> {
    match iter.clone().next() {
        Some(TokenTree::Ident(ident)) => {
            if ident.to_string() == keyword {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}

pub(crate) fn require_keyword(iter: &mut TokenIter, keyword: &str, err_span : Span) -> Result<(), SyntaxError> {
    let token = next_token(iter, err_span)?;
    if let TokenTree::Ident(ident) = &token {
        if ident.to_string() == keyword {
            return Ok(());
        }
    }
    Err(syntax(token, format!("expected `{}`", keyword)))
}

pub(crate) fn require_value(iter: &mut TokenIter, err_span : Span) -> Result<Value, SyntaxError> {
    let mut token = next_token(iter, err_span)?;

    loop {
        match token {
            TokenTree::Group(group) => {
                let delimiter = group.delimiter();
                let mut stream = group.stream().into_iter();
                token = TokenTree::Group(group);
                if delimiter != Delimiter::None {
                    break;
                }
                let first = match stream.next() {
                    Some(first) => first,
                    None => break,
                };
                match stream.next() {
                    Some(_) => break,
                    None => token = first,
                }
            }
            TokenTree::Literal(lit) => {
                return parse_literal(&lit).ok_or_else(|| {
                    let token = TokenTree::Literal(lit);
                    syntax(token, "expected unsuffixed integer literal")
                });
            }
            _ => break,
        }
    }

    Err(syntax(token, "expected integer"))
}

pub(crate) fn if_punct(iter: &TokenIter, ch: char) -> Result<bool, SyntaxError> {
    match iter.clone().next() {
        Some(TokenTree::Punct(punct)) => {
            if punct.as_char() == ch {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}

pub(crate) fn require_punct(iter: &mut TokenIter, ch: char, err_span : Span) -> Result<(), SyntaxError> {
    let token = next_token(iter, err_span)?;
    if let TokenTree::Punct(punct) = &token {
        if punct.as_char() == ch {
            return Ok(());
        }
    }
    Err(syntax(token, format!("expected `{}`", ch)))
}

pub(crate) fn if_group(iter: &mut TokenIter, delimiter: Delimiter) -> Result<bool, SyntaxError> {
    match iter.clone().next() {
        Some(TokenTree::Group(group)) => {
            if group.delimiter() == delimiter {
                Ok(true)
            } else {
                Ok(false)
            }
        }
        _ => Ok(false),
    }
}

pub(crate) fn require_group(iter: &mut TokenIter, delimiter: Delimiter, err_span: Span, err_msg: &str) -> Result<Group, SyntaxError> {
    
    if if_end(iter)? {
        return Err(SyntaxError {
            message: err_msg.to_owned(),
            span: err_span,
        });
    }

    let token = next_token(iter, err_span)?;
    match token {
        TokenTree::Group(group) => {
            if group.delimiter() == delimiter {
                Ok(group)
            } else {
                Err(syntax(TokenTree::Group(group), err_msg))
            }    
        },
        other => Err(syntax(other, err_msg))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AngleGroup {
    pub open_bracket: Punct,
    pub close_bracket: Punct,
    pub interior_tokens: TokenStream,
}

pub(crate) fn require_angle_group(iter: &mut TokenIter, err_span: Span, err_msg: &str) -> Result<AngleGroup, SyntaxError> {
    
    let mut bracket_balance: usize = 0;
    let mut interior_tokens = TokenStream::new();
    let mut open_bracket = Punct::new('<', Spacing::Alone); //This will never be used but the compiler doesn't know that

    loop {
        let token = next_token(iter, err_span)?;
        match token {
            TokenTree::Punct(punct) => {
                match punct.as_char() {
                    '<' => {
                        if bracket_balance == 0 {
                            open_bracket = punct;
                            bracket_balance = 1;
                        } else {
                            interior_tokens.extend([TokenTree::Punct(punct)]);
                            bracket_balance += 1;
                        }
                    },
                    '>' => {
                        if bracket_balance == 1 {
                            let angle_group = AngleGroup {
                                open_bracket,
                                close_bracket: punct,
                                interior_tokens
                            };
                            return Ok(angle_group);
                        } else {
                            interior_tokens.extend([TokenTree::Punct(punct)]);
                            bracket_balance -= 1;
                        }
                    },
                    _ => {
                        interior_tokens.extend([TokenTree::Punct(punct)]);
                    }
                }
            }
            other => {
                if bracket_balance == 0 {
                    return Err(syntax(other, err_msg));
                } else {
                    interior_tokens.extend([other]);
                }
            }
        }        
    }
}

pub(crate) fn require_end(iter: &mut TokenIter) -> Result<(), SyntaxError> {
    match iter.next() {
        Some(token) => Err(syntax(token, "unexpected token")),
        None => Ok(()),
    }
}

pub(crate) fn if_end(iter: &TokenIter) -> Result<bool, SyntaxError> {
    match iter.clone().next() {
        Some(_) => Ok(false),
        None => Ok(true)
    }
}

//Iterates over all tokens in a TokenIter, descending into nested TokenTree:Group tokens in a depth-first manner
pub(crate) fn recursive_scan<F : FnMut(TokenTree, &mut TokenStream) -> Result<(), String>>(iter: &mut TokenIter, func: &mut F) -> Result<TokenStream, SyntaxError> {
    let mut new_stream = TokenStream::new();
    while let Some(token) = iter.next() {
        match token {
            TokenTree::Group(group) => {
                let mut inner_iter = group.stream().into_iter();
                let inner_stream = recursive_scan(&mut inner_iter, func)?;
                let new_group = TokenTree::Group(Group::new(group.delimiter(), inner_stream));
                if let Err(err_str) = func(new_group.clone(), &mut new_stream) {
                    return Err(syntax(new_group, err_str));
                }
            },
            token => {
                if let Err(err_str) = func(token.clone(), &mut new_stream) {
                    return Err(syntax(token, err_str));
                }
            }
        }
    }
    Ok(new_stream)
}

pub(crate) fn validate_range(
    begin: Value,
    end: Value,
    inclusive: bool,
) -> Result<Range, SyntaxError> {
    let kind = if begin.kind == end.kind {
        begin.kind
    } else {
        let expected = match begin.kind {
            Kind::Int => "integer",
            Kind::Byte => "byte",
            Kind::Char => "character",
        };
        return Err(SyntaxError {
            message: format!("expected {} literal", expected),
            span: end.span,
        });
    };

    let suffix = if begin.suffix.is_empty() {
        end.suffix
    } else if end.suffix.is_empty() || begin.suffix == end.suffix {
        begin.suffix
    } else {
        return Err(SyntaxError {
            message: format!("expected suffix `{}`", begin.suffix),
            span: end.span,
        });
    };

    let radix = if begin.radix == end.radix {
        begin.radix
    } else if begin.radix == Radix::LowerHex && end.radix == Radix::UpperHex
        || begin.radix == Radix::UpperHex && end.radix == Radix::LowerHex
    {
        Radix::UpperHex
    } else {
        let expected = match begin.radix {
            Radix::Binary => "binary",
            Radix::Octal => "octal",
            Radix::Decimal => "base 10",
            Radix::LowerHex | Radix::UpperHex => "hexadecimal",
        };
        return Err(SyntaxError {
            message: format!("expected {} literal", expected),
            span: end.span,
        });
    };

    Ok(Range {
        begin: begin.int,
        end: end.int,
        inclusive,
        kind,
        suffix,
        width: cmp::min(begin.width, end.width),
        radix,
    })
}

fn parse_literal(lit: &Literal) -> Option<Value> {
    let span = lit.span();
    let repr = lit.to_string();
    assert!(!repr.starts_with('_'));

    if repr.starts_with("b'") && repr.ends_with('\'') && repr.len() == 4 {
        return Some(Value {
            int: repr.as_bytes()[2] as u64,
            kind: Kind::Byte,
            suffix: String::new(),
            width: 0,
            radix: Radix::Decimal,
            span,
        });
    }

    if repr.starts_with('\'') && repr.ends_with('\'') && repr.chars().count() == 3 {
        return Some(Value {
            int: repr[1..].chars().next().unwrap() as u64,
            kind: Kind::Char,
            suffix: String::new(),
            width: 0,
            radix: Radix::Decimal,
            span,
        });
    }

    let (mut radix, radix_n) = if repr.starts_with("0b") {
        (Radix::Binary, 2)
    } else if repr.starts_with("0o") {
        (Radix::Octal, 8)
    } else if repr.starts_with("0x") {
        (Radix::LowerHex, 16)
    } else if repr.starts_with("0X") {
        (Radix::UpperHex, 16)
    } else {
        (Radix::Decimal, 10)
    };

    let mut iter = repr.char_indices();
    let mut digits = String::new();
    let mut suffix = String::new();

    if radix != Radix::Decimal {
        let _ = iter.nth(1);
    }

    for (i, ch) in iter {
        match ch {
            '_' => continue,
            '0'..='9' => digits.push(ch),
            'A'..='F' if radix == Radix::LowerHex => {
                digits.push(ch);
                radix = Radix::UpperHex;
            }
            'a'..='f' | 'A'..='F' if radix_n == 16 => digits.push(ch),
            '.' => return None,
            _ => {
                if digits.is_empty() {
                    return None;
                }
                suffix = repr;
                suffix.replace_range(..i, "");
                break;
            }
        }
    }

    let int = u64::from_str_radix(&digits, radix_n).ok()?;
    let kind = Kind::Int;
    let width = digits.len();
    Some(Value {
        int,
        kind,
        suffix,
        width,
        radix,
        span,
    })
}

//Parses a function signature
//
//Positive examples:
// fn min_max(val: i32, min: &i32, max: &i32);
// pub fn min_max(val: i32, min: &i32, max: &i32) -> Result<i32, String>;
// pub(crate) fn min_max(val: i32, min: &i32, max: &i32) -> Result<i32, String>;
// fn min_max<A, B>(val: i32, min: &A, max: &B) -> Result<A, String>;
// fn min_max<A:From<i32>, B>(val: i32, min: &A, max: &B) -> Result<A, String>;
// fn min_max(val: i32, min: &i32, max: &i32);
// fn min_max(i32, &i32, &i32);
// fn min_max();
// fn min_max<A>() -> Result<A, String>;
//
//This function should also succeed with special markup tokens, as in:
// fn min_max(val: i32, min: &dyn #A, max: &dyn #B) -> Result<i32, String>;
//
#[derive(Clone, Debug)]
pub(crate) struct FnSignature {
    pub pub_qualifiers : TokenStream,
    pub fn_name : Ident,
    pub generics : TokenStream,
    pub args : Vec<FnArg>,
    pub result: TokenStream,
}

pub(crate) fn require_fn_signature(iter: &mut TokenIter, expect_semicolon: bool, err_span : Span) -> Result<FnSignature, SyntaxError> {

    //First see if we have any visibility qualifiers, i.e. pub, pub(crate), etc.
    let mut pub_qualifiers = TokenStream::new();
    if if_keyword(iter, "pub")? {
        require_keyword(iter, "pub", err_span.clone())?;
        pub_qualifiers.extend([TokenTree::Ident(Ident::new("pub", err_span.clone()))]);

        //See if we have any additional qualifiers in parentheses, e.g. "(crate)"
        if if_group(iter, Delimiter::Parenthesis)? {
            let group = require_group(iter, Delimiter::Parenthesis, err_span.clone(), "expected visibility qualifier, e.g. \"(crate)\"")?;
            pub_qualifiers.extend([TokenTree::Group(group)]);
        }
    }

    //The fn keyword
    require_keyword(iter, "fn", err_span.clone())?;

    //Next parse the function name
    let fn_name = require_ident(iter, err_span.clone())?;

    //Next parse the generics, if we have any
    let generics = if if_punct(iter, '<')? {
        let angle_group = require_angle_group(iter, err_span.clone(), "expecting angle brackets")?;
        angle_group.interior_tokens
    } else {
        TokenStream::new()
    };

    //Now move on to parsing the args
    let mut args = vec![];
    let args_group = require_group(iter, Delimiter::Parenthesis, err_span.clone(), "expected function args")?;
    let mut args_group_iter = args_group.stream().into_iter();
    while args_group_iter.clone().next().is_some() {
        let next_arg = require_fn_arg(&mut args_group_iter, err_span.clone())?;
        args.push(next_arg);
    }

    //Now parse the function's return type, if there is one
    let result = if if_punct(iter, '-')? {
        require_punct(iter, '-', err_span.clone())?;
        require_punct(iter, '>', err_span.clone())?;
        require_type(iter, err_span.clone())?
    } else {
        TokenStream::new()
    };

    //If the expect_semicolon arg was passed then the last token must be a semicolon (';')
    if expect_semicolon {
        require_punct(iter, ';', err_span.clone())?;
    }
    
    let new_sig = FnSignature {
        pub_qualifiers,
        fn_name,
        generics,
        args,
        result,
    };

    Ok(new_sig)
}

#[derive(Clone, Debug)]
pub(crate) struct FnArg {
    pub arg_name: Option<Ident>,
    pub arg_type: TokenStream
}

//Parses one arg, with or without a name
//
//Positive Examples:
// i32
// val: i32
// a: &dyn PrimInt
// &i32
// &Vec<&i32>
// Box<dyn PrimInt>
// HashMap<String, Box<dyn PrimInt>>
//
//Negative Examples:
// NULL (no tokens)
// val:
// HashMap<String
//
pub(crate) fn require_fn_arg(iter: &mut TokenIter, err_span : Span) -> Result<FnArg, SyntaxError> {

    //See if we have "ident:", because that means we have a name.
    let arg_name = if if_ident(iter)? {

        let mut temp_iter = iter.clone();
        let arg_name_ident = require_ident(&mut temp_iter, err_span.clone())?;
        if if_punct(&mut temp_iter, ':')? {
            //We already have the arg_name, so just pop two tokens off the iter
            iter.next();
            iter.next();
            Some(arg_name_ident)
        } else {
            None
        }
    } else {
        None
    };

    //Interpret all the remaining tokens as the arg_type until we get to a ',' or the end
    let arg_type = require_type(iter, err_span.clone())?;

    //If we're in a comma-separated list, then eat the trailing comma
    if if_punct(iter, ',')? {
        require_punct(iter, ',', err_span.clone())?;
    }

    //That should be all for this arg
    let new_arg = FnArg{
        arg_name,
        arg_type
    };

    Ok(new_arg)
}

pub(crate) fn require_type(iter: &mut TokenIter, err_span : Span) -> Result<TokenStream, SyntaxError> {

    let mut new_err_span = err_span;
    let mut type_tokens = TokenStream::new();
    let mut found_arg_type_name = false; //A type needs to have at least one identifier
    while let Some(token) = iter.clone().next() {
        match token {
            TokenTree::Ident(_) => {
                let popped_ident = require_ident(iter, err_span.clone())?;
                type_tokens.extend([TokenTree::Ident(popped_ident)]);
                found_arg_type_name = true;
            },
            TokenTree::Punct(punct) => {
                match punct.as_char() {
                    '<' => {
                        let angle_group = require_angle_group(iter, err_span.clone(), "")?;
                        type_tokens.extend([TokenTree::Punct(angle_group.open_bracket)]);
                        type_tokens.extend(angle_group.interior_tokens);
                        type_tokens.extend([TokenTree::Punct(angle_group.close_bracket)]);        
                    },
                    ',' | ';' => { //A comma or a semicolon signals the end of the type
                        new_err_span = punct.span();
                        break;
                    },
                    _ => {
                        let punct_token = iter.next().unwrap();
                        type_tokens.extend([punct_token]);
                    }
                }
            },
            TokenTree::Group(group) => {
                //A group also signals the end of a type, like you find in a function result type followed by
                // open brackets for the function implementation
                new_err_span = group.span();
                break;
            },
            other => {
                //Otherwise, we have an error
                return Err(syntax(other, "unexpected tokens"));
            }
        }
    }

    //If we didn't find at least one identifier token then the type is invalid
    if !found_arg_type_name {
        return Err(SyntaxError {
            message: format!("expected type identifier"),
            span: new_err_span,
        });
    }

    Ok(type_tokens)
}

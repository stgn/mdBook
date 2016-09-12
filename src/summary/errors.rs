use std::convert::From;
use std::error::Error;
use std::io::{self, ErrorKind};

#[derive(Debug, Clone)]
pub struct ParseError {
    pub desc: String,
    pub byte: usize,
}

impl ParseError {
    pub fn new<T>(desc: T, b: usize) -> Self where T: Into<String> {
        ParseError {
            desc: desc.into(),
            byte: b,
        }
    }
}

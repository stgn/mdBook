use std::path::Path;
use std::fs::File;
use std::error::Error;
use std::io::{Read, ErrorKind};
use std::iter::{Peekable};
use std::str::{self, Lines};

use super::{Link, ParseError};

// The code for this parser is heavily inspired by toml-rs: https://github.com/alexcrichton/toml-rs
// which is licensed under MIT / Apache

#[derive(Clone, Debug)]
pub struct Summary<'a> {
    input: &'a str,
    cur: str::CharIndices<'a>,

    chapters: Chapters,

    pub errors: Vec<ParseError>,
}

#[derive(Clone, Debug)]
pub struct Chapters {
    pub prefaces: Vec<Link>,
    pub chapters: Vec<Link>,
    pub appendices: Vec<Link>,
}


impl<'a> Summary<'a> {
    pub fn new(s: &'a str) -> Self {
        Summary {
            input: s,
            cur: s.char_indices(),
            chapters: Chapters::new(),
            errors: Vec::new(),
        }
    }


    /// Converts a byte offset from an error message to a (line, column) pair
    ///
    /// All indexes are 0-based.
    pub fn to_linecol(&self, offset: usize) -> (usize, usize) {
        let mut cur = 0;
        for (i, line) in self.input.lines().enumerate() {
            if cur + line.len() + 1 > offset {
                return (i, offset - cur)
            }
            cur += line.len() + 1;
        }
        (self.input.lines().count(), 0)
    }

    fn next_pos(&self) -> usize {
        self.cur.clone().next().map(|p| p.0).unwrap_or(self.input.len())
    }

    // Returns true and consumes the next character if it matches `ch`,
    // otherwise do nothing and return false
    fn eat(&mut self, ch: char) -> bool {
        match self.peek(0) {
            Some((_, c)) if c == ch => { self.cur.next(); true }
            Some(_) | None => false,
        }
    }

    fn eat_until_eol(&mut self) {
        for (_, ch) in self.cur.by_ref() {
            if ch == '\n' { break }
        }
    }

    // Peeks ahead `n` characters
    fn peek(&self, n: usize) -> Option<(usize, char)> {
        self.cur.clone().skip(n).next()
    }

    fn expect(&mut self, ch: char) -> bool {
        if self.eat(ch) { return true }

        let mut it = self.cur.clone();
        let byte = it.next().map(|p| p.0).unwrap_or(self.input.len());

        self.errors.push(ParseError::new(
            match self.cur.clone().next() {
                Some((_, c)) => format!("expected `{}`, but found `{}`", ch, c),
                None => format!("expected `{}`, but found eof", ch)
            },
            byte
        ));
        false
    }

    // Consumes whitespace ('\t' and ' ') until another character (or EOF) is
    // reached. Returns if any whitespace was consumed
    fn whitespace(&mut self) -> bool {
        let mut ret = false;
        loop {
            match self.peek(0) {
                Some((_, '\t')) |
                Some((_, ' ')) => { self.cur.next(); ret = true; }
                _ => break,
            }
        }
        ret
    }

    // Consumes a newline if one is next
    fn newline(&mut self) -> bool {
        match self.peek(0) {
            Some((_, '\n')) => { self.cur.next(); true }
            Some((_, '\r')) if self.peek(1).map(|c| c.1) == Some('\n') => {
                self.cur.next(); self.cur.next(); true
            }
            _ => false
        }
    }

    // Match EOF
    fn eof(&self) -> bool {
        self.peek(0).is_none()
    }

    /// Run the parser
    pub fn parse(&mut self) -> Option<Chapters> {
        self.parse_header();
        self.parse_prefaces();

        if self.errors.is_empty() { Some(self.chapters.clone()) }
        else { None }
    }

    // Skips the header, if the line begins with '#' it will consume all characters until the end of the line
    fn parse_header(&mut self) {
        if self.eat('#') {
            self.eat_until_eol()
        }
    }

    // Parse a markdown link `[title](destination)` return Some(Link) on success and None on failure
    fn parse_link(&mut self) -> Option<Link> {
        self.eat('[');

        let mut title = String::new();
        let mut balance = 1;

        while self.peek(0).is_some() {
            if self.newline() { return None }

            match self.peek(0).expect("Can't be None") {
                (_, '[')                => balance += 1,
                (_, ']') if balance > 1 => balance -= 1,
                (_, c @ '\\')           => { title.push(c); self.cur.next(); },
                (_, ']')                => { self.cur.next(); break }
                (_, _)                  => {},
            }
            title.push(self.cur.next().expect("Can't be None").1)
        }

        if !self.expect('(') { return None }

        let mut dest = String::new();
        let mut balance = 1;

        while self.peek(0).is_some() {

            if self.newline() { return None }

            match self.peek(0).expect("Can't be None") {
                (_, '(')                => balance += 1,
                (_, ')') if balance > 1 => balance -= 1,
                (_, c @ '\\')           => { dest.push(c); self.cur.next(); },
                (_, ')')                => { self.cur.next(); break }
                (_, _)                  => {},
            }
            dest.push(self.cur.next().expect("Can't be None").1)
        }

        Some(Link::new(title, dest))
    }

    fn parse_prefaces(&mut self) {
        while self.peek(0).is_some() {
            if self.newline() { continue }

            match self.peek(0).expect("Can't be None") {
                (_, '[') => {
                    if let Some(p) = self.parse_link() {
                        self.chapters.prefaces.push(p);
                    } else {
                        self.eat_until_eol();
                    }
                },
                (_, '-') => return,
                (i, c) => {
                    self.errors.push(ParseError::new(format!("Unexpected character '{}'", c), i));
                    self.eat_until_eol();
                },
            }   // FIXME: New lines containing spaces generate an error
        }
    }

    pub fn contains_errors(&self) -> bool {
        self.errors.len() > 0
    }
}

impl Chapters {
    pub fn new() -> Self {
        Chapters {
            prefaces: Vec::new(),
            chapters: Vec::new(),
            appendices: Vec::new(),
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preface_chapters() {
        let case = r#"# SUMMARY

[Preface 1](destination)
[Preface [2]](destination)
[Preface \[3](destination)
        "#;

        let mut s = Summary::new(case);
        if let Some(ch) = s.parse() {
            if ch.prefaces.len() != 3 { panic!("There should be 3 preface chapters: {:?}", ch.prefaces) }
        }

        if s.contains_errors() {
            panic!("The parser should not contains any errors: {:?}", s.errors)
        }


    }
}

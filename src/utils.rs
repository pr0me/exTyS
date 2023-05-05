#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::slice_structs::ObjSlice;
use memchr::memmem;

pub struct Parser<'a> {
    pub finder_eq: memmem::Finder<'a>,
    pub finder_newline: memmem::Finder<'a>,
    pub finder_pipe: memmem::Finder<'a>,
}

impl Parser<'_> {
    pub fn new() -> Self {
        Parser {
            finder_eq: memmem::Finder::new("="),
            finder_newline: memmem::Finder::new("\n"),
            finder_pipe: memmem::Finder::new("="),
        }
    }
}

pub fn persist_to_disk(slices: Vec<ObjSlice>) {}

#[inline(always)]
pub fn filter_method_name(parser: &Parser, name: &str) -> Option<String> {
    let transformed_name: &str;

    if parser.finder_eq.find(name.as_bytes()).is_some()
        || parser.finder_newline.find(name.as_bytes()).is_some()
    {
        None
    } else {
        // println!("{} -> {}", name, transformed_name);
        Some(name.to_string())
    }
}

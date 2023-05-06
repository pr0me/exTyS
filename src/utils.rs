#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::slice_structs::ObjSlice;
use memchr::memmem;

pub struct Parser<'a> {
    pub finder_eq: memmem::Finder<'a>,
    pub finder_newline: memmem::Finder<'a>,
    pub finder_pipe: memmem::Finder<'a>,
    pub finder_curly_bracket: memmem::Finder<'a>,
    pub finder_square_bracket: memmem::Finder<'a>,
    pub finder_as: memmem::Finder<'a>,
    pub finder_chain: memmem::Finder<'a>,
    pub finder_recv: memmem::Finder<'a>,
    pub finder_recv_q: memmem::Finder<'a>,
}

impl Parser<'_> {
    pub fn new() -> Self {
        Parser {
            finder_eq: memmem::Finder::new("="),
            finder_newline: memmem::Finder::new("\n"),
            finder_pipe: memmem::Finder::new("|"),
            finder_curly_bracket: memmem::Finder::new("{"),
            finder_square_bracket: memmem::Finder::new("["),
            finder_as: memmem::Finder::new(" as "),
            finder_chain: memmem::Finder::new("chain(["),
            finder_recv: memmem::Finder::new(")."),
            finder_recv_q: memmem::Finder::new(")?."),
        }
    }
}

pub fn persist_to_disk(slices: Vec<ObjSlice>) {}

#[inline(always)]
pub fn clean_method_name(parser: &Parser, mut name: &str) -> Option<String> {
    if name.is_empty()
        || parser.finder_eq.find(name.as_bytes()).is_some()
        || parser.finder_newline.find(name.as_bytes()).is_some()
        || parser.finder_pipe.find(name.as_bytes()).is_some()
        || parser.finder_curly_bracket.find(name.as_bytes()).is_some()
        || parser.finder_square_bracket.find(name.as_bytes()).is_some()
        || parser.finder_chain.find(name.as_bytes()).is_some()
    {
        None
    } else {
        let orig_name = name.to_owned();

        if name.starts_with("<operators>") {
            name = &name[12..];
        } else {
            // remove unnecessarily complex receivers
            if name.starts_with("(") {
                match parser.finder_recv_q.find(name.as_bytes()) {
                    Some(i) => name = &name[i + 3..],
                    None => match parser.finder_recv.find(name.as_bytes()) {
                        Some(i) => name = &name[i + 2..],
                        None => {}
                    },
                }
            }

            // limit the total length
            if name.len() > 48 {
                // remove type assertions
                match parser.finder_as.find(name.as_bytes()) {
                    Some(i) => {
                        name = &name[..i];
                        match memmem::find(name.as_bytes(), "(".as_bytes()) {
                            Some(i) => name = &name[..i],
                            None => {}
                        }
                    }
                    None => {}
                }

                if name.len() > 48 {
                    // remove arguments
                    if name.ends_with(")") {
                        match memmem::find(name.as_bytes(), "(".as_bytes()) {
                            Some(i) => name = &name[..i],
                            None => {}
                        }
                    }
                }
            }
        }

        // if name.ne(&orig_name) {
        //     println!("{} -> {}", orig_name, name);
        // }
        Some(name.to_string())
    }
}

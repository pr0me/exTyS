#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::slice_structs::ObjSlice;
use memchr::memmem;

pub struct Parser<'a> {
    pub finder_eq: memmem::Finder<'a>,
    pub finder_newline: memmem::Finder<'a>,
    pub finder_pipe: memmem::Finder<'a>,
    pub finder_slash: memmem::Finder<'a>,
    pub finder_curly_bracket: memmem::Finder<'a>,
    pub finder_square_bracket: memmem::Finder<'a>,
    pub finder_as: memmem::Finder<'a>,
    pub finder_chain: memmem::Finder<'a>,
    pub finder_recv: memmem::Finder<'a>,
    pub finder_recv_q: memmem::Finder<'a>,
    pub finder_union: memmem::Finder<'a>,
    pub finder_import: memmem::Finder<'a>,
    pub finder_angle_bracket_o: memmem::Finder<'a>,
    pub finder_angle_bracket_c: memmem::Finder<'a>,
}

impl Parser<'_> {
    pub fn new() -> Self {
        Parser {
            finder_eq: memmem::Finder::new("="),
            finder_newline: memmem::Finder::new("\n"),
            finder_pipe: memmem::Finder::new("|"),
            finder_slash: memmem::Finder::new("/"),
            finder_curly_bracket: memmem::Finder::new("{"),
            finder_square_bracket: memmem::Finder::new("["),
            finder_as: memmem::Finder::new(" as "),
            finder_chain: memmem::Finder::new("chain(["),
            finder_recv: memmem::Finder::new(")."),
            finder_recv_q: memmem::Finder::new(")?."),
            finder_union: memmem::Finder::new(" | "),
            finder_import: memmem::Finder::new(".ts::program:"),
            finder_angle_bracket_o: memmem::Finder::new("<"),
            finder_angle_bracket_c: memmem::Finder::new(">"),
        }
    }
}

pub fn persist_to_disk(slices: Vec<ObjSlice>) {}

/// Performs denoising on the type name and local resolution imports and returns multiple flattened types in case of a union
#[inline(always)]
pub fn clean_type(parser: &Parser, name: &str) -> Vec<String> {
    let mut new_name = name.to_string();

    if name.starts_with("<export>") {
        match memmem::rfind(name.as_bytes(), "/".as_bytes()) {
            Some(i) => new_name = format!("<export>::{}", name[i + 1..].to_string()),
            None => {}
        }
    } else {
        if name.ends_with("[]") || name.starts_with("Array<") {
            new_name = "Array".to_string();
        } else {
            // strip generics
            while let Some(i_o) = parser.finder_angle_bracket_o.find(new_name.as_bytes()) {
                match memmem::rfind(new_name.as_bytes(), ">".as_bytes()) {
                    Some(i_c) => {
                        new_name = format!(
                            "{}{}",
                            new_name[..i_o].to_string(),
                            new_name[i_c + 1..].to_string()
                        )
                    }
                    None => break,
                }
            }

            // resolve local imports
            if new_name.starts_with("import(") {
                let right_side = match memmem::rfind(new_name.as_bytes(), "\").".as_bytes()) {
                    Some(i) => &new_name[i + 3..],
                    None => "",
                };

                let mut left_side: &str = "";
                if let Some(i_l) = memmem::rfind(new_name.as_bytes(), "/".as_bytes()) {
                    if let Some(i_r) = memmem::rfind(new_name.as_bytes(), "\")".as_bytes()) {
                        left_side = &new_name[i_l + 1..i_r];
                    }
                } else {
                    if let Some(i_l) = memmem::rfind(new_name.as_bytes(), "(\"".as_bytes()) {
                        if let Some(i_r) = memmem::rfind(new_name.as_bytes(), "\")".as_bytes()) {
                            left_side = &new_name[i_l + 2..i_r];
                        }
                    }
                }

                new_name = format!("{}.{}", left_side.to_string(), right_side.to_string());
            } else if let Some(i) = parser.finder_import.find(new_name.as_bytes()) {
                new_name = format!(
                    "{}.{}",
                    new_name[..i].to_string(),
                    new_name[i + 13..].to_string()
                );
            }

            // if name.ne(&new_name) {
            //     println!("{} -> {}", name, new_name);
            // }

            // let union_types = new_name.split(" | ");
            // if parser.finder_union.find(new_name.as_bytes()).is_some() {
            //     // println!("{}:::::::::::::::", name);
            //     for t in union_types {
            //         // println!("{}", t);
            //     }
            // }
        }
    }

    new_name = new_name.trim().to_string();
    [new_name].to_vec()
}

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

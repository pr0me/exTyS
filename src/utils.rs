#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]

use crate::slice_structs::{Call, ObjSlice};
use memchr::memmem;
use std::fs;
use std::fs::OpenOptions;
use std::io::prelude::*;
use std::{cmp::min, num};

pub struct Parser<'a> {
    pub finder_eq: memmem::Finder<'a>,
    pub finder_newline: memmem::Finder<'a>,
    pub finder_pipe: memmem::Finder<'a>,
    pub finder_slash: memmem::Finder<'a>,
    pub finder_colon: memmem::Finder<'a>,
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
            finder_colon: memmem::Finder::new(":"),
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

/// Create full feature vector from raw data in order to be fed into an LLM
pub fn assemble(obj: &ObjSlice, calls: &Vec<String>, arg_tos: &Vec<String>) -> String {
    let call_names = if calls.len() > 0 {
        format!(" Calls: {};", calls.join(", "))
    } else {
        "".to_string()
    };

    let arg_names = if arg_tos.len() > 0 {
        format!(" Argument to: {};", arg_tos.join(", "))
    } else {
        "".to_string()
    };

    format!(
        "Variable: {}; Scope: {};{}{}",
        obj.name, obj.scope, call_names, arg_names
    )
}

pub fn generate_splits<T>(a: Vec<T>, b: Vec<T>, threshold: usize) -> Vec<(Vec<T>, Vec<T>)>
where
    T: Clone,
{
    let combined_length = a.len() + b.len();
    if combined_length <= threshold {
        vec![(a, b)]
    } else {
        // determine the number of tuples needed to split the lists
        let num_tuples = (combined_length + threshold - 1) / threshold;

        let a_len = a.len();
        let b_len = b.len();

        // determine the minimum number of elements needed from each list for each tuple
        let mut min_len_a = a_len / num_tuples;
        let mut min_len_b = b_len / num_tuples;

        // adjust for cases where one list is smaller than the other
        if a_len % num_tuples != 0 {
            min_len_a += 1;
        }
        if b_len % num_tuples != 0 {
            min_len_b += 1;
        }

        // split the lists into tuples
        let mut splits: Vec<(Vec<T>, Vec<T>)> = Vec::new();
        let mut a_start = 0;
        let mut b_start = 0;
        for _ in 0..num_tuples {
            let a_end = min(a_start + min_len_a, a_len);
            let b_end = min(b_start + min_len_b, b_len);
            splits.push((a[a_start..a_end].to_vec(), b[b_start..b_end].to_vec()));
            a_start = a_end;
            b_start = b_end;
        }

        // re-use the first element of the smaller list if necessary
        if a_len < num_tuples && a_len > 0 {
            for i in a_len..num_tuples {
                splits[i] = (a[0..1].to_vec(), (&splits[i].1).to_vec());
            }
        }
        if b_len < num_tuples && b_len > 0 {
            for i in b_len..num_tuples {
                splits[i] = ((&splits[i].0).to_vec(), b[0..1].to_vec());
            }
        }

        // make sure one element is the same for all tuples
        if a_len > b_len {
            let el = &a[0];
            for i in 1..num_tuples {
                splits[i].0.push(el.clone());
            }
        } else {
            let el = &b[0];
            for i in 1..num_tuples {
                splits[i].1.push(el.clone());
            }
        }

        splits
    }
}

/// Extract most relevant namespace from full scope path
#[inline(always)]
pub fn extract_func_name(full_qualified_name: &str) -> String {
    let nested_namespaces: Vec<&str> = full_qualified_name.split(':').collect();

    let mut i = nested_namespaces.len() - 1;
    while nested_namespaces[i].starts_with("anonymous") {
        i -= 1;
    }

    if nested_namespaces.len() > 3 && i != 1 && nested_namespaces[i - 1].ne("program") {
        format!("{}.{}", nested_namespaces[i - 1], nested_namespaces[i])
    } else {
        nested_namespaces[i].to_string()
    }
}

pub fn persist_to_disk(data: Vec<(String, String, usize)>) {
    let t0 = std::time::Instant::now();

    let mut features = Vec::with_capacity(data.len());
    let mut labels = Vec::with_capacity(data.len());

    for (a, b, _) in data.iter() {
        features.push(format!("\"{}\"", a.to_owned()));
        labels.push(format!("\"{}\"", b.to_owned()));
    }

    let feat_buf = features.join(",\n");
    let label_buf = labels.join(",\n");

    let mut feat_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("./feature_vec.json")
        .expect("Failed to open feature file");
    let mut label_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("./class_label_vec.json")
        .expect("Failed to open label file");

    feat_file
        .write("[\n".as_bytes())
        .expect("Failed to write preamble to feature file");
    label_file
        .write("[\n".as_bytes())
        .expect("Failed to write preamble to label file");

    feat_file
        .write_all(feat_buf.as_bytes())
        .expect("Failed to write data to feature file");
    label_file
        .write_all(label_buf.as_bytes())
        .expect("Failed to write data to label file");

    feat_file
        .write("\n]".as_bytes())
        .expect("Failed to write to feature file");
    label_file
        .write("\n]".as_bytes())
        .expect("Failed to write to label file");

    println!(
        "[i] Persisting vectors to disk took {:.2} sec",
        t0.elapsed().as_secs_f32()
    );
}

#![feature(test)]

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[cfg(test)]
pub mod bench;
pub mod slice_structs;
pub mod utils;

use crate::slice_structs::ObjSlice;
use clap::Parser;
use glob::glob;
use indicatif::ProgressBar;
use itertools::Itertools;
use memchr::memmem;
use serde_json;
use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Read;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to Directory with Slices
    #[arg(short, long)]
    slices: String,

    /// Path to Directory with Slices
    #[arg(short, long, default_value = "./")]
    output_dir: String,

    /// Language of the parsed Slices (typescript, python)
    #[arg(short, long)]
    language: Option<String>,

    /// Number of how many observations an object needs to be considered
    #[arg(short, long, default_value_t = 1)]
    lower_usage_bound: usize,
    /// Number of how many observations an object may have before it is being split
    #[arg(short, long, default_value_t = 8)]
    upper_usage_bound: usize,

    /// Number of observations per class we require to be present in the dataset
    #[arg(short, long, default_value_t = 32)]
    class_occurence_threshold: usize,

    /// If not 0, outputs a `top_n.json` file with the most common classes in the dataset
    #[arg(short, long, default_value_t = 0)]
    top_n_classes: u16,

    /// Cap the size of the generated dataset at the specified number
    #[arg(short, long)]
    max_samples: Option<usize>,
}

/// Imports slice information from json files generated by joern-slice
fn import_slices(args: &Args) -> Vec<ObjSlice> {
    let mut slice_candidates = Vec::new();
    let mut num_scopes: u32 = 0;
    let mut num_obj: u32 = 0;

    println!("[*] Processing slices from '{}'.", args.slices);
    let t0 = Instant::now();

    let finder_lambda = memmem::Finder::new("=>");
    let finder_struct = memmem::Finder::new("{");
    let finder_init = memmem::Finder::new(" = new ");

    let mut paths: Vec<std::path::PathBuf> = Vec::with_capacity(400_000);
    for entry in glob(&format!("{}/**/*.json", args.slices))
        .expect("Failed to read provided slice path as glob pattern")
    {
        match entry {
            Ok(path) => paths.push(path),
            Err(e) => println!("{:?}", e),
        }
    }

    let num_files = paths.len();
    println!(
        "[*] Found {} slice files. This might take a while...",
        num_files
    );

    // iterate over slice files
    let bar = ProgressBar::new(num_files as _);
    for path in paths {
        // println!("{:?}", path);

        let mut c = String::new();
        File::open(path).unwrap().read_to_string(&mut c).unwrap();
        if c.is_empty() {
            continue;
        }

        // parse slice file as json
        let curr_slice_json: slice_structs::FullSlice =
            serde_json::from_str(&c).expect("Failed to parse JSON file");

        // iterate over scopes in file
        for (scope, vars) in curr_slice_json.object_slices {
            num_scopes += 1;

            // iterate over objects in scope
            for curr_obj in vars {
                num_obj += 1;

                let mut curr_type_name: &str = &curr_obj.target_obj.type_full_name;

                if curr_type_name.is_empty()
                    || curr_obj.invoked_calls.len() + curr_obj.arg_to_calls.len()
                        < args.lower_usage_bound as usize
                    || finder_lambda.find(curr_type_name.as_bytes()).is_some()
                    || finder_struct.find(curr_type_name.as_bytes()).is_some()
                {
                    continue;
                }

                // try to recover type name from constructor call
                if curr_type_name.eq("ANY") {
                    if curr_obj.arg_to_calls.len() != 0 {
                        let maybe_init_call = &curr_obj.arg_to_calls[0].0.call_name;

                        match finder_init.find(maybe_init_call.as_bytes()) {
                            Some(i) => curr_type_name = &maybe_init_call[i + 7..],
                            None => continue,
                        }
                    } else {
                        continue;
                    }
                }

                let func_scope = utils::extract_func_name(&scope);
                let curr_slice = slice_structs::ObjSlice {
                    name: curr_obj.target_obj.name,
                    scope: func_scope,
                    type_name: curr_type_name.to_string(),
                    invoked_calls: curr_obj.invoked_calls,
                    arg_to_calls: curr_obj.arg_to_calls,
                };

                // println!("Slice: {:?}\n", curr_slice);
                slice_candidates.push(curr_slice);
            }
        }

        bar.inc(1);
    }
    bar.finish();
    println!(
        "[i] Importing slices took {:.3}s",
        t0.elapsed().as_secs_f32()
    );
    println!(
        "[i] Found an average of {:.2} scopes in {} slice files",
        num_scopes as f32 / num_files as f32,
        num_files
    );
    println!(
        "[i] Found {:?} slice candidates ({:.1}% of {} total)",
        slice_candidates.len(),
        slice_candidates.len() as f32 / num_obj as f32 * 100.0,
        num_obj
    );
    println!(
        "    - average per file:                  {:.2}",
        slice_candidates.len() as f32 / num_files as f32
    );
    println!(
        "    - average of total objects per file: {:.2}",
        num_obj as f32 / num_files as f32
    );

    slice_candidates
}

/// Performs filtering, denoising and vectorization of slices and its field
fn vectorize_slices(args: &Args, slices: Vec<ObjSlice>) {
    println!("[*] Begin Vectorizing Slices");
    let t0 = Instant::now();

    let parser = utils::Parser::new(&args.language);
    let mut candidates: Vec<(String, String, usize)> = Vec::new();

    let bar = ProgressBar::new(slices.len() as _);
    for mut curr_slice in slices {
        if let Some(i) = parser.finder_colon.find(curr_slice.name.as_bytes()) {
            curr_slice.name = curr_slice.name[..i].to_string();
        }

        let calls: Vec<String> = curr_slice
            .invoked_calls
            .clone()
            .into_iter()
            .map(|c| c.call_name)
            .unique()
            .collect();

        let mut arg_tos: Vec<String> = Vec::with_capacity(curr_slice.arg_to_calls.len());
        for c in &curr_slice.arg_to_calls {
            let curr_call = &c.0;

            if let Some(call_name) = utils::clean_method_name(&parser, &curr_call.call_name) {
                // if let Some(recv) = &curr_call.receiver {
                //     if !(recv.eq("this") || recv.starts_with("_tmp_") || recv.eq("_")) {
                //         call_name = format!("{}.{}", recv, call_name);
                //     }
                // }

                arg_tos.push(call_name);
            }
        }

        arg_tos = arg_tos.into_iter().unique().collect();

        // if we only observe a single assignment, this variable is not interesting
        if calls.len() == 0 && arg_tos.len() == 1 {
            if arg_tos[0].starts_with("assignment") {
                continue;
            }
        }

        let total_usages = calls.len() + arg_tos.len();
        if total_usages >= args.lower_usage_bound {
            curr_slice.type_name = utils::clean_type(&parser, &curr_slice.type_name)[0].to_owned();

            // generate multiple samples from one usage slice if it is too long
            if total_usages > args.upper_usage_bound {
                let splits = utils::generate_splits(calls, arg_tos, args.upper_usage_bound);
                for s in splits {
                    let feat_str = utils::assemble(&curr_slice, &(s.0), &(s.1), &args.language);
                    candidates.push((
                        feat_str,
                        curr_slice.type_name.to_owned(),
                        s.0.len() + s.1.len(),
                    ));
                }
            } else {
                let feat_str = utils::assemble(&curr_slice, &calls, &arg_tos, &args.language);
                candidates.push((
                    feat_str,
                    curr_slice.type_name.to_owned(),
                    calls.len() + arg_tos.len(),
                ));
            }
        }

        bar.inc(1);
    }
    bar.finish();

    println!("[*] Deduplication and Generation of Type Histograms");
    let mut unq_candidates = candidates
        .into_iter()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    utils::merge_common_types(&mut unq_candidates);

    let mut class_counts = HashMap::new();

    // count occurrences of each class
    for (_, class, _) in &unq_candidates {
        let curr_type = class.to_owned();
        *class_counts.entry(curr_type).or_insert(0) += 1;
    }

    unq_candidates.retain(|(_, class, _)| {
        class_counts.get(class).unwrap_or(&0) >= &args.class_occurence_threshold
    });

    // TODO: optimize
    // resize the vector while maintaining the distribution of observed classes
    if let Some(n) = args.max_samples {
        let mut filtered_candidates = Vec::new();
        let mut count_vec: Vec<_> = class_counts.iter().collect();
        count_vec.sort_by(|a, b| b.1.cmp(a.1));

        let cut_off = count_vec
            .iter()
            .position(|&c| c.1 < &args.class_occurence_threshold)
            .unwrap();

        let mut class_to_filtered_indices = HashMap::<&String, Vec<usize>>::new();
        for (index, (_, class, _)) in unq_candidates.iter().enumerate() {
            if class_counts.get(class).unwrap() >= &args.class_occurence_threshold {
                class_to_filtered_indices
                    .entry(class)
                    .or_insert_with(Vec::new)
                    .push(index);
            }
        }

        for type_tuple in count_vec[..cut_off].iter().rev() {
            let mut curr_count = 0;
            let max_count = max(
                *class_counts.get(type_tuple.0).unwrap() * n / unq_candidates.len(),
                args.class_occurence_threshold,
            );

            if let Some(indices) = class_to_filtered_indices.get(type_tuple.0) {
                for &index in indices {
                    let (feature, class, u) = &unq_candidates[index];
                    filtered_candidates.push((feature.clone(), class.clone(), *u));
                    curr_count += 1;
                    if curr_count > max_count || filtered_candidates.len() >= n {
                        break;
                    }
                }
            }

            if filtered_candidates.len() >= n {
                break;
            }
        }

        unq_candidates = filtered_candidates;
    }

    println!(
        "[*] Finished Vectorizing Slices in {:.2}sec",
        t0.elapsed().as_secs_f32()
    );

    // generate stats
    let type_set = unq_candidates
        .clone()
        .into_iter()
        .map(|c| c.1)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    let num_types = type_set.len();
    let mut occ: Vec<usize> = type_set
        .into_iter()
        .map(|t| *class_counts.get(&t).unwrap())
        .collect();
    occ.sort();

    println!(
        "[i] Using {} slice candidates after filtering",
        unq_candidates.len()
    );
    println!("[i] Found {} unique classes", num_types);
    println!(
        "[i] Occurences per type:\n    - average: {:.2}\n    - median:  {} ",
        occ.iter().sum::<usize>() as f32 / occ.len() as f32,
        occ[occ.len() / 2]
    );

    utils::persist_to_disk(unq_candidates);
}

fn main() {
    let args = Args::parse();

    let imported_slices = import_slices(&args);
    vectorize_slices(&args, imported_slices);
}

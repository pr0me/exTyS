pub mod slice_structs;
pub mod utils;

use crate::slice_structs::ObjSlice;
use clap::Parser;
use glob::glob;
use serde_json;
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
    #[arg(short, long, default_value = "typescript")]
    language: String,

    /// Number of how many observations an object needs to be considered
    #[arg(long, default_value_t = 1)]
    usage_lower_bound: u16,
    /// Number of how many observations an object may have before it is being split
    #[arg(long, default_value_t = 8)]
    usage_upper_bound: u16,

    /// Number of observations per class we require to be present in the dataset
    #[arg(short, long, default_value_t = 3)]
    class_occurence_threshold: u16,

    /// If not 0, outputs a `top_n.json` file with the most common classes in the dataset
    #[arg(short, long, default_value_t = 0)]
    top_n_classes: u16,
}

fn import_slices(args: Args) -> Vec<ObjSlice> {
    let mut slice_candidates = Vec::new();
    let mut num_scopes: u32 = 0;
    let mut num_obj: u32 = 0;
    let mut num_files: u32 = 0;

    println!(
        "[*] Processing slices from '{}'. This might take a while...",
        args.slices
    );
    let t0 = Instant::now();

    // iterate over slice files
    for entry in glob(&format!("{}/**/*.json", args.slices))
        .expect("Failed to read provided slice path as glob pattern")
    {
        if let Ok(path) = entry {
            let mut c = String::new();
            File::open(path).unwrap().read_to_string(&mut c).unwrap();
            if c.is_empty() {
                continue;
            }
            num_files += 1;

            // parse slice file as json
            let curr_slice_json: slice_structs::FullSlice =
                serde_json::from_str(&c).expect("Failed to parse JSON file");

            // iterate over scopes in file
            for (scope, vars) in curr_slice_json.object_slices {
                num_scopes += 1;

                // iterate over objects in scope
                for curr_obj in vars {
                    num_obj += 1;

                    let mut curr_type_name = curr_obj.target_obj.type_full_name;

                    if curr_type_name.is_empty()
                        || curr_obj.invoked_calls.len() + curr_obj.arg_to_calls.len()
                            < args.usage_lower_bound as usize
                        || curr_type_name.contains("=>")
                        || curr_type_name.contains("{")
                    {
                        continue;
                    }

                    if curr_type_name.eq("ANY") {
                        if curr_obj.arg_to_calls.len() != 0 {
                            let maybe_init_call = &curr_obj.arg_to_calls[0].0.call_name;

                            if maybe_init_call.contains(" = new ") {
                                let recovered_type = maybe_init_call
                                    .split(" = new ")
                                    .collect::<Vec<&str>>()
                                    .last()
                                    .copied()
                                    .unwrap();
                                curr_type_name = recovered_type.to_string();
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    let curr_slice = slice_structs::ObjSlice {
                        name: curr_obj.target_obj.name,
                        scope: scope.to_string(),
                        type_name: curr_type_name.to_string(),
                        invoked_calls: curr_obj.invoked_calls,
                        arg_to_methods: curr_obj.arg_to_calls,
                    };

                    // println!("Slice: {:?}\n", curr_slice);
                    slice_candidates.push(curr_slice);
                }
            }
        }
    }
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

fn main() {
    let args = Args::parse();

    import_slices(args);

    println!("Hello, world!");
}

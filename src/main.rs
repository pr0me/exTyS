pub mod utils;

use clap::Parser;
use glob::glob;
use serde_json;
use std::fs::File;
use std::io::{BufReader, Read};
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

#[derive(Debug)]
struct Call {
    name: String,
    receiver: Option<String>,
}

#[derive(Debug)]
struct Slice {
    name: String,
    scope: String,
    type_name: String,
    invoked_calls: Vec<Call>,
    arg_to_methods: Vec<Call>,
}

fn import_slices(args: Args) -> Vec<Slice> {
    let mut slice_candidates = Vec::new();
    let mut num_scopes: u32 = 0;
    let mut num_obj: u32 = 0;
    let mut num_files: u32 = 0;

    println!(
        "[*] Processing slice file from '{}'. This might take a while...",
        args.slices
    );
    let t0 = Instant::now();

    // iterate over slice files
    for entry in glob(&format!("{}/**/*.json", args.slices))
        .expect("Failed to read provided slice path as glob pattern")
    {
        if let Ok(path) = entry {
            // let mut file = File::open(path).unwrap();
            // let mut contents = String::new();
            // file.read_to_string(&mut buf).unwrap();

            let file = File::open(path).unwrap();
            let mut buf_reader = BufReader::new(file);
            let mut contents = String::new();
            buf_reader.read_to_string(&mut contents).unwrap();
            if contents.len() < 1 {
                continue;
            }

            // parse slice file as json
            let curr_slice_json: serde_json::Value =
                serde_json::from_str(&contents).expect("Failed to parse JSON file");
            let object_slices = curr_slice_json["objectSlices"].as_object().unwrap();
            num_files += 1;

            // iterate over scopes in file
            for (scope, vars) in object_slices {
                num_scopes += 1;

                // iterate over objects in scope
                for curr_obj in vars.as_array().unwrap() {
                    num_obj += 1;

                    let invoked_calls_arr = curr_obj["invokedCalls"].as_array().unwrap();
                    let arg_to_calls_arr = curr_obj["argToCalls"].as_array().unwrap();
                    let mut curr_type_name =
                        curr_obj["targetObj"]["typeFullName"].as_str().unwrap();

                    if curr_type_name.is_empty()
                        || invoked_calls_arr.len() + arg_to_calls_arr.len()
                            < args.usage_lower_bound as usize
                        || curr_type_name.contains("=>")
                        || curr_type_name.contains("{")
                    {
                        continue;
                    }

                    if curr_type_name.eq("ANY") {
                        if arg_to_calls_arr.len() != 0 {
                            let maybe_init_call =
                                arg_to_calls_arr[0][0]["callName"].as_str().unwrap();

                            if maybe_init_call.contains(" = new ") {
                                let recovered_type = maybe_init_call
                                    .split(" = new ")
                                    .collect::<Vec<&str>>()
                                    .last()
                                    .copied()
                                    .unwrap();
                                curr_type_name = recovered_type;
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }

                    let mut curr_slice = Slice {
                        name: curr_obj["targetObj"]["name"].as_str().unwrap().to_string(),
                        scope: scope.to_string(),
                        type_name: curr_type_name.to_string(),
                        invoked_calls: Vec::new(),
                        arg_to_methods: Vec::new(),
                    };

                    for call in invoked_calls_arr {
                        let call_name = call["callName"].as_str().unwrap();
                        curr_slice.invoked_calls.push(Call {
                            name: call_name.to_string(),
                            receiver: None,
                        });
                    }

                    for call in arg_to_calls_arr {
                        let call_name = call[0]["callName"].as_str().unwrap();
                        let call_receiver = call[0]["receiver"].as_str();
                        curr_slice.invoked_calls.push(Call {
                            name: call_name.to_string(),
                            receiver: call_receiver.map(|s| s.to_string()),
                        });
                    }

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

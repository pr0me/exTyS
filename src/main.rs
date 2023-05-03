use clap::Parser;
use glob::glob;
use serde_json;
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

struct Call {
    name: String,
    receiver: Option<String>,
}

struct Slice {
    name: String,
    invoked_calls: Vec<Call>,
    arg_to_methods: Vec<Call>,
}

fn import_slices(path: &str) -> Vec<String> {
    let mut slice_candidates = Vec::new();
    for entry in glob(&format!("{}/**/*.json", path))
        .expect("Failed to read provided slice path as glob pattern")
    {
        match entry {
            Ok(path) => slice_candidates.push(path.to_str().unwrap().to_string()),
            Err(e) => println!("{:?}", e),
        }
    }

    slice_candidates
}

fn main() {
    let args = Args::parse();

    import_slices(&args.slices);

    println!("Hello, world!");
}

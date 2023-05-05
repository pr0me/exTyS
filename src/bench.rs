use memchr::memmem;
use rand::{distributions::Alphanumeric, Rng};
use regex::Regex;
use std::time::Instant;

#[test]
pub fn benchmark_string_matchers() -> std::io::Result<()> {
    let mut strings: Vec<String> = Vec::new();

    for i in 0..2048 {
        let s: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        strings.push(s);
    }

    lazy_static! {
        static ref RE: Regex = Regex::new(":").unwrap();
    }
    let t0 = Instant::now();
    for s in &strings {
        if let Some(m) = RE.find(&s) {
            std::hint::black_box(m);
        }
    }
    let t1 = t0.elapsed().as_micros();
    let t0 = Instant::now();
    let finder = memmem::Finder::new(":");
    for s in &strings {
        if let Some(m) = finder.find(s.as_bytes()) {
            std::hint::black_box(m);
        }
    }
    println!(
        "regex:        {:4} μs\nmemmem:       {:4} μs",
        t1,
        t0.elapsed().as_micros()
    );

    lazy_static! {
        static ref RE_SET: regex::RegexSet = regex::RegexSet::new(&["=", "\n"]).unwrap();
    }
    let t0 = Instant::now();
    for s in &strings {
        let matches: Vec<_> = RE_SET.matches(&s).into_iter().collect();
        if matches.is_empty() {
            std::hint::black_box(&s);
        }
    }
    let t1 = t0.elapsed().as_micros();
    let t0 = Instant::now();
    let finder = memmem::Finder::new("=");
    let finder_1 = memmem::Finder::new("\n");
    for s in &strings {
        if finder.find(s.as_bytes()).is_some() || finder_1.find(s.as_bytes()).is_some() {
            std::hint::black_box(&s);
        }
    }

    println!(
        "regexSet:     {:4} μs\nmulti-memmem: {:4} μs",
        t1,
        t0.elapsed().as_micros()
    );

    Ok(())
}

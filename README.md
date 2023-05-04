# exTyS: Extracting Types from Slices
This is a post-processing tool for [Joern](https://github.com/joernio/joern) slices, extracting type information for the use in ML models.

Make sure to compile the optimized version:
```
cargo build --release
```

And then run on the directory containing the slice `.json`s:
```
./target/release/extys --slices ./ti_datasets/v3/slices/
```

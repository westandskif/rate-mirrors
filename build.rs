use std::{env, fs, path::Path};

// Embeds src/countries.json without its line breaks and indentation. Lossless
// for any valid JSON: strings cannot hold raw newlines, so leading whitespace
// on a line is never inside a string.
fn main() {
    println!("cargo:rerun-if-changed=src/countries.json");
    let source =
        fs::read_to_string("src/countries.json").expect("failed to read src/countries.json");
    let compact: String = source.lines().map(str::trim_start).collect();
    let out = Path::new(&env::var("OUT_DIR").unwrap()).join("countries.json");
    fs::write(out, compact).expect("failed to write countries.json to OUT_DIR");
}

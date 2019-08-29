use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;

fn main() -> io::Result<()> {
    generate(
        Path::new("vocabulary/adjectives.txt"),
        "ADJECTIVES",
        Path::new("src/names/adjectives.rs"),
    )?;
    generate(Path::new("vocabulary/nouns.txt"), "NOUNS", Path::new("src/names/nouns.rs"))
}

fn generate(src_path: &Path, identifier: &str, dst_path: &Path) -> io::Result<()> {
    let src = BufReader::new(File::open(src_path)?);
    let mut dst = BufWriter::new(File::create(dst_path)?);
    writeln!(dst, "// File generated from \"{}\", do not modify.", src_path.to_string_lossy())?;
    writeln!(dst, "pub const {}: &'static [&'static str] = &[", identifier)?;
    for word in src.lines() {
        writeln!(dst, "    \"{}\",", &word?)?;
    }
    writeln!(dst, "];")
}

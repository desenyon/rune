//! Synthetic repository generator for S066 performance tiers.
//!
//! Standalone for now (`rustc tests/performance/gen.rs`). Not a workspace
//! member. Do not commit generated output.

use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy)]
enum Tier {
    Small,
    Medium,
    Large,
    VeryLarge,
}

impl Tier {
    fn parse(s: &str) -> Option<Self> {
        match s {
            "small" => Some(Self::Small),
            "medium" => Some(Self::Medium),
            "large" => Some(Self::Large),
            "very_large" | "very-large" => Some(Self::VeryLarge),
            _ => None,
        }
    }

    fn file_count(self) -> usize {
        match self {
            Self::Small => 100,
            Self::Medium => 1_000,
            Self::Large => 10_000,
            Self::VeryLarge => 100_000,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Medium => "medium",
            Self::Large => "large",
            Self::VeryLarge => "very_large",
        }
    }
}

fn usage() -> ! {
    eprintln!(
        "usage: gen --tier <small|medium|large|very_large> --out <dir>\n\
         Generates a synthetic Rust-like tree. Do not commit the output."
    );
    std::process::exit(2);
}

fn parse_args() -> (Tier, PathBuf) {
    let mut tier = None;
    let mut out = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--tier" => {
                let value = args.next().unwrap_or_else(|| usage());
                tier = Tier::parse(&value);
            }
            "--out" => out = args.next().map(PathBuf::from),
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }
    match (tier, out) {
        (Some(tier), Some(out)) => (tier, out),
        _ => usage(),
    }
}

fn write_module(path: &Path, index: usize) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    writeln!(
        file,
        "//! Generated fixture module {index}. Do not commit generated trees.\n\
         \n\
         pub fn symbol_{index}(input: u64) -> u64 {{\n\
             input.wrapping_mul({index} + 1).wrapping_add({index})\n\
         }}\n\
         \n\
         pub fn calls_neighbor_{index}(input: u64) -> u64 {{\n\
             symbol_{index}(input) ^ {index}\n\
         }}\n\
         \n\
         #[cfg(test)]\n\
         mod tests {{\n\
             use super::*;\n\
         \n\
             #[test]\n\
             fn symbol_{index}_is_stable() {{\n\
                 assert_eq!(symbol_{index}(3), symbol_{index}(3));\n\
             }}\n\
         }}\n"
    )?;
    Ok(())
}

fn main() -> io::Result<()> {
    let (tier, out) = parse_args();
    fs::create_dir_all(&out)?;
    let n = tier.file_count();
    let src = out.join("src");
    fs::create_dir_all(&src)?;

    for i in 0..n {
        let bucket = i % 128;
        let dir = src.join(format!("b{bucket:03}"));
        write_module(&dir.join(format!("m{i}.rs")), i)?;
    }

    let mut lib = fs::File::create(src.join("lib.rs"))?;
    writeln!(
        lib,
        "//! Generated {}-tier fixture ({n} modules).\n\
         //! This file is produced by tests/performance/gen.rs.\n",
        tier.as_str()
    )?;
    for i in 0..n {
        let bucket = i % 128;
        writeln!(lib, "#[path = \"b{bucket:03}/m{i}.rs\"]")?;
        writeln!(lib, "pub mod m{i};")?;
    }

    let mut manifest = fs::File::create(out.join("Cargo.toml"))?;
    writeln!(
        manifest,
        "[package]\n\
         name = \"rune_perf_{}\"\n\
         version = \"0.0.0\"\n\
         edition = \"2021\"\n\
         publish = false\n\
         \n\
         [lib]\n\
         path = \"src/lib.rs\"\n",
        tier.as_str()
    )?;

    let mut meta = fs::File::create(out.join("TIER.txt"))?;
    writeln!(meta, "tier={} files={n}", tier.as_str())?;
    eprintln!("wrote {n} files under {}", out.display());
    Ok(())
}

# gpmf-rs

Rust crate for parsing GoPro GPMF data, directly from MP4, from "raw" GPMF-files extracted via ffmpeg, or byte slices.

Usage (not yet on crates.io):

`Cargo.toml`:
```toml
[dependencies]
gpmf-rs = {git = "https://github.com/jenslar/gpmf-rs.git"}
```

`src/main.rs`:
```rs
use gpmf_rs::Gpmf;
use std::path::Path;

fn main() -> std::io::Result<()> {
    let path = Path::new("GOPRO_VIDEO.MP4");
    let gpmf = Gpmf::new(&path)?;
    println!("{gpmf:#?}");
    Ok(())
}
```
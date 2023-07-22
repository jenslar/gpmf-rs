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
use gpmf_rs::{Gpmf, SensorType};
use std::path::Path;

fn main() -> std::io::Result<()> {
    let path = Path::new("GOPRO_VIDEO.MP4");

    // Extract GPMF data without printing debug info while parsing
    let gpmf = Gpmf::new(&path, false)?;
    println!("{gpmf:#?}");

    // Filter and process GPS log, prune points that do not have at least a 2D fix
    let gps = gpmf.gps().prune(2);
    println!("{gps:#?}");

    // Filter and process accelerometer data.
    let sensor = gpmf.sensor(&SensorType::Accelerometer);
    println!("{sensor:#?}");

    Ok(())
}
```
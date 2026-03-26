# gpmf-rs

Rust crate for parsing GoPro GPMF data, directly from MP4, from "raw" GPMF-files extracted via ffmpeg, or byte slices.

If `GoProSession::from_path()` or `GoProSession::from_goprofile()` can not locate the remaining clips for a recording session,
it is usually because I do not have enough sample data for that model (a multi-clip recording is required).
GoPro sometimes change how the `MUID` and `GUMI` identifiers are used between models, which creates this issue.

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

    // Filter and export GPS log, prune points that do not have at least a 2D fix,
    // and dilution of precision above 5.0.
    let gps = gpmf.gps().prune(Some(2), Some(5.0));
    println!("{gps:#?}");

    // Filter and export accelerometer data.
    let sensor = gpmf.sensor(&SensorType::Accelerometer);
    println!("{sensor:#?}");
    
    // Locate all clips in a recording session,
    // where ever they may be. If a dir is not
    // specified, the parent dir of the clip
    // will be used.
    let dir = Path::new("PATH/TO/SOME/DIR")
    let session = GoProSession::from_path(&path, Some(&dir), false, false, true)?;
    
    // Compile GPMF data for the entire session.
    let gpmf_session = session.gpmf()?;
    println!("{gpmf_session:#?}");
    
    // Then export GPS data etc as usual...
    let gps_session = gpmf_session.gps().prune(Some(2), Some(5.0));
    println!("{gps_session:#?}");

    Ok(())
}
```

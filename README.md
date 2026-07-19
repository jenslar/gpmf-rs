# gpmf-rs

Rust crate for parsing GoPro GPMF data, directly from MP4, from "raw" GPMF-files extracted via ffmpeg, or byte slices.

If `GoProSession::from_path()` or `GoProSession::from_goprofile()` can not locate the remaining clips for a recording session,
it is usually because I do not have enough sample data for that model (a recording that is long enough to span at least two clips is required).
GoPro sometimes change how the `MUID` and `GUMI` identifiers are used between models, which creates this issue. From Hero 13 (and on?) there is also the new `CPID` (presumably "clip ID"), which is only listed in the GPMF section of the `udta` MP4 atom.

Usage (not yet on crates.io):

`Cargo.toml`:
```toml
[dependencies]
gpmf-rs = {git = "https://github.com/jenslar/gpmf-rs.git"}
```

`src/main.rs`:
```rs
use gpmf_rs::{Gpmf, ImuType};
use std::path::Path;

fn main() -> std::io::Result<()> {
    let clip_path = Path::new("GOPRO_VIDEO.MP4");

    // Extract GPMF data without printing debug info while parsing
    let gpmf = Gpmf::new(&clip_path, false)?;
    println!("{gpmf:#?}");

    // Export GPS log, prune points below a 2D satellite lock,
    // and dilution of precision above 5.0.
    let gps = gpmf.gps().prune(Some(2), Some(5.0));
    println!("{gps:#?}");

    // Export accelerometer data.
    let sensor = gpmf.imu(&ImuType::Accelerometer);
    println!("{sensor:#?}");
    
    // Locate all clips in a recording session.
    // If a dir is not specified, the parent dir of the clip
    // will be used.
    let dir = Path::new("PATH/TO/SOME/DIR")
    let session = GoProSession::locate(&clip_path, Some(&dir), true)?;
    
    // Compile GPMF data for the entire session.
    let gpmf_session = session.gpmf()?;
    println!("{gpmf_session:#?}");
    
    // Then export GPS data etc as usual...
    let gps_session = gpmf_session.gps().prune(Some(2), Some(5.0));
    println!("{gps_session:#?}");

    Ok(())
}
```

## Identifying clips from the same recording session

GoPro cameras split recordings into clips (in case of catastrophic failure not all is lost). `gpmf-rs` uses a combination of embedded identifiers, telemetry hashes and MP4 video resolution to determine which clips belong to the same recording session. GoPro somtimes change how these identifiers are used between models so raw footage is needed to add support for new devices.

Identifiers:
- `GUMI`: Global unique media identifier.
- `MUID`: Media unique identifier.
- `CPID`: Clip (?) identifier. (Hero 13 and newer?)
- `CPIN`: Clip (?) index. First clip in session has index 1, second 2 and so one. (Hero 13 and newer?)

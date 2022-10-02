// use std::{fs::File, path::{PathBuf, Path}, io::Write, env};

// Necessary because of this issue: https://github.com/rust-lang/cargo/issues/9641
fn main() -> anyhow::Result<()> {
    embuild::build::CfgArgs::output_propagated("ESP_IDF")?;
    embuild::build::LinkArgs::output_propagated("ESP_IDF")?;

    // let out_dir = env::var("OUT_DIR").unwrap();
    // let out_dir_path = Path::new(out_dir.as_str());

    // let decoder = png::Decoder::new(File::open("rust-logo-512x512-blk.png").unwrap());
    // let mut reader = decoder.read_info().unwrap();
    // let mut buf = vec![0; reader.output_buffer_size()];
    // let info = reader.next_frame(&mut buf).unwrap();
    // let bytes = &buf[..info.buffer_size()];

    // let output_path: PathBuf = [out_dir.as_str(), "rust-logo.bin"].iter().collect();
    // let mut file = File::create(output_path )?;
    // file.write_all(bytes)?;

    Ok(())
}

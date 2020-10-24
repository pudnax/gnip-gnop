use eyre::*;
use fs_extra::copy_items;
use fs_extra::dir::CopyOptions;
use std::env;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=res/*");
    println!("cargo:rerun-if-changed=shaders/*");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let paths_from_copy = vec!["res/", "shaders/"];
    match copy_items(&paths_from_copy, out_dir, &copy_options) {
        Ok(_) => {}
        Err(e) => eprintln!("{}", e),
    }

    Ok(())
}

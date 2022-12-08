// Necessary because of this issue: https://github.com/rust-lang/cargo/issues/9641

use build_target::{target_os, Os};

fn build_lgfx_linux() -> anyhow::Result<()> {
    println!("cargo:rustc-link-search=native=/usr/lib/x86_64-linux-gnu");
    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .flag("-std=c++17")
        .flag("-v")
        .flag("-g")
        .flag("-DLGFX_SDL")
        .file("lgfx/lgfx_c/lgfx_c.cpp")
        .include("lgfx/lgfx_c")
        .include("LovyanGFX/src")
        .include("/usr/include/x86_64-linux-gnu")
        .compile("liblgfx_c.a");
    
    use glob::glob;
    let mut lgfx_c_files = vec![];
    lgfx_c_files.extend(glob("LovyanGFX/src/lgfx/Fonts/efont/*.c").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));
    lgfx_c_files.extend(glob("LovyanGFX/src/lgfx/Fonts/efont/*.c").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));
    lgfx_c_files.extend(glob("LovyanGFX/src/lgfx/Fonts/IPA/*.c").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));
    lgfx_c_files.extend(glob("LovyanGFX/src/lgfx/utility/*.c").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));
    
    let mut lgfx_cpp_files = vec![];
    lgfx_cpp_files.extend(glob("LovyanGFX/src/lgfx/v1/*.cpp").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));
    lgfx_cpp_files.extend(glob("LovyanGFX/src/lgfx/v1/misc/*.cpp").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));
    lgfx_cpp_files.extend(glob("LovyanGFX/src/lgfx/v1/panel/Panel_Device.cpp").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));
    lgfx_cpp_files.extend(glob("LovyanGFX/src/lgfx/v1/platforms/sdl/*.cpp").expect("Fail to find LGFX files.").into_iter().map(|path| path.unwrap()));

    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .flag("-std=c++17")
        .flag("-g")
        .flag("-DLGFX_SDL")
        .files(lgfx_cpp_files)
        .include("LovyanGFX/src")
        .include("/usr/include/x86_64-linux-gnu")
        .compile("libLovyanGFX_cpp.a");
    cc::Build::new()
        .warnings(false)
        .flag("-std=c11")
        .flag("-g")
        .flag("-DLGFX_SDL")
        .files(lgfx_c_files)
        .include("LovyanGFX/src")
        .include("/usr/include/x86_64-linux-gnu")
        .compile("libLovyanGFX_c.a");
    println!("cargo:rustc-link-lib=SDL2");
    println!("cargo:rustc-link-lib=SDL2main");
    Ok(())
}

fn main() -> anyhow::Result<()> {
    match target_os().unwrap() {
        Os::Other(other) => {
            if other == "espidf" {
                embuild::build::CfgArgs::output_propagated("ESP_IDF")?;
                embuild::build::LinkArgs::output_propagated("ESP_IDF")?;
            }
        },
        Os::Linux => {
            build_lgfx_linux()?;
        },
        _ => {},
    }
    Ok(())
}

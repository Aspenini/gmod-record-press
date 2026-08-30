use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=icons/icon.ico");
    copy_steam_api();
    tauri_build::build();
}

fn steam_lib_name() -> &'static str {
    #[cfg(all(windows, target_pointer_width = "64"))]
    {
        "steam_api64.dll"
    }
    #[cfg(all(windows, not(target_pointer_width = "64")))]
    {
        "steam_api.dll"
    }
    #[cfg(target_os = "macos")]
    {
        "libsteam_api.dylib"
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        "libsteam_api.so"
    }
}

fn copy_steam_api() {
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(target_os = "linux")]
    {
        // Dev builds keep libsteam_api.so next to the binary. Packaged Linux
        // builds also install it under ../lib/gmod-record-press so linuxdeploy
        // and the installed /usr/bin wrapper can both resolve it.
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,$ORIGIN/../lib/gmod-record-press");
    }
    #[cfg(target_os = "macos")]
    println!("cargo:rustc-link-arg=-Wl,-rpath,@loader_path");

    let lib = steam_lib_name();
    let Some(src) = find_steamworks_lib(lib) else {
        println!("cargo:warning=Steam redistributable {lib} not found; rebuild after steamworks-sys compiles");
        return;
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let profile_dir = cargo_profile_dir(&out_dir).expect("unexpected OUT_DIR layout");

    let _ = fs::copy(&src, out_dir.join(lib));
    let _ = fs::copy(&src, profile_dir.join(lib));
    let _ = fs::create_dir_all(profile_dir.join("deps"));
    let _ = fs::copy(&src, profile_dir.join("deps").join(lib));

    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR"));
    let _ = fs::copy(&src, manifest.join(lib));
}

fn cargo_build_dir(out_dir: &Path) -> Option<PathBuf> {
    out_dir.ancestors().find_map(|p| {
        p.file_name()
            .is_some_and(|name| name == "build")
            .then(|| p.to_path_buf())
    })
}

fn cargo_profile_dir(out_dir: &Path) -> Option<PathBuf> {
    cargo_build_dir(out_dir)?.parent().map(PathBuf::from)
}

fn find_steamworks_lib(lib: &str) -> Option<PathBuf> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").ok()?);
    let build_dir = cargo_build_dir(&out_dir)?;

    let new_layout = build_dir.join("steamworks-sys");
    if new_layout.is_dir() {
        if let Ok(entries) = fs::read_dir(&new_layout) {
            for entry in entries.flatten() {
                let candidate = entry.path().join("out").join(lib);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    if let Ok(entries) = fs::read_dir(&build_dir) {
        for entry in entries.flatten() {
            if !entry
                .file_name()
                .to_string_lossy()
                .starts_with("steamworks-sys-")
            {
                continue;
            }
            let candidate = entry.path().join("out").join(lib);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

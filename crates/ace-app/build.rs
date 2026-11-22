use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        return;
    }

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let ico_path = manifest_dir
        .join("..")
        .join("..")
        .join("assets")
        .join("icons")
        .join("ace.ico");

    println!("cargo:rerun-if-changed={}", ico_path.display());
    println!("cargo:rerun-if-changed=build.rs");

    if !ico_path.exists() {
        println!(
            "cargo:warning=Ace icon not found at {}, skipping Windows resource embedding",
            ico_path.display()
        );
        return;
    }

    let rc_exe = match find_rc_exe() {
        Some(path) => path,
        None => {
            println!("cargo:warning=rc.exe not found, skipping Windows icon embedding");
            return;
        }
    };

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let rc_path = out_dir.join("ace.rc");
    let res_path = out_dir.join("ace.res");

    let canonical_ico = fs::canonicalize(&ico_path).unwrap_or(ico_path);
    // Strip Windows extended path prefix (\\?\) because rc.exe rejects it.
    let ico_str = canonical_ico
        .to_string_lossy()
        .trim_start_matches(r"\\?\")
        .replace('\\', "/");

    let version = env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "0.1.0".into());
    let parts: Vec<u16> = version
        .split('.')
        .filter_map(|p| p.parse::<u16>().ok())
        .collect();
    let (maj, min, pat) = (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(1),
        parts.get(2).copied().unwrap_or(0),
    );

    // Resource ID 1 is used by both Windows Explorer and GPUI's LoadImageW(..., PCWSTR(1), IMAGE_ICON, ...).
    let rc_content = format!(
        r#"1 ICON "{ico_str}"

1 VERSIONINFO
FILEVERSION {maj},{min},{pat},0
PRODUCTVERSION {maj},{min},{pat},0
FILEOS 0x00040004
FILETYPE 0x00000001
BEGIN
    BLOCK "StringFileInfo"
    BEGIN
        BLOCK "040904b0"
        BEGIN
            VALUE "ProductName", "Ace\0"
            VALUE "FileDescription", "Ace - Cross-platform Database Manager\0"
            VALUE "FileVersion", "{version}\0"
            VALUE "ProductVersion", "{version}\0"
            VALUE "InternalName", "ace\0"
            VALUE "OriginalFilename", "ace.exe\0"
        END
    END
    BLOCK "VarFileInfo"
    BEGIN
        VALUE "Translation", 0x0409, 1200
    END
END
"#
    );

    fs::write(&rc_path, rc_content).expect("failed to write ace.rc");

    let status = Command::new(&rc_exe)
        .arg("/nologo")
        .arg("/fo")
        .arg(&res_path)
        .arg(&rc_path)
        .status()
        .expect("failed to execute rc.exe");

    if !status.success() {
        panic!("rc.exe failed with status: {status}");
    }

    println!("cargo:rustc-link-arg-bin=ace={}", res_path.display());
}

fn find_rc_exe() -> Option<PathBuf> {
    if Command::new("rc.exe")
        .arg("/?")
        .output()
        .map(|_| true)
        .unwrap_or(false)
    {
        return Some(PathBuf::from("rc.exe"));
    }

    let arch = match env::var("CARGO_CFG_TARGET_ARCH").as_deref() {
        Ok("aarch64") => "arm64",
        Ok("x86") => "x86",
        _ => "x64",
    };

    let kits_root = Path::new(r"C:\Program Files (x86)\Windows Kits\10\bin");
    let mut versions: Vec<PathBuf> = fs::read_dir(kits_root)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.is_dir())
        .collect();
    versions.sort();
    versions.reverse();

    for ver in versions {
        let candidate = ver.join(arch).join("rc.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

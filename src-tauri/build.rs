fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let package: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("package.json")).unwrap()).unwrap();
    let config: serde_json::Value = serde_json::from_slice(&std::fs::read(root.join("src-tauri/tauri.conf.json")).unwrap()).unwrap();
    let version = package["version"].as_str().unwrap();
    assert_eq!(version, std::env::var("CARGO_PKG_VERSION").unwrap(), "Cargo version differs from authoritative package.json");
    assert_eq!(version, config["version"].as_str().unwrap(), "Tauri version differs from package.json");
    assert_eq!(version.split('-').next().unwrap(), config["bundle"]["windows"]["wix"]["version"].as_str().unwrap(), "MSI numeric version differs from package.json");
    for path in ["package.json", "src", "src-tauri/src", "src-tauri/adapters", "src-tauri/tauri.conf.json", ".git/HEAD", ".git/index", ".git/refs/heads", ".git/packed-refs"] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    for name in ["GITHUB_SHA", "GITHUB_REF", "CI"] { println!("cargo:rerun-if-env-changed={name}"); }
    let git = |args: &[&str]| std::process::Command::new("git").args(args).current_dir(root).output().ok().filter(|o| o.status.success()).and_then(|o| String::from_utf8(o.stdout).ok()).map(|s| s.trim().to_owned());
    let sha = git(&["rev-parse", "HEAD"]).filter(|s| s.len()==40 && s.bytes().all(|b| b.is_ascii_hexdigit())).unwrap_or_else(|| "unavailable".into());
    if let Some(files) = git(&["ls-files"]) {for file in files.lines(){println!("cargo:rerun-if-changed={}",root.join(file).display());}}
    let dirty = git(&["status", "--porcelain", "--untracked-files=normal"]).is_none_or(|s| !s.is_empty());
    if let Ok(expected) = std::env::var("GITHUB_SHA") { assert_eq!(sha, expected, "Build SHA differs from CI source"); assert!(!dirty, "CI source is dirty"); }
    if let Ok(tag) = std::env::var("GITHUB_REF") { if let Some(tag) = tag.strip_prefix("refs/tags/") { assert_eq!(tag, format!("v{version}"), "Release tag differs from embedded version"); } }
    println!("cargo:rustc-env=SAFEPATCH_BUILD_SHA={sha}");
    println!("cargo:rustc-env=SAFEPATCH_BUILD_DIRTY={dirty}");
    #[cfg(feature = "desktop")]
    tauri_build::build();
}

use serde::Serialize;
#[derive(Serialize)]
pub struct BuildInfo { pub version: &'static str, pub build_sha: &'static str, pub dirty: bool, pub channel: &'static str, pub release_url: String }
pub fn current() -> BuildInfo {
    let version=env!("CARGO_PKG_VERSION");
    BuildInfo { version, build_sha:env!("SAFEPATCH_BUILD_SHA"), dirty:env!("SAFEPATCH_BUILD_DIRTY")=="true", channel:if version.contains('-'){"Preview"}else{"Stable"}, release_url:format!("https://github.com/henryfir456/SafePatch/releases/tag/v{version}") }
}
#[cfg(test)] mod tests {
    #[test] fn embedded_version_matches_all_manifests() {
        let info=super::current();
        let package:serde_json::Value=serde_json::from_str(include_str!("../../package.json")).unwrap();
        let config:serde_json::Value=serde_json::from_str(include_str!("../tauri.conf.json")).unwrap();
        assert_eq!(info.version,package["version"]); assert_eq!(info.version,config["version"]);
        assert!(info.build_sha=="unavailable" || (info.build_sha.len()==40 && info.build_sha.bytes().all(|b|b.is_ascii_hexdigit())));
        assert_eq!(info.channel,"Preview");
    }
}

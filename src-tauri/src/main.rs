#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if std::env::args().any(|s|s=="--build-info") {println!("{}",serde_json::to_string(&safepatch_lib::build_info::current()).expect("embedded metadata"));return;}
    if safepatch_lib::npm_helper::run_if_requested(){return;}
    #[cfg(feature="acceptance")]
    if safepatch_lib::npm_helper::run_control_probe_if_requested(){return;}
    safepatch_lib::desktop::run();
}

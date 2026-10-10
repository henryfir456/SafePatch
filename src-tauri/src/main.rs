#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() {
    if safepatch_lib::npm_helper::run_if_requested(){return;}
    #[cfg(feature="acceptance")]
    if safepatch_lib::npm_helper::run_control_probe_if_requested(){return;}
    safepatch_lib::desktop::run();
}

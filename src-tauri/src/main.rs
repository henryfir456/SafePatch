#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
fn main() { if !safepatch_lib::npm_helper::run_if_requested(){safepatch_lib::desktop::run();} }

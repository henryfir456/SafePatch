// Self-owned Windows desktop integration harness. Excluded from production builds.
use crate::ScanService;
use serde::{Deserialize,Serialize};
use std::{path::PathBuf,sync::Arc};
use tauri::Manager;
pub struct Acceptance {pub mode:String,pub directory:PathBuf}
#[derive(Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    mode:String,scan_id:Option<String>,asset_count:usize,rendered_rows:usize,
    versioned_count:usize,rendered_versions:usize,cancellation:bool,passed:bool,error_code:Option<String>,
}
#[tauri::command]
pub fn acceptance_mode(state:tauri::State<'_,Acceptance>)->String{state.mode.clone()}
#[tauri::command]
pub fn acceptance_report(app:tauri::AppHandle,state:tauri::State<'_,Acceptance>,service:tauri::State<'_,Arc<ScanService>>,mut report:Report)->Result<(),String>{
    let path=state.directory.join("desktop-acceptance.json");
    if let Some(id)=&report.scan_id{
        let assets=service.assets(id)?;
        let versioned=assets.iter().filter(|a|a.installed_version.is_some()).count();
        report.passed &= report.mode==state.mode && !assets.is_empty() && report.asset_count==assets.len() && report.rendered_rows==assets.len() && report.versioned_count==versioned && report.rendered_versions==versioned;
        if state.mode=="restart" {
            let previous:Report=serde_json::from_slice(&std::fs::read(&path).map_err(|_|"ACCEPTANCE_PREVIOUS_MISSING")?).map_err(|_|"ACCEPTANCE_PREVIOUS_INVALID")?;
            report.passed &= previous.passed && previous.cancellation && previous.scan_id==report.scan_id && previous.asset_count==report.asset_count;
        }else{report.passed &= report.cancellation;}
    }else{report.passed=false;}
    let bytes=serde_json::to_vec_pretty(&report).map_err(|_|"ACCEPTANCE_WRITE_FAILED")?;
    std::fs::write(state.directory.join(if state.mode=="restart"{"desktop-restart.json"}else{"desktop-acceptance.json"}),bytes).map_err(|_|"ACCEPTANCE_WRITE_FAILED")?;
    app.exit(if report.passed{0}else{1});Ok(())
}
pub fn configure(app:&mut tauri::App)->Result<(),Box<dyn std::error::Error>>{
    let mode=if std::env::args().any(|s|s=="--acceptance-restart"){"restart"}else{"scan"};
    let directory=std::env::current_exe()?.parent().ok_or("missing executable directory")?.to_owned();
    let service=ScanService::new(directory.join("desktop-acceptance.sqlite")).map_err(std::io::Error::other)?;
    app.manage(service);app.manage(Acceptance{mode:mode.into(),directory});
    let window=app.get_webview_window("main").ok_or("missing main window")?;
    window.hide()?;
    let handle=app.handle().clone();
    std::thread::spawn(move||{std::thread::sleep(std::time::Duration::from_secs(150));handle.exit(2);});
    Ok(())
}

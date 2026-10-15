// Self-owned Windows desktop integration harness. Excluded from production builds.
use crate::ScanService;
use serde::{Deserialize,Serialize};
use std::{path::PathBuf,sync::Arc};
use tauri::Manager;
pub struct Acceptance {pub mode:String,pub directory:PathBuf,pub project_root:Option<String>}
#[derive(Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct SecurityReport {mode:String,scan_id:Option<String>,item_count:usize,rendered_rows:usize,cancellation:bool,metadata_verified:bool,passed:bool,error_code:Option<String>}
#[tauri::command]
pub fn acceptance_security_report(app:tauri::AppHandle,state:tauri::State<'_,Acceptance>,service:tauri::State<'_,Arc<ScanService>>,mut report:SecurityReport)->Result<(),String>{
    let previous=state.directory.join("security-desktop-scan.json");
    if let Some(id)=&report.scan_id{
        let actual=service.last_security_report()?.ok_or("ACCEPTANCE_REPORT_MISSING")?;
        report.passed &= report.mode==state.mode&&actual.scan_id==*id&&report.item_count==actual.items.len()&&report.rendered_rows==actual.items.len().min(200)&&report.metadata_verified&&actual.items.iter().any(|i|i.checks.iter().any(|c|c.status=="COMPLETE"&&matches!(c.source.as_str(),"OSV"|"NVD")));
        if state.mode=="security-restart"{let prior:SecurityReport=serde_json::from_slice(&std::fs::read(previous).map_err(|_|"ACCEPTANCE_PREVIOUS_MISSING")?).map_err(|_|"ACCEPTANCE_PREVIOUS_INVALID")?;report.passed &= prior.passed&&prior.cancellation&&prior.scan_id==report.scan_id&&prior.item_count==report.item_count;}else{report.passed &= report.cancellation;}
    }else{report.passed=false;}
    let path=state.directory.join(if state.mode=="security-restart"{"security-desktop-restart.json"}else{"security-desktop-scan.json"});
    std::fs::write(path,serde_json::to_vec_pretty(&report).map_err(|_|"ACCEPTANCE_WRITE_FAILED")?).map_err(|_|"ACCEPTANCE_WRITE_FAILED")?;
    app.exit(if report.passed{0}else{1});Ok(())
}
#[derive(Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    mode:String,scan_id:Option<String>,asset_count:usize,rendered_rows:usize,
    #[serde(default)] update_count:usize,
    #[serde(default)] rendered_updates:usize,
    #[serde(default)] unknown_update_count:usize,
    versioned_count:usize,rendered_versions:usize,cancellation:bool,passed:bool,error_code:Option<String>,
}
#[tauri::command]
pub fn acceptance_mode(state:tauri::State<'_,Acceptance>)->String{state.mode.clone()}
#[tauri::command]
pub fn acceptance_project_root(state:tauri::State<'_,Acceptance>)->Result<String,String>{state.project_root.clone().ok_or("ACCEPTANCE_ROOT_MISSING".into())}
#[derive(Deserialize,Serialize)]
#[serde(deny_unknown_fields)]
pub struct NpmReport{mode:String,scan_id:Option<String>,package_count:usize,edge_count:usize,rendered_rows:usize,versioned_count:usize,cancellation:bool,passed:bool,error_code:Option<String>}
#[tauri::command]
pub fn acceptance_npm_report(app:tauri::AppHandle,state:tauri::State<'_,Acceptance>,service:tauri::State<'_,Arc<ScanService>>,mut report:NpmReport)->Result<(),String>{
    let previous_path=state.directory.join("npm-desktop-scan.json");
    if let Some(id)=&report.scan_id{
        let graph=service.project_graph(id)?.ok_or("ACCEPTANCE_GRAPH_MISSING")?;
        report.passed &= report.mode==state.mode&&!graph.packages.is_empty()&&report.package_count==graph.packages.len()&&report.edge_count==graph.edges.len()&&report.rendered_rows==graph.packages.len()&&report.versioned_count==graph.packages.iter().filter(|p|p.version.is_some()).count();
        if state.mode=="npm-restart"{let previous:NpmReport=serde_json::from_slice(&std::fs::read(previous_path).map_err(|_|"ACCEPTANCE_PREVIOUS_MISSING")?).map_err(|_|"ACCEPTANCE_PREVIOUS_INVALID")?;report.passed &= previous.passed&&previous.cancellation&&previous.scan_id==report.scan_id&&previous.package_count==report.package_count;}else{report.passed &= report.cancellation;}
    }else{report.passed=false;}
    std::fs::write(state.directory.join(if state.mode=="npm-restart"{"npm-desktop-restart.json"}else{"npm-desktop-scan.json"}),serde_json::to_vec_pretty(&report).map_err(|_|"ACCEPTANCE_WRITE_FAILED")?).map_err(|_|"ACCEPTANCE_WRITE_FAILED")?;
    app.exit(if report.passed{0}else{1});Ok(())
}
#[tauri::command]
pub fn acceptance_report(app:tauri::AppHandle,state:tauri::State<'_,Acceptance>,service:tauri::State<'_,Arc<ScanService>>,mut report:Report)->Result<(),String>{
    let path=state.directory.join("desktop-acceptance.json");
    if let Some(id)=&report.scan_id{
        let assets=service.assets(id)?;
        let versioned=assets.iter().filter(|a|a.installed_version.is_some()).count();
        report.passed &= report.mode==state.mode && !assets.is_empty() && report.asset_count==assets.len() && report.rendered_rows==assets.len() && report.versioned_count==versioned && report.rendered_versions==versioned;
        let updates=assets.iter().filter(|a|a.update.status=="UPDATE_AVAILABLE").count();
        let unknown=assets.iter().filter(|a|matches!(a.update.status.as_str(),"UNKNOWN"|"ERROR")).count();
        report.passed &= report.update_count==updates && report.rendered_updates==updates && report.unknown_update_count==unknown;
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
    let mode=if std::env::args().any(|s|s=="--acceptance-security-restart"){"security-restart"}else if std::env::args().any(|s|s=="--acceptance-security"){"security-scan"}else if std::env::args().any(|s|s=="--acceptance-npm-restart"){"npm-restart"}else if std::env::args().any(|s|s=="--acceptance-npm"){"npm-scan"}else if std::env::args().any(|s|s=="--acceptance-restart"){"restart"}else{"scan"};
    let directory=std::env::current_exe()?.parent().ok_or("missing executable directory")?.to_owned();
    let project_root=if mode.starts_with("npm-"){Some(std::env::var("SAFEPATCH_NPM_PROJECT_ROOT")?)}else{None};
    let service=ScanService::new(directory.join(if mode.starts_with("security-"){"security-desktop-acceptance.sqlite"}else if project_root.is_some(){"npm-desktop-acceptance.sqlite"}else{"desktop-acceptance.sqlite"})).map_err(std::io::Error::other)?;
    app.manage(service);app.manage(Acceptance{mode:mode.into(),directory,project_root});
    let window=app.get_webview_window("main").ok_or("missing main window")?;
    window.hide()?;
    let handle=app.handle().clone();let timeout=if mode.starts_with("security-"){600}else{450};
    std::thread::spawn(move||{std::thread::sleep(std::time::Duration::from_secs(timeout));handle.exit(2);});
    Ok(())
}

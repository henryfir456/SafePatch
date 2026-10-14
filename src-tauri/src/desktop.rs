use crate::{model::{Asset,Scan},ScanService};
use std::sync::Arc;
use tauri::{Manager,State};
#[tauri::command]
fn build_info()->crate::build_info::BuildInfo{crate::build_info::current()}
#[tauri::command]
fn start_security_scan(service:State<'_,Arc<ScanService>>,windows_scan_id:Option<String>,npm_scan_id:Option<String>,external_queries:bool)->Result<Scan,String>{service.start_security(windows_scan_id,npm_scan_id,external_queries)}
#[tauri::command]
fn latest_security_scan(service:State<'_,Arc<ScanService>>)->Result<Option<Scan>,String>{service.latest_security()}
#[tauri::command]
fn last_security_report(service:State<'_,Arc<ScanService>>)->Result<Option<crate::security::SecurityReport>,String>{service.last_security_report()}
#[tauri::command]
fn start_scan(service:State<'_,Arc<ScanService>>,scope:String)->Result<Scan,String>{service.start(&scope)}
#[tauri::command]
fn cancel_scan(service:State<'_,Arc<ScanService>>,scan_id:String)->Result<(),String>{service.cancel(&scan_id)}
#[tauri::command]
fn scan_status(service:State<'_,Arc<ScanService>>,scan_id:String)->Result<Scan,String>{service.status(&scan_id)}
#[tauri::command]
fn latest_scan(service:State<'_,Arc<ScanService>>)->Result<Option<Scan>,String>{service.latest()}
#[tauri::command]
fn list_assets(service:State<'_,Arc<ScanService>>,scan_id:String)->Result<Vec<Asset>,String>{service.assets(&scan_id)}
#[tauri::command]
fn start_npm_scan(service:State<'_,Arc<ScanService>>,project_root:String)->Result<Scan,String>{service.start_project(project_root)}
#[tauri::command]
fn latest_npm_scan(service:State<'_,Arc<ScanService>>)->Result<Option<Scan>,String>{service.latest_project()}
#[tauri::command]
fn npm_graph(service:State<'_,Arc<ScanService>>,scan_id:String)->Result<Option<crate::npm::DependencyGraph>,String>{service.project_graph(&scan_id)}
pub fn run(){
    tauri::Builder::default().setup(|app|{
        #[cfg(feature="acceptance")]
        if std::env::args().any(|s|matches!(s.as_str(),"--acceptance-check"|"--acceptance-restart"|"--acceptance-npm"|"--acceptance-npm-restart")){return crate::acceptance::configure(app)}
        let dir=app.path().app_local_data_dir()?;
        std::fs::create_dir_all(&dir)?;
        let service=ScanService::new(dir.join("inventory.sqlite")).map_err(std::io::Error::other)?;
        app.manage(service);Ok(())
    }).on_page_load(|_window,_payload|{
        #[cfg(feature="acceptance")]
        if _payload.event()==tauri::webview::PageLoadEvent::Finished {if let Some(state)=_window.try_state::<crate::acceptance::Acceptance>(){let _=_window.eval(if state.mode.starts_with("npm-"){include_str!("acceptance_npm.js")}else{include_str!("acceptance.js")});}}
    }).invoke_handler({
        #[cfg(feature="acceptance")]
        {tauri::generate_handler![build_info,start_security_scan,latest_security_scan,last_security_report,start_scan,cancel_scan,scan_status,latest_scan,list_assets,start_npm_scan,latest_npm_scan,npm_graph,crate::acceptance::acceptance_mode,crate::acceptance::acceptance_report,crate::acceptance::acceptance_project_root,crate::acceptance::acceptance_npm_report]}
        #[cfg(not(feature="acceptance"))]
        {tauri::generate_handler![build_info,start_security_scan,latest_security_scan,last_security_report,start_scan,cancel_scan,scan_status,latest_scan,list_assets,start_npm_scan,latest_npm_scan,npm_graph]}
    })
        .run(tauri::generate_context!()).expect("SafePatch could not start; check local data directory permissions");
}

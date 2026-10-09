use crate::{model::{Asset,Scan},ScanService};
use std::sync::Arc;
use tauri::{Manager,State};
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
pub fn run(){
    tauri::Builder::default().setup(|app|{
        #[cfg(feature="acceptance")]
        if std::env::args().any(|s|matches!(s.as_str(),"--acceptance-check"|"--acceptance-restart")){return crate::acceptance::configure(app)}
        let dir=app.path().app_local_data_dir()?;
        std::fs::create_dir_all(&dir)?;
        let service=ScanService::new(dir.join("inventory.sqlite")).map_err(std::io::Error::other)?;
        app.manage(service);Ok(())
    }).on_page_load(|_window,_payload|{
        #[cfg(feature="acceptance")]
        if _payload.event()==tauri::webview::PageLoadEvent::Finished && _window.try_state::<crate::acceptance::Acceptance>().is_some(){let _=_window.eval(include_str!("acceptance.js"));}
    }).invoke_handler({
        #[cfg(feature="acceptance")]
        {tauri::generate_handler![start_scan,cancel_scan,scan_status,latest_scan,list_assets,crate::acceptance::acceptance_mode,crate::acceptance::acceptance_report]}
        #[cfg(not(feature="acceptance"))]
        {tauri::generate_handler![start_scan,cancel_scan,scan_status,latest_scan,list_assets]}
    })
        .run(tauri::generate_context!()).expect("SafePatch could not start; check local data directory permissions");
}

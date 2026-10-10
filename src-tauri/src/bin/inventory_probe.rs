// Real scanner integration probe. Output contains only aggregate evidence.
use safepatch_lib::ScanService;
use std::time::{Duration,Instant};
fn main()->Result<(),Box<dyn std::error::Error>>{
    let cancellation=std::env::args().nth(1).as_deref()==Some("--cancel");
    let dir=tempfile::tempdir()?;
    let service=ScanService::new(dir.path().join("inventory.sqlite"))?;
    let started=Instant::now();let scan=service.start("windows")?;
    if cancellation{service.cancel(&scan.id)?;}
    loop{
        let s=service.status(&scan.id)?;
        if s.status!="RUNNING"{
            let assets=service.assets(&s.id)?;
            drop(service);
            let restart_started=Instant::now();
            let recreated=loop{
                match ScanService::new(dir.path().join("inventory.sqlite")){
                    Ok(service)=>break service,
                    Err(code) if code=="DATABASE_IN_USE" && restart_started.elapsed()<Duration::from_secs(3)=>std::thread::sleep(Duration::from_millis(20)),
                    Err(code)=>return Err(code.into()),
                }
            };
            let persisted=recreated.assets(&s.id)?;
            let preserved=serde_json::to_value(&assets)?==serde_json::to_value(&persisted)?;
            println!("{}",serde_json::to_string_pretty(&serde_json::json!({"status":s.status,"coverage":s.coverage,"error_code":s.error_code,"asset_count":assets.len(),"versioned_count":assets.iter().filter(|a|a.installed_version.is_some()).count(),"publisher_count":assets.iter().filter(|a|a.publisher.is_some()).count(),"duplicate_candidates":assets.iter().filter(|a|a.duplicate_group.is_some()).count(),"restart_preserved":preserved,"elapsed_seconds":started.elapsed().as_secs(),"verification":"REAL_RUST_ADAPTER_SQLITE; DESKTOP_UI_NOT_VERIFIED"}))?);
            if !preserved || (cancellation && s.status!="CANCELLED") || (!cancellation&&(assets.is_empty()||matches!(s.status.as_str(),"FAILED"|"CANCELLED"))){return Err("inventory verification failed".into())}break;
        }
        if started.elapsed()>Duration::from_secs(145){service.cancel(&scan.id)?;return Err("probe timeout".into())}
        std::thread::sleep(Duration::from_millis(100));
    }
    Ok(())
}

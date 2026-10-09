pub mod db;
pub mod model;
pub mod windows;
#[cfg(feature="desktop")]
pub mod desktop;
use model::{Asset,Scan};
use std::{path::PathBuf,sync::{Arc,Mutex,atomic::{AtomicBool,Ordering}}};

struct Active { scan:Scan,cancel:Arc<AtomicBool> }
pub struct ScanService { path:PathBuf,active:Mutex<Option<Active>> }
impl ScanService {
    pub fn new(path:PathBuf)->Result<Arc<Self>,String>{
        let db=db::open(&path).map_err(|_|"DATABASE_OPEN_FAILED")?;
        db::interrupt(&db).map_err(|_|"DATABASE_RECOVERY_FAILED")?;
        Ok(Arc::new(Self{path,active:Mutex::new(None)}))
    }
    pub fn start(self:&Arc<Self>,scope:&str)->Result<Scan,String>{
        if scope!="windows"{return Err("UNSUPPORTED_SCOPE".into())}
        let mut active=self.active.lock().map_err(|_|"SCAN_STATE_FAILED")?;
        if active.as_ref().is_some_and(|a|a.scan.status=="RUNNING"){return Err("SCAN_ALREADY_RUNNING".into())}
        let scan=Scan{id:uuid::Uuid::new_v4().to_string(),status:"RUNNING".into(),phase:"collecting Registry / WinGet".into(),progress:10,started_at:chrono::Utc::now().to_rfc3339(),finished_at:None,coverage:vec![],error_code:None};
        let db=db::open(&self.path).map_err(|_|"DATABASE_OPEN_FAILED")?;
        db::insert_scan(&db,&scan).map_err(|_|"DATABASE_START_FAILED")?;
        let cancel=Arc::new(AtomicBool::new(false));
        *active=Some(Active{scan:scan.clone(),cancel:cancel.clone()});
        let service=self.clone();
        let spawned=std::thread::Builder::new().name("safepatch-inventory".into()).spawn(move||{
            let result=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||windows::collect(cancel.clone()).and_then(|o|model::normalize(o,&chrono::Utc::now().to_rfc3339()).map_err(String::from)))).unwrap_or_else(|_|Err("SCANNER_PANIC".into()));
            service.complete(result,cancel.load(Ordering::Relaxed));
        });
        if spawned.is_err(){drop(active);self.complete(Err("SCAN_WORKER_FAILED".into()),false);return Err("SCAN_WORKER_FAILED".into())}
        Ok(scan)
    }
    fn complete(&self,result:Result<(Vec<Asset>,Vec<model::Coverage>),String>,cancelled:bool){
        if let Ok(mut guard)=self.active.lock(){if let Some(active)=guard.as_mut(){
            active.scan.phase="saving local results".into();active.scan.progress=90;
            let assets=match result{Ok((assets,coverage))=>{active.scan.coverage=coverage;active.scan.status=if cancelled{"CANCELLED"}else if active.scan.coverage.iter().all(|c|c.status=="COMPLETE"){"COMPLETE"}else{"PARTIAL"}.into();assets},Err(code)=>{active.scan.status=if cancelled{"CANCELLED"}else{"FAILED"}.into();active.scan.error_code=Some(code);vec![]}};
            active.scan.finished_at=Some(chrono::Utc::now().to_rfc3339());active.scan.phase="finished".into();active.scan.progress=100;
            match db::open(&self.path).map_err(|_|"DATABASE_OPEN_FAILED".to_string()).and_then(|mut d|db::finish(&mut d,&active.scan,&assets)){
                Ok(())=>{},Err(code)=>{active.scan.status="FAILED".into();active.scan.error_code=Some(code);}
            }
        }}
    }
    fn validate_id(id:&str)->Result<(),String>{uuid::Uuid::parse_str(id).map(|_|()).map_err(|_|"INVALID_SCAN_ID".into())}
    pub fn cancel(&self,id:&str)->Result<(),String>{Self::validate_id(id)?;let guard=self.active.lock().map_err(|_|"SCAN_STATE_FAILED")?;if let Some(a)=guard.as_ref(){if a.scan.id==id && a.scan.status=="RUNNING"{a.cancel.store(true,Ordering::Relaxed);return Ok(())}}Err("SCAN_NOT_RUNNING".into())}
    pub fn status(&self,id:&str)->Result<Scan,String>{Self::validate_id(id)?;if let Some(a)=self.active.lock().map_err(|_|"SCAN_STATE_FAILED")?.as_ref(){if a.scan.id==id{return Ok(a.scan.clone())}}db::scan(&db::open(&self.path).map_err(|_|"DATABASE_OPEN_FAILED")?,Some(id))?.ok_or_else(||"SCAN_NOT_FOUND".into())}
    pub fn latest(&self)->Result<Option<Scan>,String>{if let Some(a)=self.active.lock().map_err(|_|"SCAN_STATE_FAILED")?.as_ref(){return Ok(Some(a.scan.clone()))}db::scan(&db::open(&self.path).map_err(|_|"DATABASE_OPEN_FAILED")?,None)}
    pub fn assets(&self,id:&str)->Result<Vec<Asset>,String>{Self::validate_id(id)?;db::assets(&db::open(&self.path).map_err(|_|"DATABASE_OPEN_FAILED")?,id)}
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn ipc_scope_and_ids_are_validated(){let dir=tempfile::tempdir().unwrap();let s=ScanService::new(dir.path().join("db.sqlite")).unwrap();assert_eq!(s.start("C:\\private; execute").err().unwrap(),"UNSUPPORTED_SCOPE");assert_eq!(s.status("../../../secret").err().unwrap(),"INVALID_SCAN_ID");assert!(s.latest().unwrap().is_none());}
}

//! Actual opt-in environment probe. Output contains aggregate evidence only.
use safepatch_lib::{ScanService,python};
use std::{path::PathBuf,time::{Instant,Duration},sync::atomic::AtomicBool};
fn run()->Result<(),String>{
    let root=PathBuf::from(std::env::var("SAFEPATCH_PYTHON_ENVIRONMENT_ROOT").map_err(|_|"PYTHON_PROBE_ROOT_REQUIRED")?);
    let directory=std::env::current_exe().map_err(|_|"PROBE_EXE")?.parent().ok_or("PROBE_DIRECTORY")?.to_owned();let path=directory.join("python-probe.sqlite");
    let service=ScanService::new(path.clone())?;let started=Instant::now();
    let cancelled=service.start_python(root.to_string_lossy().into())?;service.cancel(&cancelled.id)?;
    while service.status(&cancelled.id)?.status=="RUNNING"{if started.elapsed()>Duration::from_secs(25){return Err("PROBE_CANCEL_TIMEOUT".into())}std::thread::sleep(Duration::from_millis(25));}
    if service.status(&cancelled.id)?.status!="CANCELLED"{return Err("PROBE_CANCEL_FAILED".into())}
    let scan=service.start_python(root.to_string_lossy().into())?;
    while service.status(&scan.id)?.status=="RUNNING"{if started.elapsed()>Duration::from_secs(45){return Err("PROBE_SCAN_TIMEOUT".into())}std::thread::sleep(Duration::from_millis(25));}
    let terminal=service.status(&scan.id)?;if terminal.status!="PARTIAL"{return Err(terminal.error_code.unwrap_or("PROBE_SCAN_FAILED".into()))}
    let snapshot=service.python_snapshot(&scan.id)?.ok_or("PROBE_SNAPSHOT_MISSING")?;
    let mut expected=python::read_python_environment(&root,&AtomicBool::new(false)).map_err(String::from)?;expected.observed_at=snapshot.observed_at.clone();
    let serialized=serde_json::to_string(&snapshot).map_err(|_|"PROBE_SERIALIZATION")?;
    if serde_json::to_string(&expected).map_err(|_|"PROBE_SERIALIZATION")?!=serialized{return Err("PROBE_SNAPSHOT_MISMATCH".into())}
    if snapshot.packages.is_empty()||snapshot.interpreters.is_empty(){return Err("PROBE_POSITIVE_COVERAGE_MISSING".into())}
    drop(service);let reopening=Instant::now();let reopened=loop{match ScanService::new(path.clone()){Ok(service)=>break service,Err(code) if code=="DATABASE_IN_USE"&&reopening.elapsed()<Duration::from_secs(2)=>std::thread::sleep(Duration::from_millis(25)),Err(code)=>return Err(code)}};
    if serde_json::to_string(&reopened.python_snapshot(&scan.id)?.ok_or("PROBE_REOPEN_MISSING")?).map_err(|_|"PROBE_SERIALIZATION")?!=serialized{return Err("PROBE_REOPEN_MISMATCH".into())}
    println!("{}",serde_json::json!({"passed":true,"packages":snapshot.packages.len(),"versions":snapshot.packages.iter().filter(|p|p.version.is_some()).count(),"interpreters":snapshot.interpreters.len(),"interpreter_versions":snapshot.interpreters.iter().filter(|p|p.version.is_some()).count(),"unknown_relationships":snapshot.packages.iter().filter(|p|p.direct_status=="UNKNOWN").count(),"venv_metadata":snapshot.venv_metadata.is_some(),"cancellation":true,"reopen_exact":true,"duration_ms":started.elapsed().as_millis()}));Ok(())
}
fn main(){
    if safepatch_lib::python_helper::run_if_requested(){return;}
    if let Err(code)=run(){eprintln!("{code}");std::process::exit(1);}
}


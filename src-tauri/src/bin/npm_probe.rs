//! Opt-in local probe: only aggregate evidence leaves this process.
use safepatch_lib::{ScanService,model::hash,npm};
use std::{path::{Path,PathBuf},time::{Instant,Duration},sync::atomic::AtomicBool};
fn metadata_hashes(root:&Path)->Vec<Option<String>>{["package-lock.json","npm-shrinkwrap.json","package.json"].iter().map(|name|std::fs::read(root.join(name)).ok().map(|bytes|hash(&String::from_utf8_lossy(&bytes)))).collect()}
fn run()->Result<(),String>{
    let root=PathBuf::from(std::env::var("SAFEPATCH_NPM_PROJECT_ROOT").map_err(|_|"NPM_PROBE_ROOT_REQUIRED")?);
    let directory=std::env::current_exe().map_err(|_|"PROBE_EXE")?.parent().ok_or("PROBE_DIRECTORY")?.to_owned();let path=directory.join("npm-probe.sqlite");
    let before=metadata_hashes(&root);let service=ScanService::new(path.clone())?;let started=Instant::now();
    let cancelled=service.start_project(root.to_string_lossy().into())?;service.cancel(&cancelled.id)?;
    while service.status(&cancelled.id)?.status=="RUNNING"{if started.elapsed()>Duration::from_secs(25){return Err("PROBE_CANCEL_TIMEOUT".into())}std::thread::sleep(Duration::from_millis(25));}
    if service.status(&cancelled.id)?.status!="CANCELLED"{return Err("PROBE_CANCEL_FAILED".into())}
    let scan=service.start_project(root.to_string_lossy().into())?;
    while service.status(&scan.id)?.status=="RUNNING"{if started.elapsed()>Duration::from_secs(45){return Err("PROBE_SCAN_TIMEOUT".into())}std::thread::sleep(Duration::from_millis(25));}
    let terminal=service.status(&scan.id)?;if terminal.status!="PARTIAL"{return Err(terminal.error_code.unwrap_or("PROBE_SCAN_FAILED".into()))}
    let graph=service.project_graph(&scan.id)?.ok_or("PROBE_GRAPH_MISSING")?;
    let expected=npm::read_npm_project(&root,&AtomicBool::new(false)).map_err(String::from)?;
    // Timestamp reflects each observation; all inventory data must match exactly.
    let mut expected=expected;expected.observed_at=graph.observed_at.clone();
    let snapshot=serde_json::to_string(&graph).map_err(|_|"PROBE_SERIALIZATION")?;
    if serde_json::to_string(&expected).map_err(|_|"PROBE_SERIALIZATION")?!=snapshot{return Err("PROBE_GRAPH_MISMATCH".into())}
    drop(service);let reopened=ScanService::new(path)?;
    if serde_json::to_string(&reopened.project_graph(&scan.id)?.ok_or("PROBE_REOPEN_MISSING")?).map_err(|_|"PROBE_SERIALIZATION")?!=snapshot{return Err("PROBE_REOPEN_MISMATCH".into())}
    if metadata_hashes(&root)!=before{return Err("PROBE_PROJECT_CHANGED".into())}
    println!("{}",serde_json::json!({"passed":true,"packages":graph.packages.len(),"edges":graph.edges.len(),"versions":graph.packages.iter().filter(|p|p.version.is_some()).count(),"direct":graph.packages.iter().filter(|p|p.direct_status=="DIRECT").count(),"transitive":graph.packages.iter().filter(|p|p.direct_status=="TRANSITIVE").count(),"unknown_relation":graph.packages.iter().filter(|p|p.direct_status=="UNKNOWN").count(),"lockfile_kind":graph.lockfile_kind,"lockfile_version":graph.lockfile_version,"cancellation":true,"readonly_unchanged":true,"reopen_exact":true,"duration_ms":started.elapsed().as_millis()}));Ok(())
}
fn main(){
    if safepatch_lib::npm_helper::run_if_requested(){return;}
    if let Err(code)=run(){eprintln!("{code}");std::process::exit(1);}
}

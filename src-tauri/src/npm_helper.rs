//! Only our own executable is launched, never a project binary or package script.
use crate::npm::DependencyGraph;
use serde::{Deserialize,Serialize};
use std::{io::{Read,Write},path::Path,sync::{Arc,atomic::{AtomicBool,Ordering}},time::{Duration,Instant}};
const MAX_RESPONSE:u64=32*1024*1024;
#[derive(Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
struct Response {graph:Option<DependencyGraph>,error:Option<String>}
fn safe_code(code:&str)->bool {code=="CANCELLED"||(code.starts_with("NPM_")&&code.len()<=80&&code.bytes().all(|b|b.is_ascii_uppercase()||b==b'_'))}
/// Called before Tauri startup. The root is private stdin data, never shell syntax.
pub fn run_if_requested()->bool {
    if !std::env::args().any(|a|a=="--safepatch-npm-helper"){return false}
    let result=std::panic::catch_unwind(||{
        let mut input=Vec::new();std::io::stdin().take(24577).read_to_end(&mut input).map_err(|_|"NPM_HELPER_INPUT")?;
        if input.len()>24576{return Err("NPM_ROOT_INVALID")}
        let root:String=serde_json::from_slice(&input).map_err(|_|"NPM_ROOT_INVALID")?;
        // Process-control regression only. Not present in production executables.
        #[cfg(feature="acceptance")]
        if std::env::var_os("SAFEPATCH_HELPER_CONTROL_DELAY").is_some(){std::thread::sleep(Duration::from_secs(20));return Err("NPM_HELPER_TEST_DELAY")}
        crate::npm::read_npm_project(Path::new(&root),&AtomicBool::new(false))
    }).unwrap_or(Err("NPM_HELPER_FAILED"));
    let response=match result {Ok(graph)=>Response{graph:Some(graph),error:None},Err(code)=>Response{graph:None,error:Some(code.into())}};
    if let Ok(bytes)=serde_json::to_vec(&response){let _=std::io::stdout().write_all(&bytes);}
    true
}
pub fn collect(root:&Path,cancel:Arc<AtomicBool>)->Result<DependencyGraph,String>{
    if cancel.load(Ordering::Relaxed){return Err("CANCELLED".into())}
    let root=root.to_str().filter(|s|s.len()<=4096).ok_or("NPM_ROOT_INVALID")?;
    let input=serde_json::to_vec(root).map_err(|_|"NPM_ROOT_INVALID")?;
    let directory=tempfile::tempdir().map_err(|_|"NPM_HELPER_TEMP")?;
    let mut command=std::process::Command::new(std::env::current_exe().map_err(|_|"NPM_HELPER_EXE")?);
    command.arg("--safepatch-npm-helper").current_dir(directory.path()).stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null());
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let mut child=command.spawn().map_err(|_|"NPM_HELPER_LAUNCH")?;
    #[cfg(windows)]
    let _job=match crate::windows::OwnedJob::assign(&child){Ok(job)=>job,Err(_)=>{let _=child.kill();let _=child.wait();return Err("NPM_HELPER_JOB".into())}};
    let mut stdin=child.stdin.take().ok_or("NPM_HELPER_INPUT")?;
    let stdout=child.stdout.take().ok_or("NPM_HELPER_OUTPUT")?;
    let writer=std::thread::spawn(move||stdin.write_all(&input));
    let reader=std::thread::spawn(move||{let mut buffer=Vec::new();stdout.take(MAX_RESPONSE+1).read_to_end(&mut buffer).map(|_|buffer)});
    let started=Instant::now();
    let (status,failure)=loop {
        if cancel.load(Ordering::Relaxed){let _=child.kill();break(child.wait(),Some("CANCELLED"))}
        if started.elapsed()>Duration::from_secs(15){let _=child.kill();break(child.wait(),Some("NPM_TIMEOUT"))}
        match child.try_wait(){Ok(Some(status))=>break(Ok(status),None),Ok(None)=>{},Err(_)=>{let _=child.kill();break(child.wait(),Some("NPM_HELPER_WAIT"))}}
        std::thread::sleep(Duration::from_millis(50));
    };
    let written=writer.join();let output=reader.join();
    let status=status.map_err(|_|"NPM_HELPER_WAIT")?;
    if let Some(code)=failure{return Err(code.into())}
    if !status.success(){return Err("NPM_HELPER_FAILED".into())}
    written.map_err(|_|"NPM_HELPER_INPUT")?.map_err(|_|"NPM_HELPER_INPUT")?;
    let output=output.map_err(|_|"NPM_HELPER_OUTPUT")?.map_err(|_|"NPM_HELPER_OUTPUT")?;
    if output.len() as u64>MAX_RESPONSE{return Err("NPM_OUTPUT_LIMIT".into())}
    let response:Response=serde_json::from_slice(&output).map_err(|_|"NPM_HELPER_RESPONSE")?;
    match (response.graph,response.error){
        (Some(graph),None)=>{crate::npm::validate_graph(&graph).map_err(String::from)?;Ok(graph)},
        (None,Some(code)) if safe_code(&code)=>Err(code),_=>Err("NPM_HELPER_RESPONSE".into())
    }
}
#[cfg(feature="acceptance")]
pub fn run_control_probe_if_requested()->bool{
    if !std::env::args().any(|a|a=="--npm-helper-control-probe"){return false}
    std::env::set_var("SAFEPATCH_HELPER_CONTROL_DELAY","owned-test-only");
    let cancel=Arc::new(AtomicBool::new(false));let flag=cancel.clone();
    let trigger=std::thread::spawn(move||{std::thread::sleep(Duration::from_millis(250));flag.store(true,Ordering::Relaxed);});
    let started=Instant::now();let cancelled=collect(Path::new(r"C:\never-read-test-root"),cancel);let cancelled_ms=started.elapsed().as_millis();let _=trigger.join();
    let started=Instant::now();let timed_out=collect(Path::new(r"C:\never-read-test-root"),Arc::new(AtomicBool::new(false)));let timeout_ms=started.elapsed().as_millis();
    std::env::remove_var("SAFEPATCH_HELPER_CONTROL_DELAY");
    let passed=cancelled.err().as_deref()==Some("CANCELLED")&&cancelled_ms<2000&&timed_out.err().as_deref()==Some("NPM_TIMEOUT")&&(15000..18000).contains(&timeout_ms);
    let report=serde_json::json!({"passed":passed,"test_kind":"authored_owned_helper_control_only","cancellation_ms":cancelled_ms,"timeout_ms":timeout_ms,"children_killed_and_waited":passed});
    if let Ok(exe)=std::env::current_exe(){if let Some(dir)=exe.parent(){let _=std::fs::write(dir.join("npm-helper-control.json"),report.to_string());}}
    std::process::exit(if passed{0}else{1});
}

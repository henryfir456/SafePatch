use crate::model::AdapterOutput;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::{Duration, Instant};
pub const SCRIPT:&str=include_str!("../adapters/windows_inventory.ps1");
const MAX_OUTPUT:u64=8*1024*1024;

#[cfg(windows)]
struct OwnedJob(windows_sys::Win32::Foundation::HANDLE);
#[cfg(windows)]
impl OwnedJob {
    fn assign(child:&std::process::Child)->Result<Self,String>{
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::{Foundation::CloseHandle,System::JobObjects::*};
        unsafe {
            let handle=CreateJobObjectW(std::ptr::null(),std::ptr::null());
            if handle.is_null(){return Err("SCAN_JOB_FAILED".into())}
            let mut info:JOBOBJECT_EXTENDED_LIMIT_INFORMATION=std::mem::zeroed();
            info.BasicLimitInformation.LimitFlags=JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            if SetInformationJobObject(handle,JobObjectExtendedLimitInformation,&info as *const _ as *const _,std::mem::size_of_val(&info) as u32)==0
                || AssignProcessToJobObject(handle,child.as_raw_handle() as _)==0 {
                CloseHandle(handle);return Err("SCAN_JOB_FAILED".into())
            }
            Ok(Self(handle))
        }
    }
}
#[cfg(windows)]
impl Drop for OwnedJob {fn drop(&mut self){unsafe{windows_sys::Win32::Foundation::CloseHandle(self.0);}}}

#[cfg(windows)]
fn powershell()->Result<std::path::PathBuf,String>{
    // GetWindowsDirectoryW avoids PATH and an attacker-controlled SystemRoot variable.
    #[link(name="kernel32")]
    extern "system" { fn GetWindowsDirectoryW(buffer:*mut u16,size:u32)->u32; }
    let mut buffer=[0u16;32768];
    let size=unsafe{GetWindowsDirectoryW(buffer.as_mut_ptr(),buffer.len() as u32)} as usize;
    if size==0 || size>=buffer.len(){return Err("WINDOWS_DIRECTORY_UNAVAILABLE".into())}
    Ok(std::path::PathBuf::from(String::from_utf16(&buffer[..size]).map_err(|_|"WINDOWS_DIRECTORY_UNAVAILABLE")?).join("System32/WindowsPowerShell/v1.0/powershell.exe"))
}

#[cfg(not(windows))]
fn powershell()->Result<std::path::PathBuf,String>{Err("WINDOWS_REQUIRED".into())}

pub fn collect(cancel:Arc<AtomicBool>)->Result<AdapterOutput,String>{
    let exe=powershell()?;
    let dir=tempfile::tempdir().map_err(|_|"SCAN_TEMP_FAILED")?;
    let cancel_file=dir.path().join("cancel");
    let mut command=Command::new(exe);
    command.args(["-NoLogo","-NoProfile","-NonInteractive","-Command","-"])
        .env("SAFEPATCH_CANCEL_FILE",&cancel_file).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
    let mut child=command.spawn().map_err(|_|"ADAPTER_LAUNCH_FAILED")?;
    // Before supplying any script, contain only our child and its descendants.
    // Handles close on app exit too, preventing orphan WinGet/PowerShell processes.
    #[cfg(windows)]
    let _job=match OwnedJob::assign(&child){Ok(job)=>job,Err(code)=>{let _=child.kill();let _=child.wait();return Err(code)}};
    let mut stdin=child.stdin.take().ok_or("ADAPTER_INPUT_FAILED")?;
    let stdout=child.stdout.take().ok_or("ADAPTER_OUTPUT_FAILED")?;
    let stderr=child.stderr.take().ok_or("ADAPTER_OUTPUT_FAILED")?;
    // Drain pipes concurrently; do not retain stderr containing raw paths or data.
    let output_thread=std::thread::spawn(move||{let mut buffer=Vec::new();stdout.take(MAX_OUTPUT+1).read_to_end(&mut buffer).map(|_|buffer)});
    let error_thread=std::thread::spawn(move||{let mut stderr=stderr;std::io::copy(&mut stderr,&mut std::io::sink())});
    // Strip BOM/CRLF consistently and force a terminating newline for -Command -.
    let script=format!("{}\nexit\n",SCRIPT.replace("\r\n","\n"));
    if stdin.write_all(script.as_bytes()).is_err(){let _=child.kill();let _=child.wait();return Err("ADAPTER_INPUT_FAILED".into())}
    drop(stdin);
    let started=Instant::now();
    let status=loop {
        if cancel.load(Ordering::Relaxed){let _=std::fs::write(&cancel_file,b"cancel");}
        match child.try_wait(){Ok(Some(status))=>break status,Ok(None)=>{},Err(_)=>{let _=child.kill();let _=child.wait();return Err("ADAPTER_WAIT_FAILED".into())}}
        // Adapter bounds registry at 30s and WinGet at 45s; allow cleanup overhead.
        if started.elapsed()>Duration::from_secs(100){let _=child.kill();let _=child.wait();return Err("ADAPTER_TIMEOUT".into())}
        std::thread::sleep(Duration::from_millis(100));
    };
    let output=output_thread.join().map_err(|_|"ADAPTER_READ_FAILED")?.map_err(|_|"ADAPTER_READ_FAILED")?;
    let _=error_thread.join();
    if !status.success(){return Err("ADAPTER_FAILED".into())}
    if output.len() as u64>MAX_OUTPUT{return Err("ADAPTER_OUTPUT_LIMIT".into())}
    let json=std::str::from_utf8(&output).map_err(|_|"ADAPTER_ENCODING_FAILED")?.trim_start_matches('\u{feff}').trim();
    serde_json::from_str(json).map_err(|_|"ADAPTER_INVALID_JSON".into())
}

#[cfg(test)]mod tests{
    use super::*;
    #[test]fn fixed_adapter_has_no_install_upgrade_or_untrusted_binary_probe(){assert!(!SCRIPT.contains("winget install"));assert!(!SCRIPT.contains("winget upgrade"));assert!(SCRIPT.contains("Get-AuthenticodeSignature"));assert!(SCRIPT.contains("OpenSubKey($name,$false)"));assert!(!SCRIPT.contains("--accept-source-agreements"));}
    #[cfg(not(windows))]
    #[test]fn unsupported_os_is_an_error(){assert_eq!(collect(Arc::new(AtomicBool::new(false))).err().unwrap(),"WINDOWS_REQUIRED");}
}

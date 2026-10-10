//! Opt-in selected environment contracts; never execute discovered programs.
use serde::{Deserialize,Serialize};
use std::{path::Path,sync::atomic::AtomicBool};
use crate::python_metadata::VenvMetadata;
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonPackage {
    pub id:String,pub name:String,pub display_name:String,pub version:Option<String>,
    pub direct_status:String,pub source:String,pub installer:String,pub slot_hash:String,pub codes:Vec<String>,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonInterpreter {
    pub id:String,pub path_hash:String,pub version:Option<String>,pub file_version:Option<String>,
    pub product_name:Option<String>,pub publisher:Option<String>,pub source:String,pub codes:Vec<String>,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PythonSnapshot {
    pub environment_id:String,pub root_path_hash:String,pub observed_at:String,pub environment_kind:String,
    pub venv_metadata:Option<VenvMetadata>,pub interpreters:Vec<PythonInterpreter>,pub packages:Vec<PythonPackage>,pub codes:Vec<String>,
}
#[cfg(not(windows))]
pub fn read_python_environment(_root:&Path,_cancel:&AtomicBool)->Result<PythonSnapshot,&'static str>{Err("WINDOWS_REQUIRED")}
#[cfg(windows)]
mod local {
    use super::*;
    use crate::{model::hash,project_files::local::Root,python_metadata::{parse_distribution,parse_venv_config,exact_version,MAX_METADATA_BYTES},pe_metadata::{parse_pe_metadata,MAX_PE_BYTES}};
    use std::{collections::BTreeMap,fs,sync::atomic::Ordering};
    fn cancelled(cancel:&AtomicBool)->Result<(),&'static str>{if cancel.load(Ordering::Relaxed){Err("CANCELLED")}else{Ok(())}}
    fn error(code:&'static str)->&'static str{match code {
        "CANCELLED"=>"CANCELLED","NPM_ROOT_INVALID"|"NPM_ROOT_UNSUPPORTED"=>"PYTHON_ROOT_INVALID",
        "NPM_ROOT_UNREADABLE"=>"PYTHON_ROOT_UNREADABLE","NPM_ROOT_CHANGED"=>"PYTHON_ROOT_CHANGED",
        "NPM_METADATA_UNSAFE"=>"PYTHON_METADATA_UNSAFE","NPM_FILE_SIZE_LIMIT"=>"PYTHON_METADATA_SIZE_LIMIT",_=>"PYTHON_METADATA_UNREADABLE",
    }}
    fn read(root:&Root,name:&str,limit:usize,cancel:&AtomicBool)->Result<Option<Vec<u8>>,&'static str>{root.read_metadata(name,limit,cancel).map_err(error)}
    fn child(root:&Root,path:&Path)->Result<Root,&'static str>{
        root.unchanged().map_err(error)?;let child=Root::open(path).map_err(error)?;
        let prefix=format!("{}\\",root.path.to_string_lossy().trim_end_matches('\\')).to_ascii_lowercase();
        if !child.path.to_string_lossy().to_ascii_lowercase().starts_with(&prefix){return Err("PYTHON_METADATA_UNSAFE")}
        Ok(child)
    }
    fn marker(path:&Path)->bool{fs::symlink_metadata(path).is_ok()}
    fn runtime_version(value:&str)->bool{
        let release=value.split(|c:char|!c.is_ascii_digit()&&c!='.').next().unwrap_or("").trim_end_matches('.');
        release.split('.').count()==3&&exact_version(value)&&!value.contains(['!','+'])
    }
    pub fn collect(path:&Path,cancel:&AtomicBool)->Result<PythonSnapshot,&'static str>{
        cancelled(cancel)?;let root=Root::open(path).map_err(error)?;
        let venv=marker(&root.path.join("pyvenv.cfg"));let root_exe=marker(&root.path.join("python.exe"));let script_exe=marker(&root.path.join("Scripts/python.exe"));
        if !venv&&!root_exe&&!script_exe{return Err("PYTHON_ENVIRONMENT_NOT_RECOGNIZED")}
        let root_path_hash=hash(&root.path.to_string_lossy().to_ascii_lowercase());let environment_id=hash(&format!("python-environment:{root_path_hash}"));
        let mut snapshot=PythonSnapshot{environment_id,root_path_hash,observed_at:chrono::Utc::now().to_rfc3339(),environment_kind:if venv{"VENV_METADATA"}else{"INTERPRETER_DIRECTORY_METADATA"}.into(),venv_metadata:None,interpreters:Vec::new(),packages:Vec::new(),codes:vec!["PTH_EDITABLE_USER_AND_EXTRA_PATHS_NOT_SCANNED".into(),"LEGACY_EGG_INFO_AND_IMPORT_PATHS_NOT_SCANNED".into(),"INSTALLED_METADATA_NOT_CONTENT_PROOF".into()]};
        if venv {
            match read(&root,"pyvenv.cfg",65536,cancel).and_then(|b|b.ok_or("PYTHON_VENV_CONFIG_UNREADABLE")).and_then(|b|parse_venv_config(&b)){
                Ok(config)=>{snapshot.codes.extend(config.codes.clone());snapshot.venv_metadata=Some(config)},
                Err("CANCELLED")=>return Err("CANCELLED"),Err(code)=>snapshot.codes.push(code.into()),
            }
        }
        for (present,directory) in [(root_exe,None),(script_exe,Some("Scripts"))]{
            if !present{continue}cancelled(cancel)?;
            let parent=directory.map(|name|child(&root,&root.path.join(name))).transpose();
            let parent=match parent{Ok(p)=>p,Err(code)=>{snapshot.codes.push(code.into());continue}};let parent=parent.as_ref().unwrap_or(&root);
            let bytes=match read(parent,"python.exe",MAX_PE_BYTES,cancel){Ok(Some(b))=>b,Ok(None)=>continue,Err("CANCELLED")=>return Err("CANCELLED"),Err(code)=>{snapshot.codes.push(code.into());continue}};
            let path_hash=hash(&parent.path.join("python.exe").to_string_lossy().to_ascii_lowercase());
            let mut interpreter=PythonInterpreter{id:hash(&format!("python-interpreter:{path_hash}")),path_hash,version:None,file_version:None,product_name:None,publisher:None,source:"PE_VERSION_RESOURCE_METADATA".into(),codes:vec!["INTERPRETER_IDENTITY_NOT_VERIFIED".into(),"PE_RESOURCE_METADATA_NOT_EXECUTION_PROOF".into()]};
            match parse_pe_metadata(&bytes){
                Ok(pe)=>{interpreter.version=pe.product_version.filter(|v|runtime_version(v));interpreter.file_version=pe.file_version;interpreter.product_name=pe.product_name;interpreter.publisher=pe.company_name;},
                Err(code)=>interpreter.codes.push(code.into()),
            }
            if interpreter.version.is_none(){interpreter.codes.push("PYTHON_INTERPRETER_VERSION_UNKNOWN".into())}
            snapshot.interpreters.push(interpreter);
        }
        if snapshot.interpreters.is_empty(){snapshot.codes.push("PYTHON_INTERPRETER_NOT_FOUND".into())}
        let site_path=root.path.join("Lib/site-packages");
        let site=if marker(&site_path){match child(&root,&site_path){Ok(site)=>Some(site),Err(code)=>{snapshot.codes.push(code.into());None}}}else{snapshot.codes.push("PYTHON_SITE_PACKAGES_NOT_FOUND".into());None};
        if let Some(site)=site {
            let entries=fs::read_dir(&site.path).map_err(|_|"PYTHON_METADATA_UNREADABLE")?;let mut total=0usize;let mut names=BTreeMap::new();
            for (index,entry) in entries.enumerate(){
                cancelled(cancel)?;if index>=10000||snapshot.packages.len()>=2500{snapshot.codes.push("PYTHON_DISTRIBUTION_LIMIT".into());break}
                let entry=match entry{Ok(e)=>e,Err(_)=>{snapshot.codes.push("PYTHON_METADATA_UNREADABLE".into());continue}};
                let Some(name)=entry.file_name().to_str().map(str::to_owned)else{snapshot.codes.push("PYTHON_METADATA_UNSAFE".into());continue};
                if !name.to_ascii_lowercase().ends_with(".dist-info"){continue}
                let package_root=match child(&root,&site.path.join(&name)){Ok(r)=>r,Err(code)=>{snapshot.codes.push(code.into());continue}};
                let bytes=match read(&package_root,"METADATA",MAX_METADATA_BYTES,cancel){Ok(Some(b))=>b,Ok(None)=>{snapshot.codes.push("PYTHON_METADATA_MISSING".into());continue},Err("CANCELLED")=>return Err("CANCELLED"),Err(code)=>{snapshot.codes.push(code.into());continue}};
                total+=bytes.len();if total>32*1024*1024{snapshot.codes.push("PYTHON_TOTAL_METADATA_SIZE_LIMIT".into());break}
                let installer=match read(&package_root,"INSTALLER",128,cancel){Ok(value)=>value,Err("CANCELLED")=>return Err("CANCELLED"),Err(code)=>{snapshot.codes.push(code.into());None}};
                let metadata=match parse_distribution(&bytes,installer.as_deref()){Ok(p)=>p,Err(code)=>{snapshot.codes.push(code.into());continue}};
                *names.entry(metadata.name.clone()).or_insert(0usize)+=1;
                let slot_hash=hash(&package_root.path.to_string_lossy().to_ascii_lowercase());
                snapshot.packages.push(PythonPackage{id:hash(&format!("{}:{slot_hash}:{}:{}",snapshot.environment_id,metadata.name,metadata.version.as_deref().unwrap_or("UNKNOWN"))),name:metadata.name,display_name:metadata.display_name,version:metadata.version,direct_status:"UNKNOWN".into(),source:"DIST_INFO_METADATA".into(),installer:metadata.installer,slot_hash,codes:metadata.codes});
            }
            if names.values().any(|count|*count>1){snapshot.codes.push("POTENTIAL_DUPLICATE_DISTRIBUTION_METADATA".into())}
        }
        root.unchanged().map_err(error)?;cancelled(cancel)?;snapshot.packages.sort_by(|a,b|a.name.cmp(&b.name).then(a.id.cmp(&b.id)));snapshot.codes.sort();snapshot.codes.dedup();Ok(snapshot)
    }
}
#[cfg(windows)]
pub fn read_python_environment(root:&Path,cancel:&AtomicBool)->Result<PythonSnapshot,&'static str>{local::collect(root,cancel)}
#[cfg(all(test,windows))]
mod tests {
    use super::*;
    use std::{fs,path::Path,sync::atomic::AtomicBool};
    fn environment()->tempfile::TempDir {
        let t=tempfile::tempdir().unwrap();fs::write(t.path().join("pyvenv.cfg"),b"version = 3.13.2\ninclude-system-site-packages = false\nhome = C:\\outside-private\n").unwrap();t
    }
    fn distribution(root:&Path,slot:&str,name:&str,version:&str)->std::path::PathBuf {
        let d=root.join("Lib/site-packages").join(slot);fs::create_dir_all(&d).unwrap();
        fs::write(d.join("METADATA"),format!("Metadata-Version: 2.6\nName: {name}\nVersion: {version}\n\nprivate body")).unwrap();fs::write(d.join("INSTALLER"),b"pip\n").unwrap();d
    }
    #[test]fn selected_venv_metadata_retains_distinct_slots_versions_and_unknown_runtime(){
        let t=environment();distribution(t.path(),"Demo_pkg-1.0.dist-info","Demo_pkg","1.0");distribution(t.path(),"demo_pkg-2.0.dist-info","demo-pkg","2.0");
        let s=read_python_environment(t.path(),&AtomicBool::new(false)).unwrap();assert_eq!(s.environment_kind,"VENV_METADATA");assert_eq!(s.packages.len(),2);assert!(s.interpreters.is_empty());
        assert_ne!(s.packages[0].id,s.packages[1].id);assert!(s.packages.iter().all(|p|p.name=="demo-pkg"&&p.direct_status=="UNKNOWN"&&p.source=="DIST_INFO_METADATA"));
        assert!(s.codes.contains(&"PYTHON_INTERPRETER_NOT_FOUND".into()));assert!(s.codes.contains(&"POTENTIAL_DUPLICATE_DISTRIBUTION_METADATA".into()));
        let output=serde_json::to_string(&s).unwrap();assert!(!output.contains("outside-private")&&!output.contains("private body")&&!output.contains(&t.path().to_string_lossy().to_string()));
    }
    #[test]fn unsupported_root_and_missing_site_packages_do_not_invent_results(){
        let empty=tempfile::tempdir().unwrap();assert_eq!(read_python_environment(empty.path(),&AtomicBool::new(false)).err(),Some("PYTHON_ENVIRONMENT_NOT_RECOGNIZED"));
        let t=environment();let s=read_python_environment(t.path(),&AtomicBool::new(false)).unwrap();assert!(s.packages.is_empty());assert!(s.codes.contains(&"PYTHON_SITE_PACKAGES_NOT_FOUND".into()));
        assert_eq!(read_python_environment(Path::new(r"\\server\private"),&AtomicBool::new(false)).err(),Some("PYTHON_ROOT_INVALID"));
    }
    #[test]fn metadata_hardlinks_never_read_outside_selected_environment(){
        let t=environment();let outside=tempfile::tempdir().unwrap();let d=distribution(t.path(),"trap-1.0.dist-info","trap","1.0");
        fs::remove_file(d.join("METADATA")).unwrap();fs::write(outside.path().join("secret"),b"Metadata-Version: 2.6\nName: outside-secret\nVersion: 1.0\n\n").unwrap();fs::hard_link(outside.path().join("secret"),d.join("METADATA")).unwrap();
        let s=read_python_environment(t.path(),&AtomicBool::new(false)).unwrap();assert!(s.packages.is_empty());assert!(s.codes.contains(&"PYTHON_METADATA_UNSAFE".into()));assert!(!serde_json::to_string(&s).unwrap().contains("outside-secret"));
    }
    #[test]fn corrupt_oversized_metadata_are_partial_and_do_not_hide_valid_distribution(){
        let t=environment();distribution(t.path(),"valid-1.0.dist-info","valid","1.0");let bad=distribution(t.path(),"bad-1.0.dist-info","bad","1.0");fs::write(bad.join("METADATA"),vec![b'a';crate::python_metadata::MAX_METADATA_BYTES+1]).unwrap();
        let s=read_python_environment(t.path(),&AtomicBool::new(false)).unwrap();assert_eq!(s.packages.len(),1);assert_eq!(s.packages[0].name,"valid");assert!(s.codes.contains(&"PYTHON_METADATA_SIZE_LIMIT".into()));
    }
    #[test]fn cancellation_and_ignored_pth_never_trigger_interpreter_or_path_expansion(){
        let t=environment();let d=distribution(t.path(),"valid-1.0.dist-info","valid","1.0");fs::write(d.parent().unwrap().join("trap.pth"),b"import malicious; run()\nC:\\outside-private").unwrap();
        assert_eq!(read_python_environment(t.path(),&AtomicBool::new(true)).err(),Some("CANCELLED"));
        let s=read_python_environment(t.path(),&AtomicBool::new(false)).unwrap();assert_eq!(s.packages.len(),1);assert!(s.codes.contains(&"PTH_EDITABLE_USER_AND_EXTRA_PATHS_NOT_SCANNED".into()));
        assert!(!serde_json::to_string(&s).unwrap().contains("malicious"));
    }
}

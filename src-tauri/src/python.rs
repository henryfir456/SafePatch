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
fn digest(value:&str)->bool{value.len()==64&&value.bytes().all(|c|c.is_ascii_digit()||matches!(c,b'a'..=b'f'))}
fn codes(values:&[String])->bool{values.len()<=128&&values.iter().all(|v|!v.is_empty()&&v.len()<=96&&v.bytes().all(|c|c.is_ascii_uppercase()||c==b'_'))}
fn display(value:&Option<String>)->bool{value.as_ref().is_none_or(|v|!v.is_empty()&&v.len()<=256&&!v.contains(['\\','/',':','@'])&&!v.chars().any(char::is_control))}
fn resource_version(value:&str)->bool{value.len()<=128&&value.split('.').count()<=4&&value.split('.').all(|s|!s.is_empty()&&s.bytes().all(|c|c.is_ascii_digit()))}
pub fn validate_snapshot(snapshot:&PythonSnapshot)->Result<(),&'static str>{
    use crate::{model::hash,python_metadata::{valid_name,normalized_name,exact_version,runtime_version}};
    let bad="PYTHON_SNAPSHOT_INVALID";
    if !digest(&snapshot.root_path_hash)||snapshot.environment_id!=hash(&format!("python-environment:{}",snapshot.root_path_hash))||chrono::DateTime::parse_from_rfc3339(&snapshot.observed_at).is_err()||snapshot.observed_at.len()>64||!matches!(snapshot.environment_kind.as_str(),"VENV_METADATA"|"INTERPRETER_DIRECTORY_METADATA")||snapshot.packages.len()>2500||snapshot.interpreters.len()>2||!codes(&snapshot.codes){return Err(bad)}
    if let Some(config)=&snapshot.venv_metadata{if !codes(&config.codes)||config.declared_version.as_ref().is_some_and(|v|!runtime_version(v)){return Err(bad)}}
    let mut ids=std::collections::BTreeSet::new();let mut slots=std::collections::BTreeSet::new();
    for p in &snapshot.packages{
        if !digest(&p.slot_hash)||!ids.insert(&p.id)||!slots.insert(&p.slot_hash)||!valid_name(&p.display_name)||p.name!=normalized_name(&p.display_name)||p.version.as_ref().is_some_and(|v|!exact_version(v))||p.direct_status!="UNKNOWN"||p.source!="DIST_INFO_METADATA"||!matches!(p.installer.as_str(),"PIP_INSTALLER_METADATA"|"UV_INSTALLER_METADATA"|"CONDA_INSTALLER_METADATA"|"UNKNOWN")||!codes(&p.codes)||p.id!=hash(&format!("{}:{}:{}:{}",snapshot.environment_id,p.slot_hash,p.name,p.version.as_deref().unwrap_or("UNKNOWN"))){return Err(bad)}
    }
    let mut paths=std::collections::BTreeSet::new();
    for p in &snapshot.interpreters{
        if !digest(&p.path_hash)||!paths.insert(&p.path_hash)||p.id!=hash(&format!("python-interpreter:{}",p.path_hash))||p.version.as_ref().is_some_and(|v|!runtime_version(v))||p.file_version.as_ref().is_some_and(|v|!resource_version(v))||!display(&p.product_name)||!display(&p.publisher)||p.source!="PE_VERSION_RESOURCE_METADATA"||!codes(&p.codes){return Err(bad)}
    }
    Ok(())
}
#[cfg(not(windows))]
pub fn read_python_environment(_root:&Path,_cancel:&AtomicBool)->Result<PythonSnapshot,&'static str>{Err("WINDOWS_REQUIRED")}
#[cfg(windows)]
mod local {
    use super::*;
    use crate::{model::hash,project_files::local::Root,python_metadata::{parse_distribution,parse_venv_config,runtime_version,MAX_METADATA_BYTES},pe_metadata::{parse_pe_metadata,MAX_PE_BYTES}};
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
    fn marker(root:&Root,name:&str)->Result<bool,&'static str>{
        if name.is_empty()||name.contains(['\\','/',':'])||name=="."||name==".."||name.chars().any(char::is_control){return Err("PYTHON_METADATA_UNSAFE")}
        root.unchanged().map_err(error)?;Ok(fs::symlink_metadata(root.path.join(name)).is_ok())
    }
    pub fn collect(path:&Path,cancel:&AtomicBool)->Result<PythonSnapshot,&'static str>{
        cancelled(cancel)?;let root=Root::open(path).map_err(error)?;
        let venv=marker(&root,"pyvenv.cfg")?;let root_exe=marker(&root,"python.exe")?;
        let scripts=if marker(&root,"Scripts")?{Some(child(&root,&root.path.join("Scripts")))}else{None};
        let script_exe=match scripts.as_ref(){Some(Ok(scripts))=>marker(scripts,"python.exe")?,_=>false};
        if !venv&&!root_exe{if let Some(Err(code))=scripts.as_ref(){return Err(code)}}
        if !venv&&!root_exe&&!script_exe{return Err("PYTHON_ENVIRONMENT_NOT_RECOGNIZED")}
        let root_path_hash=hash(&root.path.to_string_lossy().to_ascii_lowercase());let environment_id=hash(&format!("python-environment:{root_path_hash}"));
        let mut snapshot=PythonSnapshot{environment_id,root_path_hash,observed_at:chrono::Utc::now().to_rfc3339(),environment_kind:if venv{"VENV_METADATA"}else{"INTERPRETER_DIRECTORY_METADATA"}.into(),venv_metadata:None,interpreters:Vec::new(),packages:Vec::new(),codes:vec!["PTH_EDITABLE_USER_AND_EXTRA_PATHS_NOT_SCANNED".into(),"LEGACY_EGG_INFO_AND_IMPORT_PATHS_NOT_SCANNED".into(),"INSTALLED_METADATA_NOT_CONTENT_PROOF".into()]};
        if let Some(Err(code))=scripts.as_ref(){snapshot.codes.push((*code).into())}
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
                Ok(pe)=>{interpreter.version=pe.product_version.filter(|v|runtime_version(v));interpreter.file_version=pe.file_version;interpreter.product_name=pe.product_name;interpreter.publisher=pe.company_name;interpreter.codes.extend(pe.codes);},
                Err(code)=>interpreter.codes.push(code.into()),
            }
            if interpreter.version.is_none(){interpreter.codes.push("PYTHON_INTERPRETER_VERSION_UNKNOWN".into())}
            snapshot.interpreters.push(interpreter);
        }
        if snapshot.interpreters.is_empty(){snapshot.codes.push("PYTHON_INTERPRETER_NOT_FOUND".into())}
        let site=if marker(&root,"Lib")?{
            match child(&root,&root.path.join("Lib")){
                Ok(lib)=>if marker(&lib,"site-packages")?{match child(&root,&lib.path.join("site-packages")){Ok(site)=>Some(site),Err(code)=>{snapshot.codes.push(code.into());None}}}else{snapshot.codes.push("PYTHON_SITE_PACKAGES_NOT_FOUND".into());None},
                Err(code)=>{snapshot.codes.push(code.into());None},
            }
        }else{snapshot.codes.push("PYTHON_SITE_PACKAGES_NOT_FOUND".into());None};
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
    #[cfg(test)]mod marker_contracts{
        use super::*;
        #[test]fn markers_cannot_query_through_unverified_descendant_paths(){let t=tempfile::tempdir().unwrap();let root=Root::open(t.path()).unwrap();for name in ["Scripts/python.exe","Lib\\site-packages","..","C:\\outside"]{assert_eq!(marker(&root,name).err(),Some("PYTHON_METADATA_UNSAFE"));}assert!(!marker(&root,"python.exe").unwrap());}
    }
}
#[cfg(windows)]
pub fn read_python_environment(root:&Path,cancel:&AtomicBool)->Result<PythonSnapshot,&'static str>{local::collect(root,cancel)}
#[cfg(test)]pub(crate) fn fixture_snapshot()->PythonSnapshot{
    let root_path_hash=crate::model::hash("owned-test-root");let environment_id=crate::model::hash(&format!("python-environment:{root_path_hash}"));
    let slot_hash=crate::model::hash("owned-test-slot");let id=crate::model::hash(&format!("{environment_id}:{slot_hash}:demo:1.0"));
    PythonSnapshot{environment_id,root_path_hash,observed_at:"2026-10-10T00:00:00Z".into(),environment_kind:"VENV_METADATA".into(),venv_metadata:None,interpreters:vec![],packages:vec![PythonPackage{id,name:"demo".into(),display_name:"Demo".into(),version:Some("1.0".into()),direct_status:"UNKNOWN".into(),source:"DIST_INFO_METADATA".into(),installer:"PIP_INSTALLER_METADATA".into(),slot_hash,codes:vec!["INSTALLED_METADATA_NOT_CONTENT_PROOF".into()]}],codes:vec!["INSTALLED_METADATA_NOT_CONTENT_PROOF".into()]}
}
#[cfg(test)]mod validation_contracts{
    use super::*;
    #[test]fn cache_and_helper_reject_invented_relationships_paths_duplicate_identity_and_versions(){
        let valid=fixture_snapshot();assert!(validate_snapshot(&valid).is_ok());
        for field in 0..7{let mut invalid=valid.clone();match field{
            0=>invalid.packages[0].direct_status="DIRECT".into(),1=>invalid.root_path_hash="C:\\private".into(),
            2=>invalid.packages.push(invalid.packages[0].clone()),3=>invalid.packages[0].version=Some(">=1.0".into()),
            4=>invalid.packages[0].source="https://private@server".into(),5=>invalid.environment_id="a".repeat(64),
            _=>invalid.packages[0].id="a".repeat(64),
        }assert!(validate_snapshot(&invalid).is_err(),"case {field}");}
    }
    #[test]fn runtime_observations_and_venv_hints_require_python_release_shape(){
        for version in ["1","3!1.2.3","3.13.2+private"]{let mut s=fixture_snapshot();s.venv_metadata=Some(VenvMetadata{declared_version:Some(version.into()),include_system_packages:Some(false),codes:vec![]});assert!(validate_snapshot(&s).is_err(),"venv {version}");
            s.venv_metadata=None;let path_hash=crate::model::hash("owned-interpreter");s.interpreters.push(PythonInterpreter{id:crate::model::hash(&format!("python-interpreter:{path_hash}")),path_hash,version:Some(version.into()),file_version:None,product_name:None,publisher:None,source:"PE_VERSION_RESOURCE_METADATA".into(),codes:vec![]});assert!(validate_snapshot(&s).is_err(),"runtime {version}");}
        let mut s=fixture_snapshot();s.venv_metadata=Some(VenvMetadata{declared_version:Some("3.15.0rc1".into()),include_system_packages:Some(false),codes:vec![]});assert!(validate_snapshot(&s).is_ok());
    }
}
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
    fn junction(path:&Path,target:&Path){
        use std::{fs::OpenOptions,os::windows::{ffi::OsStrExt,fs::OpenOptionsExt,io::AsRawHandle}};
        #[link(name="kernel32")]extern "system"{fn DeviceIoControl(handle:*mut std::ffi::c_void,code:u32,input:*const std::ffi::c_void,input_size:u32,output:*mut std::ffi::c_void,output_size:u32,returned:*mut u32,overlapped:*mut std::ffi::c_void)->i32;}
        fs::create_dir(path).unwrap();let substitute:Vec<u16>=format!(r"\??\{}",target.display()).encode_utf16().collect();let print:Vec<u16>=target.as_os_str().encode_wide().collect();
        let mut bytes=Vec::new();bytes.extend(0xa0000003u32.to_le_bytes());bytes.extend(((8+2*(substitute.len()+1+print.len()+1)) as u16).to_le_bytes());bytes.extend(0u16.to_le_bytes());
        for value in [0,substitute.len()*2,(substitute.len()+1)*2,print.len()*2]{bytes.extend((value as u16).to_le_bytes())}for value in substitute.iter().chain([0].iter()).chain(print.iter()).chain([0].iter()){bytes.extend(value.to_le_bytes())}
        let handle=OpenOptions::new().write(true).custom_flags(0x02000000|0x00200000).open(path).unwrap();let mut returned=0;
        assert_ne!(unsafe{DeviceIoControl(handle.as_raw_handle() as _,0x900a4,bytes.as_ptr() as _,bytes.len() as u32,std::ptr::null_mut(),0,&mut returned,std::ptr::null_mut())},0);
    }
    #[test]fn scripts_and_lib_junctions_are_rejected_before_nested_metadata_probes(){
        for name in ["Scripts","Lib"]{let t=environment();let outside=tempfile::tempdir().unwrap();fs::write(outside.path().join("python.exe"),b"private-outside").unwrap();distribution(outside.path(),"outside-1.dist-info","outside-secret","1");
            let path=t.path().join(name);junction(&path,outside.path());let s=read_python_environment(t.path(),&AtomicBool::new(false)).unwrap();assert!(s.interpreters.is_empty()&&s.packages.is_empty());assert!(s.codes.contains(&"PYTHON_METADATA_UNSAFE".into()));assert!(!serde_json::to_string(&s).unwrap().contains("outside-secret"));fs::remove_dir(path).unwrap();
        }
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

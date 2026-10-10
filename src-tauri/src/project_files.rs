// Fixed metadata filenames only. No traversal of project source or package tools.
use crate::npm::DependencyGraph;
use std::{path::Path,sync::atomic::AtomicBool};
#[cfg(not(windows))]
pub fn read_npm_project(_root:&Path,_cancel:&AtomicBool)->Result<DependencyGraph,&'static str>{Err("WINDOWS_REQUIRED")}
#[cfg(windows)]
mod local {
    use super::*;
    use crate::{model::hash,npm::{parse_lock,MAX_LOCK_BYTES}};
    use std::{fs::{self,File,OpenOptions},io::Read,os::windows::{fs::{MetadataExt,OpenOptionsExt},io::AsRawHandle},path::PathBuf,sync::atomic::Ordering};
    use windows_sys::Win32::Storage::FileSystem::*;
    use windows_sys::Win32::System::WindowsProgramming::{DRIVE_FIXED, DRIVE_RAMDISK};
    fn cancelled(cancel:&AtomicBool)->Result<(),&'static str>{if cancel.load(Ordering::Relaxed){Err("CANCELLED")}else{Ok(())}}
    fn lexical_root(root:&Path)->Result<(),&'static str>{
        let text=root.to_str().ok_or("NPM_ROOT_INVALID")?;let bytes=text.as_bytes();
        if bytes.len()<3||bytes.len()>4096||!bytes[0].is_ascii_alphabetic()||bytes[1]!=b':'||!matches!(bytes[2],b'\\'|b'/')||text[2..].contains(':')||text.chars().any(char::is_control)||text.split(['\\','/']).any(|part|part=="."||part==".."){return Err("NPM_ROOT_INVALID")}
        let drive=[bytes[0] as u16,b':' as u16,b'\\' as u16,0];
        if !matches!(unsafe{GetDriveTypeW(drive.as_ptr())},DRIVE_FIXED|DRIVE_RAMDISK){return Err("NPM_ROOT_UNSUPPORTED")}
        Ok(())
    }
    fn safe_ancestors(path:&Path)->Result<(),&'static str>{
        for ancestor in path.ancestors(){
            let metadata=fs::symlink_metadata(ancestor).map_err(|_|"NPM_METADATA_UNREADABLE")?;
            if metadata.file_attributes()&FILE_ATTRIBUTE_REPARSE_POINT!=0{return Err("NPM_METADATA_UNSAFE")}
        }Ok(())
    }
    fn final_path(file:&File)->Result<PathBuf,&'static str>{
        let mut buffer=vec![0u16;32768];
        let count=unsafe{GetFinalPathNameByHandleW(file.as_raw_handle() as _,buffer.as_mut_ptr(),buffer.len() as u32,0)} as usize;
        if count==0||count>=buffer.len(){return Err("NPM_METADATA_UNSAFE")}
        let text=String::from_utf16(&buffer[..count]).map_err(|_|"NPM_METADATA_UNSAFE")?;
        let text=text.strip_prefix(r"\\?\").ok_or("NPM_METADATA_UNSAFE")?;
        let path=PathBuf::from(text);lexical_root(&path)?;Ok(path)
    }
    fn lexical_key(path:&Path)->String{path.to_string_lossy().replace('/',"\\").trim_end_matches('\\').to_ascii_lowercase()}
    fn selected_key(path:&Path)->Result<String,&'static str>{
        // Expand Windows 8.3 spelling only; this is not link canonicalization.
        // The directory and its parents are already held without DELETE sharing.
        use std::os::windows::ffi::OsStrExt;
        let wide:Vec<_>=path.as_os_str().encode_wide().chain(Some(0)).collect();let mut buffer=vec![0u16;32768];
        let count=unsafe{GetLongPathNameW(wide.as_ptr(),buffer.as_mut_ptr(),buffer.len() as u32)} as usize;
        if count==0||count>=buffer.len(){return Err("NPM_METADATA_UNSAFE")}
        let expanded=PathBuf::from(String::from_utf16(&buffer[..count]).map_err(|_|"NPM_METADATA_UNSAFE")?);
        Ok(lexical_key(&expanded))
    }
    struct Root {path:PathBuf,handle:File,_ancestors:Vec<File>}
    impl Root {
        fn open(path:&Path)->Result<Self,&'static str>{
            lexical_root(path)?;
            // Acquire from drive root down. OPEN_REPARSE_POINT applies to each leaf;
            // retained handles without DELETE sharing prevent ancestor replacement.
            // Never promote unexpected final-path resolution to a trusted boundary.
            let mut chain:Vec<_>=path.ancestors().collect();chain.reverse();let mut handles=Vec::new();
            for ancestor in chain {
                let handle=OpenOptions::new().read(true).share_mode(FILE_SHARE_READ|FILE_SHARE_WRITE)
                    .custom_flags(FILE_FLAG_BACKUP_SEMANTICS|FILE_FLAG_OPEN_REPARSE_POINT).open(ancestor).map_err(|_|"NPM_ROOT_UNREADABLE")?;
                let metadata=handle.metadata().map_err(|_|"NPM_ROOT_UNREADABLE")?;
                if !metadata.is_dir()||metadata.file_attributes()&FILE_ATTRIBUTE_REPARSE_POINT!=0||lexical_key(&final_path(&handle)?)!=selected_key(ancestor)?{return Err("NPM_METADATA_UNSAFE")}
                handles.push(handle);
            }
            let handle=handles.pop().ok_or("NPM_ROOT_INVALID")?;
            let path=final_path(&handle)?;Ok(Self{path,handle,_ancestors:handles})
        }
        fn unchanged(&self)->Result<(),&'static str>{if final_path(&self.handle)?!=self.path{return Err("NPM_ROOT_CHANGED")}Ok(())}
        fn marker_exists(&self,name:&str)->bool{fs::symlink_metadata(self.path.join(name)).is_ok()}
        fn read_metadata(&self,name:&str,limit:usize,cancel:&AtomicBool)->Result<Option<Vec<u8>>,&'static str>{
            cancelled(cancel)?;self.unchanged()?;let path=self.path.join(name);
            let metadata=match fs::symlink_metadata(&path){Ok(m)=>m,Err(e) if e.kind()==std::io::ErrorKind::NotFound=>return Ok(None),Err(_)=>return Err("NPM_METADATA_UNREADABLE")};
            if !metadata.is_file()||metadata.file_attributes()&FILE_ATTRIBUTE_REPARSE_POINT!=0{return Err("NPM_METADATA_UNSAFE")}
            safe_ancestors(&path)?;
            let file=OpenOptions::new().read(true).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT|FILE_FLAG_SEQUENTIAL_SCAN).open(&path).map_err(|_|"NPM_METADATA_UNREADABLE")?;
            let opened=file.metadata().map_err(|_|"NPM_METADATA_UNREADABLE")?;
            let mut info:BY_HANDLE_FILE_INFORMATION=unsafe{std::mem::zeroed()};
            if unsafe{GetFileInformationByHandle(file.as_raw_handle() as _,&mut info)}==0||info.nNumberOfLinks!=1||opened.file_attributes()&FILE_ATTRIBUTE_REPARSE_POINT!=0{return Err("NPM_METADATA_UNSAFE")}
            let final_name=final_path(&file)?;
            let prefix=format!("{}\\",self.path.to_string_lossy().trim_end_matches('\\')).to_ascii_lowercase();
            if !final_name.to_string_lossy().to_ascii_lowercase().starts_with(&prefix){return Err("NPM_METADATA_UNSAFE")}
            self.unchanged()?;
            if opened.len()>limit as u64{return Err("NPM_FILE_SIZE_LIMIT")}
            let mut bytes=Vec::new();file.take(limit as u64+1).read_to_end(&mut bytes).map_err(|_|"NPM_METADATA_UNREADABLE")?;
            cancelled(cancel)?;self.unchanged()?;
            if bytes.len()>limit{return Err("NPM_FILE_SIZE_LIMIT")}
            Ok(Some(bytes))
        }
    }
    pub fn read(root:&Path,cancel:&AtomicBool)->Result<DependencyGraph,&'static str>{
        cancelled(cancel)?;let root=Root::open(root)?;
        let (bytes,kind)=if let Some(bytes)=root.read_metadata("npm-shrinkwrap.json",MAX_LOCK_BYTES,cancel)?{(bytes,"npm-shrinkwrap")}
            else if let Some(bytes)=root.read_metadata("package-lock.json",MAX_LOCK_BYTES,cancel)?{(bytes,"package-lock")}
            else if root.marker_exists("pnpm-lock.yaml")||root.marker_exists("yarn.lock"){return Err("NPM_UNSUPPORTED_PROJECT_FORMAT")}
            else{return Err("NPM_LOCKFILE_NOT_FOUND")};
        let manifest=root.read_metadata("package.json",1024*1024,cancel);
        let root_hash=hash(&root.path.to_string_lossy().to_ascii_lowercase());
        let environment=hash(&format!("npm-project:{root_hash}"));
        let mut graph=parse_lock(&bytes,&environment,manifest.as_ref().ok().and_then(|v|v.as_deref()),cancel)?;
        graph.root_path_hash=root_hash;graph.lockfile_kind=kind.into();
        if manifest.is_err(){graph.codes.push("NPM_MANIFEST_UNAVAILABLE".into())}
        if root.marker_exists("pnpm-lock.yaml"){graph.codes.push("PNPM_FORMAT_NOT_SCANNED".into())}
        if root.marker_exists("yarn.lock"){graph.codes.push("YARN_FORMAT_NOT_SCANNED".into())}
        graph.codes.sort();graph.codes.dedup();cancelled(cancel)?;Ok(graph)
    }
    #[cfg(test)]mod tests{
        use super::*;
        #[test]fn verified_ancestor_handles_prevent_boundary_replacement(){
            let temporary=tempfile::tempdir().unwrap();let parent=temporary.path().join("parent");let selected=parent.join("project");fs::create_dir_all(&selected).unwrap();
            let root=Root::open(&selected).unwrap();
            assert!(fs::rename(&parent,temporary.path().join("outside")).is_err());
            assert!(fs::rename(&selected,parent.join("outside-project")).is_err());
            assert!(root.unchanged().is_ok());drop(root);
            fs::rename(&parent,temporary.path().join("outside")).unwrap();
        }
        #[test]fn junction_root_and_junction_ancestor_cannot_read_outside_metadata(){
            #[link(name="kernel32")]
            extern "system" {fn DeviceIoControl(handle:*mut std::ffi::c_void,code:u32,input:*const std::ffi::c_void,input_size:u32,output:*mut std::ffi::c_void,output_size:u32,returned:*mut u32,overlapped:*mut std::ffi::c_void)->i32;}
            use std::os::windows::ffi::OsStrExt;
            let temporary=tempfile::tempdir().unwrap();let target=temporary.path().join("outside");let leaf=target.join("project");fs::create_dir_all(&leaf).unwrap();
            fs::write(leaf.join("package-lock.json"),br#"{"lockfileVersion":3,"packages":{}}"#).unwrap();
            // Owned fixture, native mount-point reparse data. No shell or elevation.
            let junction=temporary.path().join("selected");fs::create_dir(&junction).unwrap();
            let substitute:Vec<u16>=format!(r"\??\{}",target.display()).encode_utf16().collect();let print:Vec<u16>=target.as_os_str().encode_wide().collect();
            let mut data=Vec::new();data.extend_from_slice(&0xa0000003u32.to_le_bytes());
            let payload_length=8+2*(substitute.len()+1+print.len()+1);data.extend_from_slice(&(payload_length as u16).to_le_bytes());data.extend_from_slice(&0u16.to_le_bytes());
            for value in [0,substitute.len()*2,(substitute.len()+1)*2,print.len()*2]{data.extend_from_slice(&(value as u16).to_le_bytes());}
            for value in substitute.iter().chain([0].iter()).chain(print.iter()).chain([0].iter()){data.extend_from_slice(&value.to_le_bytes());}
            let handle=OpenOptions::new().write(true).custom_flags(FILE_FLAG_BACKUP_SEMANTICS|FILE_FLAG_OPEN_REPARSE_POINT).open(&junction).unwrap();let mut returned=0;
            assert_ne!(unsafe{DeviceIoControl(handle.as_raw_handle() as _,0x900a4,data.as_ptr() as _,data.len() as u32,std::ptr::null_mut(),0,&mut returned,std::ptr::null_mut())},0,"owned junction fixture failed: {:?}",std::io::Error::last_os_error().raw_os_error());drop(handle);
            assert_eq!(Root::open(&junction).err(),Some("NPM_METADATA_UNSAFE"));
            assert_eq!(read(&junction.join("project"),&AtomicBool::new(false)).err(),Some("NPM_METADATA_UNSAFE"));
            fs::remove_dir(&junction).unwrap();assert!(leaf.join("package-lock.json").is_file());
        }
    }
}
#[cfg(windows)]
pub fn read_npm_project(root:&Path,cancel:&AtomicBool)->Result<DependencyGraph,&'static str>{local::read(root,cancel)}

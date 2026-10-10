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
    struct Root {path:PathBuf,handle:File}
    impl Root {
        fn open(path:&Path)->Result<Self,&'static str>{
            lexical_root(path)?;safe_ancestors(path)?;
            if !fs::metadata(path).map_err(|_|"NPM_ROOT_INVALID")?.is_dir(){return Err("NPM_ROOT_INVALID")}
            let handle=OpenOptions::new().read(true).custom_flags(FILE_FLAG_BACKUP_SEMANTICS|FILE_FLAG_OPEN_REPARSE_POINT).open(path).map_err(|_|"NPM_ROOT_UNREADABLE")?;
            if handle.metadata().map_err(|_|"NPM_ROOT_UNREADABLE")?.file_attributes()&FILE_ATTRIBUTE_REPARSE_POINT!=0{return Err("NPM_METADATA_UNSAFE")}
            let path=final_path(&handle)?;safe_ancestors(&path)?;Ok(Self{path,handle})
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
}
#[cfg(windows)]
pub fn read_npm_project(root:&Path,cancel:&AtomicBool)->Result<DependencyGraph,&'static str>{local::read(root,cancel)}

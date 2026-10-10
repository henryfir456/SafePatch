// A named kernel object exists only while this database owner's handle is alive.
// No mutex ownership is acquired, so drop may safely occur on a worker thread.
#[cfg(windows)]
pub struct InstanceLock(usize);
#[cfg(windows)]
impl InstanceLock {
    pub fn acquire(path:&std::path::Path)->Result<Self,String>{
        use windows_sys::Win32::{Foundation::{CloseHandle,GetLastError,ERROR_ALREADY_EXISTS},System::Threading::CreateMutexW};
        let name=format!("Local\\SafePatch-{}",crate::model::hash(&path.to_string_lossy().to_lowercase()));
        let wide:Vec<u16>=name.encode_utf16().chain(Some(0)).collect();
        unsafe {
            let handle=CreateMutexW(std::ptr::null(),0,wide.as_ptr());
            if handle.is_null(){return Err("DATABASE_LOCK_FAILED".into())}
            if GetLastError()==ERROR_ALREADY_EXISTS{CloseHandle(handle);return Err("DATABASE_IN_USE".into())}
            Ok(Self(handle as usize))
        }
    }
}
#[cfg(windows)]
impl Drop for InstanceLock {fn drop(&mut self){unsafe{windows_sys::Win32::Foundation::CloseHandle(self.0 as _);}}}

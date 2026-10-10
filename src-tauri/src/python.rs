//! Opt-in selected environment contracts; never execute discovered programs.
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

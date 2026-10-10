//! Pure, bounded metadata contracts. No interpreter or installed code runs.
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn distribution_headers_keep_exact_versions_and_normalized_identity() {
        let p=parse_distribution(b"Metadata-Version: 2.4\r\nName: Example_pkg\r\nVersion: 1!2.0rc1.post2.dev3+win_amd64\r\n\r\nDescription",Some(b"pip\n")).unwrap();
        assert_eq!(p.display_name,"Example_pkg");assert_eq!(p.name,"example-pkg");
        assert_eq!(p.version.as_deref(),Some("1!2.0rc1.post2.dev3+win_amd64"));
        assert_eq!(p.installer,"PIP_INSTALLER_METADATA");
        assert!(p.codes.contains(&"INSTALLED_METADATA_NOT_CONTENT_PROOF".into()));
    }
    #[test]
    fn missing_or_nonexact_versions_stay_unknown() {
        for version in [None,Some(">=1.2"),Some("https://user:secret@example.invalid/pkg"),Some("1.2.*"),Some("bad-version")] {
            let bytes=format!("Metadata-Version: 2.1\nName: known-package\n{}\n",version.map(|v|format!("Version: {v}\n")).unwrap_or_default());
            let p=parse_distribution(bytes.as_bytes(),None).unwrap();assert!(p.version.is_none());assert!(p.codes.contains(&"PYTHON_VERSION_UNKNOWN".into()));
            let serialized=serde_json::to_string(&p).unwrap();assert!(!serialized.contains("secret")&&!serialized.contains("example.invalid"));
        }
    }
    #[test]
    fn names_duplicates_folded_identity_and_poisoned_bytes_fail_closed() {
        for name in ["../outside","<script>alert(1)</script>","naïve","a@b","-prefix","suffix-"] {
            let bytes=format!("Metadata-Version: 2.1\nName: {name}\nVersion: 1.0\n\n");
            assert_eq!(parse_distribution(bytes.as_bytes(),None).err(),Some("PYTHON_PACKAGE_NAME_INVALID"));
        }
        for bytes in [b"Metadata-Version: 2.1\nName: a\nName: b\nVersion: 1\n\n".as_slice(),b"Metadata-Version: 2.1\nName: a\nVersion: 1\nversion: 2\n\n".as_slice(),b"Metadata-Version: 2.1\nName: a\n continuation\nVersion: 1\n\n".as_slice(),b"Metadata-Version: 2.1\nName: a\0\nVersion: 1\n\n".as_slice()] {
            assert!(parse_distribution(bytes,None).is_err());
        }
    }
    #[test]
    fn descriptions_paths_entrypoints_and_installer_output_are_not_retained() {
        let p=parse_distribution(b"Metadata-Version: 2.1\nName: valid\nVersion: 2.0\nHome-page: https://user:secret@example.invalid\nRequires-Dist: private-secret\n\nprivate file contents C:\\private\\project",Some(b"pip\nsecret\n")).unwrap();
        let value=serde_json::to_string(&p).unwrap();assert!(!value.contains("secret")&&!value.contains("private")&&!value.contains("example.invalid"));assert_eq!(p.installer,"UNKNOWN");
    }
    #[test]
    fn exact_python_package_versions_follow_pep440_not_semver_ranges() {
        for version in ["1","1.2","v1.2.3","1.0a1","1.0b2","1.0rc1","1.0.post1","1.0-1","1.0.dev2","2!1.0","1.0+local.1","1.0rc1.post2.dev3+win_amd64"] {assert!(exact_version(version),"valid version rejected: {version}");}
        for version in ["","latest","^1.0","~=1.0",">1","1.*","1.0,2.0","1.0/secret","1..0","!1.0","1!","1.0+","1.0++local","1.0.dev1.post2","1.0rc1rc2","1.0\n"] {assert!(!exact_version(version),"invalid version accepted: {version}");}
    }
    #[test]
    fn venv_config_records_hints_without_following_base_paths() {
        let cfg=parse_venv_config(b"home = C:\\private\\outside\ninclude-system-site-packages = true\nversion = 3.13.2\nexecutable = C:\\private\\python.exe\ncommand = malicious --run\n").unwrap();
        assert_eq!(cfg.declared_version.as_deref(),Some("3.13.2"));assert_eq!(cfg.include_system_packages,Some(true));
        assert!(cfg.codes.contains(&"BASE_AND_EXTRA_SITE_PACKAGES_NOT_SCANNED".into()));
        let value=serde_json::to_string(&cfg).unwrap();assert!(!value.contains("private")&&!value.contains("malicious"));
        assert!(parse_venv_config(b"version = 3.13.2\nversion = 3.12.0\n").is_err());
        let invalid=parse_venv_config(b"include-system-site-packages = maybe\nversion = not-a-version\n").unwrap();
        assert!(invalid.declared_version.is_none()&&invalid.include_system_packages.is_none());assert!(invalid.codes.contains(&"PYTHON_VENV_CONFIG_PARTIAL".into()));
    }
    #[test]
    fn corrupt_oversized_and_future_metadata_have_stable_errors() {
        assert_eq!(parse_distribution(&vec![b'a';MAX_METADATA_BYTES+1],None).err(),Some("PYTHON_METADATA_SIZE_LIMIT"));
        assert_eq!(parse_distribution(&[0xff],None).err(),Some("PYTHON_METADATA_INVALID"));
        assert_eq!(parse_distribution(b"Metadata-Version: 3.0\nName: a\nVersion: 1\n\n",None).err(),Some("PYTHON_METADATA_FORMAT_UNSUPPORTED"));
        assert!(parse_distribution(b"Metadata-Version: 2.1\nVersion: 1\n\n",None).is_err());
        assert_eq!(parse_venv_config(&vec![b'a';65537]).err(),Some("PYTHON_VENV_CONFIG_SIZE_LIMIT"));
    }
}

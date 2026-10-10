//! Pure, bounded metadata contracts. No interpreter or installed code runs.
use serde::{Deserialize,Serialize};
pub const MAX_METADATA_BYTES:usize=1024*1024;
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DistributionMetadata {
    pub display_name:String,pub name:String,pub version:Option<String>,pub installer:String,pub codes:Vec<String>,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VenvMetadata {
    pub declared_version:Option<String>,pub include_system_packages:Option<bool>,pub codes:Vec<String>,
}
fn digits(value:&str)->&str{&value[value.bytes().take_while(u8::is_ascii_digit).count()..]}
fn separator(value:&str)->&str{if value.as_bytes().first().is_some_and(|c|matches!(c,b'.'|b'_'|b'-')){&value[1..]}else{value}}
fn suffix<'a>(value:&'a str,tags:&[&str])->Option<&'a str>{
    let candidate=separator(value);
    tags.iter().find_map(|tag|candidate.strip_prefix(*tag).map(|rest|digits(separator(rest))))
}
/// PEP 440 syntax only. Preserves the exact declaration, without range matching.
pub fn exact_version(value:&str)->bool {
    if value.is_empty()||value.len()>128||!value.is_ascii()||value.bytes().any(|c|c.is_ascii_whitespace()||c.is_ascii_control()){return false}
    let lowered=value.to_ascii_lowercase();let mut public=lowered.strip_prefix('v').unwrap_or(&lowered);
    if let Some((base,local))=public.split_once('+'){
        if local.is_empty()||!local.split(['.','_','-']).all(|part|!part.is_empty()&&part.bytes().all(|c|c.is_ascii_alphanumeric())){return false}
        public=base;
    }
    if let Some((epoch,base))=public.split_once('!'){
        if epoch.is_empty()||!epoch.bytes().all(|c|c.is_ascii_digit()){return false}
        public=base;
    }
    let mut rest=digits(public);if rest.len()==public.len(){return false}
    while let Some(next)=rest.strip_prefix('.'){
        if !next.as_bytes().first().is_some_and(u8::is_ascii_digit){break}
        rest=digits(next);
    }
    if let Some(next)=suffix(rest,&["preview","alpha","beta","pre","rc","a","b","c"]){rest=next;}
    if let Some(next)=rest.strip_prefix('-').filter(|s|s.as_bytes().first().is_some_and(u8::is_ascii_digit)){rest=digits(next);}
    else if let Some(next)=suffix(rest,&["post","rev","r"]){rest=next;}
    if let Some(next)=suffix(rest,&["dev"]){rest=next;}
    rest.is_empty()
}
fn valid_name(name:&str)->bool{
    !name.is_empty()&&name.len()<=214&&name.as_bytes().first().is_some_and(u8::is_ascii_alphanumeric)&&name.as_bytes().last().is_some_and(u8::is_ascii_alphanumeric)&&name.bytes().all(|c|c.is_ascii_alphanumeric()||matches!(c,b'.'|b'_'|b'-'))
}
fn normalized_name(name:&str)->String{
    let mut result=String::new();let mut previous_separator=false;
    for c in name.bytes(){if matches!(c,b'.'|b'_'|b'-'){if !previous_separator{result.push('-')}previous_separator=true;}else{result.push(c.to_ascii_lowercase() as char);previous_separator=false;}}
    result
}
fn installer_kind(value:Option<&[u8]>)->String{
    match value.filter(|v|v.len()<=128).and_then(|v|std::str::from_utf8(v).ok()).map(str::trim){
        Some("pip")=>"PIP_INSTALLER_METADATA",Some("uv")=>"UV_INSTALLER_METADATA",Some("conda")=>"CONDA_INSTALLER_METADATA",_=>"UNKNOWN",
    }.into()
}
pub fn parse_distribution(bytes:&[u8],installer:Option<&[u8]>)->Result<DistributionMetadata,&'static str>{
    if bytes.len()>MAX_METADATA_BYTES{return Err("PYTHON_METADATA_SIZE_LIMIT")}
    let text=std::str::from_utf8(bytes).map_err(|_|"PYTHON_METADATA_INVALID")?;
    if text.contains('\0'){return Err("PYTHON_METADATA_INVALID")}
    let mut name=None;let mut version=None;let mut format=None;let mut previous=String::new();
    for (index,line) in text.lines().enumerate(){
        if line.is_empty(){break}
        if index>=256||line.len()>8192{return Err("PYTHON_METADATA_SIZE_LIMIT")}
        if line.starts_with([' ','\t']){
            if matches!(previous.as_str(),"name"|"version"|"metadata-version"){return Err("PYTHON_METADATA_INVALID")}
            continue;
        }
        let (key,value)=line.split_once(':').ok_or("PYTHON_METADATA_INVALID")?;
        if !key.bytes().all(|c|c.is_ascii_alphanumeric()||c==b'-'){return Err("PYTHON_METADATA_INVALID")}
        previous=key.to_ascii_lowercase();let value=value.trim();
        if value.chars().any(char::is_control){return Err("PYTHON_METADATA_INVALID")}
        let target=match previous.as_str(){"name"=>&mut name,"version"=>&mut version,"metadata-version"=>&mut format,_=>continue};
        if target.is_some(){return Err("PYTHON_METADATA_DUPLICATE")}
        if value.len()>256{return Err("PYTHON_METADATA_SIZE_LIMIT")}
        *target=Some(value.to_owned());
    }
    if !matches!(format.as_deref(),Some("1.0"|"1.1"|"1.2"|"2.0"|"2.1"|"2.2"|"2.3"|"2.4"|"2.5")){return Err("PYTHON_METADATA_FORMAT_UNSUPPORTED")}
    let display_name=name.ok_or("PYTHON_PACKAGE_NAME_INVALID")?;if !valid_name(&display_name){return Err("PYTHON_PACKAGE_NAME_INVALID")}
    let version=version.filter(|v|exact_version(v));let mut codes=vec!["INSTALLED_METADATA_NOT_CONTENT_PROOF".into()];
    if version.is_none(){codes.push("PYTHON_VERSION_UNKNOWN".into())}
    Ok(DistributionMetadata{name:normalized_name(&display_name),display_name,version,installer:installer_kind(installer),codes})
}
pub fn parse_venv_config(bytes:&[u8])->Result<VenvMetadata,&'static str>{
    if bytes.len()>65536{return Err("PYTHON_VENV_CONFIG_SIZE_LIMIT")}
    let text=std::str::from_utf8(bytes).map_err(|_|"PYTHON_VENV_CONFIG_INVALID")?;if text.contains('\0'){return Err("PYTHON_VENV_CONFIG_INVALID")}
    let mut version=None;let mut system=None;let mut seen_version=false;let mut seen_system=false;let mut partial=false;
    for (index,line) in text.lines().enumerate(){
        if index>=512||line.len()>4096{return Err("PYTHON_VENV_CONFIG_SIZE_LIMIT")}
        let line=line.trim();if line.is_empty()||line.starts_with('#'){continue}
        let Some((key,value))=line.split_once('=') else{partial=true;continue};
        let key=key.trim().to_ascii_lowercase();let value=value.trim();
        match key.as_str(){
            "version"=>{if seen_version{return Err("PYTHON_VENV_CONFIG_DUPLICATE")}seen_version=true;
                let release=value.split(|c:char|!c.is_ascii_digit()&&c!='.').next().unwrap_or("").trim_end_matches('.');
                if exact_version(value)&&release.split('.').count()==3&&!value.contains(['!','+']){version=Some(value.to_owned())}else{partial=true;}
            },
            "include-system-site-packages"=>{if seen_system{return Err("PYTHON_VENV_CONFIG_DUPLICATE")}seen_system=true;
                system=match value.to_ascii_lowercase().as_str(){"true"=>Some(true),"false"=>Some(false),_=>{partial=true;None}};
            },
            _=>{},
        }
    }
    partial|=!seen_version||!seen_system;
    let mut codes=vec!["VENV_CONFIG_METADATA_NOT_RUNTIME_PROOF".into(),"BASE_AND_EXTRA_SITE_PACKAGES_NOT_SCANNED".into()];
    if partial{codes.push("PYTHON_VENV_CONFIG_PARTIAL".into())}
    Ok(VenvMetadata{declared_version:version,include_system_packages:system,codes})
}
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
    fn current_core_metadata_versions_and_ascii_names_are_supported() {
        for format in ["1.0","1.1","1.2","2.1","2.2","2.3","2.4","2.5","2.6"] {
            let bytes=format!("Metadata-Version: {format}\nName: naive\nVersion: 1.0\n\n");
            assert_eq!(parse_distribution(bytes.as_bytes(),None).unwrap().name,"naive");
        }
        assert_eq!(parse_distribution(b"Metadata-Version: 2.0\nName: naive\nVersion: 1.0\n\n",None).err(),Some("PYTHON_METADATA_FORMAT_UNSUPPORTED"));
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

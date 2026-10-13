use serde::{Deserialize, Serialize};

pub const SOURCE: &str = "https://cdn.winget.microsoft.com/cache";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateEvidence {
    pub status: String,
    pub target_version: Option<String>,
    pub package_id: Option<String>,
    pub source_url: Option<String>,
    pub checked_at: String,
    pub reason: String,
}
impl Default for UpdateEvidence {
    fn default() -> Self { Self {status:"UNKNOWN".into(),target_version:None,package_id:None,source_url:None,checked_at:String::new(),reason:"UPDATE_NOT_CHECKED".into()} }
}
pub fn not_checked() -> String { "NOT_CHECKED".into() }
fn numeric(version: &str) -> Option<Vec<u64>> {
    if version.len()>128 {return None}
    let parts:Vec<_>=version.split('.').collect();
    if parts.is_empty()||parts.len()>8||parts.iter().any(|p|p.is_empty()||!p.bytes().all(|b|b.is_ascii_digit())){return None}
    let mut result:Vec<u64>=parts.into_iter().map(str::parse).collect::<Result<_,_>>().ok()?;
    while result.len()>1 && result.last()==Some(&0){result.pop();}
    Some(result)
}
pub fn validate(mut e:UpdateEvidence,scanner:&str,id:Option<&str>,installed:Option<&str>)->Result<UpdateEvidence,&'static str> {
    if scanner!="winget" || e.source_url.as_deref()!=Some(SOURCE) || e.package_id.as_deref()!=id || id.is_none()
        || id.is_some_and(|s|s.len()>256||!s.bytes().all(|b|b.is_ascii_alphanumeric()||b"._+-".contains(&b)))
        || chrono::DateTime::parse_from_rfc3339(&e.checked_at).is_err()
        || !matches!(e.status.as_str(),"UPDATE_AVAILABLE"|"NO_UPDATE_REPORTED"|"UNKNOWN"|"ERROR")
        || e.reason.len()>80||!e.reason.bytes().all(|b|b.is_ascii_uppercase()||b==b'_')
        || e.target_version.as_ref().is_some_and(|s|s.is_empty()||s.len()>128||s.chars().any(char::is_control))
        || (e.status!="UPDATE_AVAILABLE"&&e.target_version.is_some()) {
        return Err("ADAPTER_INVALID_UPDATE");
    }
    if matches!(e.status.as_str(),"UPDATE_AVAILABLE"|"NO_UPDATE_REPORTED") {
        if e.reason!="WINGET_EXACT_ID_VERSION" {return Err("ADAPTER_INVALID_UPDATE")}
        let versions=installed.and_then(numeric);
        if e.status=="UPDATE_AVAILABLE" {
            let target=e.target_version.as_deref().and_then(numeric);
            let is_newer=versions.zip(target).is_some_and(|(mut a,mut b)|{let width=a.len().max(b.len());a.resize(width,0);b.resize(width,0);b>a});
            if !is_newer {e.status="UNKNOWN".into();e.target_version=None;e.reason="VERSION_NOT_COMPARABLE_OR_NEWER".into();}
        } else if versions.is_none() {e.status="UNKNOWN".into();e.reason="VERSION_NOT_COMPARABLE_OR_NEWER".into();}
    }
    Ok(e)
}
#[cfg(test)]mod tests {
    use super::*;
    fn evidence(status:&str,target:Option<&str>)->UpdateEvidence{UpdateEvidence{status:status.into(),target_version:target.map(String::from),package_id:Some("Vendor.Product".into()),source_url:Some(SOURCE.into()),checked_at:"2026-10-13T05:00:00Z".into(),reason:"WINGET_EXACT_ID_VERSION".into()}}
    #[test]fn availability_and_no_report_are_separate_from_vulnerability(){let e=validate(evidence("UPDATE_AVAILABLE",Some("2.0")),"winget",Some("Vendor.Product"),Some("1.9")).unwrap();assert_eq!(e.status,"UPDATE_AVAILABLE");assert_eq!(validate(evidence("NO_UPDATE_REPORTED",None),"winget",Some("Vendor.Product"),Some("2.0")).unwrap().status,"NO_UPDATE_REPORTED");assert_eq!(not_checked(),"NOT_CHECKED");}
    #[test]fn unsupported_equal_downgrade_and_overflow_versions_are_unknown(){for (a,b) in [("beta","2"),("1","2beta"),("2.0","2"),("10","9"),("18446744073709551616","2"),("1..2","3")]{let e=validate(evidence("UPDATE_AVAILABLE",Some(b)),"winget",Some("Vendor.Product"),Some(a)).unwrap();assert_eq!(e.status,"UNKNOWN");assert!(e.target_version.is_none());}}
    #[test]fn numeric_comparison_is_not_lexical(){assert_eq!(validate(evidence("UPDATE_AVAILABLE",Some("10.0.0")),"winget",Some("Vendor.Product"),Some("9.9")).unwrap().status,"UPDATE_AVAILABLE");}
    #[test]fn mismatched_products_registry_claims_and_poisoned_sources_rejected(){assert!(validate(evidence("UPDATE_AVAILABLE",Some("2")),"winget",Some("Other.Product"),Some("1")).is_err());assert!(validate(evidence("UPDATE_AVAILABLE",Some("2")),"registry",Some("Vendor.Product"),Some("1")).is_err());let mut e=evidence("UPDATE_AVAILABLE",Some("2"));e.source_url=Some("javascript:alert(1)".into());assert!(validate(e,"winget",Some("Vendor.Product"),Some("1")).is_err());}
    #[test]fn failures_do_not_become_no_update(){for status in ["ERROR","UNKNOWN"]{let mut e=evidence(status,None);e.reason="WINGET_UPDATE_TIMEOUT".into();assert_eq!(validate(e,"winget",Some("Vendor.Product"),Some("1")).unwrap().status,status);}}
    #[test]fn legacy_defaults_are_explicitly_unchecked(){assert_eq!(UpdateEvidence::default().status,"UNKNOWN");assert_eq!(UpdateEvidence::default().reason,"UPDATE_NOT_CHECKED");}
}

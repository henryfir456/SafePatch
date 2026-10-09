use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Provenance {
    pub scanner: String,
    pub source_id: String,
    pub scope: String,
    pub view: String,
    pub catalog_source: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub display_name: String,
    pub publisher: Option<String>,
    pub installed_version: Option<String>,
    pub normalized_id: Option<String>,
    pub install_path_hash: Option<String>,
    pub installation_source: String,
    pub identity_confidence: String,
    pub duplicate_group: Option<String>,
    pub provenance: Vec<Provenance>,
    pub observed_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Coverage {
    pub scanner: String,
    pub status: String,
    pub count: usize,
    pub codes: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Scan {
    pub id: String,
    pub status: String,
    pub phase: String,
    pub progress: u8,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub coverage: Vec<Coverage>,
    pub error_code: Option<String>,
}
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawRecord {
    pub display_name: String,
    pub publisher: Option<String>,
    pub installed_version: Option<String>,
    pub normalized_id: Option<String>,
    pub install_path_hash: Option<String>,
    pub installation_source: String,
    pub scanner: String,
    pub source_id: String,
    pub scope: String,
    pub view: String,
    pub catalog_source: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdapterOutput {
    pub records: Vec<RawRecord>,
    pub coverage: Vec<Coverage>,
}
pub fn hash(value: &str) -> String {
    format!("{:x}", Sha256::digest(value.as_bytes()))
}
fn clean(value: Option<String>) -> Option<String> {
    value.map(|s| s.chars().filter(|c| !c.is_control()).take(1024).collect::<String>().trim().to_owned())
        .filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("unknown"))
}
fn hash_valid(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())
}
pub fn normalize(output: AdapterOutput, now: &str) -> Result<(Vec<Asset>, Vec<Coverage>), &'static str> {
    if output.records.len() > 40000 || output.coverage.len() > 8 {
        return Err("ADAPTER_LIMIT");
    }
    for c in &output.coverage {
        if !matches!(c.status.as_str(), "COMPLETE" | "PARTIAL" | "UNKNOWN")
            || c.scanner.len() > 80 || c.count > 40000 || c.codes.len() > 20
            || c.codes.iter().any(|s| s.len() > 80 || !s.bytes().all(|b| b.is_ascii_uppercase() || b == b'_')) {
            return Err("ADAPTER_INVALID_COVERAGE");
        }
    }
    let mut assets: BTreeMap<String, Asset> = BTreeMap::new();
    for r in output.records {
        if !matches!(r.scanner.as_str(), "registry" | "winget") || !hash_valid(&r.source_id)
            || r.install_path_hash.as_ref().is_some_and(|s| !hash_valid(s))
            || !matches!(r.scope.as_str(), "machine" | "user" | "unknown")
            || !matches!(r.view.as_str(), "Registry64" | "Registry32" | "catalog")
            || r.installation_source != "UNKNOWN" {
            return Err("ADAPTER_INVALID_RECORD");
        }
        let name = clean(Some(r.display_name)).ok_or("ADAPTER_EMPTY_NAME")?;
        let publisher = clean(r.publisher);
        let version = clean(r.installed_version);
        let normalized_id = clean(r.normalized_id);
        let strong_registry = r.scanner == "registry" && r.install_path_hash.is_some() && publisher.is_some() && version.is_some();
        // Collapse reflected registry views only with same key, scope, path and metadata.
        // Missing path/publisher/version, separate scopes or paths retain separate assets.
        let view_identity = if strong_registry { "shared" } else { &r.view };
        let identity = serde_json::to_string(&(&r.scanner, &r.source_id, &r.scope, view_identity,
            &name.to_lowercase(), &publisher.as_ref().map(|s| s.to_lowercase()), &version, &r.install_path_hash)).map_err(|_| "IDENTITY_FAILED")?;
        let id = hash(&identity);
        let provenance = Provenance { scanner: r.scanner, source_id: r.source_id, scope: r.scope, view: r.view, catalog_source: clean(r.catalog_source) };
        if let Some(existing) = assets.get_mut(&id) {
            if !existing.provenance.iter().any(|p| p.scanner == provenance.scanner && p.source_id == provenance.source_id && p.scope == provenance.scope && p.view == provenance.view) {
                existing.provenance.push(provenance);
            }
        } else {
            assets.insert(id.clone(), Asset { id, display_name: name, publisher, installed_version: version,
                normalized_id, install_path_hash: r.install_path_hash, installation_source: "UNKNOWN".into(),
                identity_confidence: "UNKNOWN".into(), duplicate_group: None, provenance: vec![provenance], observed_at: now.into() });
        }
    }
    let mut candidates: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for a in assets.values() {
        if let Some(version) = &a.installed_version {
            let identity = if let Some(id) = &a.normalized_id {
                Some(format!("id:{}", id.to_lowercase()))
            } else { a.publisher.as_ref().map(|p| format!("name:{}|publisher:{}", a.display_name.to_lowercase(), p.to_lowercase())) };
            if let Some(identity) = identity {
                let key = hash(&serde_json::to_string(&(identity, version)).map_err(|_| "IDENTITY_FAILED")?);
                candidates.entry(key).or_default().push(a.id.clone());
            }
        }
    }
    for (group, ids) in candidates {
        if ids.len() > 1 { for id in ids { let a = assets.get_mut(&id).ok_or("IDENTITY_FAILED")?; a.duplicate_group = Some(group.clone()); a.identity_confidence = "POTENTIAL".into(); } }
    }
    let mut result: Vec<Asset> = assets.into_values().collect();
    result.sort_by(|a,b| a.display_name.to_lowercase().cmp(&b.display_name.to_lowercase()).then(a.id.cmp(&b.id)));
    Ok((result, output.coverage))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record() -> RawRecord { RawRecord { display_name: "Example".into(),publisher:Some("Vendor".into()),installed_version:Some("1".into()),normalized_id:None,install_path_hash:Some(hash("path")),installation_source:"UNKNOWN".into(),scanner:"registry".into(),source_id:hash("key"),scope:"user".into(),view:"Registry64".into(),catalog_source:None } }
    fn run(records:Vec<RawRecord>)->Vec<Asset>{normalize(AdapterOutput{records,coverage:vec![]},"now").unwrap().0}
    #[test] fn reflected_views_merge_but_preserve_provenance(){let a=record();let mut b=a.clone();b.view="Registry32".into();let r=run(vec![a,b]);assert_eq!(r.len(),1);assert_eq!(r[0].provenance.len(),2);}
    #[test] fn multiple_versions_paths_scopes_and_vendors_stay_separate(){for field in 0..4 {let a=record();let mut b=a.clone();match field{0=>b.installed_version=Some("2".into()),1=>b.install_path_hash=Some(hash("another")),2=>b.scope="machine".into(),_=>b.publisher=Some("Other Vendor".into())};assert_eq!(run(vec![a,b]).len(),2);}}
    #[test] fn missing_metadata_does_not_merge_views(){let mut a=record();a.install_path_hash=None;let mut b=a.clone();b.view="Registry32".into();let r=run(vec![a,b]);assert_eq!(r.len(),2);assert!(r.iter().all(|a|a.identity_confidence=="POTENTIAL"));}
    #[test] fn vendor_collision_has_no_duplicate_group(){let a=record();let mut b=a.clone();b.publisher=Some("Other Vendor".into());assert!(run(vec![a,b]).iter().all(|a|a.duplicate_group.is_none()));}
    #[test] fn controls_and_unknown_version_are_handled_as_data(){let mut a=record();a.display_name="<script>\0payload</script>".into();a.installed_version=Some("Unknown".into());let r=run(vec![a]);assert_eq!(r[0].display_name,"<script>payload</script>");assert_eq!(r[0].installed_version,None);assert_eq!(r[0].identity_confidence,"UNKNOWN");}
    #[test] fn raw_paths_and_fake_scanner_are_rejected(){let mut a=record();a.install_path_hash=Some("C:\\Private".into());assert!(normalize(AdapterOutput{records:vec![a],coverage:vec![]},"now").is_err());let mut b=record();b.scanner="mock".into();assert!(normalize(AdapterOutput{records:vec![b],coverage:vec![]},"now").is_err());}
}

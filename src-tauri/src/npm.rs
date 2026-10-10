// Issue #7: lockfile declarations are data, not proof of installed/public packages.
use crate::model::hash;
use serde::{Deserialize,Serialize};
use serde_json::{Map,Value};
use std::{collections::{BTreeMap,BTreeSet},sync::atomic::{AtomicBool,Ordering}};
pub const MAX_LOCK_BYTES:usize=8*1024*1024;
const MAX_PACKAGES:usize=20000;
const MAX_EDGES:usize=50000;
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct DependencyPackage {
    pub id:String,pub name:String,pub version:Option<String>,pub direct_status:String,
    pub source:String,pub slot_hash:String,
}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct DependencyEdge {pub from_id:String,pub to_id:String,pub kinds:Vec<String>}
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct DependencyGraph {
    pub environment_id:String,pub lockfile_version:u8,pub lockfile_hash:String,
    pub packages:Vec<DependencyPackage>,pub edges:Vec<DependencyEdge>,pub codes:Vec<String>,
}
struct Declaration {slot:String,descriptor:Map<String,Value>}
fn check_cancel(cancel:&AtomicBool)->Result<(),&'static str>{if cancel.load(Ordering::Relaxed){Err("CANCELLED")}else{Ok(())}}
fn valid_name(name:&str)->bool {
    if name.is_empty()||name.len()>214{return false}
    let part=|s:&str|!s.is_empty()&&s!="."&&s!=".."&&s.bytes().all(|b|b.is_ascii_alphanumeric()||matches!(b,b'-'|b'_'|b'.'));
    if let Some(scoped)=name.strip_prefix('@'){let fields:Vec<_>=scoped.split('/').collect();fields.len()==2&&fields.iter().all(|s|part(s))}else{part(name)}
}
fn valid_location(slot:&str)->bool {
    !slot.is_empty()&&slot.len()<=2048&&!slot.starts_with('/')&&!slot.contains(['\\',':'])&&
        !slot.chars().any(char::is_control)&&slot.split('/').all(|p|!p.is_empty()&&p!="."&&p!="..")
}
fn slot_name(slot:&str)->Option<&str>{
    let name=slot.rsplit_once("node_modules/")?.1;
    valid_name(name).then_some(name)
}
fn exact_version(s:&str)->bool {
    if s.len()>256{return false}
    let identifiers=|v:&str,leading_zero:bool|!v.is_empty()&&v.split('.').all(|p|!p.is_empty()&&p.bytes().all(|b|b.is_ascii_alphanumeric()||b==b'-')&&
        (leading_zero||!p.bytes().all(|b|b.is_ascii_digit())||p.len()==1||!p.starts_with('0')));
    let (release,build)=s.split_once('+').map_or((s,None),|(a,b)|(a,Some(b)));
    if build.is_some_and(|b|!identifiers(b,true)){return false}
    let (core,prerelease)=release.split_once('-').map_or((release,None),|(a,b)|(a,Some(b)));
    if prerelease.is_some_and(|p|!identifiers(p,false)){return false}
    let parts:Vec<_>=core.split('.').collect();parts.len()==3&&parts.iter().all(|p|!p.is_empty()&&p.bytes().all(|b|b.is_ascii_digit())&&(p.len()==1||!p.starts_with('0')))
}
fn source_kind(descriptor:&Map<String,Value>)->String {
    if descriptor.get("link").and_then(Value::as_bool)==Some(true){return "LOCAL_OR_LINK_METADATA".into()}
    let resolved=descriptor.get("resolved").and_then(Value::as_str).or_else(||descriptor.get("version").and_then(Value::as_str)).unwrap_or("");
    if resolved.starts_with("https://registry.npmjs.org/"){"PUBLIC_REGISTRY_METADATA"}
    else if resolved.starts_with("git+")||resolved.starts_with("git:"){"GIT_METADATA"}
    else if resolved.starts_with("file:"){"LOCAL_OR_LINK_METADATA"}else{"UNKNOWN"}.into()
}
fn legacy_entries(map:&Map<String,Value>,parent:&str,depth:usize,entries:&mut Vec<Declaration>,codes:&mut BTreeSet<String>,cancel:&AtomicBool)->Result<(),&'static str>{
    if depth>64{return Err("NPM_DEPTH_LIMIT")}
    for (name,value) in map{
        check_cancel(cancel)?;
        if !valid_name(name){codes.insert("NPM_IDENTITY_INVALID".into());continue}
        let Some(descriptor)=value.as_object()else{codes.insert("NPM_ENTRY_INVALID".into());continue};
        let slot=if parent.is_empty(){format!("node_modules/{name}")}else{format!("{parent}/node_modules/{name}")};
        if !valid_location(&slot){codes.insert("NPM_LOCATION_INVALID".into());continue}
        if entries.len()>=MAX_PACKAGES{return Err("NPM_PACKAGE_LIMIT")}
        entries.push(Declaration{slot:slot.clone(),descriptor:descriptor.clone()});
        if let Some(nested)=descriptor.get("dependencies"){
            if let Some(nested)=nested.as_object(){legacy_entries(nested,&slot,depth+1,entries,codes,cancel)?}else{codes.insert("NPM_ENTRY_INVALID".into());}
        }
    }Ok(())
}
fn resolve_slot(from:&str,name:&str,slots:&BTreeMap<String,usize>)->Option<usize>{
    let mut directory=from;
    loop{
        if directory.rsplit('/').next()!=Some("node_modules"){
            let candidate=if directory.is_empty(){format!("node_modules/{name}")}else{format!("{directory}/node_modules/{name}")};
            if let Some(index)=slots.get(&candidate){return Some(*index)}
        }
        if directory.is_empty(){return None}
        directory=directory.rsplit_once('/').map_or("",|(parent,_)|parent);
    }
}
fn dependency_names<'a>(descriptor:&'a Map<String,Value>,fields:&[(&str,&str)],codes:&mut BTreeSet<String>)->Vec<(&'a str,String)>{
    let mut result=Vec::new();
    for (field,kind) in fields{
        if let Some(value)=descriptor.get(*field){
            let Some(names)=value.as_object()else{codes.insert("NPM_DEPENDENCY_METADATA_INVALID".into());continue};
            for (name,spec) in names{if valid_name(name)&&spec.as_str().is_some_and(|s|s.len()<=4096&&!s.chars().any(char::is_control)){result.push((name.as_str(),(*kind).into()))}else{codes.insert("NPM_DEPENDENCY_METADATA_INVALID".into());}}
        }
    }result
}
pub fn parse_lock(bytes:&[u8],environment_id:&str,manifest:Option<&[u8]>,cancel:&AtomicBool)->Result<DependencyGraph,&'static str>{
    check_cancel(cancel)?;
    if bytes.len()>MAX_LOCK_BYTES{return Err("NPM_FILE_SIZE_LIMIT")}
    if environment_id.len()!=64||!environment_id.bytes().all(|b|b.is_ascii_hexdigit()){return Err("NPM_ENVIRONMENT_INVALID")}
    let value:Value=serde_json::from_slice(bytes.strip_prefix(&[0xef,0xbb,0xbf]).unwrap_or(bytes)).map_err(|_|"NPM_INVALID_JSON")?;
    let root=value.as_object().ok_or("NPM_INVALID_JSON")?;
    let version=root.get("lockfileVersion").and_then(Value::as_u64).filter(|v|matches!(v,1..=3)).ok_or("NPM_UNSUPPORTED_LOCKFILE")? as u8;
    let mut codes=BTreeSet::from(["LOCKFILE_DECLARATIONS_NOT_INSTALLED_PROOF".to_owned()]);
    let mut entries=Vec::new();let mut metadata=BTreeMap::<String,Map<String,Value>>::new();
    if version==1{
        let dependencies=root.get("dependencies").and_then(Value::as_object).ok_or("NPM_PACKAGES_INVALID")?;
        legacy_entries(dependencies,"",0,&mut entries,&mut codes,cancel)?;
    }else{
        let packages=root.get("packages").and_then(Value::as_object).ok_or("NPM_PACKAGES_INVALID")?;
        if packages.len()>MAX_PACKAGES+1{return Err("NPM_PACKAGE_LIMIT")}
        for (slot,descriptor) in packages{
            check_cancel(cancel)?;
            let Some(descriptor)=descriptor.as_object()else{codes.insert("NPM_ENTRY_INVALID".into());continue};
            if slot.is_empty(){metadata.insert(slot.clone(),descriptor.clone());continue}
            if !valid_location(slot){codes.insert("NPM_LOCATION_INVALID".into());continue}
            metadata.insert(slot.clone(),descriptor.clone());entries.push(Declaration{slot:slot.clone(),descriptor:descriptor.clone()});
        }
    }
    let mut packages=Vec::new();let mut slots=BTreeMap::new();let mut descriptors=Vec::new();
    for entry in entries{
        check_cancel(cancel)?;
        let fallback=slot_name(&entry.slot);
        let name=match entry.descriptor.get("name"){Some(Value::String(name)) if valid_name(name)=>Some(name.as_str()),None=>fallback,_=>None};
        let Some(name)=name else{codes.insert("NPM_IDENTITY_INVALID".into());continue};
        let mut version_descriptor=&entry.descriptor;
        if entry.descriptor.get("link").and_then(Value::as_bool)==Some(true){
            let mut visited=BTreeSet::new();let mut found=false;
            for _ in 0..16{
                let Some(target)=version_descriptor.get("resolved").and_then(Value::as_str).filter(|s|valid_location(s))else{break};
                if !visited.insert(target.to_owned()){break}
                let Some(next)=metadata.get(target)else{break};version_descriptor=next;
                if next.get("link").and_then(Value::as_bool)!=Some(true){found=true;break}
            }
            if !found{codes.insert("NPM_LINK_TARGET_UNKNOWN".into());version_descriptor=&entry.descriptor;}
        }
        let version=version_descriptor.get("version").and_then(Value::as_str).filter(|v|exact_version(v)).map(str::to_owned);
        if version.is_none(){codes.insert("NPM_VERSION_UNKNOWN".into());}
        let index=packages.len();slots.insert(entry.slot.clone(),index);
        packages.push(DependencyPackage{id:hash(&format!("{environment_id}\0{}",entry.slot)),name:name.into(),version,direct_status:"UNKNOWN".into(),source:source_kind(&entry.descriptor),slot_hash:hash(&entry.slot)});
        descriptors.push(entry);
    }
    let direct_fields=[("dependencies","DEPENDENCY"),("devDependencies","DEV"),("optionalDependencies","OPTIONAL"),("peerDependencies","PEER")];
    let manifest_value=manifest.filter(|b|b.len()<=1024*1024).and_then(|b|serde_json::from_slice::<Value>(b).ok());
    if manifest.is_some()&&manifest_value.is_none(){codes.insert("NPM_MANIFEST_INVALID".into());}
    let direct_root=if version==1{manifest_value.as_ref().and_then(Value::as_object)}else{metadata.get("").or_else(||manifest_value.as_ref().and_then(Value::as_object))};
    if let Some(descriptor)=direct_root.filter(|d|direct_fields.iter().any(|(field,_)|d.get(*field).is_some_and(Value::is_object))){
        for package in &mut packages{package.direct_status="TRANSITIVE".into()}
        for (name,_) in dependency_names(descriptor,&direct_fields,&mut codes){
            if let Some(index)=resolve_slot("",name,&slots){packages[index].direct_status="DIRECT".into()}else{codes.insert("NPM_DEPENDENCY_UNRESOLVED".into());}
        }
    }else{codes.insert("NPM_DIRECT_RELATION_UNKNOWN".into());}
    let mut edges=BTreeMap::<(String,String),BTreeSet<String>>::new();
    for (index,entry) in descriptors.iter().enumerate(){
        check_cancel(cancel)?;
        let fields=if version==1{vec![("requires","DEPENDENCY")]}else{direct_fields.to_vec()};
        for (name,kind) in dependency_names(&entry.descriptor,&fields,&mut codes){
            if let Some(target)=resolve_slot(&entry.slot,name,&slots){
                if edges.len()>=MAX_EDGES{return Err("NPM_EDGE_LIMIT")}
                edges.entry((packages[index].id.clone(),packages[target].id.clone())).or_default().insert(kind);
            }else{codes.insert("NPM_DEPENDENCY_UNRESOLVED".into());}
        }
    }
    check_cancel(cancel)?;
    Ok(DependencyGraph{environment_id:environment_id.into(),lockfile_version:version,lockfile_hash:hash(&String::from_utf8_lossy(bytes)),packages,edges:edges.into_iter().map(|((from_id,to_id),kinds)|DependencyEdge{from_id,to_id,kinds:kinds.into_iter().collect()}).collect(),codes:codes.into_iter().collect()})
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};
    use std::sync::atomic::AtomicBool;
    fn parse(value:Value)->DependencyGraph {
        parse_lock(&serde_json::to_vec(&value).unwrap(),&"e".repeat(64),None,&AtomicBool::new(false)).unwrap()
    }
    fn modern(version:u8)->Value {json!({"lockfileVersion":version,"packages":{
        "":{"dependencies":{"a":"^1"},"devDependencies":{"@scope/dev":"2.0.0"}},
        "node_modules/a":{"version":"1.2.3","resolved":"https://registry.npmjs.org/a/-/a-1.2.3.tgz","dependencies":{"leaf":"^1"}},
        "node_modules/a/node_modules/leaf":{"version":"1.0.0"},
        "node_modules/leaf":{"version":"2.0.0"},
        "node_modules/@scope/dev":{"version":"2.0.0","dependencies":{"leaf":"2.0.0"}}
    }})}
    #[test]fn modern_versions_and_resolved_graph_preserve_nested_slots(){
        for version in [2,3]{let g=parse(modern(version));assert_eq!(g.packages.len(),4);assert_eq!(g.lockfile_version,version);
            let a=g.packages.iter().find(|p|p.name=="a").unwrap();let dev=g.packages.iter().find(|p|p.name=="@scope/dev").unwrap();
            let old=g.packages.iter().find(|p|p.version.as_deref()==Some("1.0.0")).unwrap();
            let newer=g.packages.iter().find(|p|p.name=="leaf"&&p.version.as_deref()==Some("2.0.0")).unwrap();
            assert_eq!(a.direct_status,"DIRECT");assert_eq!(old.direct_status,"TRANSITIVE");
            assert!(g.edges.iter().any(|e|e.from_id==a.id&&e.to_id==old.id));
            assert!(g.edges.iter().any(|e|e.from_id==dev.id&&e.to_id==newer.id));assert_ne!(old.id,newer.id);
        }
    }
    #[test]fn legacy_hoisting_does_not_fabricate_direct_relationships(){
        let lock=json!({"lockfileVersion":1,"dependencies":{"a":{"version":"1.0.0","requires":{"leaf":"^2"}},"leaf":{"version":"2.1.0"}}});
        let bytes=serde_json::to_vec(&lock).unwrap();let manifest=br#"{"dependencies":{"a":"^1"}}"#;
        let g=parse_lock(&bytes,&"e".repeat(64),Some(manifest),&AtomicBool::new(false)).unwrap();
        assert_eq!(g.packages.iter().filter(|p|p.direct_status=="DIRECT").count(),1);assert_eq!(g.edges.len(),1);
        let uncertain=parse(lock);assert!(uncertain.packages.iter().all(|p|p.direct_status=="UNKNOWN"));
    }
    #[test]fn version_specifiers_are_not_exact_observations(){
        let g=parse(json!({"lockfileVersion":3,"packages":{"":{"dependencies":{"a":"*"}},"node_modules/a":{"version":"https://user:secret@example.invalid/private.tgz","resolved":"https://user:secret@example.invalid/private.tgz"}}}));
        assert_eq!(g.packages.len(),1);assert!(g.packages[0].version.is_none());assert_ne!(g.packages[0].source,"PUBLIC_REGISTRY_METADATA");
        let text=serde_json::to_string(&g).unwrap();assert!(!text.contains("secret")&&!text.contains("example.invalid"));
    }
    #[test]fn npm_alias_uses_explicit_package_identity(){
        let g=parse(json!({"lockfileVersion":3,"packages":{"":{"dependencies":{"shortcut":"npm:real-package@^2"}},"node_modules/shortcut":{"name":"real-package","version":"2.0.0"}}}));
        assert_eq!(g.packages[0].name,"real-package");assert_eq!(g.packages[0].direct_status,"DIRECT");
    }
    #[test]fn unsupported_corrupt_oversized_and_cancelled_inputs_have_stable_errors(){
        for v in [0,4,99]{assert_eq!(parse_lock(&serde_json::to_vec(&json!({"lockfileVersion":v})).unwrap(),&"e".repeat(64),None,&AtomicBool::new(false)).err(),Some("NPM_UNSUPPORTED_LOCKFILE"));}
        assert_eq!(parse_lock(b"{",&"e".repeat(64),None,&AtomicBool::new(false)).err(),Some("NPM_INVALID_JSON"));
        assert_eq!(parse_lock(&vec![b' ';8*1024*1024+1],&"e".repeat(64),None,&AtomicBool::new(false)).err(),Some("NPM_FILE_SIZE_LIMIT"));
        assert_eq!(parse_lock(b"{}",&"e".repeat(64),None,&AtomicBool::new(true)).err(),Some("CANCELLED"));
    }
    #[test]fn poisoned_locations_names_and_versions_do_not_escape_metadata(){
        let g=parse(json!({"lockfileVersion":3,"packages":{"":{"dependencies":{"good":"*"}},"../secret":{"name":"evil","version":"1.0.0"},"node_modules/<script>":{"version":"1.0.0"},"node_modules/good":{"version":"01.2.3"},"node_modules/other":{"version":"1.2.3\u{0000}"}}}));
        assert_eq!(g.packages.len(),2);assert!(g.packages.iter().all(|p|p.version.is_none()));
        assert!(!serde_json::to_string(&g).unwrap().contains("../secret"));
    }
    #[test]fn unresolved_optional_and_peer_edges_are_explicit_gaps(){
        let g=parse(json!({"lockfileVersion":3,"packages":{"":{"dependencies":{"a":"1.0.0"}},"node_modules/a":{"version":"1.0.0","optionalDependencies":{"missing":"*"},"peerDependencies":{"peer":"*"}}}}));
        assert_eq!(g.packages.len(),1);assert!(g.edges.is_empty());assert!(g.codes.iter().any(|c|c=="NPM_DEPENDENCY_UNRESOLVED"));
    }
    #[test]fn link_targets_are_json_metadata_only_and_cycles_remain_unknown(){
        let g=parse(json!({"lockfileVersion":3,"packages":{"":{"dependencies":{"a":"file:../outside"}},"node_modules/a":{"link":true,"resolved":"../outside"},"node_modules/b":{"link":true,"resolved":"node_modules/c"},"node_modules/c":{"link":true,"resolved":"node_modules/b"}}}));
        assert_eq!(g.packages.len(),3);assert!(g.packages.iter().all(|p|p.version.is_none()));
        assert!(g.codes.iter().any(|c|c=="NPM_LINK_TARGET_UNKNOWN"));
        let text=serde_json::to_string(&g).unwrap();assert!(!text.contains("../outside")&&!text.contains("node_modules"));
    }
    #[test]fn environment_identity_prevents_cross_project_package_collision(){
        let bytes=serde_json::to_vec(&modern(3)).unwrap();
        let a=parse_lock(&bytes,&"a".repeat(64),None,&AtomicBool::new(false)).unwrap();let b=parse_lock(&bytes,&"b".repeat(64),None,&AtomicBool::new(false)).unwrap();
        assert!(a.packages.iter().all(|p|!b.packages.iter().any(|q|q.id==p.id)));
    }
}

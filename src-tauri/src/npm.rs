// Issue #7: test-first lockfile parser contract. No file or tool execution here.
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

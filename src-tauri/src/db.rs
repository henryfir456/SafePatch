use crate::model::{Asset, Scan};
use rusqlite::{params, Connection};
use std::path::Path;
pub const MIGRATION: &str = include_str!("../migrations/001_initial.sql");
pub const PROJECT_MIGRATION:&str=include_str!("../migrations/002_project_graphs.sql");
pub const MEMBERSHIP_MIGRATION:&str=include_str!("../migrations/003_current_membership.sql");
pub const PYTHON_MIGRATION:&str=include_str!("../migrations/004_python_snapshots.sql");
pub fn open(path:&Path)->rusqlite::Result<Connection>{
    let mut db=Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
    migrate(&mut db)?;Ok(db)
}
pub fn migrate(db:&mut Connection)->rusqlite::Result<()>{
    let version:i64=db.query_row("PRAGMA user_version",[],|r|r.get(0))?;
    if version>4{return Err(rusqlite::Error::InvalidQuery)}
    if version==0{let tx=db.transaction()?;tx.execute_batch(MIGRATION)?;tx.pragma_update(None,"user_version",1)?;tx.commit()?;}
    if version<2{let tx=db.transaction()?;tx.execute_batch(PROJECT_MIGRATION)?;tx.pragma_update(None,"user_version",2)?;tx.commit()?;}
    if version<3{let tx=db.transaction()?;tx.execute_batch(MEMBERSHIP_MIGRATION)?;tx.pragma_update(None,"user_version",3)?;tx.commit()?;}
    if version<4{
        let foreign_keys:bool=db.pragma_query_value(None,"foreign_keys",|r|r.get(0))?;
        db.pragma_update(None,"foreign_keys",false)?;
        let result=(||{let tx=db.transaction()?;tx.execute_batch(PYTHON_MIGRATION)?;
            {let mut check=tx.prepare("PRAGMA foreign_key_check")?;if check.query([])?.next()?.is_some(){return Err(rusqlite::Error::InvalidQuery)}}
            tx.pragma_update(None,"user_version",4)?;tx.commit()
        })();
        db.pragma_update(None,"foreign_keys",foreign_keys)?;result?;
    }
    Ok(())
}
pub fn interrupt(db:&Connection)->rusqlite::Result<usize>{db.execute("UPDATE scan_runs SET status='INTERRUPTED',finished_at=?1,error_summary='PROCESS_INTERRUPTED' WHERE status='RUNNING'",[chrono::Utc::now().to_rfc3339()])}
pub fn insert_scan(db:&Connection,scan:&Scan)->rusqlite::Result<()>{insert_scoped_scan(db,scan,"windows")}
pub fn insert_scoped_scan(db:&Connection,scan:&Scan,scope:&str)->rusqlite::Result<()>{db.execute("INSERT INTO scan_runs(id,started_at,status,scanner_versions,coverage_json,scope) VALUES (?1,?2,'RUNNING','safepatch/1','[]',?3)",params![scan.id,scan.started_at,scope])?;Ok(())}
pub fn finish(db:&mut Connection,scan:&Scan,assets:&[Asset])->Result<(),String>{
    finish_inner(db,scan,assets,None,None)
}
pub fn finish_project(db:&mut Connection,scan:&Scan,graph:Option<&crate::npm::DependencyGraph>)->Result<(),String>{finish_inner(db,scan,&[],graph,None)}
pub fn finish_python(db:&mut Connection,scan:&Scan,snapshot:Option<&crate::python::PythonSnapshot>)->Result<(),String>{finish_inner(db,scan,&[],None,snapshot)}
fn finish_inner(db:&mut Connection,scan:&Scan,assets:&[Asset],graph:Option<&crate::npm::DependencyGraph>,python:Option<&crate::python::PythonSnapshot>)->Result<(),String>{
    if let Some(graph)=graph{crate::npm::validate_graph(graph).map_err(String::from)?;}
    if let Some(snapshot)=python{crate::python::validate_snapshot(snapshot).map_err(String::from)?;}
    let tx=db.transaction().map_err(|_|"DATABASE_TRANSACTION_FAILED")?;
    for a in assets {
        let provenance=serde_json::to_string(&a.provenance).map_err(|_|"SERIALIZATION_FAILED")?;
        let snapshot=serde_json::to_string(a).map_err(|_|"SERIALIZATION_FAILED")?;
        tx.execute("INSERT INTO assets VALUES (?1,'windows',?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET observed_at=excluded.observed_at,provenance_json=excluded.provenance_json,identity_confidence=excluded.identity_confidence",params![a.id,a.display_name,a.publisher,a.normalized_id,a.installation_source,a.installed_version,a.install_path_hash,a.identity_confidence,provenance,a.observed_at]).map_err(|_|"DATABASE_ASSET_FAILED")?;
        tx.execute("INSERT INTO scan_assets VALUES (?1,?2,?3)",params![scan.id,a.id,snapshot]).map_err(|_|"DATABASE_SNAPSHOT_FAILED")?;
    }
    if let Some(g)=graph{
        tx.execute("INSERT INTO environments VALUES (?1,'npm',?2,?3,?4) ON CONFLICT(id) DO UPDATE SET observed_at=excluded.observed_at",params![g.environment_id,format!("npm-project {}",&g.root_path_hash[..12]),g.root_path_hash,g.observed_at]).map_err(|_|"DATABASE_ENVIRONMENT_FAILED")?;
        for p in &g.packages{
            tx.execute("INSERT INTO packages(id,environment_id,ecosystem,name,version,direct,source,observed_at,direct_status,slot_hash) VALUES (?1,?2,'npm',?3,?4,?5,?6,?7,?8,?9) ON CONFLICT(id) DO UPDATE SET name=excluded.name,version=excluded.version,direct=excluded.direct,source=excluded.source,observed_at=excluded.observed_at,direct_status=excluded.direct_status,slot_hash=excluded.slot_hash",params![p.id,g.environment_id,p.name,p.version,p.direct_status=="DIRECT",p.source,g.observed_at,p.direct_status,p.slot_hash]).map_err(|_|"DATABASE_PACKAGE_FAILED")?;
        }
        tx.execute("DELETE FROM dependency_edges WHERE environment_id=?1",[&g.environment_id]).map_err(|_|"DATABASE_GRAPH_FAILED")?;
        tx.execute("DELETE FROM environment_current_packages WHERE environment_id=?1",[&g.environment_id]).map_err(|_|"DATABASE_GRAPH_FAILED")?;
        for p in &g.packages{tx.execute("INSERT INTO environment_current_packages VALUES (?1,?2)",params![g.environment_id,p.id]).map_err(|_|"DATABASE_GRAPH_FAILED")?;}
        for e in &g.edges{
            tx.execute("INSERT INTO dependency_edges(id,from_package_id,to_package_id,environment_id,kinds_json) VALUES (?1,?2,?3,?4,?5)",params![crate::model::hash(&format!("{}:{}:{}",g.environment_id,e.from_id,e.to_id)),e.from_id,e.to_id,g.environment_id,serde_json::to_string(&e.kinds).map_err(|_|"SERIALIZATION_FAILED")?]).map_err(|_|"DATABASE_GRAPH_FAILED")?;
        }
        tx.execute("INSERT INTO scan_project_graphs VALUES (?1,?2,?3)",params![scan.id,g.environment_id,serde_json::to_string(g).map_err(|_|"SERIALIZATION_FAILED")?]).map_err(|_|"DATABASE_SNAPSHOT_FAILED")?;
    }
    if let Some(s)=python{
        tx.execute("INSERT INTO environments VALUES (?1,'python',?2,?3,?4) ON CONFLICT(id) DO UPDATE SET observed_at=excluded.observed_at",params![s.environment_id,format!("python-environment {}",&s.root_path_hash[..12]),s.root_path_hash,s.observed_at]).map_err(|_|"DATABASE_ENVIRONMENT_FAILED")?;
        for p in &s.packages{tx.execute("INSERT INTO packages(id,environment_id,ecosystem,name,version,direct,source,observed_at,direct_status,slot_hash) VALUES (?1,?2,'PyPI',?3,?4,0,?5,?6,'UNKNOWN',?7) ON CONFLICT(id) DO UPDATE SET name=excluded.name,version=excluded.version,source=excluded.source,observed_at=excluded.observed_at",params![p.id,s.environment_id,p.name,p.version,p.source,s.observed_at,p.slot_hash]).map_err(|_|"DATABASE_PACKAGE_FAILED")?;}
        tx.execute("DELETE FROM environment_current_packages WHERE environment_id=?1",[&s.environment_id]).map_err(|_|"DATABASE_MEMBERSHIP_FAILED")?;
        for p in &s.packages{tx.execute("INSERT INTO environment_current_packages VALUES (?1,?2)",params![s.environment_id,p.id]).map_err(|_|"DATABASE_MEMBERSHIP_FAILED")?;}
        tx.execute("INSERT INTO scan_python_snapshots VALUES (?1,?2,?3)",params![scan.id,s.environment_id,serde_json::to_string(s).map_err(|_|"SERIALIZATION_FAILED")?]).map_err(|_|"DATABASE_SNAPSHOT_FAILED")?;
    }
    let coverage=serde_json::to_string(&scan.coverage).map_err(|_|"SERIALIZATION_FAILED")?;
    tx.execute("UPDATE scan_runs SET finished_at=?2,status=?3,coverage_json=?4,error_summary=?5 WHERE id=?1",params![scan.id,scan.finished_at,scan.status,coverage,scan.error_code]).map_err(|_|"DATABASE_FINISH_FAILED")?;
    tx.commit().map_err(|_|"DATABASE_COMMIT_FAILED")?;Ok(())
}
pub fn scan(db:&Connection,id:Option<&str>)->Result<Option<Scan>,String>{
    scan_in_scope(db,id,"windows")
}
pub fn scan_in_scope(db:&Connection,id:Option<&str>,scope:&str)->Result<Option<Scan>,String>{
    let sql=if id.is_some(){"SELECT id,started_at,finished_at,status,coverage_json,error_summary FROM scan_runs WHERE id=?1"}else{"SELECT id,started_at,finished_at,status,coverage_json,error_summary FROM scan_runs WHERE scope=?1 ORDER BY started_at DESC LIMIT 1"};
    let mut stmt=db.prepare(sql).map_err(|_|"DATABASE_READ_FAILED")?;
    let mut rows=stmt.query([id.unwrap_or(scope)]).map_err(|_|"DATABASE_READ_FAILED")?;
    if let Some(r)=rows.next().map_err(|_|"DATABASE_READ_FAILED")?{
        let coverage:String=r.get(4).map_err(|_|"DATABASE_READ_FAILED")?;
        let status:String=r.get(3).map_err(|_|"DATABASE_READ_FAILED")?;
        Ok(Some(Scan{id:r.get(0).map_err(|_|"DATABASE_READ_FAILED")?,started_at:r.get(1).map_err(|_|"DATABASE_READ_FAILED")?,finished_at:r.get(2).map_err(|_|"DATABASE_READ_FAILED")?,phase:if status=="RUNNING"{"collecting"}else{"finished"}.into(),progress:if status=="RUNNING"{10}else{100},status,coverage:serde_json::from_str(&coverage).map_err(|_|"DATABASE_CORRUPT")?,error_code:r.get(5).map_err(|_|"DATABASE_READ_FAILED")?}))
    }else{Ok(None)}
}
pub fn project_graph(db:&Connection,id:&str)->Result<Option<crate::npm::DependencyGraph>,String>{
    use rusqlite::OptionalExtension;
    let text:Option<String>=db.query_row("SELECT snapshot_json FROM scan_project_graphs WHERE scan_run_id=?1",[id],|r|r.get(0)).optional().map_err(|_|"DATABASE_READ_FAILED")?;
    text.map(|text|{let graph=serde_json::from_str(&text).map_err(|_|"DATABASE_CORRUPT")?;crate::npm::validate_graph(&graph).map_err(String::from)?;Ok(graph)}).transpose()
}
pub fn assets(db:&Connection,id:&str)->Result<Vec<Asset>,String>{
    let mut stmt=db.prepare("SELECT snapshot_json FROM scan_assets WHERE scan_run_id=?1 ORDER BY asset_id").map_err(|_|"DATABASE_READ_FAILED")?;
    let rows=stmt.query_map([id],|r|r.get::<_,String>(0)).map_err(|_|"DATABASE_READ_FAILED")?;
    let mut result=Vec::new();for row in rows{result.push(serde_json::from_str(&row.map_err(|_|"DATABASE_READ_FAILED")?).map_err(|_|"DATABASE_CORRUPT")?);}Ok(result)
}
pub fn python_snapshot(db:&Connection,id:&str)->Result<Option<crate::python::PythonSnapshot>,String>{
    use rusqlite::OptionalExtension;
    let text:Option<String>=db.query_row("SELECT snapshot_json FROM scan_python_snapshots WHERE scan_run_id=?1",[id],|r|r.get(0)).optional().map_err(|_|"DATABASE_READ_FAILED")?;
    text.map(|text|{let snapshot=serde_json::from_str(&text).map_err(|_|"DATABASE_CORRUPT")?;crate::python::validate_snapshot(&snapshot).map_err(String::from)?;Ok(snapshot)}).transpose()
}
#[cfg(test)] mod tests {
    use super::*;
    #[test]fn python_snapshots_preserve_history_and_remove_only_current_environment_membership(){
        let temporary=tempfile::tempdir().unwrap();let path=temporary.path().join("python.sqlite");let initial=crate::python::fixture_snapshot();
        {let mut d=open(&path).unwrap();assert_eq!(d.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),4);
            for i in 0..2{let mut snapshot=initial.clone();if i==1{snapshot.packages.clear();}let scan=Scan{id:format!("python{i}"),status:"PARTIAL".into(),phase:"finished".into(),progress:100,started_at:i.to_string(),finished_at:Some(i.to_string()),coverage:vec![],error_code:None};insert_scoped_scan(&d,&scan,"python").unwrap();finish_python(&mut d,&scan,Some(&snapshot)).unwrap();}
            assert_eq!(d.query_row("SELECT count(*) FROM environment_current_packages WHERE environment_id=?1",[&initial.environment_id],|r|r.get::<_,usize>(0)).unwrap(),0);
            assert_eq!(d.query_row("SELECT count(*) FROM packages WHERE ecosystem='PyPI' AND direct_status='UNKNOWN'",[],|r|r.get::<_,usize>(0)).unwrap(),1);
        }
        let d=open(&path).unwrap();assert_eq!(serde_json::to_string(&python_snapshot(&d,"python0").unwrap().unwrap()).unwrap(),serde_json::to_string(&initial).unwrap());assert!(python_snapshot(&d,"python1").unwrap().unwrap().packages.is_empty());assert!(scan(&d,None).unwrap().is_none());assert!(scan_in_scope(&d,None,"python").unwrap().is_some());
    }
    #[test]fn invalid_python_snapshot_does_not_commit_terminal_scan_or_packages(){
        let mut d=Connection::open_in_memory().unwrap();d.execute_batch("PRAGMA foreign_keys=ON").unwrap();migrate(&mut d).unwrap();
        let scan=Scan{id:"bad-python".into(),status:"PARTIAL".into(),phase:"finished".into(),progress:100,started_at:"now".into(),finished_at:Some("now".into()),coverage:vec![],error_code:None};insert_scoped_scan(&d,&scan,"python").unwrap();
        let mut invalid=crate::python::fixture_snapshot();invalid.packages[0].direct_status="DIRECT".into();assert!(finish_python(&mut d,&scan,Some(&invalid)).is_err());assert_eq!(scan_in_scope(&d,Some(&scan.id),"python").unwrap().unwrap().status,"RUNNING");assert_eq!(d.query_row("SELECT count(*) FROM packages",[],|r|r.get::<_,usize>(0)).unwrap(),0);
    }
    #[test]fn schema_three_upgrade_preserves_parent_references_and_restores_foreign_keys_on_failure(){
        let schema3=|d:&Connection|{d.execute_batch(MIGRATION).unwrap();d.execute_batch(PROJECT_MIGRATION).unwrap();d.execute_batch(MEMBERSHIP_MIGRATION).unwrap();d.pragma_update(None,"user_version",3).unwrap();};
        let mut d=Connection::open_in_memory().unwrap();schema3(&d);d.execute_batch("PRAGMA foreign_keys=ON; INSERT INTO scan_runs(id,started_at,status,scanner_versions,coverage_json) VALUES ('old','now','COMPLETE','1','[]'); INSERT INTO assets VALUES ('a','windows','Owned fixture',NULL,NULL,'UNKNOWN','1',NULL,'UNKNOWN','[]','now'); INSERT INTO scan_assets VALUES ('old','a','{}');").unwrap();migrate(&mut d).unwrap();
        assert_eq!(d.query_row("PRAGMA foreign_keys",[],|r|r.get::<_,i64>(0)).unwrap(),1);assert_eq!(d.query_row("SELECT count(*) FROM scan_assets WHERE scan_run_id='old'",[],|r|r.get::<_,i64>(0)).unwrap(),1);assert!(d.prepare("PRAGMA foreign_key_check").unwrap().query([]).unwrap().next().unwrap().is_none());
        let mut broken=Connection::open_in_memory().unwrap();schema3(&broken);broken.execute_batch("INSERT INTO scan_assets VALUES ('missing','missing','{}'); PRAGMA foreign_keys=ON;").unwrap();assert!(migrate(&mut broken).is_err());assert_eq!(broken.query_row("PRAGMA foreign_keys",[],|r|r.get::<_,i64>(0)).unwrap(),1);assert_eq!(broken.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),3);assert_eq!(broken.query_row("SELECT count(*) FROM sqlite_master WHERE name='scan_python_snapshots'",[],|r|r.get::<_,i64>(0)).unwrap(),0);
    }
    #[test]fn migration_is_idempotent_and_rejects_future_schema(){let mut d=Connection::open_in_memory().unwrap();d.execute_batch("PRAGMA foreign_keys=ON").unwrap();migrate(&mut d).unwrap();migrate(&mut d).unwrap();assert_eq!(d.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),4);d.pragma_update(None,"user_version",99).unwrap();assert!(migrate(&mut d).is_err());}
    #[test]fn history_survives_restart_and_interrupted_runs_are_explicit(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("test.sqlite");{let d=open(&path).unwrap();for i in 0..30{let s=Scan{id:i.to_string(),status:"RUNNING".into(),phase:"queued".into(),progress:0,started_at:i.to_string(),finished_at:None,coverage:vec![],error_code:None};insert_scan(&d,&s).unwrap();}}let d=open(&path).unwrap();assert_eq!(interrupt(&d).unwrap(),30);assert_eq!(scan(&d,Some("0")).unwrap().unwrap().status,"INTERRUPTED");assert_eq!(d.query_row("SELECT count(*) FROM scan_runs",[],|r|r.get::<_,i64>(0)).unwrap(),30);}
    #[test]fn missing_parent_asset_is_rejected(){let mut d=Connection::open_in_memory().unwrap();d.execute_batch("PRAGMA foreign_keys=ON").unwrap();migrate(&mut d).unwrap();assert!(d.execute("INSERT INTO scan_assets VALUES ('missing','missing','{}')",[]).is_err());}
    #[test]fn legacy_schema_upgrade_preserves_windows_and_unknown_relationships(){
        let mut d=Connection::open_in_memory().unwrap();d.execute_batch(MIGRATION).unwrap();d.pragma_update(None,"user_version",1).unwrap();
        d.execute_batch("INSERT INTO scan_runs VALUES ('old','now',NULL,'COMPLETE','1','[]',NULL); INSERT INTO environments VALUES ('e','npm','hashed',NULL,'now'); INSERT INTO packages VALUES ('p','e','npm','a','1',0,'UNKNOWN','now');").unwrap();
        migrate(&mut d).unwrap();migrate(&mut d).unwrap();
        assert_eq!(d.query_row("SELECT scope FROM scan_runs WHERE id='old'",[],|r|r.get::<_,String>(0)).unwrap(),"windows");
        assert_eq!(d.query_row("SELECT direct_status FROM packages WHERE id='p'",[],|r|r.get::<_,String>(0)).unwrap(),"UNKNOWN");
    }
    #[test]fn immutable_project_graph_survives_restart_and_current_version_changes(){
        let temporary=tempfile::tempdir().unwrap();let path=temporary.path().join("inventory.sqlite");
        let mut graph=crate::npm::parse_lock(br#"{"lockfileVersion":3,"packages":{"":{"dependencies":{"a":"*"}},"node_modules/a":{"version":"1.0.0"}}}"#,&"e".repeat(64),None,&std::sync::atomic::AtomicBool::new(false)).unwrap();
        let initial=serde_json::to_string(&graph).unwrap();
        {let mut d=open(&path).unwrap();for i in 0..2{let scan=Scan{id:format!("npm{i}"),status:"PARTIAL".into(),phase:"finished".into(),progress:100,started_at:i.to_string(),finished_at:Some(i.to_string()),coverage:vec![],error_code:None};insert_scoped_scan(&d,&scan,"npm").unwrap();finish_project(&mut d,&scan,Some(&graph)).unwrap();graph.packages[0].version=Some("2.0.0".into());}}
        let d=open(&path).unwrap();assert_eq!(serde_json::to_string(&project_graph(&d,"npm0").unwrap().unwrap()).unwrap(),initial);
        assert_eq!(project_graph(&d,"npm1").unwrap().unwrap().packages[0].version.as_deref(),Some("2.0.0"));
        assert!(scan(&d,None).unwrap().is_none());assert!(scan_in_scope(&d,None,"npm").unwrap().is_some());
    }
    #[test]fn current_membership_and_alias_identity_change_without_rewriting_history(){
        let mut d=Connection::open_in_memory().unwrap();d.execute_batch("PRAGMA foreign_keys=ON").unwrap();migrate(&mut d).unwrap();
        let parse=|bytes:&[u8]|crate::npm::parse_lock(bytes,&"e".repeat(64),None,&std::sync::atomic::AtomicBool::new(false)).unwrap();
        let first=parse(br#"{"lockfileVersion":3,"packages":{"node_modules/alias":{"name":"first","version":"1.0.0"},"node_modules/removed":{"version":"3.0.0"}}}"#);
        let second=parse(br#"{"lockfileVersion":3,"packages":{"node_modules/alias":{"name":"second","version":"2.0.0"}}}"#);
        let empty=parse(br#"{"lockfileVersion":3,"packages":{}}"#);
        for (i,graph) in [&first,&second,&empty].into_iter().enumerate(){let scan=Scan{id:format!("p{i}"),status:"PARTIAL".into(),phase:"finished".into(),progress:100,started_at:i.to_string(),finished_at:Some(i.to_string()),coverage:vec![],error_code:None};insert_scoped_scan(&d,&scan,"npm").unwrap();finish_project(&mut d,&scan,Some(graph)).unwrap();
            assert_eq!(d.query_row("SELECT count(*) FROM environment_current_packages",[],|r|r.get::<_,usize>(0)).unwrap(),graph.packages.len());
            if i==1{assert_eq!(d.query_row("SELECT name FROM packages JOIN environment_current_packages ON package_id=packages.id",[],|r|r.get::<_,String>(0)).unwrap(),"second");}
        }
        assert_eq!(serde_json::to_string(&project_graph(&d,"p0").unwrap().unwrap()).unwrap(),serde_json::to_string(&first).unwrap());
        assert_eq!(d.query_row("SELECT count(*) FROM packages",[],|r|r.get::<_,usize>(0)).unwrap(),2);
    }
}

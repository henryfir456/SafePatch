use crate::model::{Asset, Scan};
use rusqlite::{params, Connection};
use std::path::Path;
pub const MIGRATION: &str = include_str!("../migrations/001_initial.sql");
pub fn open(path:&Path)->rusqlite::Result<Connection>{
    let mut db=Connection::open(path)?;
    db.busy_timeout(std::time::Duration::from_secs(5))?;
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")?;
    migrate(&mut db)?;Ok(db)
}
pub fn migrate(db:&mut Connection)->rusqlite::Result<()>{
    let version:i64=db.query_row("PRAGMA user_version",[],|r|r.get(0))?;
    if version>1{return Err(rusqlite::Error::InvalidQuery)}
    if version==0{let tx=db.transaction()?;tx.execute_batch(MIGRATION)?;tx.pragma_update(None,"user_version",1)?;tx.commit()?;}
    Ok(())
}
pub fn interrupt(db:&Connection)->rusqlite::Result<usize>{db.execute("UPDATE scan_runs SET status='INTERRUPTED',finished_at=?1,error_summary='PROCESS_INTERRUPTED' WHERE status='RUNNING'",[chrono::Utc::now().to_rfc3339()])}
pub fn insert_scan(db:&Connection,scan:&Scan)->rusqlite::Result<()>{db.execute("INSERT INTO scan_runs VALUES (?1,?2,NULL,'RUNNING','windows-adapter/1', '[]',NULL)",params![scan.id,scan.started_at])?;Ok(())}
pub fn finish(db:&mut Connection,scan:&Scan,assets:&[Asset])->Result<(),String>{
    let tx=db.transaction().map_err(|_|"DATABASE_TRANSACTION_FAILED")?;
    for a in assets {
        let provenance=serde_json::to_string(&a.provenance).map_err(|_|"SERIALIZATION_FAILED")?;
        let snapshot=serde_json::to_string(a).map_err(|_|"SERIALIZATION_FAILED")?;
        tx.execute("INSERT INTO assets VALUES (?1,'windows',?2,?3,?4,?5,?6,?7,?8,?9,?10) ON CONFLICT(id) DO UPDATE SET observed_at=excluded.observed_at,provenance_json=excluded.provenance_json,identity_confidence=excluded.identity_confidence",params![a.id,a.display_name,a.publisher,a.normalized_id,a.installation_source,a.installed_version,a.install_path_hash,a.identity_confidence,provenance,a.observed_at]).map_err(|_|"DATABASE_ASSET_FAILED")?;
        tx.execute("INSERT INTO scan_assets VALUES (?1,?2,?3)",params![scan.id,a.id,snapshot]).map_err(|_|"DATABASE_SNAPSHOT_FAILED")?;
    }
    let coverage=serde_json::to_string(&scan.coverage).map_err(|_|"SERIALIZATION_FAILED")?;
    tx.execute("UPDATE scan_runs SET finished_at=?2,status=?3,coverage_json=?4,error_summary=?5 WHERE id=?1",params![scan.id,scan.finished_at,scan.status,coverage,scan.error_code]).map_err(|_|"DATABASE_FINISH_FAILED")?;
    tx.commit().map_err(|_|"DATABASE_COMMIT_FAILED")?;Ok(())
}
pub fn scan(db:&Connection,id:Option<&str>)->Result<Option<Scan>,String>{
    let sql=if id.is_some(){"SELECT id,started_at,finished_at,status,coverage_json,error_summary FROM scan_runs WHERE id=?1"}else{"SELECT id,started_at,finished_at,status,coverage_json,error_summary FROM scan_runs ORDER BY started_at DESC LIMIT 1"};
    let mut stmt=db.prepare(sql).map_err(|_|"DATABASE_READ_FAILED")?;
    let mut rows=if let Some(id)=id{stmt.query([id])}else{stmt.query([])}.map_err(|_|"DATABASE_READ_FAILED")?;
    if let Some(r)=rows.next().map_err(|_|"DATABASE_READ_FAILED")?{
        let coverage:String=r.get(4).map_err(|_|"DATABASE_READ_FAILED")?;
        let status:String=r.get(3).map_err(|_|"DATABASE_READ_FAILED")?;
        Ok(Some(Scan{id:r.get(0).map_err(|_|"DATABASE_READ_FAILED")?,started_at:r.get(1).map_err(|_|"DATABASE_READ_FAILED")?,finished_at:r.get(2).map_err(|_|"DATABASE_READ_FAILED")?,phase:if status=="RUNNING"{"collecting"}else{"finished"}.into(),progress:if status=="RUNNING"{10}else{100},status,coverage:serde_json::from_str(&coverage).map_err(|_|"DATABASE_CORRUPT")?,error_code:r.get(5).map_err(|_|"DATABASE_READ_FAILED")?}))
    }else{Ok(None)}
}
pub fn assets(db:&Connection,id:&str)->Result<Vec<Asset>,String>{
    let mut stmt=db.prepare("SELECT snapshot_json FROM scan_assets WHERE scan_run_id=?1 ORDER BY asset_id").map_err(|_|"DATABASE_READ_FAILED")?;
    let rows=stmt.query_map([id],|r|r.get::<_,String>(0)).map_err(|_|"DATABASE_READ_FAILED")?;
    let mut result=Vec::new();for row in rows{result.push(serde_json::from_str(&row.map_err(|_|"DATABASE_READ_FAILED")?).map_err(|_|"DATABASE_CORRUPT")?);}Ok(result)
}
#[cfg(test)] mod tests {
    use super::*;
    #[test]fn migration_is_idempotent_and_rejects_future_schema(){let mut d=Connection::open_in_memory().unwrap();d.execute_batch("PRAGMA foreign_keys=ON").unwrap();migrate(&mut d).unwrap();migrate(&mut d).unwrap();assert_eq!(d.query_row("PRAGMA user_version",[],|r|r.get::<_,i64>(0)).unwrap(),1);d.pragma_update(None,"user_version",99).unwrap();assert!(migrate(&mut d).is_err());}
    #[test]fn history_survives_restart_and_interrupted_runs_are_explicit(){let dir=tempfile::tempdir().unwrap();let path=dir.path().join("test.sqlite");{let d=open(&path).unwrap();for i in 0..30{let s=Scan{id:i.to_string(),status:"RUNNING".into(),phase:"queued".into(),progress:0,started_at:i.to_string(),finished_at:None,coverage:vec![],error_code:None};insert_scan(&d,&s).unwrap();}}let d=open(&path).unwrap();assert_eq!(interrupt(&d).unwrap(),30);assert_eq!(scan(&d,Some("0")).unwrap().unwrap().status,"INTERRUPTED");assert_eq!(d.query_row("SELECT count(*) FROM scan_runs",[],|r|r.get::<_,i64>(0)).unwrap(),30);}
    #[test]fn missing_parent_asset_is_rejected(){let mut d=Connection::open_in_memory().unwrap();d.execute_batch("PRAGMA foreign_keys=ON").unwrap();migrate(&mut d).unwrap();assert!(d.execute("INSERT INTO scan_assets VALUES ('missing','missing','{}')",[]).is_err());}
}

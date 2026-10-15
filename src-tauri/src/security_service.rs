use crate::{
    db,
    model::{Coverage, Scan},
    security::SecurityReport,
    Active, ScanService,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
impl ScanService {
    pub fn start_security(
        self: &Arc<Self>,
        windows_scan_id: Option<String>,
        npm_scan_id: Option<String>,
        external_queries: bool,
    ) -> Result<Scan, String> {
        if !external_queries {
            return Err("EXTERNAL_QUERY_CONSENT_REQUIRED".into());
        }
        if windows_scan_id.is_none() && npm_scan_id.is_none() {
            return Err("SECURITY_INVENTORY_REQUIRED".into());
        }
        let mut guard = self.active.lock().map_err(|_| "SCAN_STATE_FAILED")?;
        if guard.as_ref().is_some_and(|a| a.scan.status == "RUNNING") {
            return Err("SCAN_ALREADY_RUNNING".into());
        }
        let database = db::open(&self.path).map_err(|_| "DATABASE_OPEN_FAILED")?;
        let windows = windows_scan_id
            .map(|id| {
                Self::validate_id(&id)?;
                let scan = db::scan(&database, Some(&id))?.ok_or("SCAN_NOT_FOUND")?;
                if !matches!(scan.status.as_str(), "COMPLETE" | "PARTIAL") {
                    return Err("SECURITY_INVENTORY_REQUIRED".into());
                }
                let assets = db::assets(&database, &id)?;
                if assets.is_empty() {
                    return Err("SECURITY_INVENTORY_REQUIRED".into());
                }
                Ok::<_, String>((id, assets))
            })
            .transpose()?;
        let npm = npm_scan_id
            .map(|id| {
                Self::validate_id(&id)?;
                let graph =
                    db::project_graph(&database, &id)?.ok_or("SECURITY_INVENTORY_REQUIRED")?;
                Ok::<_, String>((id, graph))
            })
            .transpose()?;
        let scan = Scan {
            id: uuid::Uuid::new_v4().to_string(),
            status: "RUNNING".into(),
            phase: "checking OSV / NVD / CISA KEV".into(),
            progress: 10,
            started_at: chrono::Utc::now().to_rfc3339(),
            finished_at: None,
            coverage: vec![],
            error_code: None,
        };
        database
            .execute(
                "INSERT INTO security_runs(id,started_at,status) VALUES (?1,?2,'RUNNING')",
                rusqlite::params![scan.id, scan.started_at],
            )
            .map_err(|_| "DATABASE_START_FAILED")?;
        let cancel = Arc::new(AtomicBool::new(false));
        *guard = Some(Active {
            scan: scan.clone(),
            cancel: cancel.clone(),
            scope: "security".into(),
        });
        let service = self.clone();
        let id = scan.id.clone();
        if std::thread::Builder::new()
            .name("safepatch-security".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    crate::security_client::collect(
                        &service.path,
                        &id,
                        windows,
                        npm,
                        cancel.clone(),
                    )
                }))
                .unwrap_or_else(|_| Err("SECURITY_WORKER_FAILED".into()));
                service.complete_security(result);
            })
            .is_err()
        {
            drop(guard);
            self.complete_security(Err("SECURITY_WORKER_FAILED".into()));
            return Err("SECURITY_WORKER_FAILED".into());
        }
        Ok(scan)
    }
    fn complete_security(&self, result: Result<SecurityReport, String>) {
        if let Ok(mut guard) = self.active.lock() {
            if let Some(active) = guard.as_mut() {
                // Check under the same state lock used by cancel: an acknowledged cancellation cannot publish a snapshot.
                let cancelled = active.cancel.load(Ordering::Relaxed);
                let report = match result {
                    Ok(report) if !cancelled => {
                        active.scan.status = "PARTIAL".into();
                        active.scan.coverage = vec![Coverage {
                            scanner: "security".into(),
                            status: "PARTIAL".into(),
                            count: report.items.len(),
                            codes: vec!["COVERAGE_PARTIAL".into()],
                        }];
                        Some(report)
                    }
                    Ok(_) => {
                        active.scan.status = "CANCELLED".into();
                        None
                    }
                    Err(code) => {
                        active.scan.status = if cancelled || code == "CANCELLED" {
                            "CANCELLED"
                        } else {
                            "FAILED"
                        }
                        .into();
                        active.scan.error_code = Some(code);
                        None
                    }
                };
                active.scan.progress = 100;
                active.scan.phase = "finished".into();
                active.scan.finished_at = Some(chrono::Utc::now().to_rfc3339());
                let write = || -> Result<(), String> {
                    let d = db::open(&self.path).map_err(|_| "DATABASE_OPEN_FAILED")?;
                    let body = report
                        .as_ref()
                        .map(serde_json::to_string)
                        .transpose()
                        .map_err(|_| "SERIALIZATION_FAILED")?;
                    d.execute("UPDATE security_runs SET status=?2,finished_at=?3,report_json=?4,error_code=?5 WHERE id=?1",rusqlite::params![active.scan.id,active.scan.status,active.scan.finished_at,body,active.scan.error_code]).map_err(|_|"DATABASE_FINISH_FAILED")?;
                    Ok(())
                };
                if let Err(code) = write() {
                    active.scan.status = "FAILED".into();
                    active.scan.error_code = Some(code);
                }
            }
        }
    }
    pub fn latest_security(&self) -> Result<Option<Scan>, String> {
        if let Some(a) = self
            .active
            .lock()
            .map_err(|_| "SCAN_STATE_FAILED")?
            .as_ref()
        {
            if a.scope == "security" {
                return Ok(Some(a.scan.clone()));
            }
        }
        security_scan(
            &db::open(&self.path).map_err(|_| "DATABASE_OPEN_FAILED")?,
            None,
        )
    }
    pub fn security_report(&self, id: &str) -> Result<Option<SecurityReport>, String> {
        Self::validate_id(id)?;
        use rusqlite::OptionalExtension;
        let d = db::open(&self.path).map_err(|_| "DATABASE_OPEN_FAILED")?;
        let body: Option<String> = d
            .query_row(
                "SELECT report_json FROM security_runs WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| "DATABASE_READ_FAILED")?
            .flatten();
        body.map(|body| serde_json::from_str(&body).map_err(|_| "DATABASE_CORRUPT".into()))
            .transpose()
    }
    pub fn last_security_report(&self) -> Result<Option<SecurityReport>, String> {
        use rusqlite::OptionalExtension;
        let d = db::open(&self.path).map_err(|_| "DATABASE_OPEN_FAILED")?;
        let body:Option<String>=d.query_row("SELECT report_json FROM security_runs WHERE report_json IS NOT NULL ORDER BY started_at DESC LIMIT 1",[],|r|r.get(0)).optional().map_err(|_|"DATABASE_READ_FAILED")?;
        body.map(|body| serde_json::from_str(&body).map_err(|_| "DATABASE_CORRUPT".into()))
            .transpose()
    }
}
pub fn security_scan(db: &rusqlite::Connection, id: Option<&str>) -> Result<Option<Scan>, String> {
    use rusqlite::OptionalExtension;
    let sql = if id.is_some() {
        "SELECT id,started_at,finished_at,status,error_code FROM security_runs WHERE id=?1"
    } else {
        "SELECT id,started_at,finished_at,status,error_code FROM security_runs WHERE ?1='' ORDER BY started_at DESC LIMIT 1"
    };
    db.query_row(sql, [id.unwrap_or("")], |r| {
        let status: String = r.get(3)?;
        Ok(Scan {
            id: r.get(0)?,
            started_at: r.get(1)?,
            finished_at: r.get(2)?,
            progress: if status == "RUNNING" { 10 } else { 100 },
            phase: if status == "RUNNING" {
                "checking security sources"
            } else {
                "finished"
            }
            .into(),
            status,
            coverage: vec![],
            error_code: r.get(4)?,
        })
    })
    .optional()
    .map_err(|_| "DATABASE_READ_FAILED".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requires_consent_and_existing_inventory() {
        let dir = tempfile::tempdir().unwrap();
        let service = ScanService::new(dir.path().join("db")).unwrap();
        assert_eq!(
            service.start_security(None, None, false).unwrap_err(),
            "EXTERNAL_QUERY_CONSENT_REQUIRED"
        );
        assert_eq!(
            service.start_security(None, None, true).unwrap_err(),
            "SECURITY_INVENTORY_REQUIRED"
        );
        assert!(service
            .start_security(Some("not-an-id".into()), None, true)
            .is_err());
    }
    #[test]
    fn cancelled_snapshot_does_not_replace_last_report() {
        let dir = tempfile::tempdir().unwrap();
        let service = ScanService::new(dir.path().join("db")).unwrap();
        let d = db::open(&service.path).unwrap();
        let report = SecurityReport {
            scan_id: uuid::Uuid::new_v4().to_string(),
            windows_scan_id: None,
            npm_scan_id: None,
            checked_at: "now".into(),
            status: "PARTIAL".into(),
            items: vec![],
        };
        d.execute("INSERT INTO security_runs(id,started_at,status,report_json) VALUES (?1,'1','PARTIAL',?2)",rusqlite::params![report.scan_id,serde_json::to_string(&report).unwrap()]).unwrap();
        let current = Scan {
            id: uuid::Uuid::new_v4().to_string(),
            status: "RUNNING".into(),
            phase: "checking".into(),
            progress: 10,
            started_at: "2".into(),
            finished_at: None,
            coverage: vec![],
            error_code: None,
        };
        d.execute(
            "INSERT INTO security_runs(id,started_at,status) VALUES (?1,'2','RUNNING')",
            [&current.id],
        )
        .unwrap();
        *service.active.lock().unwrap() = Some(Active {
            scan: current.clone(),
            cancel: Arc::new(AtomicBool::new(true)),
            scope: "security".into(),
        });
        service.complete_security(Ok(report.clone()));
        assert_eq!(service.status(&current.id).unwrap().status, "CANCELLED");
        assert_eq!(
            service.last_security_report().unwrap().unwrap().scan_id,
            report.scan_id
        );
        drop(service);
        let restarted = ScanService::new(dir.path().join("db")).unwrap();
        assert_eq!(
            restarted.last_security_report().unwrap().unwrap().scan_id,
            report.scan_id
        );
    }
}

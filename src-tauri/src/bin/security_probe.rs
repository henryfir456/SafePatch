//! Actual device/service probe. Emits aggregate evidence only; external queries are opt-in.
use safepatch_lib::{
    model::Scan,
    security::{Applicability, Priority},
    ScanService,
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
fn wait(service: &Arc<ScanService>, scan: Scan) -> Result<Scan, String> {
    let at = Instant::now();
    loop {
        let s = service.status(&scan.id)?;
        if s.status != "RUNNING" {
            return Ok(s);
        }
        if at.elapsed() > Duration::from_secs(360) {
            let _ = service.cancel(&scan.id);
            return Err("PROBE_TIMEOUT".into());
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    if safepatch_lib::npm_helper::run_if_requested() {
        return Ok(());
    }
    let args: Vec<_> = std::env::args().collect();
    if !args.iter().any(|s| s == "--allow-external") {
        return Err("External queries disabled; --allow-external sends controlled CPE/version to NVD, optional npm name/version to OSV and downloads public KEV; never sends paths/full inventory".into());
    }
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("probe.sqlite");
    let service = ScanService::new(path.clone())?;
    let windows = wait(&service, service.start("windows")?)?;
    if !matches!(windows.status.as_str(), "PARTIAL" | "COMPLETE") {
        return Err("PROBE_INVENTORY_FAILED".into());
    }
    let npm = if let Some(index) = args.iter().position(|s| s == "--npm-root") {
        let root = args.get(index + 1).ok_or("PROBE_ROOT_REQUIRED")?.clone();
        let s = wait(&service, service.start_project(root)?)?;
        if !matches!(s.status.as_str(), "PARTIAL" | "COMPLETE") {
            return Err("PROBE_NPM_FAILED".into());
        }
        Some(s.id)
    } else {
        None
    };
    let cancelled = service.start_security(Some(windows.id.clone()), npm.clone(), true)?;
    service.cancel(&cancelled.id)?;
    let cancelled = wait(&service, cancelled)?;
    if cancelled.status != "CANCELLED" {
        return Err("PROBE_CANCELLATION_FAILED".into());
    }
    let scan = wait(
        &service,
        service.start_security(Some(windows.id), npm, true)?,
    )?;
    let report = service
        .last_security_report()?
        .ok_or("PROBE_REPORT_MISSING")?;
    let before = serde_json::to_value(&report)?;
    drop(service);
    let at = Instant::now();
    let service = loop {
        match ScanService::new(path.clone()) {
            Ok(s) => break s,
            Err(c) if c == "DATABASE_IN_USE" && at.elapsed() < Duration::from_secs(3) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(c) => return Err(c.into()),
        }
    };
    let preserved = before
        == serde_json::to_value(
            service
                .last_security_report()?
                .ok_or("PROBE_REPORT_MISSING")?,
        )?;
    let mut checks = std::collections::BTreeMap::<String, usize>::new();
    for item in &report.items {
        for c in &item.checks {
            *checks
                .entry(format!(
                    "{}:{}:{}",
                    c.source,
                    c.status,
                    c.reason.as_deref().unwrap_or("NONE")
                ))
                .or_default() += 1;
        }
    }
    println!(
        "{}",
        serde_json::to_string_pretty(
            &serde_json::json!({"verification":"REAL_DEVICE_SERVICE_SQLITE; NATIVE_SECURITY_UI_NOT_VERIFIED","build":safepatch_lib::build_info::current(),"status":scan.status,"item_count":report.items.len(),"matched_items":report.items.iter().filter(|i|i.applicability==Applicability::Matched).count(),"potential_items":report.items.iter().filter(|i|i.applicability==Applicability::Potential).count(),"critical_items":report.items.iter().filter(|i|i.priority==Priority::Critical).count(),"source_checks":checks,"cancellation":cancelled.status=="CANCELLED","restart_preserved":preserved})
        )?
    );
    if !preserved || scan.status != "PARTIAL" || report.items.is_empty() {
        return Err("PROBE_VERIFICATION_FAILED".into());
    }
    if !report.items.iter().any(|i| {
        i.checks
            .iter()
            .any(|c| c.status == "COMPLETE" && matches!(c.source.as_str(), "NVD" | "OSV"))
    }) {
        return Err("PROBE_NO_SOURCE_SUCCESS".into());
    }
    Ok(())
}

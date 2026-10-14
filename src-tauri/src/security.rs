//! #28: applicability and urgency are independent; no name-search confirmation.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Applicability {
    Matched,
    Potential,
    NotAffected,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Priority {
    Critical,
    High,
    Medium,
    Informational,
    Undetermined,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceCheck {
    pub source: String,
    pub status: String,
    pub checked_at: String,
    pub reason: Option<String>,
    pub cache: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Kev {
    pub date_added: String,
    pub due_date: String,
    pub action: String,
    pub source_url: String,
    pub checked_at: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub cves: Vec<String>,
    pub source: String,
    pub source_url: String,
    pub queried_at: String,
    pub applicability: Applicability,
    pub priority: Priority,
    pub confidence: String,
    pub evidence: Vec<String>,
    pub affected_range: String,
    pub fixed_versions: Vec<String>,
    pub cvss: Option<String>,
    pub severity: Option<String>,
    pub references: Vec<String>,
    pub kev: Option<Kev>,
    pub kev_status: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityItem {
    pub id: String,
    pub kind: String,
    pub display_name: String,
    pub installed_version: Option<String>,
    pub update: crate::updates::UpdateEvidence,
    pub applicability: Applicability,
    pub priority: Priority,
    pub reasons: Vec<String>,
    #[serde(default)]
    pub identity_reasons: Vec<String>,
    #[serde(default)]
    pub update_reasons: Vec<String>,
    pub checks: Vec<SourceCheck>,
    pub findings: Vec<Finding>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SecurityReport {
    pub scan_id: String,
    pub windows_scan_id: Option<String>,
    pub npm_scan_id: Option<String>,
    pub checked_at: String,
    pub status: String,
    pub items: Vec<SecurityItem>,
}
pub fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}
pub fn is_cve(s: &str) -> bool {
    let p: Vec<_> = s.split('-').collect();
    p.len() == 3
        && p[0] == "CVE"
        && p[1].len() == 4
        && p[2].len() >= 4
        && p[2].len() <= 12
        && p[1..].iter().all(|s| s.bytes().all(|b| b.is_ascii_digit()))
}
fn text(v: &Value) -> Option<String> {
    v.as_str()
        .filter(|s| s.len() <= 4096 && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
pub fn https_reference(v: &Value) -> Option<String> {
    let s = text(v)?;
    let url = reqwest::Url::parse(&s).ok()?;
    (url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.host_str().is_some())
    .then_some(s)
}
fn arr(v: &Value) -> &[Value] {
    v.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub fn priority(app: &Applicability, severity: Option<&str>, kev: bool) -> Priority {
    if *app != Applicability::Matched {
        return Priority::Undetermined;
    }
    if kev {
        Priority::Critical
    } else if matches!(severity, Some("HIGH" | "CRITICAL")) {
        Priority::High
    } else {
        Priority::Medium
    }
}
// Numeric vendor versions only; ambiguous suffixes never receive a lexical comparison.
pub fn numeric_compare(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |s: &str| -> Option<Vec<u64>> {
        if s.len() > 128 {
            return None;
        }
        let p: Vec<_> = s.split('.').collect();
        if p.is_empty()
            || p.len() > 8
            || p.iter()
                .any(|p| p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        p.into_iter().map(|p| p.parse().ok()).collect()
    };
    let (mut a, mut b) = (parse(a)?, parse(b)?);
    let width = a.len().max(b.len());
    a.resize(width, 0);
    b.resize(width, 0);
    Some(a.cmp(&b))
}
fn osv_range(range: &Value, version: &semver::Version) -> Option<(bool, Vec<String>)> {
    if range["type"] != "SEMVER" {
        return None;
    }
    let events = range["events"].as_array()?;
    if events.is_empty() || events.len() > 256 {
        return None;
    }
    let mut start: Option<Option<semver::Version>> = None;
    let mut matched = false;
    let mut fixes = vec![];
    let mut previous: Option<semver::Version> = None;
    for e in events {
        let obj = e.as_object()?;
        if obj.len() != 1 {
            return None;
        }
        let (key, raw) = obj.iter().next()?;
        let s = raw.as_str()?;
        let bound = if key == "introduced" && s == "0" {
            None
        } else {
            Some(semver::Version::parse(s).ok()?)
        };
        if let (Some(prev), Some(next)) = (&previous, &bound) {
            if next.cmp_precedence(prev).is_lt() {
                return None;
            }
        }
        if let Some(b) = &bound {
            previous = Some(b.clone())
        }
        if key == "introduced" {
            if start.is_some() {
                return None;
            }
            start = Some(bound);
            continue;
        }
        let lo = start.take()?;
        let upper = bound?;
        let below = if key == "last_affected" {
            !version.cmp_precedence(&upper).is_gt()
        } else if matches!(key.as_str(), "fixed" | "limit") {
            version.cmp_precedence(&upper).is_lt()
        } else {
            return None;
        };
        let in_interval = lo
            .as_ref()
            .is_none_or(|l| !version.cmp_precedence(l).is_lt())
            && below;
        matched |= in_interval;
        if in_interval && key == "fixed" {
            fixes.push(s.to_owned());
        }
    }
    if let Some(lo) = start {
        matched |= lo
            .as_ref()
            .is_none_or(|l| !version.cmp_precedence(l).is_lt())
    }
    Some((matched, fixes))
}
/// Returns evidence for an exact npm identity. `installed` is false for lockfile declarations.
pub fn parse_osv(
    doc: &Value,
    name: &str,
    version: &str,
    installed: bool,
    now: &str,
) -> Result<Vec<Finding>, &'static str> {
    if doc
        .get("next_page_token")
        .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()))
    {
        return Err("COVERAGE_PARTIAL");
    }
    if doc.get("vulns").is_some_and(|v| !v.is_array()) {
        return Err("QUERY_FAILED");
    }
    let vulns = arr(&doc["vulns"]);
    if vulns.len() > 1000 {
        return Err("COVERAGE_PARTIAL");
    }
    let installed_version = semver::Version::parse(version).map_err(|_| "VERSION_UNRESOLVED")?;
    let mut result = vec![];
    let mut ids = BTreeSet::new();
    for v in vulns {
        if v.get("withdrawn").is_some() {
            continue;
        }
        let id = v["id"]
            .as_str()
            .filter(|s| valid_id(s))
            .ok_or("QUERY_FAILED")?;
        if !ids.insert(id.to_owned()) {
            continue;
        }
        let affected: Vec<_> = arr(&v["affected"])
            .iter()
            .filter(|a| a["package"]["ecosystem"] == "npm" && a["package"]["name"] == name)
            .collect();
        if affected.is_empty() {
            continue;
        }
        let mut matched = false;
        let mut uncertain = false;
        let mut fixes = BTreeSet::new();
        for a in &affected {
            let mut rules = 0;
            for raw in arr(&a["versions"]) {
                rules += 1;
                match raw.as_str().and_then(|s| semver::Version::parse(s).ok()) {
                    Some(ver) => matched |= ver.cmp_precedence(&installed_version).is_eq(),
                    None => uncertain = true,
                }
            }
            for r in arr(&a["ranges"]) {
                rules += 1;
                match osv_range(r, &installed_version) {
                    Some((yes, f)) => {
                        matched |= yes;
                        fixes.extend(f);
                    }
                    None => uncertain = true,
                }
            }
            if rules == 0 {
                uncertain = true
            }
        }
        let app = if matched && installed {
            Applicability::Matched
        } else if matched || uncertain {
            Applicability::Potential
        } else {
            Applicability::NotAffected
        };
        if uncertain {
            fixes.clear();
        }
        let severity = text(&v["database_specific"]["severity"]).filter(|s| {
            matches!(
                s.as_str(),
                "LOW" | "MODERATE" | "MEDIUM" | "HIGH" | "CRITICAL"
            )
        });
        let cves: Vec<_> = arr(&v["aliases"])
            .iter()
            .filter_map(Value::as_str)
            .chain(std::iter::once(id))
            .filter(|s| is_cve(s))
            .map(str::to_owned)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let cvss = arr(&v["severity"])
            .iter()
            .find_map(|s| text(&s["score"]).filter(|s| s.starts_with("CVSS:")));
        result.push(Finding {
            id: id.into(),
            cves,
            source: "OSV".into(),
            source_url: format!("https://osv.dev/vulnerability/{id}"),
            queried_at: now.into(),
            priority: priority(&app, severity.as_deref(), false),
            confidence: if installed && matched {
                "HIGH"
            } else {
                "LIMITED"
            }
            .into(),
            applicability: app,
            evidence: vec![
                format!("Exact npm identity {name}@{version}; evaluated advisory ranges"),
                if installed {
                    "Installed package evidence"
                } else {
                    "LOCKFILE_DECLARATION: not proof of installation"
                }
                .into(),
            ],
            affected_range: serde_json::to_string(&affected).map_err(|_| "QUERY_FAILED")?,
            fixed_versions: fixes.into_iter().collect(),
            cvss,
            severity,
            references: arr(&v["references"])
                .iter()
                .filter_map(|r| https_reference(&r["url"]))
                .take(32)
                .collect(),
            kev: None,
            kev_status: "NOT_CHECKED".into(),
        });
    }
    Ok(result)
}
#[derive(Clone, Copy)]
pub struct Product {
    pub winget_id: &'static str,
    pub vendor: &'static str,
    pub product: &'static str,
    pub vendor_url: &'static str,
}
pub const PRODUCTS: &[Product] = &[
    Product {
        winget_id: "Google.Chrome",
        vendor: "google",
        product: "chrome",
        vendor_url: "https://www.google.com/chrome/",
    },
    Product {
        winget_id: "Mozilla.Firefox",
        vendor: "mozilla",
        product: "firefox",
        vendor_url: "https://www.mozilla.org/firefox/",
    },
    Product {
        winget_id: "7zip.7zip",
        vendor: "7-zip",
        product: "7-zip",
        vendor_url: "https://www.7-zip.org/",
    },
    Product {
        winget_id: "Microsoft.VisualStudioCode",
        vendor: "microsoft",
        product: "visual_studio_code",
        vendor_url: "https://code.visualstudio.com/",
    },
    Product {
        winget_id: "OpenJS.NodeJS.LTS",
        vendor: "nodejs",
        product: "node.js",
        vendor_url: "https://nodejs.org/",
    },
    Product {
        winget_id: "OpenJS.NodeJS",
        vendor: "nodejs",
        product: "node.js",
        vendor_url: "https://nodejs.org/",
    },
];
pub fn product_for(asset: &crate::model::Asset) -> Option<Product> {
    // Registry labels/IDs and unverified PE names alone cannot establish this mapping.
    if !asset
        .provenance
        .iter()
        .any(|p| p.scanner == "winget" && p.catalog_source.as_deref() == Some("winget"))
    {
        return None;
    }
    PRODUCTS
        .iter()
        .find(|p| Some(p.winget_id) == asset.normalized_id.as_deref())
        .copied()
}
fn cpe_app(m: &Value, p: Product, version: &str) -> Option<Applicability> {
    let criteria = m["criteria"].as_str()?;
    let c: Vec<_> = criteria.split(':').collect();
    if c.len() != 13
        || c[..3] != ["cpe", "2.3", "a"]
        || c[3] != p.vendor
        || c[4] != p.product
        || m["vulnerable"] != true
    {
        return None;
    }
    let mut result = Applicability::Matched;
    let mut bounds = 0;
    if c[5] != "*" {
        bounds += 1;
        match numeric_compare(version, c[5]) {
            Some(std::cmp::Ordering::Equal) => {}
            Some(_) => return Some(Applicability::NotAffected),
            None => result = Applicability::Potential,
        }
    }
    for (key, lower, inclusive) in [
        ("versionStartIncluding", true, true),
        ("versionStartExcluding", true, false),
        ("versionEndIncluding", false, true),
        ("versionEndExcluding", false, false),
    ] {
        if let Some(raw) = m.get(key) {
            bounds += 1;
            let Some(bound) = raw.as_str() else {
                return Some(Applicability::Potential);
            };
            match numeric_compare(version, bound) {
                None => result = Applicability::Potential,
                Some(o) => {
                    let good = if lower {
                        o.is_gt() || (inclusive && o.is_eq())
                    } else {
                        o.is_lt() || (inclusive && o.is_eq())
                    };
                    if !good {
                        return Some(Applicability::NotAffected);
                    }
                }
            }
        }
    }
    if bounds == 0 {
        result = Applicability::Potential
    }
    // Update/edition/language/target_hw/other conditions or extension-specific target_sw cannot confirm the desktop product.
    if c[6..10].iter().any(|s| !matches!(*s, "*" | "-"))
        || !matches!(c[10], "*" | "-" | "windows")
        || c[11..].iter().any(|s| !matches!(*s, "*" | "-"))
    {
        result = Applicability::Potential
    }
    Some(result)
}
pub fn parse_nvd(
    doc: &Value,
    p: Product,
    version: &str,
    now: &str,
) -> Result<(Vec<Finding>, bool), &'static str> {
    let entries = doc["vulnerabilities"].as_array().ok_or("QUERY_FAILED")?;
    let total = doc["totalResults"].as_u64().ok_or("QUERY_FAILED")?;
    if entries.len() > 2000 {
        return Err("COVERAGE_PARTIAL");
    }
    let complete = doc["startIndex"].as_u64() == Some(0) && total == entries.len() as u64;
    let mut result = vec![];
    let mut ids = BTreeSet::new();
    for entry in entries {
        let cve = &entry["cve"];
        if cve["vulnStatus"] == "Rejected" {
            continue;
        }
        let id = cve["id"]
            .as_str()
            .filter(|s| is_cve(s))
            .ok_or("QUERY_FAILED")?;
        if !ids.insert(id.to_owned()) {
            continue;
        }
        let mut candidate = vec![];
        for config in arr(&cve["configurations"]) {
            let nodes = arr(&config["nodes"]);
            for node in nodes {
                let matches = arr(&node["cpeMatch"]);
                let simple = nodes.len() == 1
                    && config
                        .get("negate")
                        .is_none_or(|v| v.as_bool() == Some(false))
                    && (config.get("operator").is_none() || config["operator"] == "OR")
                    && node["operator"] == "OR"
                    && node
                        .get("negate")
                        .is_none_or(|v| v.as_bool() == Some(false))
                    && node
                        .get("children")
                        .is_none_or(|v| v.as_array().is_some_and(|a| a.is_empty()))
                    && matches.iter().all(|m| m["vulnerable"] == true);
                for m in matches {
                    if let Some(mut app) = cpe_app(m, p, version) {
                        if !simple {
                            app = Applicability::Potential
                        }
                        candidate.push(app);
                    }
                }
            }
        }
        if candidate.is_empty() {
            continue;
        }
        let app = if candidate.contains(&Applicability::Matched) {
            Applicability::Matched
        } else if candidate.contains(&Applicability::Potential) {
            Applicability::Potential
        } else {
            Applicability::NotAffected
        };
        let metrics = &cve["metrics"];
        let metric = [
            "cvssMetricV40",
            "cvssMetricV31",
            "cvssMetricV30",
            "cvssMetricV2",
        ]
        .into_iter()
        .flat_map(|k| arr(&metrics[k]))
        .find(|m| m["type"] == "Primary");
        let severity = metric.and_then(|m| {
            text(&m["cvssData"]["baseSeverity"]).or_else(|| text(&m["baseSeverity"]))
        });
        let cvss = metric.map(|m| {
            format!(
                "{}; score {}; source {}",
                m["cvssData"]["vectorString"].as_str().unwrap_or("unknown"),
                m["cvssData"]["baseScore"],
                m["source"].as_str().unwrap_or("unknown")
            )
        });
        result.push(Finding{id:id.into(),cves:vec![id.into()],source:"NVD".into(),source_url:format!("https://nvd.nist.gov/vuln/detail/{id}"),queried_at:now.into(),priority:priority(&app,severity.as_deref(),false),confidence:if app==Applicability::Matched{"HIGH"}else{"LIMITED"}.into(),applicability:app,evidence:vec![format!("Mapping v1: exact official WinGet ID {} → CPE a:{}:{}; {}",p.winget_id,p.vendor,p.product,p.vendor_url),format!("Compared installed version {version}; complex platform/edition conditions remain POTENTIAL"),"NVD boundary is not proof of a fixed release".into()],affected_range:serde_json::to_string(&cve["configurations"]).map_err(|_|"QUERY_FAILED")?,fixed_versions:vec![],cvss,severity,references:arr(&cve["references"]).iter().filter_map(|r|https_reference(&r["url"])).take(32).collect(),kev:None,kev_status:"NOT_CHECKED".into()});
    }
    Ok((result, complete))
}
pub fn apply_kev(findings: &mut [Finding], doc: &Value, now: &str) -> Result<(), &'static str> {
    let entries = doc["vulnerabilities"].as_array().ok_or("QUERY_FAILED")?;
    if entries.len() > 10000 || doc["count"].as_u64() != Some(entries.len() as u64) {
        return Err("COVERAGE_PARTIAL");
    }
    let mut ids = BTreeSet::new();
    for entry in entries {
        let id = entry["cveID"]
            .as_str()
            .filter(|s| is_cve(s))
            .ok_or("QUERY_FAILED")?;
        if !ids.insert(id) {
            return Err("QUERY_FAILED");
        }
        for field in ["dateAdded", "dueDate"] {
            let value = entry[field].as_str().ok_or("QUERY_FAILED")?;
            let date =
                chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| "QUERY_FAILED")?;
            if date.format("%Y-%m-%d").to_string() != value {
                return Err("QUERY_FAILED");
            }
        }
        if text(&entry["requiredAction"]).is_none_or(|s| s.is_empty()) {
            return Err("QUERY_FAILED");
        }
    }
    for f in findings {
        f.kev_status = "NOT_LISTED".into();
        if let Some(k) = entries.iter().find(|k| {
            k["cveID"]
                .as_str()
                .is_some_and(|id| f.cves.iter().any(|c| c == id))
        }) {
            let date = |v: &Value| {
                text(v).filter(|s| chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok())
            };
            f.kev = Some(Kev {
                date_added: date(&k["dateAdded"]).ok_or("QUERY_FAILED")?,
                due_date: date(&k["dueDate"]).ok_or("QUERY_FAILED")?,
                action: text(&k["requiredAction"]).ok_or("QUERY_FAILED")?,
                source_url: "https://www.cisa.gov/known-exploited-vulnerabilities-catalog".into(),
                checked_at: now.into(),
            });
            f.kev_status = "LISTED".into();
            f.evidence.push("CISA KEV lists exploitation; dates are US federal deadlines, not local device deadlines".into());
        }
        f.priority = priority(&f.applicability, f.severity.as_deref(), f.kev.is_some());
    }
    Ok(())
}
pub fn update_unknown_reasons(e: &crate::updates::UpdateEvidence) -> Vec<String> {
    if !matches!(e.status.as_str(), "UNKNOWN" | "ERROR") {
        return vec![];
    }
    let reason = if e.reason.contains("PIN") {
        "VERSION_PINNED"
    } else if e.reason.contains("TIMEOUT")
        || e.reason.contains("FAILED")
        || e.reason.contains("ERROR")
    {
        "QUERY_FAILED"
    } else if e.reason.contains("VERSION") {
        "VERSION_UNRESOLVED"
    } else if e.reason.contains("PRODUCT") || e.reason.contains("MATCH") {
        "PRODUCT_UNRESOLVED"
    } else {
        "SOURCE_UNSUPPORTED"
    };
    vec![reason.into()]
}
pub fn summarize(item: &mut SecurityItem) {
    let matched = item
        .findings
        .iter()
        .any(|f| f.applicability == Applicability::Matched);
    let potential = item
        .findings
        .iter()
        .any(|f| f.applicability == Applicability::Potential);
    item.applicability = if matched {
        Applicability::Matched
    } else if potential {
        Applicability::Potential
    } else {
        Applicability::Unknown
    };
    if item.applicability == Applicability::Unknown {
        item.reasons.push("COVERAGE_PARTIAL".into());
    }
    for reasons in [
        &mut item.reasons,
        &mut item.identity_reasons,
        &mut item.update_reasons,
    ] {
        reasons.sort();
        reasons.dedup();
    }
    item.priority = if item
        .findings
        .iter()
        .any(|f| f.priority == Priority::Critical)
    {
        Priority::Critical
    } else if item.findings.iter().any(|f| f.priority == Priority::High) {
        Priority::High
    } else if matched {
        Priority::Medium
    } else if item.update.status == "UPDATE_AVAILABLE" {
        Priority::Informational
    } else {
        Priority::Undetermined
    };
    // NOT_AFFECTED applies to a specific advisory, never to an entire device/product.
}

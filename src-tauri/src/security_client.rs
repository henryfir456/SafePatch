//! #28: fixed HTTPS destinations, bounded async requests and explicit partial coverage.
use crate::security::*;
use rusqlite::{params, OptionalExtension};
use serde_json::{json,Value};
use std::{collections::BTreeMap,path::Path,sync::{Arc,Mutex,OnceLock,atomic::{AtomicBool,Ordering}},time::{Duration,Instant}};
const MAX_BYTES:usize=8*1024*1024;
const TTL_SECONDS:i64=24*60*60;
const MAX_QUERIES:usize=80;
const OSV:&str="https://api.osv.dev/v1/query";
const NVD:&str="https://services.nvd.nist.gov/rest/json/cves/2.0";
const KEV:&str="https://www.cisa.gov/sites/default/files/feeds/known_exploited_vulnerabilities.json";
static NEXT:OnceLock<Mutex<BTreeMap<&'static str,Instant>>>=OnceLock::new();
pub enum Query { Osv{ name:String, version:String }, Nvd{ product:Product, version:String }, Kev }
impl Query {
    fn source(&self)->&'static str {match self {Self::Osv{..}=>"OSV",Self::Nvd{..}=>"NVD",Self::Kev=>"CISA_KEV"}}
    fn key(&self)->String {match self {Self::Osv{name,version}=>format!("osv-v1:{name}@{version}"),Self::Nvd{product,version}=>format!("nvd-v1:{}:{}@{version}",product.vendor,product.product),Self::Kev=>"kev-v1".into()}}
    fn request(&self,client:&reqwest::Client)->Result<reqwest::RequestBuilder,&'static str> {Ok(match self {
        Self::Osv{name,version}=>client.post(OSV).json(&json!({"package":{"ecosystem":"npm","name":name},"version":version})),
        Self::Nvd{product,version}=>{
            let mut url=reqwest::Url::parse(NVD).map_err(|_|"QUERY_FAILED")?;
            // Product + exact observed version only; returned configurations are independently evaluated.
            url.query_pairs_mut().append_pair("virtualMatchString",&format!("cpe:2.3:a:{}:{}:{}:*:*:*:*:*:*:*",product.vendor,product.product,version)).append_pair("resultsPerPage","2000").append_pair("startIndex","0");
            client.get(url)
        },Self::Kev=>client.get(KEV) })}
}
fn cancelled(cancel:&AtomicBool)->Result<(),&'static str>{if cancel.load(Ordering::Relaxed){Err("CANCELLED")}else{Ok(())}}
async fn cancellation(cancel:Arc<AtomicBool>){while !cancel.load(Ordering::Relaxed){tokio::time::sleep(Duration::from_millis(50)).await;}}
fn valid_cache_time(time:&str,now:chrono::DateTime<chrono::Utc>)->bool {chrono::DateTime::parse_from_rfc3339(time).is_ok_and(|t|{let age=now.signed_duration_since(t).num_seconds();(0..TTL_SECONDS).contains(&age)})}
fn parse_body(bytes:&[u8])->Result<Value,&'static str>{if bytes.len()>MAX_BYTES{return Err("RESPONSE_LIMIT")}serde_json::from_slice(bytes).map_err(|_|"QUERY_FAILED")}
async fn request_json(request:reqwest::RequestBuilder,wait:Duration,remaining:Duration,cancel:Arc<AtomicBool>)->Result<Value,String>{
    cancelled(&cancel)?;
    let operation=async {
        tokio::time::sleep(wait).await;
        let code=|e:reqwest::Error|if e.is_timeout(){"QUERY_TIMEOUT"}else{"QUERY_FAILED"};
        let mut response=request.send().await.map_err(code)?;
        if !response.status().is_success(){return Err(if response.status()==reqwest::StatusCode::TOO_MANY_REQUESTS{"RATE_LIMITED"}else{"QUERY_FAILED"})}
        if response.content_length().is_some_and(|n|n>MAX_BYTES as u64){return Err("RESPONSE_LIMIT")}
        let mut bytes=vec![];
        while let Some(chunk)=response.chunk().await.map_err(code)?{if bytes.len()+chunk.len()>MAX_BYTES{return Err("RESPONSE_LIMIT")}bytes.extend_from_slice(&chunk);}
        parse_body(&bytes)
    };
    let data=tokio::select!{_ = cancellation(cancel.clone())=>return Err("CANCELLED".into()), result=tokio::time::timeout(remaining,operation)=>result.map_err(|_|"QUERY_TIMEOUT")??};
    cancelled(&cancel)?;Ok(data)
}
struct Client<'a>{http:reqwest::Client,db:&'a rusqlite::Connection,cancel:Arc<AtomicBool>,started:Instant,requests:usize,memory:BTreeMap<String,(Value,SourceCheck)>}
impl<'a> Client<'a>{
    fn new(db:&'a rusqlite::Connection,cancel:Arc<AtomicBool>)->Result<Self,String>{Ok(Self{http:reqwest::Client::builder().https_only(true).redirect(reqwest::redirect::Policy::none()).no_proxy().connect_timeout(Duration::from_secs(8)).timeout(Duration::from_secs(20)).user_agent("SafePatch/0.3 security health").build().map_err(|_|"SECURITY_HTTP_FAILED")?,db,cancel,started:Instant::now(),requests:0,memory:BTreeMap::new()})}
    async fn query(&mut self,q:Query)->Result<(Value,SourceCheck),String>{
        cancelled(&self.cancel)?;let key=q.key();let source=q.source();
        if let Some(value)=self.memory.get(&key){return Ok(value.clone())}
        let cached:Option<(String,String)>=self.db.query_row("SELECT fetched_at,body FROM security_cache WHERE key=?1",[&crate::model::hash(&key)],|r|Ok((r.get(0)?,r.get(1)?))).optional().map_err(|_|"DATABASE_READ_FAILED")?;
        let mut cache_status="MISS";
        if let Some((time,body))=cached {if valid_cache_time(&time,chrono::Utc::now()) {if let Ok(data)=parse_body(body.as_bytes()){let check=SourceCheck{source:source.into(),status:"COMPLETE".into(),checked_at:time,reason:None,cache:"FRESH".into()};self.memory.insert(key,(data.clone(),check.clone()));return Ok((data,check))}}cache_status="EXPIRED";}
        if self.requests>=MAX_QUERIES || self.started.elapsed()>=Duration::from_secs(300){return Err("COVERAGE_PARTIAL".into())}
        self.requests+=1;
        let wait={let mut next=NEXT.get_or_init(||Mutex::new(BTreeMap::new())).lock().map_err(|_|"QUERY_FAILED")?;let now=Instant::now();let at=next.get(source).copied().unwrap_or(now).max(now);next.insert(source,at+Duration::from_millis(if source=="NVD"{6200}else{250}));at.saturating_duration_since(now)};
        let request=q.request(&self.http)?;
        let remaining=Duration::from_secs(300).saturating_sub(self.started.elapsed());
        let data=request_json(request,wait,remaining,self.cancel.clone()).await?;
        cancelled(&self.cancel)?;
        let time=chrono::Utc::now().to_rfc3339();let body=serde_json::to_string(&data).map_err(|_|"QUERY_FAILED")?;
        self.db.execute("INSERT INTO security_cache VALUES (?1,?2,?3) ON CONFLICT(key) DO UPDATE SET fetched_at=excluded.fetched_at,body=excluded.body",params![crate::model::hash(&key),time,body]).map_err(|_|"DATABASE_CACHE_FAILED")?;
        self.db.execute("DELETE FROM security_cache WHERE key NOT IN (SELECT key FROM security_cache ORDER BY fetched_at DESC LIMIT 128)",[]).map_err(|_|"DATABASE_CACHE_FAILED")?;
        while self.db.query_row("SELECT coalesce(sum(length(body)),0) FROM security_cache",[],|r|r.get::<_,usize>(0)).map_err(|_|"DATABASE_CACHE_FAILED")?>32*1024*1024 {self.db.execute("DELETE FROM security_cache WHERE key=(SELECT key FROM security_cache ORDER BY fetched_at LIMIT 1)",[]).map_err(|_|"DATABASE_CACHE_FAILED")?;}
        let check=SourceCheck{source:source.into(),status:"COMPLETE".into(),checked_at:time,reason:None,cache:cache_status.into()};self.memory.insert(key,(data.clone(),check.clone()));Ok((data,check))
    }
}
fn failure(item:&mut SecurityItem,source:&str,code:&str){item.reasons.push(if code=="COVERAGE_PARTIAL"{code}else if code=="VERSION_UNRESOLVED"{code}else{"QUERY_FAILED"}.into());item.checks.push(SourceCheck{source:source.into(),status:"UNKNOWN".into(),checked_at:chrono::Utc::now().to_rfc3339(),reason:Some(code.into()),cache:"UNAVAILABLE".into()});}
pub fn collect(path:&Path,scan_id:&str,windows:Option<(String,Vec<crate::model::Asset>)>,npm:Option<(String,crate::npm::DependencyGraph)>,cancel:Arc<AtomicBool>)->Result<SecurityReport,String>{
    let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build().map_err(|_|"SECURITY_RUNTIME_FAILED")?;
    runtime.block_on(async{
        let db=crate::db::open(path).map_err(|_|"DATABASE_OPEN_FAILED")?;let mut client=Client::new(&db,cancel.clone())?;
        let mut report=SecurityReport{scan_id:scan_id.into(),windows_scan_id:windows.as_ref().map(|w|w.0.clone()),npm_scan_id:npm.as_ref().map(|n|n.0.clone()),checked_at:chrono::Utc::now().to_rfc3339(),status:"PARTIAL".into(),items:vec![]};
        for a in windows.map(|w|w.1).unwrap_or_default(){
            cancelled(&cancel)?;let mut item=SecurityItem{id:a.id.clone(),kind:"WINDOWS_INSTALLED".into(),display_name:a.display_name.clone(),installed_version:a.installed_version.clone(),update:a.update.clone(),applicability:Applicability::Unknown,priority:Priority::Undetermined,reasons:vec![],checks:vec![],findings:vec![]};
            if let Some(p)=product_for(&a){if let Some(version)=a.installed_version.as_deref().filter(|v|numeric_compare(v,v).is_some()){
                match client.query(Query::Nvd{product:p,version:version.into()}).await {
                    Ok((doc,mut check))=>match parse_nvd(&doc,p,version,&check.checked_at){Ok((findings,complete))=>{item.findings=findings;if !complete{check.status="PARTIAL".into();check.reason=Some("COVERAGE_PARTIAL".into());item.reasons.push("COVERAGE_PARTIAL".into());}item.checks.push(check);},Err(code)=>failure(&mut item,"NVD",code)},
                    Err(code) if code=="CANCELLED"=>return Err(code),Err(code)=>failure(&mut item,"NVD",&code)
                }
            }else{item.reasons.push("VERSION_UNRESOLVED".into());}}else{item.reasons.extend(["PRODUCT_UNRESOLVED".into(),"SOURCE_UNSUPPORTED".into()]);}
            if item.findings.is_empty(){item.reasons.push("COVERAGE_PARTIAL".into());}
            report.items.push(item);
        }
        if let Some((_,graph))=npm{
            for p in graph.packages {
                cancelled(&cancel)?;let mut item=SecurityItem{id:p.id,kind:"NPM_LOCKFILE_DECLARATION".into(),display_name:p.name.clone(),installed_version:p.version.clone(),update:Default::default(),applicability:Applicability::Unknown,priority:Priority::Undetermined,reasons:vec!["COVERAGE_PARTIAL".into()],checks:vec![],findings:vec![]};
                if p.source=="NPM_REGISTRY_METADATA" {if let Some(version)=p.version{
                    match client.query(Query::Osv{name:p.name,version:version.clone()}).await {
                        Ok((doc,check))=>match parse_osv(&doc,&item.display_name,&version,false,&check.checked_at){Ok(findings)=>{item.findings=findings;item.checks.push(check)},Err(code)=>failure(&mut item,"OSV",code)},
                        Err(code) if code=="CANCELLED"=>return Err(code),Err(code)=>failure(&mut item,"OSV",&code)
                    }
                }else{item.reasons.push("VERSION_UNRESOLVED".into())}}else{item.reasons.push("SOURCE_UNSUPPORTED".into())}
                report.items.push(item);
            }
        }
        if report.items.iter().any(|i|i.findings.iter().any(|f|!f.cves.is_empty())) {
            match client.query(Query::Kev).await {
                Ok((doc,check))=>{
                    // Validate catalog before mutating any findings; malformed KEV is not a negative list.
                    let mut all:Vec<_>=report.items.iter().flat_map(|i|i.findings.clone()).collect();
                    if let Err(code)=apply_kev(&mut all,&doc,&check.checked_at){for i in &mut report.items{if !i.findings.is_empty(){failure(i,"CISA_KEV",code)}}}else{
                        for i in &mut report.items {if !i.findings.is_empty(){apply_kev(&mut i.findings,&doc,&check.checked_at).map_err(str::to_owned)?;i.checks.push(check.clone());}}
                    }
                },Err(code) if code=="CANCELLED"=>return Err(code),Err(code)=>{for i in &mut report.items{if !i.findings.is_empty(){failure(i,"CISA_KEV",&code);for f in &mut i.findings{f.kev_status="UNKNOWN".into();}}}}
            }
        }
        for i in &mut report.items {summarize(i);}
        cancelled(&cancel)?;Ok(report)
    })
}
#[cfg(test)]mod tests{
    use super::*;
    #[test]fn stale_and_future_cache_are_not_safe(){let now=chrono::DateTime::parse_from_rfc3339("2026-10-14T00:00:00Z").unwrap().with_timezone(&chrono::Utc);assert!(valid_cache_time("2026-10-13T23:00:00Z",now));for s in ["2026-10-13T00:00:00Z","2026-10-15T00:00:00Z","garbage"]{assert!(!valid_cache_time(s,now));}}
    #[test]fn poisoned_or_large_response_fails(){assert!(parse_body(b"<html>error</html>").is_err());assert!(parse_body(&vec![b' ';MAX_BYTES+1]).is_err());}
    #[test]fn query_payload_minimizes_metadata(){let q=Query::Osv{name:"lodash".into(),version:"4.17.20".into()};let client=reqwest::Client::new();let r=q.request(&client).unwrap().build().unwrap();assert_eq!(r.url().as_str(),OSV);let body=r.body().unwrap().as_bytes().unwrap();assert_eq!(serde_json::from_slice::<Value>(body).unwrap(),json!({"package":{"ecosystem":"npm","name":"lodash"},"version":"4.17.20"}));assert!(!q.key().contains("C:\\"));}
    #[test]fn acknowledged_cancel_prevents_network_work(){let dir=tempfile::tempdir().unwrap();let cancel=Arc::new(AtomicBool::new(true));let graph=crate::npm::parse_lock(br#"{"lockfileVersion":3,"packages":{"node_modules/lodash":{"version":"4.17.20","resolved":"https://registry.npmjs.org/lodash/-/lodash-4.17.20.tgz"}}}"#,&"a".repeat(64),None,&AtomicBool::new(false)).unwrap();assert_eq!(collect(&dir.path().join("db"),"scan",None,Some(("npm".into(),graph)),cancel).unwrap_err(),"CANCELLED");}
}

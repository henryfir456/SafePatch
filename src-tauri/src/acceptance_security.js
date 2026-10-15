// Acceptance-only: actual local inventory, actual advisory calls, native DOM and IPC.
(async()=>{
 const invoke=window.__TAURI_INTERNALS__.invoke;
 const wait=async predicate=>{const deadline=Date.now()+350000;while(Date.now()<deadline){if(await predicate())return;await new Promise(r=>setTimeout(r,100));}throw Error('ACCEPTANCE_SECURITY_TIMEOUT');};
 const buttons=()=>[...document.querySelectorAll('button')];
 const button=name=>buttons().find(b=>b.textContent===name);
 let mode='security-failed';
 try{
  mode=await invoke('acceptance_mode');
  const metadata=await invoke('build_info');
  await wait(()=>document.querySelector('[data-testid=app-version]')?.textContent==='v'+metadata.version);
  await wait(()=>document.querySelector('footer')?.textContent.includes(metadata.build_sha.slice(0,8)));
  const footer=document.querySelector('footer');
  if(!footer?.textContent.includes(metadata.build_sha.slice(0,8)))throw Error('ACCEPTANCE_BUILD_METADATA_FAILED');
  [...footer.querySelectorAll('summary')].find(s=>s.textContent==='關於 SafePatch').click();
  if(!footer.textContent.includes(metadata.build_sha)||!footer.textContent.includes(metadata.channel))throw Error('ACCEPTANCE_ABOUT_FAILED');
  let cancellation=false;
  if(mode==='security-scan'){
   await wait(()=>button('開始唯讀掃描')&&!button('開始唯讀掃描').disabled);button('開始唯讀掃描').click();
   await wait(async()=>{const s=await invoke('latest_scan');return s&&['COMPLETE','PARTIAL'].includes(s.status);});
   const label=[...document.querySelectorAll('label')].find(l=>l.textContent.includes('允許以上最小必要外部安全查詢'));
   if(!label)throw Error('ACCEPTANCE_CONSENT_MISSING');label.querySelector('input').click();
   await wait(()=>button('開始安全檢查')&&!button('開始安全檢查').disabled);button('開始安全檢查').click();
   await wait(()=>button('取消安全檢查'));button('取消安全檢查').click();
   await wait(async()=>{const s=await invoke('latest_security_scan');return s&&s.status!=='RUNNING';});
   const cancelled=await invoke('latest_security_scan');if(cancelled.status!=='CANCELLED')throw Error('ACCEPTANCE_SECURITY_CANCEL_FAILED');cancellation=true;
   if(await invoke('last_security_report'))throw Error('ACCEPTANCE_CANCEL_PUBLISHED_REPORT');
   await wait(()=>button('開始安全檢查')&&!button('開始安全檢查').disabled);button('開始安全檢查').click();
   await wait(async()=>{const s=await invoke('latest_security_scan');return s&&s.id!==cancelled.id&&s.status!=='RUNNING';});
  }
  const report=await invoke('last_security_report');if(!report||report.status!=='PARTIAL'||!report.items.length)throw Error('ACCEPTANCE_SECURITY_REPORT_MISSING');
  const order={CRITICAL:0,HIGH:1,MEDIUM:2,INFORMATIONAL:3,UNDETERMINED:4};
  const expected=[...report.items].sort((a,b)=>order[a.priority]-order[b.priority]||a.display_name.localeCompare(b.display_name)||a.id.localeCompare(b.id)).slice(0,200);
  await wait(()=>document.querySelector('table[aria-label="安全風險結果"] tbody')?.rows.length===expected.length&&document.querySelector('section[aria-label="安全健檢"]')?.textContent.includes(report.checked_at));
  const rows=[...document.querySelector('table[aria-label="安全風險結果"] tbody').rows];
  if(rows.some((r,i)=>r.cells[0].querySelector('b')?.textContent!==expected[i].display_name||r.cells[1].textContent!==(expected[i].installed_version??'UNKNOWN')||r.cells[2].textContent!==(expected[i].update.target_version??'—')||!r.cells[3].textContent.includes(expected[i].priority)||!r.cells[4].textContent.includes(expected[i].applicability)))throw Error('ACCEPTANCE_SECURITY_ROWS_FAILED');
  const sourceSuccess=report.items.some(i=>i.checks.some(c=>c.status==='COMPLETE'&&['NVD','OSV'].includes(c.source)));
  if(!sourceSuccess)throw Error('ACCEPTANCE_SECURITY_SOURCE_FAILED');
  rows[0].querySelector('summary').click();if(!rows[0].querySelector('details').open)throw Error('ACCEPTANCE_DETAILS_FAILED');
  await invoke('acceptance_security_report',{report:{mode,scan_id:report.scan_id,item_count:report.items.length,rendered_rows:rows.length,cancellation,metadata_verified:true,passed:true,error_code:null}});
 }catch(e){const code=/^ACCEPTANCE_[A-Z_]+$/.test(e?.message??'')?e.message:'ACCEPTANCE_SECURITY_FAILED';await invoke('acceptance_security_report',{report:{mode,scan_id:null,item_count:0,rendered_rows:0,cancellation:false,metadata_verified:false,passed:false,error_code:code}});}
})();

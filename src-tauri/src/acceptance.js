// Compiled ONLY into the acceptance-feature test executable. Real DOM + real IPC.
// No fixtures, device data uploads, external scripts, or discovered binaries.
(async()=>{
  const invoke=window.__TAURI_INTERNALS__.invoke;
  const wait=async predicate=>{const deadline=Date.now()+120000;while(Date.now()<deadline){if(await predicate())return;await new Promise(r=>setTimeout(r,100));}throw Error('ACCEPTANCE_TIMEOUT');};
  const buttons=()=>[...document.querySelectorAll('button')];
  const start=()=>buttons().find(b=>b.textContent==='開始唯讀掃描');
  try{
    await wait(()=>!!start());
    await new Promise(r=>setTimeout(r,300));
    const mode=await invoke('acceptance_mode');
    let cancellation=false;
    if(mode==='scan'){
      start().click();await wait(()=>buttons().some(b=>b.textContent==='取消掃描'));
      buttons().find(b=>b.textContent==='取消掃描').click();
      await wait(async()=>{const s=await invoke('latest_scan');return s&&s.status!=='RUNNING';});
      const cancelled=await invoke('latest_scan');
      if(cancelled.status!=='CANCELLED')throw Error('ACCEPTANCE_CANCEL_FAILED');
      cancellation=true;
      await wait(()=>start()&&!start().disabled);start().click();
      await wait(async()=>{const s=await invoke('latest_scan');return s&&s.id!==cancelled.id&&s.status!=='RUNNING';});
    }
    const s=await invoke('latest_scan');
    if(!s||!['PARTIAL','COMPLETE'].includes(s.status))throw Error('ACCEPTANCE_SCAN_FAILED');
    const assets=await invoke('list_assets',{scanId:s.id});
    if(!assets.length)throw Error('ACCEPTANCE_NO_ASSETS');
    await wait(()=>document.querySelectorAll('tbody tr').length===assets.length);
    const rows=[...document.querySelectorAll('tbody tr')];
    const versioned=assets.filter(a=>a.installed_version!==null).length;
    const renderedVersions=rows.filter(r=>r.cells[1].textContent!=='UNKNOWN').length;
    if(versioned!==renderedVersions)throw Error('ACCEPTANCE_VERSION_RENDER_FAILED');
    if(!document.body.textContent.includes('UNKNOWN ≠ 安全'))throw Error('ACCEPTANCE_UNKNOWN_LABEL_FAILED');
    if(s.coverage.some(c=>c.status!=='COMPLETE')&&!document.body.textContent.includes('PARTIAL'))throw Error('ACCEPTANCE_COVERAGE_RENDER_FAILED');
    await invoke('acceptance_report',{report:{mode,scan_id:s.id,asset_count:assets.length,rendered_rows:rows.length,versioned_count:versioned,rendered_versions:renderedVersions,cancellation,passed:true,error_code:null}});
  }catch{
    await invoke('acceptance_report',{report:{mode:'failed',scan_id:null,asset_count:0,rendered_rows:0,versioned_count:0,rendered_versions:0,cancellation:false,passed:false,error_code:'DESKTOP_ACCEPTANCE_FAILED'}});
  }
})();

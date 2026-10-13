// Compiled ONLY into the acceptance-feature test executable. Real DOM + real IPC.
// No fixtures, device data uploads, external scripts, or discovered binaries.
(async()=>{
  const invoke=window.__TAURI_INTERNALS__.invoke;
  const wait=async predicate=>{const deadline=Date.now()+210000;while(Date.now()<deadline){if(await predicate())return;await new Promise(r=>setTimeout(r,100));}throw Error('ACCEPTANCE_TIMEOUT');};
  const buttons=()=>[...document.querySelectorAll('button')];
  const start=()=>buttons().find(b=>b.textContent==='開始唯讀掃描');
  try{
    await wait(()=>start()&&!start().disabled);
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
    const available=assets.filter(a=>a.update?.status==='UPDATE_AVAILABLE').length;
    await wait(()=>document.querySelector('[data-testid=update-count]')?.textContent===String(available));
    const updateRows=[...document.querySelectorAll('section:not([hidden]) tbody tr')];
    if(updateRows.length!==available || updateRows.some(r=>!r.cells[3].textContent.includes('UPDATE_AVAILABLE')))throw Error('ACCEPTANCE_UPDATE_RENDER_FAILED');
    buttons().find(b=>b.textContent==='完整軟體清單').click();
    await wait(()=>document.querySelectorAll('section:not([hidden]) tbody tr').length===assets.length);
    const rows=[...document.querySelectorAll('section:not([hidden]) tbody tr')];
    const versioned=assets.filter(a=>a.installed_version!==null).length;
    const renderedVersions=rows.filter(r=>r.cells[1].textContent!=='UNKNOWN').length;
    if(versioned!==renderedVersions)throw Error('ACCEPTANCE_VERSION_RENDER_FAILED');
    if(rows.some((r,i)=>r.cells[0].querySelector('b').textContent!==assets[i].display_name||r.cells[1].textContent!==(assets[i].installed_version??'UNKNOWN')||r.cells[2].textContent!==(assets[i].publisher??'UNKNOWN')))throw Error('ACCEPTANCE_EXACT_METADATA_FAILED');
    if(rows.some((r,i)=>assets[i].provenance.some(p=>p.profile_ref&&!r.cells[3].textContent.includes('profile: '+p.profile_ref))))throw Error('ACCEPTANCE_PROFILE_RENDER_FAILED');
    if(rows.some((r,i)=>assets[i].support&&!r.cells[4].textContent.includes('support: '+assets[i].support.status)))throw Error('ACCEPTANCE_SUPPORT_RENDER_FAILED');
    if(!document.body.textContent.includes('UNKNOWN ≠ 安全'))throw Error('ACCEPTANCE_UNKNOWN_LABEL_FAILED');
    if(s.coverage.some(c=>c.status!=='COMPLETE')&&!document.body.textContent.includes('PARTIAL'))throw Error('ACCEPTANCE_COVERAGE_RENDER_FAILED');
    await invoke('acceptance_report',{report:{mode,scan_id:s.id,asset_count:assets.length,rendered_rows:rows.length,versioned_count:versioned,rendered_versions:renderedVersions,cancellation,passed:true,error_code:null}});
  }catch(error){
    const code=/^ACCEPTANCE_[A-Z_]+$/.test(error?.message??'')?error.message:'DESKTOP_ACCEPTANCE_FAILED';
    await invoke('acceptance_report',{report:{mode:'failed',scan_id:null,asset_count:0,rendered_rows:0,versioned_count:0,rendered_versions:0,cancellation:false,passed:false,error_code:code}});
  }
})();

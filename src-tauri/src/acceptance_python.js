// Acceptance feature only. Actual selected installed environment, DOM and IPC.
(async()=>{
  const invoke=window.__TAURI_INTERNALS__.invoke;
  const wait=async p=>{const end=Date.now()+60000;while(Date.now()<end){if(await p())return;await new Promise(r=>setTimeout(r,5));}throw Error('ACCEPTANCE_TIMEOUT');};
  const button=text=>[...document.querySelectorAll('button')].find(b=>b.textContent===text);
  const start=()=>button('Scan Python metadata');
  try{
    const mode=await invoke('acceptance_mode');let cancellation=false;
    if(mode==='python-scan'){
      await wait(()=>document.querySelector('[aria-label="Python environment directory"]'));
      const input=document.querySelector('[aria-label="Python environment directory"]'),root=await invoke('acceptance_project_root');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,root);input.dispatchEvent(new Event('input',{bubbles:true}));
      await wait(()=>start()&&!start().disabled);start().click();await wait(()=>button('Cancel Python scan'));button('Cancel Python scan').click();
      await wait(async()=>{const s=await invoke('latest_python_scan');return s&&s.status!=='RUNNING';});const cancelled=await invoke('latest_python_scan');if(cancelled.status!=='CANCELLED')throw Error('ACCEPTANCE_CANCEL_FAILED');cancellation=true;
      await wait(()=>start()&&!start().disabled);start().click();await wait(async()=>{const s=await invoke('latest_python_scan');return s&&s.id!==cancelled.id&&s.status!=='RUNNING';});
    }
    const scan=await invoke('latest_python_scan');if(!scan||scan.status!=='PARTIAL')throw Error('ACCEPTANCE_SCAN_FAILED');
    const snapshot=await invoke('python_snapshot',{scanId:scan.id});if(!snapshot?.packages.length||!snapshot.interpreters.length)throw Error('ACCEPTANCE_SNAPSHOT_MISSING');
    const rows=()=>[...document.querySelectorAll('[data-python-package-id]')];let rendered=0;
    await wait(()=>rows().length===Math.min(200,snapshot.packages.length));
    for(let page=0;page*200<snapshot.packages.length;page++){
      const expected=snapshot.packages.slice(page*200,(page+1)*200);await wait(()=>rows()[0]?.getAttribute('data-python-package-id')===expected[0].id);
      const actual=rows();if(actual.length!==expected.length||actual.some((r,i)=>r.getAttribute('data-python-package-id')!==expected[i].id||r.cells[0].firstChild.textContent!==expected[i].display_name||r.cells[1].textContent!==(expected[i].version??'UNKNOWN')||r.cells[2].textContent!=='UNKNOWN'||r.cells[3].textContent!==expected[i].source||r.cells[4].firstChild.textContent!==expected[i].installer))throw Error('ACCEPTANCE_EXACT_SNAPSHOT_FAILED');
      rendered+=actual.length;if((page+1)*200<snapshot.packages.length){const next=button('Next Python page');if(!next||next.disabled)throw Error('ACCEPTANCE_PAGINATION_FAILED');next.click();}
    }
    const interpreters=()=>[...document.querySelectorAll('[data-python-interpreter-id]')];await wait(()=>interpreters().length===snapshot.interpreters.length);
    if(interpreters().some((r,i)=>r.getAttribute('data-python-interpreter-id')!==snapshot.interpreters[i].id||r.querySelector('[data-python-interpreter-version]').textContent.trim()!==`· ${snapshot.interpreters[i].version??'UNKNOWN'}`||!r.textContent.includes(snapshot.interpreters[i].source)))throw Error('ACCEPTANCE_INTERPRETER_FAILED');
    if(!document.body.textContent.includes('INSTALLED_METADATA_NOT_CONTENT_PROOF'))throw Error('ACCEPTANCE_UNCERTAINTY_FAILED');
    await invoke('acceptance_python_report',{report:{mode,scan_id:scan.id,package_count:snapshot.packages.length,interpreter_count:snapshot.interpreters.length,rendered_rows:rendered,rendered_interpreters:interpreters().length,versioned_count:snapshot.packages.filter(p=>p.version!==null).length,interpreter_versions:snapshot.interpreters.filter(p=>p.version!==null).length,cancellation,passed:true,error_code:null}});
  }catch(error){const code=/^ACCEPTANCE_[A-Z_]+$/.test(error?.message??'')?error.message:'PYTHON_DESKTOP_ACCEPTANCE_FAILED';await invoke('acceptance_python_report',{report:{mode:'failed',scan_id:null,package_count:0,interpreter_count:0,rendered_rows:0,rendered_interpreters:0,versioned_count:0,interpreter_versions:0,cancellation:false,passed:false,error_code:code}});}
})();

// Acceptance-feature only. Actual DOM/IPC, selected owned checkout, no fixture graph.
(async()=>{
  const invoke=window.__TAURI_INTERNALS__.invoke;
  const wait=async p=>{const end=Date.now()+60000;while(Date.now()<end){if(await p())return;await new Promise(r=>setTimeout(r,5));}throw Error('ACCEPTANCE_TIMEOUT');};
  const button=text=>[...document.querySelectorAll('button')].find(b=>b.textContent===text);
  const start=()=>button('開始 npm 唯讀掃描');
  try{
    const mode=await invoke('acceptance_mode');let cancellation=false;
    if(mode==='npm-scan'){
      await wait(()=>document.querySelector('[aria-label="npm project directory"]'));
      const input=document.querySelector('[aria-label="npm project directory"]');const root=await invoke('acceptance_project_root');
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype,'value').set.call(input,root);input.dispatchEvent(new Event('input',{bubbles:true}));
      await wait(()=>start()&&!start().disabled);start().click();await wait(()=>button('取消 npm 掃描'));button('取消 npm 掃描').click();
      await wait(async()=>{const s=await invoke('latest_npm_scan');return s&&s.status!=='RUNNING';});const cancelled=await invoke('latest_npm_scan');
      if(cancelled.status!=='CANCELLED')throw Error('ACCEPTANCE_CANCEL_FAILED');cancellation=true;
      await wait(()=>start()&&!start().disabled);start().click();
      await wait(async()=>{const s=await invoke('latest_npm_scan');return s&&s.id!==cancelled.id&&s.status!=='RUNNING';});
    }
    const scan=await invoke('latest_npm_scan');if(!scan||scan.status!=='PARTIAL')throw Error('ACCEPTANCE_SCAN_FAILED');
    const graph=await invoke('npm_graph',{scanId:scan.id});if(!graph?.packages.length)throw Error('ACCEPTANCE_GRAPH_MISSING');
    const rows=()=>[...document.querySelectorAll('[data-npm-package-id]')];let rendered=0;
    await wait(()=>rows().length===Math.min(200,graph.packages.length));
    for(let page=0;page*200<graph.packages.length;page++){
      const expected=graph.packages.slice(page*200,(page+1)*200);
      await wait(()=>rows()[0]?.getAttribute('data-npm-package-id')===expected[0].id);
      const actual=rows();if(actual.length!==expected.length||actual.some((r,i)=>r.getAttribute('data-npm-package-id')!==expected[i].id||r.cells[0].firstChild.textContent!==expected[i].name||r.cells[1].textContent!==(expected[i].version??'UNKNOWN')||r.cells[2].textContent!==expected[i].direct_status||r.cells[3].textContent!==expected[i].source))throw Error('ACCEPTANCE_EXACT_GRAPH_FAILED');
      rendered+=actual.length;if((page+1)*200<graph.packages.length){const next=button('下一頁');if(!next||next.disabled)throw Error('ACCEPTANCE_PAGINATION_FAILED');next.click();}
    }
    if(!document.body.textContent.includes('LOCKFILE_DECLARATIONS_NOT_INSTALLED_PROOF'))throw Error('ACCEPTANCE_UNCERTAINTY_FAILED');
    await invoke('acceptance_npm_report',{report:{mode,scan_id:scan.id,package_count:graph.packages.length,edge_count:graph.edges.length,rendered_rows:rendered,versioned_count:graph.packages.filter(p=>p.version!==null).length,cancellation,passed:true,error_code:null}});
  }catch(error){const code=/^ACCEPTANCE_[A-Z_]+$/.test(error?.message??'')?error.message:'NPM_DESKTOP_ACCEPTANCE_FAILED';await invoke('acceptance_npm_report',{report:{mode:'failed',scan_id:null,package_count:0,edge_count:0,rendered_rows:0,versioned_count:0,cancellation:false,passed:false,error_code:code}});}
})();

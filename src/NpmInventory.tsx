import {useEffect,useState} from 'react';
import {api} from './api';
import type {Scan} from './api';
import {npmApi} from './npmApi';
import type {DependencyGraph} from './npmApi';
function code(error:unknown,fallback:string){return typeof error==='string'&&/^(NPM_|DATABASE_|SCAN_)[A-Z_]{1,70}$/.test(error)?error:fallback;}
export function NpmInventory({disabled,onActive}:{disabled:boolean;onActive:(active:boolean)=>void}){
  const [root,setRoot]=useState(''),[scan,setScan]=useState<Scan|null>(null),[graph,setGraph]=useState<DependencyGraph|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false),[loading,setLoading]=useState(true),[query,setQuery]=useState(''),[relation,setRelation]=useState('all'),[page,setPage]=useState(0),[selected,setSelected]=useState<string|null>(null);
  const running=scan?.status==='RUNNING';
  useEffect(()=>{onActive(busy||running);},[busy,running,onActive]);
  useEffect(()=>{let alive=true;npmApi.latest().then(async s=>{const g=s?await npmApi.graph(s.id):null;if(alive){setGraph(g);setScan(s);}}).catch(()=>{if(alive)setError('NPM_BACKEND_UNAVAILABLE');}).finally(()=>{if(alive)setLoading(false);});return()=>{alive=false;};},[]);
  useEffect(()=>{if(!running||!scan)return;let alive=true,pending=false;const id=scan.id;const timer=setInterval(async()=>{if(pending)return;pending=true;try{const s=await api.status(id);if(s.status!=='RUNNING'){const g=await npmApi.graph(id);if(alive){setGraph(g);setScan(s);}}else if(alive)setScan(s);}catch{if(alive)setError('NPM_STATUS_UNAVAILABLE');}finally{pending=false;}},250);return()=>{alive=false;clearInterval(timer);};},[running,scan?.id]);
  async function start(){setBusy(true);setError('');try{const s=await npmApi.start(root);setScan(s);setGraph(null);setPage(0);setSelected(null);}catch(e){setError(code(e,'NPM_START_FAILED'));}finally{setBusy(false);}}
  async function cancel(){if(scan)try{await api.cancel(scan.id);}catch{setError('NPM_CANCEL_FAILED');}}
  const packages=graph?.packages??[];const shown=packages.filter(p=>p.name.toLowerCase().includes(query.toLowerCase())&&(relation==='all'||p.direct_status===relation));const visible=shown.slice(page*200,(page+1)*200);
  const outgoing=graph?.edges.filter(e=>e.from_id===selected)??[];const byId=new Map(packages.map(p=>[p.id,p]));
  return <section className="inventory" aria-label="npm project inventory"><h2>npm 專案依賴盤點</h2>
    <p>只讀選定本地目錄的 package-lock.json / npm-shrinkwrap.json 與 package.json。支援 lockfile v1–v3；不讀 .npmrc、不執行 Node/npm/scripts、不安裝依賴。pnpm / Yarn 尚不支援。</p>
    <p className="notice">Lockfile 宣告不等於套件已安裝，也不證明公開來源或漏洞適用性。UNKNOWN ≠ 安全。路徑僅在掃描期間使用；資料庫保留雜湊。</p>
    <label>選定專案目錄 <input aria-label="npm project directory" value={root} onChange={e=>setRoot(e.target.value)} placeholder="C:\projects\example" disabled={running||busy}/></label>
    <button onClick={start} disabled={disabled||loading||busy||running||!root}>開始 npm 唯讀掃描</button>{running&&<button className="secondary" onClick={cancel}>取消 npm 掃描</button>}
    {error&&<p role="alert" className="error">{error}</p>}
    {scan&&<div className="progress" aria-live="polite"><b>npm: {scan.status}</b><p>{scan.phase} · {scan.progress}%</p><progress aria-label="npm scan progress" max={100} value={scan.progress}/>{scan.error_code&&<p role="alert" className="error">{scan.error_code}</p>}</div>}
    {graph&&<><p>專案 {graph.root_path_hash.slice(0,12)} · {graph.lockfile_kind} v{graph.lockfile_version} · {packages.length} 個宣告 · {graph.edges.length} 條依賴關係 · {graph.observed_at}</p>
      <ul>{graph.codes.map(c=><li key={c}>{c}</li>)}</ul>
      <label>搜尋套件 <input aria-label="npm package search" value={query} onChange={e=>{setQuery(e.target.value);setPage(0);}}/></label>
      <fieldset><legend>依賴關係</legend>{['all','DIRECT','TRANSITIVE','UNKNOWN'].map(r=><label key={r}><input type="radio" name="npm-relation" checked={relation===r} onChange={()=>{setRelation(r);setPage(0);}}/>{r}</label>)}</fieldset>
      <div className="table-wrap"><table aria-label="npm dependency declarations"><thead><tr><th>套件</th><th>宣告版本</th><th>關係</th><th>來源宣告</th><th>依賴圖</th></tr></thead><tbody>{visible.map(p=><tr key={p.id} data-npm-package-id={p.id}><td>{p.name}<small>slot: {p.slot_hash.slice(0,12)}</small></td><td>{p.version??'UNKNOWN'}</td><td>{p.direct_status}</td><td>{p.source}</td><td><button className="secondary" onClick={()=>setSelected(p.id)}>查看依賴</button></td></tr>)}</tbody></table></div>
      <p>顯示 {page*200+visible.length} / {shown.length} · 每頁 200 個宣告</p><button disabled={page===0} onClick={()=>setPage(page-1)}>上一頁</button><button disabled={(page+1)*200>=shown.length} onClick={()=>setPage(page+1)}>下一頁</button>
      {selected&&<div aria-label="npm outgoing dependencies"><h3>{byId.get(selected)?.name} 的依賴</h3>{outgoing.length?outgoing.map(e=><p key={e.to_id}>{byId.get(e.to_id)?.name??'UNKNOWN'} · {byId.get(e.to_id)?.version??'UNKNOWN'} · {e.kinds.join(', ')}</p>):<p>沒有可解析的依賴關係；不代表没有間接依賴。</p>}</div>}
    </>}
  </section>;
}

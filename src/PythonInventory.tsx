import {useEffect,useState} from 'react';
import {api} from './api';
import type {Scan} from './api';
import {pythonApi} from './pythonApi';
import type {PythonSnapshot} from './pythonApi';
function code(error:unknown,fallback:string){return typeof error==='string'&&/^(PYTHON_|DATABASE_|SCAN_)[A-Z_]{1,70}$/.test(error)?error:fallback;}
export function PythonInventory({disabled,onActive}:{disabled:boolean;onActive:(active:boolean)=>void}){
  const [root,setRoot]=useState(''),[scan,setScan]=useState<Scan|null>(null),[snapshot,setSnapshot]=useState<PythonSnapshot|null>(null),[error,setError]=useState(''),[busy,setBusy]=useState(false),[loading,setLoading]=useState(true),[query,setQuery]=useState(''),[page,setPage]=useState(0);
  const running=scan?.status==='RUNNING';
  useEffect(()=>{onActive(busy||running);},[busy,running,onActive]);
  useEffect(()=>{let alive=true;pythonApi.latest().then(async s=>{const result=s?await pythonApi.snapshot(s.id):null;if(alive){setSnapshot(result);setScan(s);}}).catch(()=>{if(alive)setError('PYTHON_BACKEND_UNAVAILABLE');}).finally(()=>{if(alive)setLoading(false);});return()=>{alive=false;};},[]);
  useEffect(()=>{if(!running||!scan)return;let alive=true,pending=false;const id=scan.id;const timer=setInterval(async()=>{if(pending)return;pending=true;try{const s=await api.status(id);if(s.status!=='RUNNING'){const result=await pythonApi.snapshot(id);if(alive){setSnapshot(result);setScan(s);}}else if(alive)setScan(s);}catch{if(alive)setError('PYTHON_STATUS_UNAVAILABLE');}finally{pending=false;}},250);return()=>{alive=false;clearInterval(timer);};},[running,scan?.id]);
  async function start(){setBusy(true);setError('');try{const s=await pythonApi.start(root);setScan(s);setSnapshot(null);setPage(0);}catch(e){setError(code(e,'PYTHON_START_FAILED'));}finally{setBusy(false);}}
  async function cancel(){if(scan)try{await api.cancel(scan.id);}catch{setError('PYTHON_CANCEL_FAILED');}}
  const packages=snapshot?.packages??[],shown=packages.filter(p=>(p.name+' '+p.display_name).toLowerCase().includes(query.toLowerCase())),visible=shown.slice(page*200,(page+1)*200);
  return <section className="inventory" aria-label="Python environment inventory"><h2>Python 環境盤點</h2>
    <p>選取 interpreter 安裝目錄或 venv 根目錄，唯讀檢查 python.exe 檔案資源、pyvenv.cfg 與 Lib/site-packages 的 .dist-info metadata。每次只掃描一個選定環境，不執行 Python、pip、activation script 或 .pth，不安裝套件。</p>
    <p className="notice">Metadata 不證明程式內容、Publisher 身分或公開套件來源；直接與間接依賴關係及漏洞適用性均為 UNKNOWN。UNKNOWN ≠ 安全。venv 的 base、user site、editable 與額外 import 路徑不在此範圍。</p>
    <label>選定環境目錄 <input aria-label="Python environment directory" value={root} onChange={e=>setRoot(e.target.value)} placeholder="C:\Python313" disabled={busy||running}/></label>
    <button onClick={start} disabled={disabled||loading||busy||running||!root}>Scan Python metadata</button>{running&&<button className="secondary" onClick={cancel}>Cancel Python scan</button>}
    {error&&<p role="alert" className="error">{error}</p>}
    {scan&&<div className="progress" aria-live="polite"><b>Python: {scan.status}</b><p>{scan.phase} · {scan.progress}%</p><progress aria-label="Python scan progress" max={100} value={scan.progress}/>{scan.error_code&&<p role="alert" className="error">{code(scan.error_code,'PYTHON_SCAN_FAILED')}</p>}</div>}
    {snapshot&&<><p>環境 {snapshot.root_path_hash.slice(0,12)} · {snapshot.environment_kind} · {packages.length} 個 distribution metadata · {snapshot.observed_at}</p>
      <div aria-label="Python interpreter observations"><h3>Interpreter 檔案資源觀測</h3>{snapshot.interpreters.length?snapshot.interpreters.map(p=><div key={p.id} data-python-interpreter-id={p.id}><b>{p.product_name??'Python candidate'}</b><span data-python-interpreter-version> · {p.version??'UNKNOWN'}</span><small>FileVersion: {p.file_version??'UNKNOWN'} · Publisher metadata: {p.publisher??'UNKNOWN'}</small><small>{p.source} · {p.path_hash.slice(0,12)} · {p.codes.join(', ')}</small></div>):<p>UNKNOWN · 未找到範圍內可讀的 interpreter。</p>}</div>
      {snapshot.venv_metadata&&<div aria-label="Python venv metadata hint"><h3>venv 設定提示</h3><p>宣告版本 {snapshot.venv_metadata.declared_version??'UNKNOWN'} · include-system-site-packages: {String(snapshot.venv_metadata.include_system_packages??'UNKNOWN')}</p><p>此宣告不等於 interpreter 實際版本；不跟隨 home / executable / command。</p><ul>{snapshot.venv_metadata.codes.map(c=><li key={c}>{c}</li>)}</ul></div>}
      <ul>{snapshot.codes.map(c=><li key={c}>{c}</li>)}</ul><label>搜尋套件 <input aria-label="Python package search" value={query} onChange={e=>{setQuery(e.target.value);setPage(0);}}/></label>
      <div className="table-wrap"><table aria-label="Python installed distribution metadata"><thead><tr><th>套件</th><th>Metadata 版本</th><th>依賴關係</th><th>觀測來源</th><th>INSTALLER metadata</th></tr></thead><tbody>{visible.map(p=><tr key={p.id} data-python-package-id={p.id}><td>{p.display_name}<small>{p.name} · slot: {p.slot_hash.slice(0,12)}</small></td><td>{p.version??'UNKNOWN'}</td><td>{p.direct_status}</td><td>{p.source}</td><td>{p.installer}<small>{p.codes.join(', ')}</small></td></tr>)}</tbody></table></div>
      <p>顯示 {page*200+visible.length} / {shown.length} · 每頁 200 個</p><button disabled={page===0} onClick={()=>setPage(page-1)}>Previous Python page</button><button disabled={(page+1)*200>=shown.length} onClick={()=>setPage(page+1)}>Next Python page</button>
    </>}
  </section>;
}

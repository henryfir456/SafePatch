import { useEffect, useState } from 'react';
import { api, Asset, Scan } from './api';
const active=(s:Scan|null)=>s?.status==='RUNNING';
export function App(){
  const [scan,setScan]=useState<Scan|null>(null),[assets,setAssets]=useState<Asset[]>([]),[error,setError]=useState(''),[busy,setBusy]=useState(false),[loading,setLoading]=useState(true),[query,setQuery]=useState(''),[source,setSource]=useState('all');
  useEffect(()=>{let alive=true;api.latest().then(async s=>{const a=s?await api.assets(s.id):[];if(alive){setAssets(a);setScan(s);}}).catch(()=>{if(alive)setError('BACKEND_UNAVAILABLE：請從 SafePatch Windows 桌面程式啟動。');}).finally(()=>{if(alive)setLoading(false);});return()=>{alive=false;};},[]);
  useEffect(()=>{if(!active(scan))return;let alive=true;let pending=false;const id=scan!.id;
    const timer=setInterval(async()=>{if(pending)return;pending=true;try{const s=await api.status(id);if(s.status!=='RUNNING'){const a=await api.assets(id);if(alive){setAssets(a);setScan(s);}}else if(alive)setScan(s);}catch{if(alive)setError('STATUS_UNAVAILABLE：無法取得掃描狀態。');}finally{pending=false;}},500);
    return()=>{alive=false;clearInterval(timer);};},[scan?.id,scan?.status]);
  async function start(){setBusy(true);setError('');try{const s=await api.start();setScan(s);setAssets([]);}catch{setError('SCAN_START_FAILED：請確認沒有其他掃描正在執行，且本地資料庫可寫入。');}finally{setBusy(false);}}
  async function cancel(){if(!scan)return;try{await api.cancel(scan.id);}catch{setError('CANCEL_FAILED：取消要求未送達。');}}
  const shown=assets.filter(a=>(a.display_name+' '+(a.publisher??'')+' '+(a.normalized_id??'')).toLowerCase().includes(query.toLowerCase())&&(source==='all'||a.provenance.some(p=>p.scanner===source)));
  return <main><header><div><span className="eyebrow">LOCAL DEVICE INVENTORY</span><h1>SafePatch</h1><p>Windows 軟體盤點 · 資料保留於本機</p></div><div className="actions"><button disabled={loading||busy||active(scan)} onClick={start}>{busy?'啟動中…':'開始唯讀掃描'}</button>{active(scan)&&<button className="secondary" onClick={cancel}>取消掃描</button>}</div></header>
    <aside className="notice"><strong>UNKNOWN ≠ 安全</strong> 僅盤點軟體，尚未執行漏洞比對。不會更新軟體、安裝依賴或要求提高權限。WinGet 可能讀取官方來源中繼資料；不傳送本機清單。</aside>
    {error&&<p role="alert" className="error">{error}</p>}
    <section className="summary" aria-label="掃描摘要"><div><span>盤點項目</span><strong>{assets.length}</strong></div><div><span>掃描狀態</span><strong>{scan?.status??'尚未掃描'}</strong></div><div><span>漏洞適用性</span><strong>UNKNOWN</strong></div></section>
    {scan&&<section className="progress" aria-live="polite"><div>{scan.phase} · {scan.progress}%</div><progress max={100} value={scan.progress}/><small>開始 {scan.started_at}{scan.finished_at&&` · 結束 ${scan.finished_at}`}</small>{scan.error_code&&<p role="alert" className="error">{scan.error_code}</p>}
      <ul>{scan.coverage.map(c=><li key={c.scanner}><b>{c.scanner}</b>：{c.status} · {c.count} 筆{c.codes.length>0&&` · ${c.codes.join(', ')}`}</li>)}</ul>
    </section>}
    <section className="inventory"><div className="toolbar"><h2>實際軟體清單</h2><input aria-label="搜尋軟體" placeholder="搜尋名稱、Publisher 或 ID" value={query} onChange={e=>setQuery(e.target.value)}/><select aria-label="掃描來源" value={source} onChange={e=>setSource(e.target.value)}><option value="all">所有來源</option><option value="registry">Registry</option><option value="winget">WinGet</option><option value="vscode">VS Code</option><option value="vscode-extension">VS Code Extensions</option></select></div>
    <p className="subtle">Registry 與 WinGet 的名稱／版本相同仍可能屬於不同安裝。候選重複項保留，來源關聯不代表由 WinGet 安裝。</p>
    {!assets.length?<p className="empty">{active(scan)?'正在讀取真實 Windows 資料…':'尚無盤點結果。請開始掃描；部分失敗會保留可用結果。'}</p>:<div className="table-wrap"><table><thead><tr><th>產品 / ID</th><th>版本</th><th>Publisher</th><th>觀測來源</th><th>身份 / 重複識別</th></tr></thead><tbody>{shown.map(a=><tr key={a.id}><td><b>{a.display_name}</b>{a.normalized_id&&<small>{a.normalized_id}</small>}</td><td>{a.installed_version??'UNKNOWN'}</td><td>{a.publisher??'UNKNOWN'}</td><td>{a.provenance.map((p,i)=><small key={i}>{p.scanner} · {p.scope} · {p.view}{p.profile_ref&&` / profile: ${p.profile_ref}`}{p.catalog_source&&` · catalog: ${p.catalog_source}`}</small>)}<small>安裝來源：{a.installation_source}</small></td><td><span className="badge">{a.identity_confidence}</span>{a.duplicate_group&&<small>候選重複 · {a.duplicate_group.slice(0,10)}</small>}</td></tr>)}</tbody></table></div>}
    <p className="subtle">顯示 {shown.length} / {assets.length} · 不會探測被盤點程式的可執行檔版本</p></section></main>;
}

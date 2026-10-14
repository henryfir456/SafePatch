import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
export interface BuildMetadata {version:string;build_sha:string;dirty:boolean;channel:string;release_url:string}
export function BuildInfo({location}:{location:'header'|'footer'}){
  const [info,setInfo]=useState<BuildMetadata|null>(null);
  useEffect(()=>{let alive=true;invoke<BuildMetadata>('build_info').then(value=>{if(alive)setInfo(value);}).catch(()=>{});return()=>{alive=false;};},[]);
  const version=info?`v${info.version}`:'版本資訊未取得';
  if(location==='header')return <small data-testid="app-version">{version}</small>;
  return <footer><p>版本：{version}　Build：{info?.build_sha.slice(0,8)??'未取得'}{info?.dirty?'（開發工作樹）':''}</p><details><summary>關於 SafePatch</summary><p>完整版本：{version}<br/>Build SHA：{info?.build_sha??'未取得'}<br/>發布通道：{info?.channel??'未取得'}</p>{info&&<a href={info.release_url} target="_blank" rel="noreferrer">GitHub Release（目標版本可能尚未發布）</a>}</details></footer>;
}

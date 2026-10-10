import { invoke } from '@tauri-apps/api/core';
export interface Provenance { scanner:string; source_id:string; scope:string; view:string; catalog_source:string|null; profile_ref?:string|null }
export interface RuntimeSupport {status:string;end_date:string|null;source_url:string;checked_at:string|null;evaluated_at:string}
export interface Asset { id:string; display_name:string; publisher:string|null; installed_version:string|null; normalized_id:string|null; install_path_hash:string|null; installation_source:string; identity_confidence:string; duplicate_group:string|null; provenance:Provenance[]; observed_at:string;support?:RuntimeSupport|null }
export interface Coverage { scanner:string; status:string; count:number; codes:string[] }
export interface Scan { id:string; status:string; phase:string; progress:number; started_at:string; finished_at:string|null; coverage:Coverage[]; error_code:string|null }
export const api = {
  start:()=>invoke<Scan>('start_scan',{scope:'windows'}),
  cancel:(scanId:string)=>invoke<void>('cancel_scan',{scanId}),
  status:(scanId:string)=>invoke<Scan>('scan_status',{scanId}),
  latest:()=>invoke<Scan|null>('latest_scan'),
  assets:(scanId:string)=>invoke<Asset[]>('list_assets',{scanId})
};

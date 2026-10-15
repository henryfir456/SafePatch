import {invoke} from '@tauri-apps/api/core';
import type {Scan,UpdateEvidence} from './api';
export type Applicability='MATCHED'|'POTENTIAL'|'NOT_AFFECTED'|'UNKNOWN';
export type Priority='CRITICAL'|'HIGH'|'MEDIUM'|'INFORMATIONAL'|'UNDETERMINED';
export interface SourceCheck {source:string;status:string;checked_at:string;reason:string|null;cache:string}
export interface Finding {id:string;cves:string[];source:string;source_url:string;queried_at:string;applicability:Applicability;priority:Priority;confidence:string;evidence:string[];affected_range:string;fixed_versions:string[];cvss:string|null;severity:string|null;references:string[];kev_status:string;kev:null|{date_added:string;due_date:string;action:string;source_url:string;checked_at:string}}
export interface SecurityItem {id:string;kind:string;display_name:string;installed_version:string|null;update:UpdateEvidence;applicability:Applicability;priority:Priority;reasons:string[];identity_reasons?:string[];update_reasons?:string[];checks:SourceCheck[];findings:Finding[]}
export interface SecurityReport {scan_id:string;windows_scan_id:string|null;npm_scan_id:string|null;checked_at:string;status:string;items:SecurityItem[]}
export const securityApi={latest:()=>invoke<Scan|null>('latest_security_scan'),report:()=>invoke<SecurityReport|null>('last_security_report'),start:(windowsScanId:string|null,npmScanId:string|null)=>invoke<Scan>('start_security_scan',{windowsScanId,npmScanId,externalQueries:true})};
export const reasons:Record<string,string>={PRODUCT_UNRESOLVED:'無法識別可信產品',VERSION_UNRESOLVED:'版本資訊不足或不可比較',SOURCE_UNSUPPORTED:'資料來源未支援',QUERY_FAILED:'查詢失敗（查看資料源錯誤）',VERSION_PINNED:'版本被釘選',COVERAGE_PARTIAL:'掃描覆蓋不足'};
export const order:Record<Priority,number>={CRITICAL:0,HIGH:1,MEDIUM:2,INFORMATIONAL:3,UNDETERMINED:4};
export const sorted=(items:SecurityItem[])=>[...items].sort((a,b)=>order[a.priority]-order[b.priority]||a.display_name.localeCompare(b.display_name)||a.id.localeCompare(b.id));
export const cveCount=(item:SecurityItem)=>new Set(item.findings.flatMap(f=>f.cves)).size;
export function updateReasons(a:{update?:UpdateEvidence}){const r=a.update?.reason??'UPDATE_NOT_CHECKED';if(r.includes('PIN'))return ['VERSION_PINNED'];if(r.includes('TIMEOUT')||r.includes('FAILED')||r.includes('ERROR'))return ['QUERY_FAILED'];if(r.includes('VERSION'))return ['VERSION_UNRESOLVED'];if(r.includes('PRODUCT')||r.includes('MATCH'))return ['PRODUCT_UNRESOLVED'];return ['SOURCE_UNSUPPORTED'];}
export const uniqueCount=(items:{id:string}[])=>new Set(items.map(i=>i.id)).size;

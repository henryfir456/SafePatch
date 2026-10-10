import { invoke } from '@tauri-apps/api/core';
import type { Scan } from './api';
export interface DependencyPackage {id:string;name:string;version:string|null;direct_status:'DIRECT'|'TRANSITIVE'|'UNKNOWN';source:string;slot_hash:string}
export interface DependencyEdge {from_id:string;to_id:string;kinds:string[]}
export interface DependencyGraph {environment_id:string;root_path_hash:string;lockfile_hash:string;lockfile_kind:string;lockfile_version:number;observed_at:string;packages:DependencyPackage[];edges:DependencyEdge[];codes:string[]}
export const npmApi={
  start:(projectRoot:string)=>invoke<Scan>('start_npm_scan',{projectRoot}),
  latest:()=>invoke<Scan|null>('latest_npm_scan'),
  graph:(scanId:string)=>invoke<DependencyGraph|null>('npm_graph',{scanId})
};

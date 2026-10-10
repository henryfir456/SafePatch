import {invoke} from '@tauri-apps/api/core';
import type {Scan} from './api';
export interface PythonPackage {id:string;name:string;display_name:string;version:string|null;direct_status:'UNKNOWN';source:'DIST_INFO_METADATA';installer:string;slot_hash:string;codes:string[]}
export interface PythonInterpreter {id:string;path_hash:string;version:string|null;file_version:string|null;product_name:string|null;publisher:string|null;source:string;codes:string[]}
export interface PythonSnapshot {environment_id:string;root_path_hash:string;observed_at:string;environment_kind:string;venv_metadata:{declared_version:string|null;include_system_packages:boolean|null;codes:string[]}|null;interpreters:PythonInterpreter[];packages:PythonPackage[];codes:string[]}
export const pythonApi={start:(environmentRoot:string)=>invoke<Scan>('start_python_scan',{environmentRoot}),latest:()=>invoke<Scan|null>('latest_python_scan'),snapshot:(scanId:string)=>invoke<PythonSnapshot|null>('python_snapshot',{scanId})};

// Verify the exact stdin/stdout transport used by the Rust adapter, on Windows.
import { spawn } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import assert from 'node:assert/strict';
if(process.platform!=='win32')throw new Error('Windows required');
const script=['winget_updates','node_inventory','vscode_inventory','windows_inventory'].map(name=>readFileSync(new URL(`../src-tauri/adapters/${name}.ps1`,import.meta.url),'utf8').replaceAll('\r\n','\n')).join('\n')+'\nexit\n';
const child=spawn(join(process.env.SystemRoot,'System32','WindowsPowerShell','v1.0','powershell.exe'),['-NoLogo','-NoProfile','-NonInteractive','-Command','-'],{windowsHide:true,env:{...process.env,SAFEPATCH_NODE_SCHEDULE_FILE:fileURLToPath(new URL('../src-tauri/adapters/node_schedule.json',import.meta.url))},stdio:['pipe','pipe','pipe']});
const chunks=[];let length=0;let errorBytes=0;
child.stdout.on('data',chunk=>{length+=chunk.length;if(length>8*1024*1024){child.kill();throw new Error('Adapter output limit');}chunks.push(chunk);});
child.stderr.on('data',chunk=>{errorBytes+=chunk.length;});
const exit=new Promise((resolve,reject)=>{child.on('error',reject);child.on('close',code=>code===0?resolve():reject(new Error('Adapter failed')));});
child.stdin.end(script);
await exit;
let output;try{output=JSON.parse(Buffer.concat(chunks).toString('utf8').replace(/^\uFEFF/,''));}catch{throw new Error('Exact adapter transport JSON parse failed; private output withheld');}
assert.ok(output.records.length>0);assert.equal(output.coverage.length>=8,true);
console.log(JSON.stringify({verification:'REAL_WINDOWS_ADAPTER_PIPE_ONLY',record_count:output.records.length,profile_associated_count:output.records.filter(r=>r.scanner==='vscode-extension'&&r.profile_ref!=='UNKNOWN').length,profile_unknown_count:output.records.filter(r=>r.scanner==='vscode-extension'&&r.profile_ref==='UNKNOWN').length,coverage:output.coverage,stderr_bytes:errorBytes,output_bytes:length},null,2));

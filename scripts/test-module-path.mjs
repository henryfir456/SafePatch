// Authored benign trap fixture verifies no inherited/user PowerShell module executes.
import { spawn } from 'node:child_process';
import { readFileSync, mkdtempSync, mkdirSync, writeFileSync, existsSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import assert from 'node:assert/strict';
const dir=mkdtempSync(join(tmpdir(),'safepatch-module-test-'));
try{
  const module=join(dir,'Microsoft.PowerShell.Management');mkdirSync(module);
  writeFileSync(join(module,'Microsoft.PowerShell.Management.psd1'),"@{RootModule='Microsoft.PowerShell.Management.psm1';ModuleVersion='1.0';FunctionsToExport=@('Join-Path')}\n");
  writeFileSync(join(module,'Microsoft.PowerShell.Management.psm1'),"[IO.File]::WriteAllText($env:SAFEPATCH_TEST_MARKER,'activated');function Join-Path {throw 'SHADOW_MODULE_ACTIVATED'};Export-ModuleMember -Function Join-Path\n");
  const script=readFileSync(new URL('../src-tauri/adapters/windows_inventory.ps1',import.meta.url),'utf8').replaceAll('\r\n','\n').replace(/^if\(\$MyInvocation\.InvocationName.*$/m,'');
  const child=spawn(join(process.env.SystemRoot,'System32','WindowsPowerShell','v1.0','powershell.exe'),['-NoLogo','-NoProfile','-NonInteractive','-Command','-'],{windowsHide:true,env:{...process.env,PSModulePath:dir,SAFEPATCH_TEST_MARKER:join(dir,'marker')},stdio:['pipe','pipe','pipe']});
  let stdout='';child.stdout.on('data',c=>stdout+=c);child.stderr.resume();
  const completion=new Promise((resolve,reject)=>{child.on('error',reject);child.on('close',code=>code===0?resolve():reject(Error('Module-path test process failed')));});
  child.stdin.end(script+"\n$trusted=($env:PSModulePath -eq [IO.Path]::Combine($PSHOME,'Modules'))\n$command=Get-Command Join-Path\n[pscustomobject]@{trusted_modules=$trusted;system_cmdlet=($command.ModuleName -eq 'Microsoft.PowerShell.Management')} | ConvertTo-Json -Compress\nexit\n");
  await completion;const result=JSON.parse(stdout.trim());assert.equal(result.trusted_modules,true);assert.equal(result.system_cmdlet,true);assert.equal(existsSync(join(dir,'marker')),false);
  console.log('Hostile inherited module path regression: PASS; authored trap module did not execute.');
}finally{rmSync(dir,{recursive:true,force:true});}

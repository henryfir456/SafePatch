# CI-only owned child with fewer rights than the runner. Never elevates a token.
param([Parameter(Mandatory=$true)][string]$ScriptPath,[Parameter(Mandatory=$true)][string]$StagePath)
$ErrorActionPreference='Stop'
if($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted'){throw 'PREVIEW_CI_ONLY'}
Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Diagnostics;
using System.Runtime.InteropServices;
using System.Text;
public static class PreviewRestrictedChild {
 [StructLayout(LayoutKind.Sequential)] struct SIDATTR {public IntPtr Sid;public uint Attributes;}
 [StructLayout(LayoutKind.Sequential,CharSet=CharSet.Unicode)] struct STARTUP {public int cb;public string reserved,desktop,title;public uint x,y,xsize,ysize,xchars,ychars,fill,flags;public ushort show,reserved2;public IntPtr reservedBytes,input,output,error;}
 [StructLayout(LayoutKind.Sequential)] struct PROCESSINFO {public IntPtr process,thread;public uint pid,tid;}
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool OpenProcessToken(IntPtr process,uint access,out IntPtr token);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool CreateRestrictedToken(IntPtr source,uint flags,uint count,ref SIDATTR sid,uint deleteCount,IntPtr deletes,uint restrictCount,IntPtr restricts,out IntPtr result);
 [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool ConvertStringSidToSid(string value,out IntPtr sid);
 [DllImport("advapi32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern bool CreateProcessAsUser(IntPtr token,string application,StringBuilder command,IntPtr processSecurity,IntPtr threadSecurity,bool inherit,uint flags,IntPtr environment,string directory,ref STARTUP startup,out PROCESSINFO info);
 [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr value);
 [DllImport("kernel32.dll")] static extern IntPtr LocalFree(IntPtr value);
 static void Check(bool ok){if(!ok)throw new Win32Exception(Marshal.GetLastWin32Error());}
 static string Quote(string value){
  var b=new StringBuilder("\"");int slash=0;
  foreach(char c in value){if(c=='\\'){slash++;continue;}if(c=='"'){b.Append('\\',slash*2+1);b.Append(c);}else{b.Append('\\',slash);b.Append(c);}slash=0;}
  b.Append('\\',slash*2);return b.Append('"').ToString();
 }
 public static int Run(string application,string script,string stage){
  IntPtr original=IntPtr.Zero,restricted=IntPtr.Zero,sid=IntPtr.Zero;PROCESSINFO info=new PROCESSINFO();
  try{
   Check(OpenProcessToken(Process.GetCurrentProcess().Handle,0x000b,out original));
   Check(ConvertStringSidToSid("S-1-5-32-544",out sid));var admin=new SIDATTR{Sid=sid};
   // LUA_TOKEN + DISABLE_MAX_PRIVILEGE; no SANDBOX_INERT/AppLocker bypass.
   Check(CreateRestrictedToken(original,5,1,ref admin,0,IntPtr.Zero,0,IntPtr.Zero,out restricted));
   var startup=new STARTUP{cb=Marshal.SizeOf<STARTUP>()};
   var command=new StringBuilder(Quote(application)+" -NoProfile -NonInteractive -File "+Quote(script)+" -StagePath "+Quote(stage));
   Check(CreateProcessAsUser(restricted,application,command,IntPtr.Zero,IntPtr.Zero,false,0x08000000,IntPtr.Zero,Environment.CurrentDirectory,ref startup,out info));
   using(var child=Process.GetProcessById((int)info.pid)){
    if(!child.WaitForExit(420000)){child.Kill(true);child.WaitForExit();throw new InvalidOperationException("PREVIEW_OWN_CHILD_TIMEOUT");}
    return child.ExitCode;
   }
  }finally{if(info.thread!=IntPtr.Zero)CloseHandle(info.thread);if(info.process!=IntPtr.Zero)CloseHandle(info.process);if(restricted!=IntPtr.Zero)CloseHandle(restricted);if(original!=IntPtr.Zero)CloseHandle(original);if(sid!=IntPtr.Zero)LocalFree(sid);}
 }
}
'@
$report=Join-Path $StagePath 'ci-lifecycle.json'
try {
 $shell=Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
 $exit=[PreviewRestrictedChild]::Run($shell,(Resolve-Path $ScriptPath).Path,(Resolve-Path $StagePath).Path)
 if($exit -ne 0){throw 'PREVIEW_RESTRICTED_CHILD_FAILED'}
 if(-not(Test-Path -LiteralPath $report)){throw 'PREVIEW_RESTRICTED_REPORT_MISSING'}
} catch {
 # A missing execution gate must remain visible, never promoted to PASS.
 if(-not(Test-Path -LiteralPath $report)){
  @{status='NOT_VERIFIED';host='GitHub Windows Server';code='RESTRICTED_CHILD_UNAVAILABLE';user_windows_10_11='NOT_VERIFIED'}|ConvertTo-Json|Set-Content -LiteralPath $report -Encoding utf8
 }
 Write-Output 'Preview restricted-token lifecycle: NOT VERIFIED (see aggregate report).'
}

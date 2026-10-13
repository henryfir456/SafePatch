# Bundled typed WinGet helper

Build with `scripts/build-winget-helper.ps1` before Windows Cargo commands. Uses pinned official NuGet SDK / projection and locked package hashes. Both the self-contained executable and paired Microsoft.Management.Deployment.winmd must be embedded; unpackaged COM marshaling fails without the metadata beside the process.

Only mode `query` is supported. The helper reads the installed/official composite catalog and returns bounded JSON. It never invokes install, upgrade, download, source mutation or agreement acceptance. Runtime failure text stays a fixed code. Names/versions are data, with control-bearing names rejected in favor of package ID. The app's owned Windows Job and 45-second child bound contain cancellation/timeouts. Fresh runtime extraction directories and cleared startup/profiler hooks avoid inherited .NET hook execution.

Microsoft.WindowsPackageManager.ComInterop and Microsoft.Windows.CsWinRT are Microsoft MIT-licensed dependencies: https://github.com/microsoft/winget-cli / https://github.com/microsoft/CsWinRT . .NET runtime license: https://github.com/dotnet/runtime/blob/main/LICENSE.TXT . No runtime dependency installation is performed by SafePatch.

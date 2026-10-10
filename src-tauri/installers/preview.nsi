; SafePatch's minimal current-user Preview installer (#22).
; Only our executable, uninstaller, HKCU registration and own shortcut are owned.
Unicode true
ManifestDPIAware true
RequestExecutionLevel user
SetCompressor /SOLID lzma
!include MUI2.nsh
Name "{{product_name}}"
OutFile "{{out_file}}"
InstallDir "$LOCALAPPDATA\Programs\SafePatchPreview-EXE"
InstallDirRegKey HKCU "Software\{{bundle_id}}\PreviewEXE" "InstallDir"
VIProductVersion "{{version_with_build}}"
VIAddVersionKey "ProductName" "{{product_name}}"
VIAddVersionKey "FileVersion" "{{version}}"
VIAddVersionKey "ProductVersion" "{{version}}"
VIAddVersionKey "FileDescription" "Read-only inventory Preview installer"
!define MUI_ABORTWARNING
!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"
Section "Install"
  SetShellVarContext current
  SetRegView 64
  SetOutPath "$INSTDIR"
  File /oname=safepatch.exe "{{main_binary_path}}"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  CreateDirectory "$SMPROGRAMS\SafePatch Preview EXE"
  CreateShortcut "$SMPROGRAMS\SafePatch Preview EXE\SafePatch Preview.lnk" "$INSTDIR\safepatch.exe"
  WriteRegStr HKCU "Software\{{bundle_id}}\PreviewEXE" "InstallDir" "$INSTDIR"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}.PreviewEXE" "DisplayName" "{{product_name}} (EXE)"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}.PreviewEXE" "DisplayVersion" "{{version}}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}.PreviewEXE" "Publisher" "{{manufacturer}}"
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}.PreviewEXE" "UninstallString" '$\"$INSTDIR\uninstall.exe$\"'
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}.PreviewEXE" "NoModify" 1
  WriteRegDWORD HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}.PreviewEXE" "NoRepair" 1
SectionEnd
Section "Uninstall"
  SetShellVarContext current
  SetRegView 64
  Delete "$INSTDIR\safepatch.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  Delete "$SMPROGRAMS\SafePatch Preview EXE\SafePatch Preview.lnk"
  RMDir "$SMPROGRAMS\SafePatch Preview EXE"
  DeleteRegKey HKCU "Software\{{bundle_id}}\PreviewEXE"
  DeleteRegKey HKCU "Software\Microsoft\Windows\CurrentVersion\Uninstall\{{bundle_id}}.PreviewEXE"
  ; Application local-data/history is deliberately not touched.
SectionEnd

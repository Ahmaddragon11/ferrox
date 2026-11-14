!define PRODUCT_NAME "Ferrox Browser"
!define PRODUCT_VERSION "@VERSION@"
!define PRODUCT_PUBLISHER "Ferrox Team"
!define PRODUCT_URL "https://github.com/Ahmaddragon11/ferrox"
!define INSTALLER_NAME "@INSTALLER_NAME@"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\Ferrox"

!define MUI_ICON "@ICON_PATH@"
!define MUI_UNICON "@ICON_PATH@"
!define MUI_WELCOMEPAGE_TITLE "Install Ferrox Browser"
!define MUI_WELCOMEPAGE_TEXT "A fast, private browser built with Rust.$\r$\n$\r$\nThe installer will create shortcuts in the Start Menu and on the desktop. Microsoft Edge WebView2 Runtime will be installed if it is missing."
!define MUI_FINISHPAGE_TITLE "Ferrox is ready"
!define MUI_FINISHPAGE_TEXT "Ferrox Browser was installed successfully."
!define MUI_FINISHPAGE_RUN "$INSTDIR\ferrox.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Launch Ferrox Browser"

!include "MUI2.nsh"

Unicode true
Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "${INSTALLER_NAME}"
InstallDir "$PROGRAMFILES64\Ferrox"
InstallDirRegKey HKLM "${UNINSTALL_KEY}" "InstallLocation"
RequestExecutionLevel admin
SetCompressor /SOLID lzma
ShowInstDetails show
ShowUninstDetails show

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES

!insertmacro MUI_LANGUAGE "English"

Function .onInit
  SetShellVarContext all
  SetRegView 64
FunctionEnd

Function un.onInit
  SetShellVarContext all
  SetRegView 64
FunctionEnd

Section "Ferrox Browser" SecApp
  SetShellVarContext all
  SetRegView 32
  ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" CheckPerUserRuntime
  StrCmp $0 "0.0.0.0" CheckPerUserRuntime
  Goto RuntimeReady

CheckPerUserRuntime:
  SetRegView 64
  ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" InstallRuntime
  StrCmp $0 "0.0.0.0" InstallRuntime
  Goto RuntimeReady

InstallRuntime:
  SetOutPath "$PLUGINSDIR"
  File "@WEBVIEW2_PATH@"
  ExecWait '"$PLUGINSDIR\MicrosoftEdgeWebview2Setup.exe" /silent /install' $1
  StrCmp $1 "0" VerifyRuntime
  StrCmp $1 "3010" VerifyRuntime
  MessageBox MB_ICONSTOP|MB_OK "WebView2 Runtime could not be installed (error $1). Ferrox needs this runtime to display web pages. Install it from https://developer.microsoft.com/microsoft-edge/webview2/ and run this installer again."
  Abort

VerifyRuntime:
  SetRegView 32
  ReadRegStr $0 HKLM "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" VerifyPerUserRuntime
  StrCmp $0 "0.0.0.0" VerifyPerUserRuntime
  Goto RuntimeReady

VerifyPerUserRuntime:
  SetRegView 64
  ReadRegStr $0 HKCU "SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}" "pv"
  StrCmp $0 "" RuntimeMissing
  StrCmp $0 "0.0.0.0" RuntimeMissing
  Goto RuntimeReady

RuntimeMissing:
  MessageBox MB_ICONSTOP|MB_OK "WebView2 Runtime setup completed but the runtime was not detected. Restart Windows and run this installer again."
  Abort

RuntimeReady:
  SetRegView 64
  SetOutPath "$INSTDIR"
  File "@EXE_PATH@"
  File "@ICON_PATH@"
  WriteUninstaller "$INSTDIR\uninstall.exe"

  CreateDirectory "$SMPROGRAMS\Ferrox Browser"
  CreateShortCut "$SMPROGRAMS\Ferrox Browser\Ferrox Browser.lnk" "$INSTDIR\ferrox.exe" "" "$INSTDIR\ferrox.ico" 0 SW_SHOWNORMAL "" "Browse privately with Ferrox" "$INSTDIR"
  CreateShortCut "$SMPROGRAMS\Ferrox Browser\Uninstall Ferrox.lnk" "$INSTDIR\uninstall.exe" "" "$INSTDIR\ferrox.ico" 0 SW_SHOWNORMAL "" "Uninstall Ferrox Browser" "$INSTDIR"
  CreateShortCut "$DESKTOP\Ferrox Browser.lnk" "$INSTDIR\ferrox.exe" "" "$INSTDIR\ferrox.ico" 0 SW_SHOWNORMAL "" "Browse privately with Ferrox" "$INSTDIR"

  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayName" "${PRODUCT_NAME}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "URLInfoAbout" "${PRODUCT_URL}"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\ferrox.ico"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKLM "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegStr HKLM "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\uninstall.exe" /S'
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKLM "${UNINSTALL_KEY}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  SetShellVarContext all
  SetRegView 64
  Delete "$DESKTOP\Ferrox Browser.lnk"
  Delete "$SMPROGRAMS\Ferrox Browser\Ferrox Browser.lnk"
  Delete "$SMPROGRAMS\Ferrox Browser\Uninstall Ferrox.lnk"
  RMDir "$SMPROGRAMS\Ferrox Browser"
  Delete "$INSTDIR\ferrox.exe"
  Delete "$INSTDIR\ferrox.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "${UNINSTALL_KEY}"
  SetRegView 32
  DeleteRegKey HKLM "Software\Ferrox Browser"
SectionEnd

!addplugindir "$Institute\Plugins\x64"
!include "MUI2.nsh"
!include "FileFunc.nsh"

!define PRODUCT_NAME "Ferrox Browser"
!define PRODUCT_VERSION "@VERSION@"
!define PRODUCT_PUBLISHER "Ferrox Team"
!define PRODUCT_URL "https://github.com/Ahmaddragon11/ferrox"
!define INSTALLER_NAME "@INSTALLER_NAME@"

Unicode true

RequestExecutionLevel highest

!insertmacro MUI_PAGE_WELCOME
!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH

!insertmacro MUI_LANGUAGE "English"

Section "Ferrox Browser" SecApp
  SetOutPath "$INSTDIR"

  File "@EXE_PATH@"

  CreateDirectory "$SMPROGRAMS\${PRODUCT_NAME}"
  CreateShortCut "$SMPROGRAMS\${PRODUCT_NAME}\Ferrox Browser.lnk" "$INSTDIR\ferrox.exe"
  CreateShortCut "$DESKTOP\${PRODUCT_NAME}.lnk" "$INSTDIR\ferrox.exe"

  WriteUninstaller "$INSTDIR\uninstall.exe"

  WriteRegStr HKLM "Software\${PRODUCT_NAME}" "DisplayIcon" "$INSTDIR\ferrox.exe"
  WriteRegStr HKLM "Software\${PRODUCT_NAME}" "DisplayName" "${PRODUCT_NAME} ${PRODUCT_VERSION}"
  WriteRegStr HKLM "Software\${PRODUCT_NAME}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr HKLM "Software\${PRODUCT_NAME}" "Publisher" "${PRODUCT_PUBLISHER}"
  WriteRegStr HKLM "Software\${PRODUCT_NAME}" "InstallLocation" "$INSTDIR\"
  WriteRegStr HKLM "Software\${PRODUCT_NAME}" "URLInfoAbout" "${PRODUCT_URL}"
  WriteRegDWORD HKLM "Software\${PRODUCT_NAME}" "NoModify" 1
  WriteRegDWORD HKLM "Software\${PRODUCT_NAME}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  RMDir /r "$SMPROGRAMS\${PRODUCT_NAME}"
  Delete "$DESKTOP\${PRODUCT_NAME}.lnk"
  Delete "$INSTDIR\ferrox.exe"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKLM "Software\${PRODUCT_NAME}"
SectionEnd

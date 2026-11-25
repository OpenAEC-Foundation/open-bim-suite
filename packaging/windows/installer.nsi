; Per-user installer of Open BIM Suite; no administrator rights needed.
; makensis /DNUMBER=2026.44.0 /DSOURCE=<folder with open-3d-viewer.exe and open-3d-viewer-mcp.exe> [/DSIGN=<windows-sign.ps1>] [/DPRODUCT=validator] installer.nsi
;
; With PRODUCT=validator it is the installer of the BIM Validator (the Collaborator before): the
; same program, installed in a folder of its own beside the suite, whose shortcut starts it on that
; product.
Unicode true
!include "MUI2.nsh"

; With SIGN, makensis hands the uninstaller it builds to the signing script before it goes into
; the installer, so the uninstall.exe that WriteUninstaller puts on disk is signed as well. A
; failed signature stops the build.
!ifdef SIGN
  !if ${NSIS_PACKEDVERSION} < 0x03008000
    !error "signing the uninstaller needs NSIS 3.08 or later (!uninstfinalize)"
  !endif
  !uninstfinalize 'pwsh -NoProfile -NonInteractive -File "${SIGN}" -Files "%1"' = 0
!endif

!ifdef PRODUCT
  !if "${PRODUCT}" != "validator"
    !error "PRODUCT is validator or left out"
  !endif
  !define TITLE "Open BIM Validator"
  !define FOLDER "Open BIM Validator"
  !define ARGS "--product validator"
  !define SETUP "open-bim-validator_${NUMBER}_x64-setup.exe"
!else
  !define TITLE "Open BIM Suite"
  ; the folder and the uninstall key keep the name the app had before, so a new version
  ; replaces the installed one instead of standing beside it
  !define FOLDER "Open 3D Viewer Studio"
  !define ARGS ""
  !define SETUP "open-3d-viewer_${NUMBER}_x64-setup.exe"
!endif

Name "${TITLE} ${NUMBER}"
OutFile "${SETUP}"
InstallDir "$LOCALAPPDATA\Programs\${FOLDER}"
RequestExecutionLevel user
SetCompressor /SOLID lzma
!define MUI_ICON "icon.ico"
!define MUI_UNICON "icon.ico"
!define MUI_FINISHPAGE_RUN "$INSTDIR\open-3d-viewer.exe"
!define MUI_FINISHPAGE_RUN_PARAMETERS "${ARGS}"
!define UNINST "Software\Microsoft\Windows\CurrentVersion\Uninstall\${FOLDER}"

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "Dutch"
!insertmacro MUI_LANGUAGE "English"

VIProductVersion "${NUMBER}.0"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "${TITLE}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "CompanyName" "OpenAEC Foundation"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${NUMBER}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${NUMBER}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "${TITLE} installer"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "OpenAEC Foundation"

Section "Install"
  SetOutPath "$INSTDIR"
  File "${SOURCE}\open-3d-viewer.exe"
  File "${SOURCE}\open-3d-viewer-mcp.exe"
  File "${SOURCE}\vertex.dll"
  File "icon.ico"
  WriteUninstaller "$INSTDIR\uninstall.exe"
  !ifndef PRODUCT
    Delete "$SMPROGRAMS\Open 3D Viewer Studio.lnk"
  !endif
  CreateShortcut "$SMPROGRAMS\${TITLE}.lnk" "$INSTDIR\open-3d-viewer.exe" "${ARGS}" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "${UNINST}" "DisplayName" "${TITLE}"
  WriteRegStr HKCU "${UNINST}" "DisplayVersion" "${NUMBER}"
  WriteRegStr HKCU "${UNINST}" "Publisher" "OpenAEC Foundation"
  WriteRegStr HKCU "${UNINST}" "DisplayIcon" "$INSTDIR\icon.ico"
  WriteRegStr HKCU "${UNINST}" "UninstallString" '"$INSTDIR\uninstall.exe"'
  WriteRegDWORD HKCU "${UNINST}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINST}" "NoRepair" 1
SectionEnd

Section "Uninstall"
  Delete "$SMPROGRAMS\${TITLE}.lnk"
  Delete "$INSTDIR\open-3d-viewer.exe"
  Delete "$INSTDIR\open-3d-viewer-mcp.exe"
  Delete "$INSTDIR\vertex.dll"
  Delete "$INSTDIR\icon.ico"
  Delete "$INSTDIR\uninstall.exe"
  RMDir "$INSTDIR"
  DeleteRegKey HKCU "${UNINST}"
SectionEnd

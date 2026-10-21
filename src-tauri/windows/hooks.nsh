; Installer hooks of rice (docs/BACKEND.md §7, step 7): package identity for
; notifications, starting with Windows, and bringing Windows' notification
; banners back before rice goes away. Each hook keeps $0 as it found it.

!include LogicLib.nsh

; The installer is a 32-bit program: "powershell.exe" would be the 32-bit
; PowerShell, whose Appx cmdlets do not work on 64-bit Windows.
!macro RICE_POWERSHELL ARGS
  ${If} ${FileExists} "$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe"
    nsExec::ExecToLog '"$WINDIR\Sysnative\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass ${ARGS}'
  ${Else}
    nsExec::ExecToLog '"$SYSDIR\WindowsPowerShell\v1.0\powershell.exe" -NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass ${ARGS}'
  ${EndIf}
  Pop $0
!macroend

!macro NSIS_HOOK_PREINSTALL
  Push $0
  ; Update: end the running rice in an orderly way, so its banners come back.
  ${If} ${FileExists} "$INSTDIR\rice.exe"
    nsExec::Exec '"$INSTDIR\rice.exe" --quit'
    Pop $0
  ${EndIf}
  Pop $0
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Push $0
  !insertmacro RICE_POWERSHELL '-File "$INSTDIR\identity\identity.ps1" -Install'
  ; Start with Windows, offered once: switching it off in rice survives updates.
  ReadRegDWORD $0 HKCU "Software\io.github.johnnyjack123.rice" "AutostartOffered"
  ${If} $0 != 1
    WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "rice" '"$INSTDIR\rice.exe"'
    WriteRegDWORD HKCU "Software\io.github.johnnyjack123.rice" "AutostartOffered" 1
  ${EndIf}
  Pop $0
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  Push $0
  ; Quit first (rice restores the banners itself), then restore whatever a
  ; crash left behind, then remove identity and autostart.
  nsExec::Exec '"$INSTDIR\rice.exe" --quit'
  Pop $0
  nsExec::Exec '"$INSTDIR\rice.exe" --restore-banners'
  Pop $0
  !insertmacro RICE_POWERSHELL '-File "$INSTDIR\identity\identity.ps1" -Uninstall'
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "rice"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "rice"
  DeleteRegKey HKCU "Software\io.github.johnnyjack123.rice"
  Pop $0
!macroend

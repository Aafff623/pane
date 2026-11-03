; The shortcut rename lives in the app (first launch after install), because
; the GUI installer creates the desktop link on its finish page — after any
; post-install hook would have run. This hook only removes the renamed link
; on uninstall, with the same guards the stock template uses for its own
; shortcuts: never during an update (the link must survive), and only when
; it actually targets this install (a legacy Pane install may own `Pane.lnk`).

!macro NSIS_HOOK_PREUNINSTALL
  ${If} $UpdateMode <> 1
    !insertmacro IsShortcutTarget "$DESKTOP\Pane.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$DESKTOP\Pane.lnk"
      Delete "$DESKTOP\Pane.lnk"
    ${EndIf}
    !insertmacro IsShortcutTarget "$SMPROGRAMS\Pane.lnk" "$INSTDIR\${MAINBINARYNAME}.exe"
    Pop $0
    ${If} $0 = 1
      !insertmacro UnpinShortcut "$SMPROGRAMS\Pane.lnk"
      Delete "$SMPROGRAMS\Pane.lnk"
    ${EndIf}
  ${EndIf}
!macroend

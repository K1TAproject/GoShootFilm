!macro NSIS_HOOK_POSTINSTALL
  ${If} ${FileExists} "$DESKTOP\goshootfilm.lnk"
    Rename "$DESKTOP\goshootfilm.lnk" "$DESKTOP\GoShootFilm.lnk"
  ${EndIf}
  ${If} ${FileExists} "$SMPROGRAMS\goshootfilm.lnk"
    Rename "$SMPROGRAMS\goshootfilm.lnk" "$SMPROGRAMS\GoShootFilm.lnk"
  ${EndIf}
!macroend

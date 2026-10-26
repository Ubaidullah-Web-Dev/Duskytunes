# Duskytunes × Dusky / Quickshell

Duskytunes publishes a native Linux MPRIS player named `duskytunes`. The optional
`DuskytunesBarWidget.qml` consumes that service directly through Quickshell's MPRIS module, so it
adds **no second WebKit window, polling script, or background helper**.

Controls:
- left click: play / pause
- middle click: previous
- right click: next
- wheel: volume

Copy the component into the module folder used by your Dusky/Quickshell configuration and instantiate
`DuskytunesBarWidget {}` in the bar layout. Dusky installations differ in how their shell tree is
structured, so V22 deliberately does not overwrite `~/.config/quickshell` automatically.

If your installed Quickshell version exposes `Mpris.players` as a model rather than `.values`, adapt
the `player` lookup to the local shell's existing MPRIS helper. The app-side contract is stable:
standard MPRIS identity `Duskytunes` / dbus name `duskytunes`.

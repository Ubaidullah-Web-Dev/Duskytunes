# QML Client Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A pure-QML Duskytunes client, run by Quickshell against `duskytunesd`, that reaches feature parity with the Svelte UI page by page while the Tauri app keeps shipping.

**Architecture:** `client/` is a Quickshell config (`qs -p client`, later installed as `qs -c duskytunes`). `Daemon.qml` owns one `Socket` to `duskytunesd` with request ids and a promise-style `call(method, params)`; `Playback.qml` mirrors daemon events into QML state; every page is a `ListView`/`GridView` with reused delegates; visual tokens come from Dusky's `Tokens` singleton (Follow System for free) with a local `Style.qml` for the app's own scale, radii and type. No timers at idle. Same window title (`Duskytunes`), so the existing Hyprland rule applies.

**Tech Stack:** Quickshell 0.3.1 (Qt 6.11): `Quickshell` (`ShellRoot`, `FloatingWindow`, `Singleton`, `IpcHandler`), `Quickshell.Io` (`Socket`, `SplitParser`, `Process`), `QtQuick`, `QtQuick.Layouts`, `QtQuick.Effects` (`MultiEffect` for the one artwork blur), `Dusky.Ui.Singletons` (`Tokens`, `I18n`). Reference implementations on this machine: `dusky/hub/quickshell/shell.qml` (FloatingWindow app), `dusky/hub/quickshell/Singletons/Settings.qml` (Socket + SplitParser client), `dusky/shell/quickshell/shell/services/Perf.qml` (power tiers).

**Spec:** `docs/superpowers/specs/2026-09-05-native-client-design.md` (section 5; phase 3 of section 8)

## Global Constraints

- The client is a view: it holds no playback truth; every mutation is a daemon method; every display value comes from a daemon event or snapshot. Optimistic UI only for the seek thumb and volume slider while dragged (the existing rule).
- No `Timer`/`FrameAnimation`/`NumberAnimation { loops: Infinite }` runs while nothing is playing and no pointer is down. The lyrics word-timer (67 ms) runs only while the lyrics surface is visible and playback is not paused. Verified by Task 9.
- Every list of unbounded length is a `ListView`/`GridView` with `reuseItems: true` and `cacheBuffer` <= 2 viewport heights. No `Repeater` over API results.
- Images: `Image { asynchronous: true; cache: true; sourceSize: Qt.size(px, px) }` with the URL rewritten by `Style.thumb(url, px)` (the `thumb.ts` rule: `=wN-hN` -> `=w{px}-h{px}`, `=sN` -> `=s{px}`, local paths -> `file://`).
- Design language per `docs/DESIGN.md`: hairlines from `Tokens.line*`, compact radii, `Space Grotesk` for UI, `SpaceMono Nerd Font` for metadata, `Noto Sans CJK JP` fallback; colours only from `Tokens` (Follow System) or the two local palettes `Style.light`/`Style.dark` ported from `ui/src/lib/duskytunes.css` `--ryo-*` variables.
- Every page keeps the Svelte page's information architecture and interactions (what each control does), not its DOM; visual parity is checked by side-by-side screenshots in Task 9.
- `qmllint` clean for every file (`qmllint -I /usr/lib/qt6/qml -I ~/.local/lib/qt6/qml client/**/*.qml`).
- Launch during development: `qs -p client` from the repo; the daemon must be running (`cargo run -p duskytunesd`). Never both the Tauri app and the daemon at once (shared SQLite/mpv).
- Commit subjects: `client: ...`, imperative, no trailing period, no attribution trailers.

## File Structure

```
client/
  shell.qml                 ShellRoot: main FloatingWindow ("Duskytunes"), mini FloatingWindow ("Duskytunes Mini"), IpcHandler("show")
  Daemon.qml                Singleton: Socket + SplitParser, call(), events, reconnect, hello/subscribe
  Playback.qml              Singleton: now/queue/position/duration/paused/volume/repeat/shuffle/lyrics/auth/settings from events
  Router.qml                Singleton: page stack (push/pop/replace), scroll offsets per entry
  Style.qml                 Singleton: uiScale, spacing, radii, type scale, thumb(), fmtTime(), light/dark palettes, motion durations
  App.qml                   Layout: TitleBar + Sidebar + page StackView + PlayerBar + NowPlaying overlay
  chrome/  TitleBar.qml Sidebar.qml PlayerBar.qml NowPlaying.qml QueuePanel.qml LyricsPanel.qml Toast.qml CommandPalette.qml
  pages/   HomePage.qml SearchPage.qml LibraryPage.qml PlaylistPage.qml AlbumPage.qml ArtistPage.qml RadioPage.qml SettingsPage.qml ListPage.qml
  components/ Artwork.qml Shelf.qml MediaCard.qml TrackRow.qml TrackList.qml Chip.qml Hairline.qml SectionHeading.qml IconButton.qml Slider.qml Menu.qml Skeleton.qml
  mini/    MiniPlayer.qml
  icons/   *.svg (Hugeicons free set, the ones ui/src imports; MIT)
```

---

### Task 1: Daemon client singleton and a smoke window

**Files:**
- Create: `client/shell.qml`, `client/Daemon.qml`, `client/qmldir` (`singleton Daemon 1.0 Daemon.qml`, later others)
- Test: `client/tests/tst_daemon.qml` run with `qmltestrunner` if available, else a manual `qs -p client` check against a `socat` fake daemon (both described below)

**Interfaces:**
- Produces: `Daemon.connected: bool`, `Daemon.protocol: int`, `Daemon.call(method: string, params: object): Promise` (resolves `result`, rejects `{code, message}`), `Daemon.event(name: string, data: var)` signal, `Daemon.subscribeAll()` (sends `subscribe` and re-sends it after every reconnect), `Daemon.socketPath: string` (`$XDG_RUNTIME_DIR/duskytunes/duskytunesd.sock`).

- [ ] **Step 1: Write `Daemon.qml`**

```qml
pragma Singleton
import QtQuick
import Quickshell
import Quickshell.Io

// One connection to duskytunesd. Requests carry an id and resolve a Promise; lines without
// an id are events. A dropped connection is retried every 2 s and re-subscribed, and every
// pending request is rejected so no page waits forever.
Singleton {
    id: root

    readonly property string socketPath: (Quickshell.env("XDG_RUNTIME_DIR") || "/tmp") + "/duskytunes/duskytunesd.sock"
    readonly property bool connected: sock.connected
    property int protocol: 0
    property string daemonVersion: ""

    signal event(string name, var data)

    property int nextId: 1
    property var pending: ({})

    function call(method, params) {
        return new Promise((resolve, reject) => {
            if (!sock.connected) {
                reject({ code: "disconnected", message: "duskytunesd is not connected" });
                return;
            }
            const id = root.nextId++;
            root.pending[id] = { resolve, reject };
            sock.write(JSON.stringify({ id, method, params: params === undefined ? null : params }) + "\n");
            sock.flush();
        });
    }

    function subscribeAll() {
        root.call("hello").then(h => { root.protocol = h.protocol; root.daemonVersion = h.daemon; });
        return root.call("subscribe", { events: ["*"] });
    }

    function handleLine(line) {
        let msg;
        try { msg = JSON.parse(line); } catch (e) { return; }
        if (msg.event !== undefined) {
            root.event(msg.event, msg.data);
            return;
        }
        const p = root.pending[msg.id];
        if (!p) return;
        delete root.pending[msg.id];
        if (msg.error) p.reject(msg.error); else p.resolve(msg.result);
    }

    Socket {
        id: sock
        path: root.socketPath
        parser: SplitParser { onRead: line => root.handleLine(line) }
        Component.onCompleted: connected = true
        onConnectionStateChanged: {
            if (!connected) {
                for (const id in root.pending) root.pending[id].reject({ code: "disconnected", message: "connection lost" });
                root.pending = {};
                retry.restart();
            }
        }
    }
    Timer { id: retry; interval: 2000; onTriggered: if (!sock.connected) sock.connected = true }
}
```

- [ ] **Step 2: Write `shell.qml` (smoke version)**

```qml
import QtQuick
import Quickshell
import Quickshell.Io
import Dusky.Ui.Singletons

ShellRoot {
    FloatingWindow {
        id: win
        title: "Duskytunes"
        color: Tokens.paper
        minimumSize: Qt.size(900, 620)
        Text {
            anchors.centerIn: parent
            color: Tokens.ink
            font.family: "Space Grotesk"
            text: Daemon.connected ? ("duskytunesd " + Daemon.daemonVersion + " (protocol " + Daemon.protocol + ")") : "connecting to duskytunesd…"
        }
        Connections {
            target: Daemon
            function onConnectedChanged() { if (Daemon.connected) Daemon.subscribeAll(); }
        }
    }
    IpcHandler {
        target: "window"
        function show(): void { win.visible = true; }
    }
}
```

- [ ] **Step 3: Test against the real daemon**

Run: `cargo run -p duskytunesd` in one terminal (hub `start`), then `qs -p client` in another.
Expected: a floating window titled `Duskytunes` (the Hyprland rule floats and centres it: `hyprctl clients -j | jq '.[]|select(.title=="Duskytunes")'` shows `floating: true`), showing `duskytunesd 2.4.1 (protocol 1)`; `duskytunes-cli next_track` makes the daemon emit events that `Daemon.event` receives (add a temporary `onEvent: console.log(name)` to see them in the qs log, then remove it).

- [ ] **Step 4: Commit**

```bash
git add client
git commit -m "client: daemon socket singleton and a smoke window"
```

---

### Task 2: `Playback` state, `Style` tokens, `Router`

**Files:**
- Create: `client/Playback.qml`, `client/Style.qml`, `client/Router.qml`; extend `client/qmldir`
- Test: `client/tests/tst_style.qml` (`thumb`, `fmtTime`), `client/tests/tst_playback.qml` (event application: `now-playing` resets position/duration, `queue-index` splices the current row, `position` ignored while `seekDrag` is set)

**Interfaces:**
- `Playback`: properties `now` (object or null), `queue` (`{items, currentIndex, playedFrom, shuffle, repeat, sourceName}`), `position`, `duration`, `paused`, `volume`, `stopAfterCurrent`, `rating`, `pendingVideoId`, `lastError`, `lyrics` (`{synced, lines}`), `auth` (`{signedIn, name, avatar}`), `settings` (object); methods `play(item)`, `playIndex(i)`, `togglePause()`, `next()`, `prev()`, `seek(secs)`, `setVolume(v)`, `toggleShuffle()`, `cycleRepeat()`; property `seekDrag: real` (NaN when not dragging) and `shownPosition`.
- `Style`: `uiScale` (from `Tokens.uiScaleFor(screen)`), `sp(n)` (n*4*uiScale), `radius`, `radiusCard`, `fontUi`, `fontMono`, `fontCjk`, `fs.{xs,sm,md,lg,xl,hero}`, `thumb(url, px)`, `fmtTime(secs)`, `motion.{snap:120, move:170, slow:260}` scaled by `Perf.reduceMotion` when the `shell.services` `Perf` singleton is importable, else constants.
- `Router`: `stack: list<{page, params, scrollY}>`, `push(page, params)`, `pop()`, `replace(page, params)`, `current`, `canGoBack`.

- [ ] **Step 1: Write the three singletons**

`Playback.qml` applies the events exactly as `ui/src/lib/player.svelte.ts` lines 916-1009 do (`now-playing`: track change resets `duration` from `n.duration` and `position` to 0; `queue-changed` replaces `queue`; `queue-index` keeps `items` and patches the current row; `position` sets `position` unless `seekDrag` is a number; `duration`, `playback-state`, `stop-after-current`, `volume` (ignored while the volume slider drags), `playback-error` -> `lastError` + toast, `playback-notice`/`cover-error` -> toast, `rating`, `auth-changed`, `lt-state`, `lt-notice`). On `Daemon.subscribeAll()` resolving, it loads the four snapshots into the same properties (`playback`, `queue`, `settings`, `auth`).

`Style.thumb` is a line-for-line port of `ui/src/lib/thumb.ts` with `convertFileSrc` replaced by `"file://" + url`.

- [ ] **Step 2: Tests**

`client/tests/tst_style.qml` with `QtTest`:

```qml
import QtQuick
import QtTest
import "../" as App

TestCase {
    name: "Style"
    function test_thumb_rewrites_sizes() {
        compare(App.Style.thumb("https://x/a=w120-h120-l90", 544), "https://x/a=w544-h544-l90");
        compare(App.Style.thumb("https://x/a=s200", 64), "https://x/a=s64");
        compare(App.Style.thumb("/music/cover.jpg", 64), "file:///music/cover.jpg");
        compare(App.Style.thumb("", 64), undefined);
    }
    function test_fmtTime() {
        compare(App.Style.fmtTime(0), "0:00");
        compare(App.Style.fmtTime(65.9), "1:05");
        compare(App.Style.fmtTime(3600), "60:00");
    }
}
```

Run: `qmltestrunner -input client/tests` (package `qt6-declarative` ships it as `/usr/lib/qt6/bin/qmltestrunner`; if absent, `qs -p client/tests` with a `TestCase` root is acceptable, report which).
Expected: all pass.

- [ ] **Step 3: Commit**

```bash
git add client
git commit -m "client: playback state, style tokens and router"
```

---

### Task 3: App chrome: title bar, sidebar, player bar, toasts

**Files:**
- Create: `client/App.qml`, `client/chrome/TitleBar.qml`, `client/chrome/Sidebar.qml`, `client/chrome/PlayerBar.qml`, `client/chrome/Toast.qml`, `client/components/{IconButton,Slider,Hairline,Artwork}.qml`, `client/icons/*.svg`
- Modify: `client/shell.qml` (host `App`)

**Interfaces:**
- `PlayerBar` shows artwork (48 px), title/artist, transport (shuffle, prev, play/pause, next, repeat), the wave seek (an `Shape`/`Canvas`-free implementation: a `Rectangle` track with a `Repeater`-free `ShapePath` sine, `Slider` overlay), elapsed/total, volume, like, queue/lyrics toggles, mini toggle, expand chevron. Seek uses `Playback.seekDrag`; release commits via `Playback.seek()`; a `HoverHandler`/`TapHandler` release outside the window is handled by `Slider.onPressedChanged` (Qt delivers release even outside, so the WebKit bug cannot recur).
- `Sidebar`: Home, Search, Radio, Library, Settings, with the active route from `Router.current.page`.
- `TitleBar`: app mark, back/forward (`Router`), account avatar menu (sign in/out, switch account, `sign_in` via `Daemon.call`), Listen Together / Discord / mini buttons as in `ui/src/routes/+layout.svelte`.

- [ ] **Step 1: Build the chrome** following `ui/src/lib/components/PlayerBar.svelte`, `Sidebar` markup in `+layout.svelte`, and `duskytunes.css` `.ryo-playerbar*`, `.ryo-sidebar*`, `.ryo-titlebar*` for spacing and hairlines.

- [ ] **Step 2: Verify by hand**

Run the daemon and `qs -p client`; `duskytunes-cli next_track`.
Expected: artwork, title and artist appear; play/pause toggles both ways (`playerctl -p duskytunes status` follows a click); seek drag then release outside the window commits (position jumps, then keeps ticking); volume slider drags without being yanked by `volume` events.

- [ ] **Step 3: Commit**

```bash
git add client
git commit -m "client: title bar, sidebar, player bar and toasts"
```

---

### Task 4: Home

**Files:**
- Create: `client/pages/HomePage.qml`, `client/components/{Shelf,MediaCard,TrackRow,SectionHeading,Chip,Skeleton}.qml`, `client/chrome/Shortcuts.qml`

**Interfaces:**
- `HomePage` calls `get_home` (`{selected}` mood chip) then `get_home_more` on a continuation when the last shelf comes within 400 px of the viewport bottom (`ListView.atYEnd`/`contentY` check), like `ui/src/routes/+page.svelte` `sentinel`. Shelves are one vertical `ListView` whose delegates are `Shelf` (horizontal `ListView`, `snapMode: SnapOneItem`, `reuseItems`). The `RECENT`, `FAMILIAR` (`ArtistIndex`) and `ForgottenFavourites` blocks are delegates chosen by `block.key`.

- [ ] **Step 1: Port the page** from `+page.svelte`, `Shelf.svelte`, `MediaCard.svelte`, `TrackRow.svelte`, `Shortcuts.svelte`, `ArtistIndex.svelte`, `RecentRail.svelte`, `ForgottenFavourites.svelte`. Card hover: `Tokens.tint5` wash + play button fade, `Style.motion.snap`; no transform scaling on hover (the CSS version disabled it for performance anyway).

- [ ] **Step 2: Verify**

Expected: Home renders the same shelves as the Tauri app for this account (compare screenshots at 900x620); scrolling 30 notches costs `qs` < 15 % of a core (`top -b -d 5 -n 2 -p $(pgrep -f 'qs -p client')`), the daemon ~3 %; `nvidia-smi` lists no `qs -p client` in hybrid GPU mode.

- [ ] **Step 3: Commit**

```bash
git add client
git commit -m "client: home page with shelves, shortcuts and artist index"
```

---

### Task 4b: The personal store moves to the daemon

Found in Task 4: Shortcuts, "Jump back in", the artist index, local saves, pins and the Home arrangement live in the Svelte app's browser `localStorage` (`ui/src/lib/personal.ts` `Personal`), which no native client can read. It is user state, so it belongs in the daemon.

**Files:**
- Modify: `crates/core/src/state.rs` (two methods), `crates/core/src/db.rs` if a JSON blob setting helper is missing, `src-tauri/src/commands.rs` + `src-tauri/src/lib.rs` (two new commands), `crates/duskytunesd/src/methods.rs` (two arms), `ui/src/lib/personal.ts` + `ui/src/lib/player.svelte.ts` (`savePersonal`/load go through the commands, with a one-time migration from `localStorage`), `client/lib/personal.js` (port of the pure functions), `client/Personal.qml` (singleton), `client/pages/HomePage.qml` (Shortcuts, RecentRail, ArtistIndex fed from it)
- Test: `crates/core/src/state.rs` round-trip test; `client/tests/tst_personal.qml` for `touchPick`, `noteArtist`, `arrangeSections`, `hydrate` tolerance (ported from the `personal.check.ts` cases)

**Interfaces:**
- `AppState::personal(&self) -> serde_json::Value` (the stored blob or `{}`), `AppState::set_personal(&self, blob: Value) -> Result<(), String>` (rejects non-objects and blobs over 1 MiB; stores under the setting key `personal_json`).
- Methods `get_personal` / `set_personal` with `{"personal": {...}}`; event `personal-changed` with the blob, emitted on every `set_personal` so a second client (mini, shell) follows.
- Svelte: on startup, if `get_personal` returns `{}` and `localStorage.personal` exists, `hydrate` it and `set_personal` once, then delete the local copy; `savePersonal()` becomes `set_personal` (debounced 300 ms as the current save already is).
- QML: `Personal` singleton mirrors the blob, applies `personal-changed`, and exposes the same reducers (`touchPick`, `noteArtist`, `addPick`, `removePick`, `pin`, `unpin`, `arrangeSections`) writing back through `set_personal`.

- [ ] **Step 1: Core + daemon + Tauri command** (one commit `core: keep the personal store in the daemon`).
- [ ] **Step 2: Svelte migration** (`ui: move the personal store to the backend`), verified by signing in on the Tauri app, seeing existing Shortcuts survive, and `duskytunes-cli get_personal` showing them.
- [ ] **Step 3: QML `Personal` + Home blocks** (`client: shortcuts, jump back in and artist index from the shared personal store`), verified by the same Shortcuts appearing in the QML client's Home.

---

### Task 5: Search, Library, Playlist, Album, Artist, List pages

**Files:**
- Create: `client/pages/{SearchPage,LibraryPage,PlaylistPage,AlbumPage,ArtistPage,ListPage}.qml`, `client/components/TrackList.qml`, `client/chrome/CommandPalette.qml`, `client/chrome/SearchSuggest.qml`, `client/components/Menu.qml` (context menu: play next, add to queue, add to playlist, go to album/artist, like, share)

**Interfaces:**
- `TrackList`: `ListView` of `TrackRow` with `reuseItems`, drag-reorder (`DragHandler` + `move()` -> `move_in_queue`/`playlist` reorder methods), the same keyboard shortcuts the Svelte list has.
- Each page mirrors its Svelte route: `search/+page.svelte` (songs/albums/artists/playlists tabs, `search_page`/`search_all`/`search_cards*`, incremental `_more`), `library/+page.svelte` (All/Playlists/Albums/Artists/Songs/Local/Insights tabs; `get_library*`, `get_local_library`, `listening_stats`), `playlist/[id]` (`get_playlist`/`get_playlist_more`, filter box with 80 ms debounce, edit details, cover, sort, delete), `album/[id]`, `artist/[id]`, `list/+page.svelte`.

- [ ] **Step 1: Port the pages**, one commit each (`client: search page`, `client: library page`, `client: playlist page`, `client: album and artist pages`, `client: list page and command palette`).

- [ ] **Step 2: Verify each** against the Tauri app with the same account: same rows, same counts, same actions; a 1000-track playlist scrolls without dropped frames (`qs` < 15 %).

---

### Task 6: Now Playing, Queue, Lyrics

**Files:**
- Create: `client/chrome/NowPlaying.qml`, `client/chrome/QueuePanel.qml`, `client/chrome/LyricsPanel.qml`

**Interfaces:**
- `NowPlaying`: artwork-first surface with one `MultiEffect { blurEnabled: true }` on a 64 px source for the wash (gated on `!Perf.blurDisabled`), tabs Queue/Lyrics.
- `LyricsPanel`: `get_lyrics` on track change; synced lines in a `ListView` with `positionViewAtIndex(current, ListView.Center)` animated by `Style.motion.slow`; click-to-seek; word timing driven by one `Timer { interval: 67; running: visible && !Playback.paused && lyrics.synced }`; manual scroll pauses auto-follow for 5 s (the Svelte rule).
- `QueuePanel`: `TrackList` of `Playback.queue.items` from `currentIndex`, "Up next", "Stop after current", search-in-queue, drag reorder -> `move_in_queue`, remove -> `remove_from_queue`.

- [ ] **Step 1: Port** from `NowPlaying.svelte`, `QueueList.svelte`, `LyricsView.svelte`.
- [ ] **Step 2: Verify**: lyrics timer stops when the panel closes or playback pauses (`qs` CPU returns to ~0 within 1 s); queue reorder reflected in `duskytunes-cli get_queue`.
- [ ] **Step 3: Commit** `client: now playing, queue and lyrics`.

---

### Task 7: Radio, Settings, Listen Together, mini player

**Files:**
- Create: `client/pages/RadioPage.qml`, `client/pages/SettingsPage.qml`, `client/chrome/ListenTogether.qml`, `client/mini/MiniPlayer.qml`
- Modify: `client/shell.qml` (second `FloatingWindow` titled `Duskytunes Mini`, `visible: Playback.miniOpen`)

**Interfaces:**
- Settings: every key in `duskytunes_core::state::UI_SETTINGS` (the daemon's `settings_snapshot`), written with `set_setting`; the theme mode (`system`/`light`/`dark`) switches `Style` between `Tokens` and the local palettes; low-resource and open-player-on-play behave as in `SettingsDialog.svelte`; file pickers use Dusky's `Dusky.Ui` `AppPicker`-style file dialog or `Process { command: ["zenity","--file-selection"] }` if none exists (report which), passing the chosen path to `add_local_folder`/`import_playlist_file`/`export_playlist_file` as the daemon expects.
- Mini: `MiniPlayer.svelte` port; its own window keeps position via `set_setting("mini_geometry", ...)` as today.

- [ ] **Step 1: Port**, commit `client: radio, settings, listen together and the mini player`.

---

### Task 8: Launcher, packaging, `show`

**Files:**
- Create: `packaging/linux/duskytunes-qml` (shell script: `exec qs -c duskytunes "$@"`), `packaging/linux/duskytunes-qml.desktop`
- Modify: `packaging/arch/PKGBUILD` (install `client/` to `/usr/share/duskytunes/client` and symlink `~/.config/quickshell/duskytunes` is NOT used; instead install to `/usr/share/quickshell/duskytunes`? Quickshell resolves `-c NAME` from `$XDG_CONFIG_HOME/quickshell/NAME` only, so install the client to `/usr/share/duskytunes/client` and make the launcher `exec qs -p /usr/share/duskytunes/client "$@"`), `crates/duskytunesd/src/lifecycle.rs` (`show` spawns `duskytunes-qml` when `DUSKYTUNES_CLIENT=qml`, else `duskytunes`), `docs/INSTALL-ARCH.md`
- Test: launch via the desktop entry; a second launch raises the window (the daemon's `show` event + the client's `IpcHandler`).
- Window rule (found in Task 1): Dusky's shipped rule `dusky/hyprland/modules/window_rules.lua` `float-duskytunes` matches `class = "^duskytunes$"`, the Tauri app's class; the QML client's class is `org.quickshell`, so it opens tiled. Cutover changes that rule to `match = { title = "^(Duskytunes)$" }` (the exact-title form `scripts/dusky-window-rule.sh` already installs), which covers both clients and keeps `Duskytunes Mini` independent. Until then, `hl.dsp.window.float` by hand during development.

- [ ] **Step 1: Wire it**, commit `client: launcher, desktop entry and packaging`.

---

### Task 9: Parity and performance sign-off

**Files:**
- Create: `docs/superpowers/reports/2026-09-xx-qml-client-parity.md`

- [ ] **Step 1: Side-by-side screenshots** of every page and surface in both clients at 900x620 and 1760x1000, listed in the report with the differences found and fixed.
- [ ] **Step 2: Measurements** with the baseline instruments (journal 2026-09-04): idle window open; playing on Home; scrolling Home 30 notches; lyrics open; window closed while playing; 30-minute soak. Record `qs -p client` and `duskytunesd` CPU (`top -b -d 5`), PSS (`smaps_rollup`), VRAM (`nvidia-smi`) and compositor frames with the client mapped and idle (`hyprctl` `debug:overlay` unchanged).
- [ ] **Step 3: Pass criteria** from the spec section 2: client <= 120 MB PSS and ~0 % idle; daemon <= 100 MB PSS playing (plus the two hidden WebKit helpers while media is loaded); scrolling in single digits of a core; zero compositor frames from the client at idle; no `/dev/nvidia*` handles in hybrid mode.
- [ ] **Step 4: Commit** `docs: qml client parity and performance report`, then hand over to the phase 4 (cutover) plan.

---

## Self-review

- Spec coverage (phase 3): 5.1 runtime and singletons (Tasks 1-2), 5.2 structure and rules (file structure, Global Constraints, Tasks 3-7), 5.3 mini (Task 7), the `show` path of 4.4 (Task 8), verification of section 9 (Task 9).
- Placeholders: Tasks 4-7 reference the Svelte files to port rather than restating them; each names its data methods, list types and the behaviour rules that are easy to get wrong (debounce, timers, drag). Task 8 records the Quickshell `-c` resolution constraint and the launcher decision inline.
- Type consistency: `Daemon.call` returns a Promise everywhere; `Playback.seekDrag` is NaN-or-number in Task 2 and consumed by `PlayerBar` in Task 3 and `MiniPlayer` in Task 7; `Style.thumb(url, px)` signature matches `thumb.ts`.


## Measured 2026-09-05 (after Task 8, commit ba8dd79)

Same instruments as the Tauri baseline (`top` process CPU over 7.5 s, PSS from smaps_rollup,
ydotool wheel input, grim screenshots). Window 1008x1163 logical, 1.25x scale, RTX 4060 dGPU pinned.

| State | qs (client) | duskytunesd | Tauri app (same state, 2026-09-04) |
|---|---|---|---|
| idle, window open, paused | 0.1% / 261 MB | 0.2% / 323 MB | 2.1% / 178 MB + 3 WebKit procs |
| playing, idle on a page | 0.7% / 258 MB | 2.1% / 322 MB | 2-3% + 6% WebKitWebProcess |
| Home scroll, 60 notches | 0.8% / 252 MB | 2.1% | 60% WebKitWebProcess + 16% host |
| Radio page scroll, 60 notches | 12.7% / 264 MB | 2.1% | (not measured) |
| 707-track playlist scroll, 60 notches | 20% / 330 MB | 2.3% | (not measured) |

`QSG_RENDER_TIMING=1` on the 707-track scroll: frames render in 0-2 ms (polish 0-1, render 0-1),
frame delta 8-19 ms while flicking, 300 ms at idle (the position tick). The 20% is the frame
cadence at the display's refresh during a continuous flick, not per-row work: rescrolling over
already-loaded rows costs the same, wheel at the bounds (no motion) costs 0.9%. The daemon's two
hidden WebKit helpers (cipher, PoToken) stay at 0.0% / 90 + 76 MB throughout.

Commit ba8dd79 replaced the per-icon Image+MultiEffect layer with QtQuick.Shapes geometry
(`lib/glyphs.js` baked from `client/icons`) and the three-pass artwork mask with a single
rounded-rect SDF shader layer (`shaders/rounded.frag`); the long-list figure did not move, the
list's cost is the flick itself.

Open: real launch through `/usr/bin/duskytunes-qml` after makepkg; tray Show / second launch with
`DUSKYTUNES_CLIENT=qml` needs the rebuilt daemon (Task 8 changed `tray.rs::show`).

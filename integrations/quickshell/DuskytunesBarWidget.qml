import QtQuick
import QtQuick.Layouts
import Quickshell
import Quickshell.Services.Mpris

// Optional Dusky/Quickshell widget. It speaks only standard MPRIS; Duskytunes itself starts no
// second renderer and no polling process for this integration.
Rectangle {
    id: root
    implicitHeight: 30
    implicitWidth: player ? Math.min(360, Math.max(150, content.implicitWidth + 20)) : 0
    radius: 10
    color: Qt.rgba(1, 1, 1, hovered ? 0.075 : 0.045)
    visible: player !== null
    clip: true

    property bool hovered: mouse.containsMouse
    readonly property var player: {
        const values = Mpris.players.values;
        for (let i = 0; i < values.length; i++) {
            const p = values[i];
            const ident = (p.identity || "").toLowerCase();
            if (ident === "duskytunes" || ident === "duskytunes") return p;
        }
        return null;
    }

    RowLayout {
        id: content
        anchors.fill: parent
        anchors.leftMargin: 9
        anchors.rightMargin: 9
        spacing: 7

        Text {
            text: root.player && root.player.playbackState === MprisPlaybackState.Playing ? "󰏤" : "󰐊"
            color: "#d7d0c8"
            font.pixelSize: 13
        }
        Text {
            Layout.fillWidth: true
            elide: Text.ElideRight
            text: root.player ? `${root.player.trackTitle || "Duskytunes"}${root.player.trackArtist ? " · " + root.player.trackArtist : ""}` : ""
            color: "#d7d0c8"
            font.pixelSize: 12
        }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        acceptedButtons: Qt.LeftButton | Qt.MiddleButton | Qt.RightButton
        onClicked: event => {
            if (!root.player) return;
            if (event.button === Qt.LeftButton) root.player.togglePlaying();
            else if (event.button === Qt.MiddleButton && root.player.canGoPrevious) root.player.previous();
            else if (event.button === Qt.RightButton && root.player.canGoNext) root.player.next();
        }
        onWheel: event => {
            if (!root.player || !root.player.canControl) return;
            const next = Math.max(0, Math.min(1, root.player.volume + (event.angleDelta.y > 0 ? 0.05 : -0.05)));
            root.player.volume = next;
        }
    }
}

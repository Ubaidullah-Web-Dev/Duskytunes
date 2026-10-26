import QtQuick
import QtQuick.Layouts
import Dusky.Ui.Singletons
import "../"

// A labelled pill action (Play / Shuffle / Save / Subscribe), the rounded header control the album,
// artist and playlist pages share. `primary` is the one filled call-to-action (drawn on the bone
// plate); `active` pins the accent outline of a toggled state (In library / Subscribed).
Item {
    id: root

    property string label: ""
    property string icon: ""
    property bool primary: false
    property bool active: false
    signal clicked()

    implicitHeight: Style.sp(10)
    implicitWidth: row.implicitWidth + Style.sp(10)
    opacity: enabled ? 1 : 0.5

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: root.primary ? Tokens.bone
            : ma.pressed ? Tokens.tint16
            : hover.hovered ? Tokens.tint5
            : "transparent"
        border.width: root.primary ? 0 : 1
        border.color: root.active ? Tokens.sun : Tokens.line
    }

    RowLayout {
        id: row
        anchors.centerIn: parent
        spacing: Style.sp(2)
        Icon {
            visible: root.icon !== ""
            name: root.icon
            size: Style.fs.md
            color: root.primary ? Tokens.inkOnBone : root.active ? Tokens.sun : Tokens.ink
        }
        Text {
            text: root.label
            color: root.primary ? Tokens.inkOnBone : root.active ? Tokens.sun : Tokens.ink
            font.family: Style.fontUi
            font.pixelSize: Style.fs.sm
            font.weight: Font.DemiBold
        }
    }

    HoverHandler { id: hover; enabled: root.enabled }
    MouseArea {
        id: ma
        anchors.fill: parent
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}

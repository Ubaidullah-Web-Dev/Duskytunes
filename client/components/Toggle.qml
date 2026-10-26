import QtQuick
import Dusky.Ui.Singletons
import "../"

// The on/off switch the settings surface uses in place of the Svelte `Switch`. Purely
// unidirectional like Slider: it renders `checked` (bound to the model) and only emits `toggled`;
// the host flips the daemon setting and the binding feeds the new value back. The knob/track
// animations run only on a state flip, never at idle, so an untouched switch costs no wakeups.
Item {
    id: root

    property bool checked: false
    signal toggled(bool value)

    implicitWidth: Style.sp(11)
    implicitHeight: Style.sp(6)
    opacity: enabled ? 1 : 0.5

    Rectangle {
        anchors.fill: parent
        radius: height / 2
        color: root.checked ? Tokens.sun : Tokens.tint16
        border.width: 1
        border.color: root.checked ? Tokens.sun : Tokens.line
        Behavior on color { ColorAnimation { duration: Style.motion.snap } }
    }

    Rectangle {
        id: knob
        width: Style.sp(4.5)
        height: Style.sp(4.5)
        radius: height / 2
        anchors.verticalCenter: parent.verticalCenter
        x: root.checked ? parent.width - width - Style.sp(0.75) : Style.sp(0.75)
        color: root.checked ? Tokens.inkOnBone : Tokens.ink
        Behavior on x { NumberAnimation { duration: Style.motion.snap; easing.type: Easing.OutQuad } }
    }

    MouseArea {
        anchors.fill: parent
        enabled: root.enabled
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggled(!root.checked)
    }
}

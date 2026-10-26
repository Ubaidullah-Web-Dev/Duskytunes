import QtQuick
import Dusky.Ui.Singletons

// A one-pixel structural hairline, the recurring divider of the design language. Horizontal by
// default (fill the row, one device pixel tall); a caller sets Layout.fillWidth or an explicit
// width/height. Colour follows the resolved ink through Tokens, never a hardcoded grey.
Rectangle {
    property bool soft: false
    property bool strong: false
    implicitHeight: 1
    implicitWidth: 1
    color: strong ? Tokens.lineStrong : soft ? Tokens.lineSoft : Tokens.line
}

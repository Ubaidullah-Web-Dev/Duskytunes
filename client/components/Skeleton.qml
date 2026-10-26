import QtQuick
import Dusky.Ui.Singletons
import "../"

// A neutral loading placeholder. Deliberately static — no shimmer loop — so a page waiting on the
// daemon still costs nothing at idle, per the performance budget.
Rectangle {
    property int corner: Style.radius
    radius: corner
    color: Tokens.tint5
}

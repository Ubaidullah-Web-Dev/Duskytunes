import QtQuick

// Home's asynchronous header can grow before the first scroll. Pin its origin only
// until the user takes control. Restoring contentY's old value (usually zero) when
// releasing the binding skips the entire tall header on the first wheel tick.
ListView {
    id: feed
    property bool stickTop: true

    onMovementStarted: feed.stickTop = false
    Binding {
        target: feed
        property: "contentY"
        value: feed.originY
        when: feed.stickTop
        restoreMode: Binding.RestoreNone
    }
    WheelScroller {}
}

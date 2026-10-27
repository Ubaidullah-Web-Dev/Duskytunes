import QtQuick

// The wheel-to-scroller bridge. A plain Flickable does not answer the wheel on
// this stack -- the shell adds a WheelHandler wherever it scrolls.
WheelHandler {
    id: wheel

    acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
    property real multiplier: 2.5

    readonly property var scroller: {
        var p = wheel.parent;
        while (p && p.contentHeight === undefined)
            p = p.parent;
        return p || null;
    }

    onWheel: (event) => {
        var flick = wheel.scroller;
        if (!flick || flick.contentHeight === undefined)
            return;

        var rawDy = event.pixelDelta.y !== 0 ? event.pixelDelta.y : (event.angleDelta.y / 120.0 * 75.0);
        var factor = wheel.multiplier;
        if (typeof Prefs !== "undefined" && Prefs && Prefs.scrollFactor > 0) {
            factor = Prefs.scrollFactor;
        }

        if (wheel.scrollBy(flick, rawDy * factor)) {
            event.accepted = true;
        }
    }

    function scrollBy(flick, dy) {
        if (!flick || flick.contentHeight === undefined)
            return false;
        var maxY = Math.max(0, flick.contentHeight - flick.height);
        if (maxY <= 0)
            return false;
        var next = Math.max(0, Math.min(maxY, flick.contentY - dy));
        if (next === flick.contentY)
            return false;
        flick.contentY = next;
        return true;
    }
}

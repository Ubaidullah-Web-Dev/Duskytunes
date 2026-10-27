import QtQuick
import Dusky.Ui.Singletons
import "../"

// WheelScroller: custom scroll multiplier & smooth-scrolling wrapper for Flickable, ListView and GridView.
// Multiplies mouse wheel / touchpad deltas by Prefs.scrollFactor for responsive, customizable scrolling.
// Can be declared inside a Flickable/ListView/GridView, wrap one as a parent, or bind to `target`.
Item {
    id: root

    property var target: null
    property real multiplier: (typeof Prefs !== "undefined" && Prefs && Prefs.scrollFactor > 0) ? Prefs.scrollFactor : 2.5
    property bool smooth: (typeof Tokens !== "undefined" && Tokens) ? !Tokens.reduceMotion : true

    // Locate the scroller: explicit target -> parent -> children
    readonly property var scroller: {
        if (target && target.contentHeight !== undefined)
            return target;
        var p = root.parent;
        while (p && p.contentHeight === undefined)
            p = p.parent;
        if (p && p.contentHeight !== undefined)
            return p;
        for (var i = 0; i < root.children.length; ++i) {
            if (root.children[i].contentHeight !== undefined)
                return root.children[i];
        }
        return null;
    }

    anchors.fill: (root.target && root.target.parent === root.parent) ? root.target : parent
    width: parent ? parent.width : 0
    height: parent ? parent.height : 0
    z: 10

    NumberAnimation {
        id: animY
        target: root.scroller
        property: "contentY"
        duration: 120
        easing.type: Easing.OutCubic
    }

    NumberAnimation {
        id: animX
        target: root.scroller
        property: "contentX"
        duration: 120
        easing.type: Easing.OutCubic
    }

    WheelHandler {
        id: handler
        target: null
        acceptedDevices: PointerDevice.Mouse | PointerDevice.TouchPad
        onWheel: (event) => {
            var flick = root.scroller;
            if (!flick || flick.contentHeight === undefined)
                return;

            // Un-stick any top pin (e.g. HomeFeed)
            if (flick.stickTop !== undefined && flick.stickTop)
                flick.stickTop = false;

            var factor = root.multiplier;
            var isTouchpad = (event.pixelDelta.x !== 0 || event.pixelDelta.y !== 0);

            // Compute vertical and horizontal deltas
            var rawDy = event.pixelDelta.y !== 0 ? event.pixelDelta.y : (event.angleDelta.y / 120.0 * 75.0);
            var rawDx = event.pixelDelta.x !== 0 ? event.pixelDelta.x : (event.angleDelta.x / 120.0 * 75.0);

            var minY = (flick.originY !== undefined) ? flick.originY : 0;
            var maxY = minY + Math.max(0, flick.contentHeight - flick.height);

            var minX = (flick.originX !== undefined) ? flick.originX : 0;
            var maxX = minX + Math.max(0, flick.contentWidth - flick.width);

            var canScrollY = (flick.contentHeight > flick.height);
            var canScrollX = (flick.contentWidth > flick.width);

            // Check if this is a horizontal-only scroller (e.g. shelf or chip rail)
            var isHorizontal = (flick.flickableDirection === Flickable.HorizontalFlick) || (!canScrollY && canScrollX);
            if (isHorizontal && rawDx === 0 && rawDy !== 0) {
                rawDx = rawDy;
                rawDy = 0;
            }

            // Shift key forces horizontal scroll
            if ((event.modifiers & Qt.ShiftModifier) && rawDy !== 0) {
                rawDx = rawDy;
                rawDy = 0;
            }

            var moved = false;

            // Vertical scroll
            if (rawDy !== 0 && canScrollY) {
                var dy = rawDy * factor;
                var startY = (root.smooth && !isTouchpad && animY.running) ? animY.to : flick.contentY;
                var targetY = Math.max(minY, Math.min(maxY, startY - dy));

                if (targetY !== flick.contentY) {
                    if (root.smooth && !isTouchpad) {
                        animY.stop();
                        animY.from = flick.contentY;
                        animY.to = targetY;
                        animY.restart();
                    } else {
                        animY.stop();
                        flick.contentY = targetY;
                    }
                    moved = true;
                }
            }

            // Horizontal scroll
            if (rawDx !== 0 && canScrollX) {
                var dx = rawDx * factor;
                var startX = (root.smooth && !isTouchpad && animX.running) ? animX.to : flick.contentX;
                var targetX = Math.max(minX, Math.min(maxX, startX - dx));

                if (targetX !== flick.contentX) {
                    if (root.smooth && !isTouchpad) {
                        animX.stop();
                        animX.from = flick.contentX;
                        animX.to = targetX;
                        animX.restart();
                    } else {
                        animX.stop();
                        flick.contentX = targetX;
                    }
                    moved = true;
                }
            }

            if (moved) {
                event.accepted = true;
            }
        }
    }
}

pragma ComponentBehavior: Bound
import QtQuick
import QtQuick.Shapes
import Dusky.Ui.Singletons
import "../lib/glyphs.js" as Glyphs

// One icon renderer for the whole client. The Hugeicons free-set glyphs the Svelte UI imports are
// baked into lib/glyphs.js as 24x24 stroke paths and drawn as scene-graph geometry, the way the
// Dusky shell's GlyphIcon does it. No image, no offscreen layer, no colorization pass: a tint is a
// stroke colour, and a list of a thousand rows pays one geometry node per icon. (The previous
// Image + layer.effect MultiEffect version cost two offscreen passes per track row.)
Item {
    id: root

    property string name: ""
    property color color: Tokens.ink
    property int size: 18

    width: size
    height: size

    readonly property var glyph: Glyphs.glyph(root.name)
    readonly property real u: root.size / 24

    Shape {
        anchors.fill: parent
        visible: root.glyph !== null
        preferredRendererType: Shape.CurveRenderer
        transform: Scale { xScale: root.u; yScale: root.u }
        ShapePath {
            strokeColor: root.color
            strokeWidth: root.glyph ? root.glyph.w : 1.5
            fillColor: "transparent"
            capStyle: root.glyph && root.glyph.cap === "round" ? ShapePath.RoundCap : ShapePath.FlatCap
            joinStyle: ShapePath.RoundJoin
            PathSvg { path: root.glyph ? root.glyph.d : "" }
        }
    }
}

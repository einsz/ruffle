// How lineStyle() takes thicknesses out of the usual range, measured by the
// size of a 100 px line. Checked against Flash Player 32, also by drawing
// them: the thickness is truncated to an integer, values outside the 32-bit
// range (and NaN and infinities) draw a hairline (0), and undefined no line,
// like no thickness.
_root.createTextField("tf", 1000, 0, 0, 550, 400);
_root.tf.multiline = true;
function log(s) {
    _root.tf.text += s + "\n";
    trace(s);
}
var values = [0, 0.5, 1.5, 2.5, 2.7, 40, 255, 300, -5, Infinity, -Infinity, NaN, 2147483520, 2147483904, -2147483520, 1e10, null, "40"];
for (var i = 0; i < values.length; i++) {
    var s = _root.createEmptyMovieClip("s" + i, i + 1);
    s.lineStyle(values[i], 0, 100, false, "normal", "none");
    s.moveTo(0, 1000);
    s.lineTo(100, 1000);
    log("lineStyle(" + values[i] + "): width " + s._width + ", height " + s._height);
}
// Bounds can't tell a hairline from no line, so check a hit test too.
// (Checked against Flash Player 32 on screen as well.)
var n = 0;
function probe(name, c) {
    c._x = 300;
    c._y = 300 + n * 10;
    n++;
    c.moveTo(0, 0);
    c.lineTo(200, 0);
    log(name + ": hit " + c.hitTest(400, c._y, true));
}
var c = _root.createEmptyMovieClip("p1", 100);
c.lineStyle(0, 0xFF0000, 100);
probe("lineStyle(0)", c);
c = _root.createEmptyMovieClip("p2", 101);
c.lineStyle(undefined, 0xFF0000, 100);
probe("lineStyle(undefined)", c);
c = _root.createEmptyMovieClip("p4", 103);
c.lineStyle();
probe("lineStyle()", c);
c = _root.createEmptyMovieClip("p5", 104);
c.lineStyle(0, 0xFF0000, 100);
c.lineStyle(undefined);
probe("lineStyle(0), lineStyle(undefined)", c);
c = _root.createEmptyMovieClip("p6", 105);
probe("no lineStyle", c);

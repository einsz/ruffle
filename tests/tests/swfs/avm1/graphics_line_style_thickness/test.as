// How lineStyle() takes thicknesses out of the usual range, measured by the
// size of a 100 px line. Checked against Flash Player 32, also by drawing
// them: the thickness is truncated to an integer, values outside the 32-bit
// range (and NaN and infinities) draw a hairline (0), and undefined no line.
_root.createTextField("tf", 1000, 0, 0, 550, 400);
_root.tf.multiline = true;
function log(s) {
    _root.tf.text += s + "\n";
    trace(s);
}
var values = [0, 2.7, 40, 255, 300, -5, Infinity, -Infinity, NaN, 2147483520, 2147483904, -2147483520, 1e10, null, "40"];
for (var i = 0; i < values.length; i++) {
    var s = _root.createEmptyMovieClip("s" + i, i + 1);
    s.lineStyle(values[i], 0, 100, false, "normal", "none");
    s.moveTo(0, 1000);
    s.lineTo(100, 1000);
    log("lineStyle(" + values[i] + "): width " + s._width + ", height " + s._height);
}

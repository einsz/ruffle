package {
    import flash.display.Shape;
    import flash.display.Sprite;
    import flash.text.TextField;

    // How lineStyle() takes thicknesses that aren't integers or are out of
    // the usual range, measured by the size of a 100 px line. Checked against
    // Flash Player 32, also by drawing them: the thickness is rounded to an
    // integer (halves to even), Infinity and values beyond the 64-bit range
    // draw a hairline (0), and beyond 2^31 only the low 32 bits count
    // (2^31 + 256 wraps to a negative hairline, 1e10 to a large value).
    public class Test extends Sprite {
        private var tf:TextField = new TextField();

        public function Test() {
            tf.width = 550;
            tf.height = 400;
            tf.multiline = true;
            addChild(tf);
            var values:Array = [0, 0.5, 1.5, 2.5, 3.5, 0.51, 2.49, 2.7, -0.5, -0.6,
                40, 254.5, 255, 255.5, 300, -5, Infinity, -Infinity, NaN,
                2147483520, 2147483647.5, 2147483904, -2147483904, 1e10, -1e10,
                4.6e17, 4.7e17, 1e19, 1e300];
            for each (var value:Number in values) {
                var s:Shape = new Shape();
                s.graphics.lineStyle(value, 0, 1, false, "normal", "none");
                s.graphics.moveTo(0, 0);
                s.graphics.lineTo(100, 0);
                log("lineStyle(" + value + "): width " + s.width + ", height " + s.height);
            }
        }

        private function log(s:String):void {
            trace(s);
            tf.appendText(s + "\n");
        }
    }
}

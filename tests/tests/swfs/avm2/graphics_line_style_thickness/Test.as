package {
    import flash.display.Shape;
    import flash.display.Sprite;
    import flash.text.TextField;

    // How lineStyle() takes thicknesses out of the usual range, measured by
    // the size of a 100 px line. Checked against Flash Player 32, also by
    // drawing them: Infinity and values beyond the 64-bit range draw a
    // hairline (0), and beyond 2^31 only the low 32 bits of the integer part
    // count (2^31 + 384 wraps to a negative hairline, 1e10 to a large value).
    public class Test extends Sprite {
        private var tf:TextField = new TextField();

        public function Test() {
            tf.width = 550;
            tf.height = 400;
            tf.multiline = true;
            addChild(tf);
            var values:Array = [0, 40, 255, 300, -5, Infinity, -Infinity, NaN,
                2147483520, 2147483904, 1e10, 4.6e17, 4.7e17, 1e19, 1e300];
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

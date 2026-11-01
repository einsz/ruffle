package {
    import flash.display.Loader;
    import flash.display.MovieClip;
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.geom.PerspectiveProjection;
    import flash.geom.Rectangle;

    // 3D display objects: getBounds/getRect follow the perspective projection,
    // x and y aren't rounded to twips, and a loaded movie's root has a
    // PerspectiveProjection. Flash Player's bounds are a little larger than
    // the projected corners (by up to about a pixel, apparently from how it
    // renders 3D objects), so they are compared with a tolerance.
    [SWF(width="550", height="400", frameRate="24")]
    public class Test extends MovieClip {
        private function log(s:String):void { trace(s); }
        private static function r(v:Number):String { return String(Math.round(v * 1000) / 1000); }
        private static function rect(b:Rectangle):String { return r(b.x) + "," + r(b.y) + "," + r(b.width) + "," + r(b.height); }
        private static function pp(p:PerspectiveProjection):String {
            return p ? "fov " + r(p.fieldOfView) + " center " + r(p.projectionCenter.x) + "," + r(p.projectionCenter.y) : "null";
        }

        // Compares bounds with the ones Flash Player 32 reports.
        private static function near(b:Rectangle, flash:Array):String {
            var got:Array = [b.x, b.y, b.width, b.height];
            for (var i:int = 0; i < 4; i++) {
                if (Math.abs(got[i] - flash[i]) > 2) {
                    return rect(b) + ", Flash: " + flash.join(",");
                }
            }
            return "within 2 px of Flash";
        }

        private function square():Sprite {
            var s:Sprite = new Sprite();
            s.graphics.beginFill(0x3366cc);
            s.graphics.drawRect(0, 0, 100, 100);
            s.graphics.endFill();
            return s;
        }

        public function Test() {
            if (loaderInfo.loader != null) {
                // Loaded by ourselves with loadBytes: nothing to do.
                return;
            }
            log("root pp: " + pp(transform.perspectiveProjection));
            log("sprite pp: " + pp(new Sprite().transform.perspectiveProjection));

            // [x, y, rotationY, Flash's bounds]
            var cases:Array = [
                [275, 200, 30, [275, 199.4, 96.25, 111.7]],
                [100, 50, 60, [99.7, 20.65, 25.75, 129.75]],
                [400, 250, -45, [399.6, 244.05, 48.15, 106.6]]];
            for each (var c:Array in cases) {
                var s:Sprite = square();
                s.x = c[0]; s.y = c[1];
                s.rotationY = c[2];
                addChild(s);
                log("rotationY " + c[2] + " at " + c[0] + "," + c[1] + ": bounds " + near(s.getBounds(this), c[3]) +
                    ", rect " + near(s.getRect(this), c[3]) + ", own bounds " + rect(s.getBounds(s)));
            }

            var box:Sprite = new Sprite();
            box.x = 50; box.y = 20;
            addChild(box);
            var t:Sprite = square();
            t.x = 150; t.y = 150; t.rotationX = 45;
            box.addChild(t);
            log("nested rotationX 45: bounds " + near(t.getBounds(this), [199.4, 169.3, 100.5, 67.05]) +
                ", in box " + near(t.getBounds(box), [149.4, 149.3, 100.5, 67.05]));

            var u:Sprite = square();
            u.x = 21.9; u.y = 33.37;
            u.rotationY = 10;
            log("x/y after rotationY: " + r(u.x) + "," + r(u.y));
            u.x = 12.345; u.y = 6.789;
            log("x/y set under 3D: " + r(u.x) + "," + r(u.y));
            u.z = 5.5;
            log("x/y/z after z: " + r(u.x) + "," + r(u.y) + "," + r(u.z));

            var ld:Loader = new Loader();
            ld.contentLoaderInfo.addEventListener(Event.INIT, function(e:Event):void {
                log("loaded root pp: " + pp(ld.content.transform.perspectiveProjection));
            });
            ld.loadBytes(loaderInfo.bytes);
        }
    }
}

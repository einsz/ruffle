package {
    import flash.display.Sprite;
    import flash.filters.GlowFilter;
    import flash.text.TextField;

    [SWF(width="400", height="300", backgroundColor="#FFFFFF")]
    public class Test extends Sprite {
        public function Test() {
            // 3D objects in a filtered 2D object.
            var a:Sprite = new Sprite();
            a.x = 100;
            a.y = 100;
            a.filters = [new GlowFilter(0xFF00FF, 1, 8, 8, 4)];
            addChild(a);
            var a1:Sprite = square();
            a1.rotationY = 45;
            a.addChild(a1);
            var a2:Sprite = square(0.4);
            a2.y = 120;
            a2.z = -100;
            a.addChild(a2);

            // The same, unfiltered, for comparison.
            var b:Sprite = new Sprite();
            b.x = 300;
            b.y = 100;
            addChild(b);
            var b1:Sprite = square();
            b1.rotationY = 45;
            b.addChild(b1);
            var b2:Sprite = square(0.4);
            b2.y = 120;
            b2.z = -100;
            b.addChild(b2);

            // A 3D text field with a background and a border.
            var t:TextField = new TextField();
            t.x = 160;
            t.y = 180;
            t.width = 80;
            t.height = 40;
            t.background = true;
            t.backgroundColor = 0xFFCC00;
            t.border = true;
            t.borderColor = 0x0000FF;
            t.z = -100;
            t.rotationY = -40;
            addChild(t);
        }

        private function square(scale:Number = 1):Sprite {
            var s:Sprite = new Sprite();
            var size:Number = 50 * scale;
            s.graphics.beginFill(0xFF0000);
            s.graphics.drawRect(-size, -size, size, size);
            s.graphics.beginFill(0x00C000);
            s.graphics.drawRect(0, -size, size, size);
            s.graphics.beginFill(0x0000FF);
            s.graphics.drawRect(-size, 0, size, size);
            s.graphics.beginFill(0x000000);
            s.graphics.drawRect(0, 0, size, size);
            return s;
        }
    }
}

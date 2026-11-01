package {
    import flash.display.Bitmap;
    import flash.display.BitmapData;
    import flash.display.Sprite;
    import flash.geom.Matrix3D;
    import flash.geom.PerspectiveProjection;
    import flash.geom.Point;
    import flash.geom.Vector3D;

    [SWF(width="600", height="400", backgroundColor="#FFFFFF")]
    public class Test extends Sprite {
        public function Test() {
            // Rotated about X.
            var a:Sprite = square();
            a.x = 100;
            a.y = 100;
            a.rotationX = 60;
            addChild(a);

            // Rotated about Y.
            var b:Sprite = square();
            b.x = 300;
            b.y = 100;
            b.rotationY = -45;
            addChild(b);

            // Moved away along Z.
            var c:Sprite = square();
            c.x = 500;
            c.y = 100;
            c.z = 400;
            addChild(c);

            // A 3D object in a 3D object.
            var d:Sprite = square();
            d.x = 100;
            d.y = 300;
            d.rotationY = 30;
            addChild(d);
            var d2:Sprite = square(0.5);
            d2.x = 30;
            d2.rotationX = 40;
            d.addChild(d2);

            // A 3D object in a scaled and rotated 2D object.
            var e:Sprite = new Sprite();
            e.x = 300;
            e.y = 300;
            e.scaleX = e.scaleY = 0.7;
            e.rotation = 20;
            addChild(e);
            var e2:Sprite = square();
            e2.rotationX = 50;
            e.addChild(e2);

            // A 3D object in an object with its own perspective projection.
            var f:Sprite = new Sprite();
            f.x = 500;
            f.y = 300;
            var projection:PerspectiveProjection = new PerspectiveProjection();
            projection.fieldOfView = 100;
            projection.projectionCenter = new Point(0, -50);
            f.transform.perspectiveProjection = projection;
            addChild(f);
            var f2:Sprite = square();
            f2.rotationY = 60;
            f.addChild(f2);

            // A cached 3D object.
            var g:Sprite = square(0.5);
            g.x = 200;
            g.y = 200;
            g.z = -150;
            g.rotationY = 50;
            g.cacheAsBitmap = true;
            addChild(g);

            // A bitmap with a 3D matrix.
            var data:BitmapData = new BitmapData(40, 40, false, 0x0000FF);
            data.fillRect(new flash.geom.Rectangle(0, 0, 20, 20), 0xFF8000);
            var bitmap:Bitmap = new Bitmap(data);
            var m:Matrix3D = new Matrix3D();
            m.appendTranslation(-20, -20, 0);
            m.appendRotation(50, Vector3D.Y_AXIS);
            m.appendRotation(20, Vector3D.X_AXIS);
            m.appendTranslation(400, 200, -100);
            bitmap.transform.matrix3D = m;
            addChild(bitmap);
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

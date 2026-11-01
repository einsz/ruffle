package {
    import flash.display.Sprite;
    import flash.geom.Matrix;
    import flash.geom.Matrix3D;
    import flash.geom.Vector3D;

    public class Test extends Sprite {
        public function Test() {
            var s:Sprite = new Sprite();
            s.x = 10;
            s.y = 20;

            trace("// matrix3D = rotation X 30, translation (5, 6, 7)");
            var m:Matrix3D = new Matrix3D();
            m.appendRotation(30, Vector3D.X_AXIS);
            m.appendTranslation(5, 6, 7);
            s.transform.matrix3D = m;
            props(s);

            trace("// x = 50, y = 60, z = 9");
            s.x = 50;
            s.y = 60;
            s.z = 9;
            props(s);

            trace("// scaleX = 2");
            s.scaleX = 2;
            props(s);

            trace("// rotation = 45");
            s.rotation = 45;
            props(s);

            trace("// scaleY = 3");
            s.scaleY = 3;
            props(s);

            trace("// transform.matrix = (1, 0, 0, 1, 3, 4)");
            s.transform.matrix = new Matrix(1, 0, 0, 1, 3, 4);
            props(s);

            trace("// matrix3D = null");
            s.transform.matrix3D = m;
            s.transform.matrix3D = null;
            props(s);

            trace("// rotationX = 100");
            var v:Sprite = new Sprite();
            v.rotationX = 100;
            props(v);

            trace("// rotationY = 90");
            v.rotationY = 90;
            props(v);

            trace("// rotationX = 10, rotationY = 20, rotationZ = 30, scaleX = -1, scaleZ = 2");
            v.rotationX = 10;
            v.rotationY = 20;
            v.rotationZ = 30;
            v.scaleX = -1;
            v.scaleZ = 2;
            props(v);

            trace("// matrix3D = a copy of that");
            var w:Sprite = new Sprite();
            w.transform.matrix3D = v.transform.matrix3D.clone();
            props(w);

            for each (var angle:Number in [90, -90]) {
                trace("// matrix3D = rotation X 30, then Y " + angle);
                var g:Sprite = new Sprite();
                var gm:Matrix3D = new Matrix3D();
                gm.appendRotation(30, Vector3D.X_AXIS);
                gm.appendRotation(angle, Vector3D.Y_AXIS);
                g.transform.matrix3D = gm;
                props(g);
            }

            trace("// rotationX = rotationY = rotationZ = 0, scaleX = scaleZ = 1");
            w.rotationX = 0;
            w.rotationY = 0;
            w.rotationZ = 0;
            w.scaleX = 1;
            w.scaleZ = 1;
            props(w);

            var setters:Array = ["z", "rotationX", "rotationY", "rotationZ", "scaleZ"];
            for each (var setter:String in setters) {
                trace("// skewed 2D, then " + setter + " = " + (setter == "scaleZ" ? 1 : 0));
                var k:Sprite = new Sprite();
                k.transform.matrix = new Matrix(2, 0.5, 0.3, 1, 7, 8);
                k[setter] = setter == "scaleZ" ? 1 : 0;
                props(k);
            }

            trace("// 2D rotation 30 scaleX 2, then rotationX = 0");
            k = new Sprite();
            k.rotation = 30;
            k.scaleX = 2;
            k.rotationX = 0;
            props(k);

            trace("// rotationX = 200, rotationY = -190, rotationZ = 370");
            var n:Sprite = new Sprite();
            n.rotationX = 200;
            n.rotationY = -190;
            n.rotationZ = 370;
            props(n);

            trace("// 3D, then rotation = 370");
            n = new Sprite();
            n.z = 0;
            n.rotation = 370;
            props(n);

            trace("// 3D, then rotation = -200");
            n.rotation = -200;
            props(n);

            trace("// concatenatedMatrix");
            var a:Sprite = new Sprite();
            a.x = 5;
            addChild(a);
            var b:Sprite = new Sprite();
            b.x = 6;
            b.z = 1;
            a.addChild(b);
            var c:Sprite = new Sprite();
            c.x = 7;
            b.addChild(c);
            var d:Sprite = new Sprite();
            d.y = 9;
            c.addChild(d);
            trace("3D: " + b.transform.concatenatedMatrix);
            trace("2D child of 3D: " + c.transform.concatenatedMatrix);
            trace("2D grandchild of 3D: " + d.transform.concatenatedMatrix);
            trace("child matrix3D: " + raw(c.transform.matrix3D) + ", matrix: " + c.transform.matrix);
            // PROBE_END
        }

        private function props(s:Sprite):void {
            trace("matrix: " + s.transform.matrix + ", matrix3D: " + raw(s.transform.matrix3D));
            trace("x " + r(s.x) + " y " + r(s.y) + " z " + r(s.z)
                + " scaleX " + r(s.scaleX) + " scaleY " + r(s.scaleY) + " scaleZ " + r(s.scaleZ)
                + " rotation " + r(s.rotation) + " rotationX " + r(s.rotationX)
                + " rotationY " + r(s.rotationY) + " rotationZ " + r(s.rotationZ));
        }

        private function raw(m:Matrix3D):String {
            if (m == null) {
                return "null";
            }
            var out:Array = [];
            for each (var v:Number in m.rawData) {
                out.push(r(v));
            }
            return out.join(",");
        }

        private function r(n:Number):Number {
            return Math.round(n * 1000) / 1000;
        }
    }
}

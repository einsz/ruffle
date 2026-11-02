package {
    import flash.display.Shape;
    import flash.display.Sprite;
    import flash.filters.BlurFilter;
    import flash.filters.DropShadowFilter;
    import flash.filters.GlowFilter;
    import flash.filters.GradientBevelFilter;
    import flash.filters.GradientGlowFilter;

    [SWF(width="400", height="470", backgroundColor="#ffffff")]
    public class Test extends Sprite {
        public function Test() {
            // Opaque red at ratio 0 shows the filter's whole area, for
            // [distance, angle, blur, quality].
            var cases:Array = [
                [0, 0, 0, 1], [0, 0, 4, 1], [0, 0, 10, 1], [0, 0, 4, 3],
                [4, 0, 0, 1], [4, 45, 4, 1], [4, 180, 4, 1], [10, 90, 0, 1]
            ];
            for (var i:int = 0; i < cases.length; i++) {
                var c:Array = cases[i];
                var s:Shape = new Shape();
                s.graphics.beginFill(0x0000ff);
                s.graphics.drawRect(0, 0, 20, 20);
                s.x = 30 + 90 * (i % 4);
                s.y = 40 + 120 * int(i / 4);
                s.filters = [new GradientGlowFilter(c[0], c[1], [0xff0000, 0x00ff00], [1, 1], [0, 255], c[2], c[2], 1, c[3], "outer")];
                addChild(s);
            }

            // A gradient bevel after a gradient glow: the bevel's middle color
            // shows that its area starts from what the glow draws to.
            var chained:Shape = new Shape();
            chained.graphics.beginFill(0x0000ff);
            chained.graphics.drawRect(0, 0, 20, 20);
            chained.x = 30;
            chained.y = 250;
            chained.filters = [
                new GradientGlowFilter(0, 0, [0xff0000, 0xff0000], [0, 1], [0, 255], 4, 4, 1, 1, "outer"),
                new GradientBevelFilter(0, 0, [0xff0000, 0x00ff00, 0xff0000], [1, 1, 1], [0, 128, 255], 4, 4, 1, 1, "outer")
            ];
            addChild(chained);

            // After other filters, which reach only about half as far as the
            // room they need: a blur, a glow, a drop shadow, with quality 0
            // a gradient glow and a glow, which blur nothing, and filters with
            // no blur.
            var red:GradientGlowFilter = new GradientGlowFilter(0, 0, [0xff0000, 0x00ff00], [1, 1], [0, 255], 0, 0, 1, 1, "outer");
            var after:Array = [
                [new BlurFilter(4, 4, 3), red],
                [new GlowFilter(0x000000, 1, 8, 8, 1, 1), red],
                [new DropShadowFilter(10, 90, 0x000000, 1, 4, 4, 1, 1), red],
                [new GradientGlowFilter(0, 0, [0xff0000, 0x00ff00], [1, 1], [0, 255], 10, 10, 1, 0, "outer")],
                [new GlowFilter(0x000000, 1, 8, 8, 1, 0), red],
                // Filters that blur nothing still add a pixel to the area.
                [new GlowFilter(0x000000, 1, 0, 0, 1, 1), red],
                [new GlowFilter(0x000000, 1, 0, 0, 1, 1), new GlowFilter(0x000000, 1, 0, 0, 1, 1), red],
                [new DropShadowFilter(4, 0, 0x000000, 1, 0, 0, 1, 1), red]
            ];
            for (i = 0; i < after.length; i++) {
                var shape:Shape = new Shape();
                shape.graphics.beginFill(0x0000ff);
                shape.graphics.drawRect(0, 0, 20, 20);
                shape.x = 30 + 90 * ((i + 1) % 4);
                shape.y = 250 + 90 * int((i + 1) / 4);
                shape.filters = after[i];
                addChild(shape);
            }
        }
    }
}

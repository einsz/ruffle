package {
    import flash.display.Bitmap;
    import flash.display.BitmapData;
    import flash.display.Shape;
    import flash.display.Sprite;
    import flash.filters.BitmapFilter;
    import flash.filters.DropShadowFilter;
    import flash.filters.GradientBevelFilter;
    import flash.filters.GradientGlowFilter;
    import flash.geom.Point;

    [SWF(width="400", height="300", backgroundColor="#ffffff")]
    public class Test extends Sprite {
        public function Test() {
            // Gradient glows on red squares: outer, inner, full, and outer
            // knockout moved by (4, 4).
            var glowColors:Array = [0x0000ff, 0x00ff00, 0xffff00];
            var glowAlphas:Array = [0, 1, 1];
            var glowRatios:Array = [0, 128, 255];
            var types:Array = ["outer", "inner", "full", "outer"];
            for (var i:int = 0; i < 4; i++) {
                addSquare(30 + 95 * i, 25, 0xff0000, new GradientGlowFilter(
                    i == 3 ? 4 * Math.SQRT2 : 0, 45, glowColors, glowAlphas, glowRatios, 12, 12, 2, 2, types[i], i == 3));
            }

            // Gradient bevels on blue squares.
            var bevelColors:Array = [0xffffff, 0xffffff, 0x000000];
            var bevelAlphas:Array = [1, 0, 1];
            var bevelRatios:Array = [0, 128, 255];
            for (i = 0; i < 4; i++) {
                addSquare(30 + 95 * i, 110, 0x3366cc, new GradientBevelFilter(
                    4, 45, bevelColors, bevelAlphas, bevelRatios, 6, 6, 1, 2, types[i], i == 3));
            }

            // A bevel whose middle colour isn't transparent, followed by a
            // filter that needs more room: the middle colour stays within the
            // bevel's area.
            var bar:Shape = new Shape();
            bar.graphics.beginFill(0x000000);
            bar.graphics.drawRect(0, 0, 60, 6);
            bar.x = 320;
            bar.y = 220;
            bar.filters = [
                new GradientBevelFilter(2, 10, [0x000000, 0x0000ff, 0x000000, 0x000000, 0x000000],
                    [0, 1, 1, 0, 0], [3, 26, 54, 136, 255], 4, 4, 1, 1, "outer"),
                new DropShadowFilter(20, 90, 0x000000, 1, 0, 0)
            ];
            addChild(bar);

            // applyFilter with blur 0: each pixel shows how the gradient's
            // colour and the source pixel combine, for outer, inner and full.
            var grid:BitmapData = new BitmapData(32, 16, true, 0);
            for (var y:int = 0; y < 16; y++) {
                for (var x:int = 0; x < 16; x++) {
                    grid.setPixel32(2 * x, y, (y * 17) << 24);
                    grid.setPixel32(2 * x + 1, y, ((x * 17) << 24) | 0xff0000);
                }
            }
            for (i = 0; i < 3; i++) {
                var dst:BitmapData = new BitmapData(32, 16, true, 0);
                dst.applyFilter(grid, grid.rect, new Point(0, 0), new GradientGlowFilter(
                    1, 0, [0x00ff00, 0x0000ff], [0, 1], [0, 255], 0, 0, 1, 1, types[i]));
                var bitmap:Bitmap = new Bitmap(dst);
                bitmap.x = 30 + 95 * i;
                bitmap.y = 205;
                bitmap.scaleX = 2;
                bitmap.scaleY = 4;
                addChild(bitmap);
            }
        }

        private function addSquare(x:Number, y:Number, color:uint, filter:BitmapFilter):void {
            var shape:Shape = new Shape();
            shape.graphics.beginFill(color);
            shape.graphics.drawRect(0, 0, 50, 50);
            shape.x = x;
            shape.y = y;
            shape.filters = [filter];
            addChild(shape);
        }
    }
}

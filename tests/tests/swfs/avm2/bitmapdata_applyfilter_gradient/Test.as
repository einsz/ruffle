package {
    import flash.display.BitmapData;
    import flash.display.Sprite;
    import flash.filters.BitmapFilter;
    import flash.filters.GradientBevelFilter;
    import flash.filters.GradientGlowFilter;
    import flash.geom.Point;
    import flash.geom.Rectangle;

    // The filters have no blur here, so each pixel shows exactly which colour
    // of the gradient the filter picks.
    public class Test extends Sprite {
        public function Test() {
            // Red at ratio 0, blue at ratio 255: the blue channel is the index.
            var colors:Array = [0xff0000, 0x0000ff];
            var alphas:Array = [1, 1];
            var ratios:Array = [0, 255];

            trace("// An opaque pixel at x = 8, distance 1 or 2");
            var dot:BitmapData = new BitmapData(17, 1, true, 0);
            dot.setPixel32(8, 0, 0xff000000);
            for each (var type:String in ["outer", "inner", "full"]) {
                pixels("glow " + type, apply(dot, new GradientGlowFilter(1, 0, colors, alphas, ratios, 0, 0, 1, 1, type)));
            }
            var bevelColors:Array = [0xff0000, 0x00ff00, 0x0000ff];
            var bevelAlphas:Array = [1, 1, 1];
            var bevelRatios:Array = [0, 128, 255];
            for each (type in ["outer", "inner", "full"]) {
                pixels("bevel " + type, apply(dot, new GradientBevelFilter(2, 0, bevelColors, bevelAlphas, bevelRatios, 0, 0, 1, 1, type)));
            }
            pixels("bevel full angle 180", apply(dot, new GradientBevelFilter(2, 180, bevelColors, bevelAlphas, bevelRatios, 0, 0, 1, 1, "full")));

            trace("// Part of the source");
            var part:BitmapData = new BitmapData(13, 1, true, 0);
            part.applyFilter(dot, new Rectangle(4, 0, 13, 1), new Point(0, 0), new GradientGlowFilter(1, 0, colors, alphas, ratios, 0, 0, 1, 1, "outer"));
            pixels("glow outer from x = 4", part);
            // The offset of 0.5 samples halfway towards the opaque pixel at
            // x = 3, which isn't part of the source.
            var outside:BitmapData = new BitmapData(17, 1, true, 0);
            outside.setPixel32(3, 0, 0xff000000);
            part.applyFilter(outside, new Rectangle(4, 0, 13, 1), new Point(0, 0), new GradientGlowFilter(0.5, 0, colors, alphas, ratios, 0, 0, 1, 1, "outer"));
            pixels("glow outer from x = 4, distance 0.5", part);
            var block:BitmapData = new BitmapData(200, 32, true, 0);
            block.fillRect(new Rectangle(104, 4, 24, 24), 0xff000000);
            var blurredPart:BitmapData = new BitmapData(60, 32, true, 0);
            blurredPart.applyFilter(block, new Rectangle(100, 0, 60, 32), new Point(0, 0), new GradientGlowFilter(0, 0, colors, alphas, ratios, 4, 4, 2, 1, "inner"));
            trace("blurred inner glow from x = 100, middle of the block: " + blurredPart.getPixel32(16, 16).toString(16) + ", outside it: " + blurredPart.getPixel32(40, 16).toString(16));

            trace("// Glow index for each alpha 0 to 255");
            // [a, 0, 0, 0] per alpha a: pixel 4a + 1 is transparent and its glow samples a.
            // (Not a power of two wide, so that the texture coordinates aren't exact.)
            var ramp:BitmapData = new BitmapData(1030, 1, true, 0);
            // [a, 255, 255, 255]: pixel 4a + 1 is opaque and its glow samples a.
            var opaqueRamp:BitmapData = new BitmapData(1030, 1, true, 0xff000000);
            for (var a:int = 0; a < 256; a++) {
                ramp.setPixel32(4 * a, 0, a << 24);
                opaqueRamp.setPixel32(4 * a, 0, a << 24);
            }
            for each (var strength:Number in [1, 2, 0.5, 0]) {
                indices("outer, strength " + strength, apply(ramp, new GradientGlowFilter(1, 0, colors, alphas, ratios, 0, 0, strength, 1, "outer")), 0);
                indices("inner, strength " + strength, apply(opaqueRamp, new GradientGlowFilter(1, 0, colors, alphas, ratios, 0, 0, strength, 1, "inner")), 0);
            }

            trace("// Bevel index for each alpha 0 to 255, behind and ahead");
            // [a, 0, b, 0]: pixel 1 samples b ahead and a behind.
            var pairs:BitmapData = new BitmapData(1030, 2, true, 0);
            for (a = 0; a < 256; a++) {
                pairs.setPixel32(4 * a, 0, a << 24);
                pairs.setPixel32(4 * a + 2, 1, a << 24);
            }
            for each (strength in [1, 2, 0.5]) {
                var bevel:BitmapData = apply(pairs, new GradientBevelFilter(1, 0, colors, alphas, ratios, 0, 0, strength, 1, "full"));
                indices("behind, strength " + strength, bevel, 0);
                indices("ahead, strength " + strength, bevel, 1);
            }
        }

        private function apply(source:BitmapData, filter:BitmapFilter):BitmapData {
            var result:BitmapData = new BitmapData(source.width, source.height, true, 0);
            result.applyFilter(source, source.rect, new Point(0, 0), filter);
            return result;
        }

        private function pixels(name:String, bitmap:BitmapData):void {
            var out:Array = [];
            for (var x:int = 0; x < bitmap.width; x++) {
                out.push(bitmap.getPixel32(x, 0).toString(16));
            }
            trace(name + ": " + out.join(" "));
        }

        // The blue channel of pixels 1, 5, 9, ...
        private function indices(name:String, bitmap:BitmapData, y:int):void {
            var out:Array = [];
            for (var x:int = 1; x < 1024; x += 4) {
                out.push(bitmap.getPixel32(x, y) & 0xff);
            }
            trace(name + ": " + out.join(" "));
        }
    }
}

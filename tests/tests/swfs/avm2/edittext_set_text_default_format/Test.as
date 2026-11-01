// Setting a TextField's text to the text it already has does nothing, unless
// defaultTextFormat was set since: then the default format is applied to the
// text again. Setting the text or htmlText ends that; setTextFormat and
// textColor don't. Checked against Flash Player 32.
package {
    import flash.display.MovieClip;
    import flash.text.TextField;
    import flash.text.TextFormat;

    public class Test extends MovieClip {
        private function field(text:String, size:int):TextField {
            var t:TextField = new TextField();
            t.defaultTextFormat = new TextFormat("_sans", size);
            t.text = text;
            return t;
        }

        private function size(t:TextField):Object {
            return t.getTextFormat(0, 1).size;
        }

        public function Test() {
            var a:TextField = field("abc", 30);
            a.defaultTextFormat = new TextFormat("_sans", 10);
            a.text = a.text;
            trace("default 10, text = text: " + size(a));

            var b:TextField = field("abc", 30);
            b.defaultTextFormat = new TextFormat("_sans", 10);
            b.text = "abc";
            trace("default 10, text = same string: " + size(b));

            var c:TextField = field("abc", 10);
            c.setTextFormat(new TextFormat(null, 20));
            c.text = c.text;
            trace("setTextFormat 20, text = text: " + size(c));

            var d:TextField = field("abc", 10);
            d.setTextFormat(new TextFormat(null, 20));
            d.text = "abd";
            trace("setTextFormat 20, other text: " + size(d));

            var h:TextField = field("abc", 10);
            h.setTextFormat(new TextFormat(null, 20));
            h.defaultTextFormat = new TextFormat("_sans", 10);
            h.text = h.text;
            trace("setTextFormat 20, the same default again, text = text: " + size(h));

            var i:TextField = field("abc", 30);
            i.defaultTextFormat = new TextFormat("_sans", 10);
            i.text = i.text;
            i.setTextFormat(new TextFormat(null, 20));
            i.text = i.text;
            trace("default 10, text = text, setTextFormat 20, text = text: " + size(i));

            var j:TextField = field("abc", 30);
            j.setTextFormat(new TextFormat(null, 20));
            j.defaultTextFormat = new TextFormat("_sans", 10);
            j.defaultTextFormat = new TextFormat("_sans", 30);
            j.text = j.text;
            trace("setTextFormat 20, default 10 then 30, text = text: " + size(j));

            var k:TextField = field("abc", 30);
            k.defaultTextFormat = new TextFormat("_sans", 10);
            trace("default 10 only: " + size(k));

            var m:TextField = field("abcd", 10);
            m.setTextFormat(new TextFormat(null, 20), 0, 2);
            m.defaultTextFormat = new TextFormat("_sans", 10, 0xff0000);
            m.text = m.text;
            trace("first half 20, default red, text = text: " + m.getTextFormat(0, 1).size + " " + m.getTextFormat(3, 4).size + " " + m.getTextFormat(0, 1).color);

            var p:TextField = field("abc", 10);
            p.defaultTextFormat = new TextFormat("_sans", 15);
            p.setTextFormat(new TextFormat(null, 20));
            p.text = p.text;
            trace("default 15, setTextFormat 20, text = text: " + size(p));

            var r:TextField = field("abc", 10);
            r.defaultTextFormat = new TextFormat("_sans", 15);
            r.htmlText = "abc";
            r.setTextFormat(new TextFormat(null, 20));
            r.text = r.text;
            trace("default 15, htmlText, setTextFormat 20, text = text: " + size(r));

            var hs:TextField = field("abc", 30);
            hs.defaultTextFormat = new TextFormat("_sans", 10);
            hs.htmlText = hs.htmlText;
            hs.setTextFormat(new TextFormat(null, 20));
            hs.text = hs.text;
            trace("default 10, htmlText = htmlText, setTextFormat 20, text = text: " + size(hs));

            var t:TextField = field("abc", 10);
            t.defaultTextFormat = new TextFormat("_sans", 15);
            t.text = "abd";
            t.setTextFormat(new TextFormat(null, 20));
            t.text = t.text;
            trace("default 15, other text, setTextFormat 20, text = text: " + size(t));

            var tc:TextField = field("abc", 10);
            tc.setTextFormat(new TextFormat(null, 20));
            tc.textColor = 0xff0000;
            tc.text = tc.text;
            trace("setTextFormat 20, textColor, text = text: " + size(tc));
        }
    }
}

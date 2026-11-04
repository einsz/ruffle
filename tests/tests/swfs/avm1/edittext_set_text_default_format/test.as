// Setting a TextField's text to the text it already has does nothing, unless
// setNewTextFormat was called since: then the new text format is applied to
// the text again. Setting the text or htmlText ends that; setTextFormat and
// textColor don't. Checked against Flash Player 32.
var n = 0;
function field(text, size) {
    n++;
    var t = _root.createTextField("t" + n, n, 0, 0, 100, 20);
    t.setNewTextFormat(new TextFormat("_sans", size));
    t.text = text;
    return t;
}
function size(t) { return t.getTextFormat(0, 1).size; }

var a = field("abc", 30);
a.setNewTextFormat(new TextFormat("_sans", 10));
a.text = a.text;
trace("new format 10, text = text: " + size(a));

var c = field("abc", 10);
c.setTextFormat(new TextFormat(null, 20));
c.text = c.text;
trace("setTextFormat 20, text = text: " + size(c));

var h = field("abc", 10);
h.setTextFormat(new TextFormat(null, 20));
h.setNewTextFormat(new TextFormat("_sans", 10));
h.text = h.text;
trace("setTextFormat 20, the same new format again, text = text: " + size(h));

var i = field("abc", 30);
i.setNewTextFormat(new TextFormat("_sans", 10));
i.text = i.text;
i.setTextFormat(new TextFormat(null, 20));
i.text = i.text;
trace("new format 10, text = text, setTextFormat 20, text = text: " + size(i));

var tc = field("abc", 10);
tc.setTextFormat(new TextFormat(null, 20));
tc.textColor = 0xff0000;
tc.text = tc.text;
trace("setTextFormat 20, textColor, text = text: " + size(tc));

var hs = field("abc", 30);
hs.setNewTextFormat(new TextFormat("_sans", 10));
hs.html = true;
hs.htmlText = hs.htmlText;
hs.setTextFormat(new TextFormat(null, 20));
hs.text = hs.text;
trace("new format 10, htmlText = htmlText, setTextFormat 20, text = text: " + size(hs));

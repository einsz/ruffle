// NetStream.close() stops the stream: time and bytes go back to 0, and no
// status events come until it is played again, which works as usual.
// a.flv: 1 s of video at 10 fps.
function log(s) { trace(s); }
function state(name, s) { return name + ": time " + Math.round(s.time * 10) / 10 + ", bytes " + s.bytesLoaded + "/" + s.bytesTotal; }

var nc = new NetConnection();
nc.connect(null);
var ns = new NetStream(nc);
var closed = false;
var frames = 0;
var phase = 0;
ns.onStatus = function(info) { if (closed) log("after close: " + info.code); };
ns.onMetaData = function(info) { log("onMetaData"); };
ns.play("a.flv");
_root.onEnterFrame = function() {
    if (phase == 0 && ns.time >= 0.5) {
        log(state("before close", ns));
        ns.close();
        closed = true;
        log(state("right after close", ns));
        phase = 1;
    } else if (phase == 1 && ++frames == 24) {
        log(state("24 frames after close", ns));
        closed = false;
        ns.play("a.flv");
        frames = 0;
        phase = 2;
    } else if (phase == 2 && (ns.time >= 0.3 || ++frames == 60)) {
        log(state(frames == 60 ? "play again gave up" : "play again reached 0.3", ns));
        ns.close();
        log("done");
        phase = 3;
    }
};

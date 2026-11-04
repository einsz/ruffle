package {
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.events.NetStatusEvent;
    import flash.net.NetConnection;
    import flash.net.NetStream;

    // NetStream.close() called by the onMetaData callback: the rest of the
    // FLV doesn't play, no status events come, and playing again works as
    // usual. Events before the close aren't logged: Ruffle fires
    // Buffer.Full before onMetaData, Flash Player doesn't.
    // a.flv: 1 s of video at 10 fps.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();
        private var s:NetStream;
        private var closed:Boolean = false;
        private var frames:int = 0;

        private function log(t:String):void { trace(t); }

        private function state(name:String):String {
            return name + ": time " + s.time.toFixed(1) + ", bytes " + s.bytesLoaded + "/" + s.bytesTotal;
        }

        public function Test() {
            connection.connect(null);
            s = new NetStream(connection);
            s.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                if (closed && frames < 24) log("event after close: " + e.info.code);
            });
            s.client = { onMetaData: function(i:Object):void {
                if (closed) return;
                closed = true;
                log("onMetaData, closing");
                s.close();
                log(state("right after close"));
            } };
            s.play("a.flv");
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (++frames == 24) {
                    log(state("24 frames later"));
                    s.play("a.flv");
                } else if (frames > 24 && (s.time >= 0.3 || frames == 84)) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    log(frames == 84 ? "playing again gave up" : "playing again reached 0.3");
                    s.close();
                    log("done");
                }
            });
        }
    }
}

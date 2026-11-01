package {
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.events.NetStatusEvent;
    import flash.net.NetConnection;
    import flash.net.NetStream;

    // NetStream.close() stops the stream: time and bytes go back to 0, and
    // no status events come until it is played again, which works as usual.
    // a.flv: 1 s of video at 10 fps.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();

        private function log(s:String):void { trace(s); }

        public function Test() {
            connection.connect(null);
            testPlay();
        }

        private function state(name:String, s:NetStream):String {
            return name + ": time " + s.time.toFixed(1) + ", bytes " + s.bytesLoaded + "/" + s.bytesTotal;
        }

        // Closes `s` once it has played to 0.5, logs its events for 24 frames,
        // then calls `after`.
        private function closeAt(name:String, s:NetStream, after:Function):void {
            var closed:Boolean = false;
            var frames:int = 0;
            s.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                if (closed && frames < 24) log(name + " after close: " + e.info.code);
            });
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (!closed && s.time >= 0.5) {
                    log(state(name + " before close", s));
                    s.close();
                    closed = true;
                    log(state(name + " right after close", s));
                } else if (closed && ++frames == 24) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    log(state(name + " 24 frames after close", s));
                    after();
                }
            });
        }

        private function waitTime(name:String, s:NetStream, t:Number, then:Function):void {
            var frames:int = 0;
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (s.time >= t || ++frames == 60) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    log(state(name + (frames == 60 ? " gave up" : " reached " + t), s));
                    then();
                }
            });
        }

        private function testPlay():void {
            var s:NetStream = new NetStream(connection);
            s.client = { onMetaData: function(i:Object):void { log("play: onMetaData"); } };
            s.play("a.flv");
            closeAt("play", s, function():void {
                s.play("a.flv");
                waitTime("play again", s, 0.3, function():void {
                    s.close();
                    log("done");
                });
            });
        }
    }
}

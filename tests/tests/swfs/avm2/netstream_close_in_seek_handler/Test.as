package {
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.events.NetStatusEvent;
    import flash.net.NetConnection;
    import flash.net.NetStream;

    // NetStream.close() and play() called by the Seek.Notify handler (this
    // crashed Ruffle). Only checks that the movie goes on: Flash Player then
    // doesn't play the new stream (its time stays below 0.5 for seconds),
    // while Ruffle does, and status events after play() come in another
    // order than in Flash Player.
    // a.flv: 1 s of video at 10 fps.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();
        private var s:NetStream;
        private var frames:int = 0;
        private var phase:int = 0;

        private function log(t:String):void { trace(t); }

        public function Test() {
            connection.connect(null);
            s = new NetStream(connection);
            s.client = { onMetaData: function(i:Object):void {} };
            s.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                if (e.info.code == "NetStream.Seek.Notify" && phase == 1) {
                    phase = 2;
                    log("Seek.Notify: close and play");
                    s.close();
                    s.play("a.flv");
                }
            });
            s.play("a.flv");
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                frames++;
                if (phase == 0 && frames == 5) {
                    phase = 1;
                    s.seek(0);
                } else if (phase == 2 && frames == 35) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    s.close();
                    log("done");
                }
            });
        }
    }
}

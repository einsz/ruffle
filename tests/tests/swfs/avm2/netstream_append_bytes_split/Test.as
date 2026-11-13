package {
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.net.NetConnection;
    import flash.net.NetStream;
    import flash.net.URLLoader;
    import flash.net.URLLoaderDataFormat;
    import flash.net.URLRequest;
    import flash.utils.ByteArray;

    // a.flv: 1 s of video at 10 fps, appended in pieces that split its
    // header and tags. Each plays as if appended whole.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();
        private var a:ByteArray;

        public function Test() {
            connection.connect(null);
            var loader:URLLoader = new URLLoader();
            loader.dataFormat = URLLoaderDataFormat.BINARY;
            loader.addEventListener(Event.COMPLETE, function(e:Event):void {
                a = loader.data as ByteArray;
                testSplit();
            });
            loader.load(new URLRequest("a.flv"));
        }

        private function log(s:String):void { trace(s); }

        // Whole, as a reference, in two halves at once, the first 5 bytes and
        // then the rest a frame later, and in 7 pieces, a frame apart.
        private function testSplit():void {
            split("whole", [], false, function():void {
                split("split in halves", [int(a.length / 2)], false, function():void {
                    split("split in the header", [5], true, function():void {
                        split("split in 7", [3, 10, 30, 200, 1000, 1500], true, function():void {
                            log("done");
                        });
                    });
                });
            });
        }

        // Appends a.flv cut at `cuts`: all at once, or a piece per frame.
        private function split(name:String, cuts:Array, perFrame:Boolean, then:Function):void {
            var s:NetStream = new NetStream(connection);
            var metas:int = 0;
            s.client = {
                onMetaData: function(info:Object):void {
                    metas++;
                    log(name + ": onMetaData " + metas + ", duration " + info.duration);
                }
            };
            var pieces:Array = [];
            var starts:Array = [0].concat(cuts, [a.length]);
            for (var i:int = 0; i + 1 < starts.length; i++) {
                var piece:ByteArray = new ByteArray();
                piece.writeBytes(a, starts[i], starts[i + 1] - starts[i]);
                pieces.push(piece);
            }
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (s.time >= 0.7) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    log(name + ": played on past 0.7");
                    s.close();
                    then();
                }
            });
            s.play(null);
            s.appendBytes(pieces.shift());
            if (!perFrame) {
                while (pieces.length > 0) {
                    s.appendBytes(pieces.shift());
                }
                return;
            }
            addEventListener(Event.ENTER_FRAME, function append(e:Event):void {
                s.appendBytes(pieces.shift());
                if (pieces.length == 0) {
                    removeEventListener(Event.ENTER_FRAME, append);
                }
            });
        }
    }
}

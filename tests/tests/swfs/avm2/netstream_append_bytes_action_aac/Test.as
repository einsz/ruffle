package {
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.events.NetStatusEvent;
    import flash.net.NetConnection;
    import flash.net.NetStream;
    import flash.net.NetStreamAppendBytesAction;
    import flash.net.URLLoader;
    import flash.net.URLLoaderDataFormat;
    import flash.net.URLRequest;
    import flash.text.TextField;
    import flash.utils.ByteArray;

    // aac.flv: 2 s of a 440 Hz tone, AAC at 22.05 kHz mono. FLV audio tags
    // can't say so for AAC (they always say 44 kHz stereo); only the AAC
    // sequence header does. aac_noconfig_tags.flv: its tags, without the file
    // header and without the AAC sequence header.
    //
    // After a seek with RESET_SEEK, data without the sequence header still
    // plays: the decoder configuration carries over (checked by recording
    // Flash Player 32's output).
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();
        private var tf:TextField = new TextField();

        private function log(s:String):void {
            trace(s);
            tf.appendText(s + "\n");
        }

        public function Test() {
            tf.width = 550; tf.height = 400; tf.multiline = true;
            addChild(tf);
            connection.connect(null);
            load("aac.flv", function(file:ByteArray):void {
                load("aac_noconfig_tags.flv", function(tags:ByteArray):void {
                    run(file, tags);
                });
            });
        }

        private static function load(url:String, then:Function):void {
            var loader:URLLoader = new URLLoader();
            loader.dataFormat = URLLoaderDataFormat.BINARY;
            loader.addEventListener(Event.COMPLETE, function(e:Event):void { then(loader.data as ByteArray); });
            loader.load(new URLRequest(url));
        }

        private function run(file:ByteArray, tags:ByteArray):void {
            var s:NetStream = new NetStream(connection);
            s.client = { onMetaData: function(info:Object):void { log("onMetaData"); } };
            var done:Boolean = false;
            s.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                if (!done) {
                    log(e.info.code);
                }
            });
            s.play(null);
            s.appendBytes(file);
            var seeked:Boolean = false;
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (!seeked && s.time >= 1) {
                    seeked = true;
                    log("seek at time " + s.time.toFixed(1));
                    s.seek(0);
                    s.appendBytesAction(NetStreamAppendBytesAction.RESET_SEEK);
                    s.appendBytes(tags);
                } else if (seeked && s.time >= 1.5) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    done = true;
                    log("played on past 1.5");
                }
            });
        }
    }
}

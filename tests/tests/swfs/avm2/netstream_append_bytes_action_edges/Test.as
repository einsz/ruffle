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
    import flash.utils.ByteArray;
    import flash.utils.Dictionary;

    // a.flv: 1 s of video at 10 fps; a_tags.flv: its tags, without the file
    // header.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();
        private var a:ByteArray, aTags:ByteArray;
        private var finished:Dictionary = new Dictionary();

        private function log(s:String):void { trace(s); }

        public function Test() {
            connection.connect(null);
            load("a.flv", function(data:ByteArray):void {
                a = data;
                load("a_tags.flv", function(data:ByteArray):void {
                    aTags = data;
                    testAppendOnBufferFull();
                });
            });
        }

        private static function load(url:String, then:Function):void {
            var loader:URLLoader = new URLLoader();
            loader.dataFormat = URLLoaderDataFormat.BINARY;
            loader.addEventListener(Event.COMPLETE, function(e:Event):void { then(loader.data as ByteArray); });
            loader.load(new URLRequest(url));
        }

        // Ruffle's close() doesn't stop the stream, so ignore it from now on.
        private function finish(s:NetStream):void {
            finished[s] = true;
            s.close();
        }

        // A stream in data generation mode that logs its status events if
        // `onStatus` is given, and calls `then` once its time has stopped
        // for 12 frames, if given.
        private function stream(name:String, onStatus:Function, then:Function):NetStream {
            var s:NetStream = new NetStream(connection);
            s.client = {};
            s.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                if (onStatus != null && !finished[s]) {
                    log(name + ": " + e.info.code);
                    onStatus(s, e.info.code);
                }
            });
            var lastTime:Number = -1;
            var stoppedFrames:int = 0;
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (then == null) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    return;
                }
                stoppedFrames = s.time > 0 && s.time == lastTime ? stoppedFrames + 1 : 0;
                lastTime = s.time;
                if (stoppedFrames == 12) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    // Flash's time for audio alone varies from run to run.
                    log(name + ": stopped" + (name == "audio format change" ? "" : " at time " + s.time.toFixed(0)));
                    finish(s);
                    then();
                }
            });
            s.play(null);
            return s;
        }

        // A NetStream.Buffer.Full listener that appends more data: the
        // stream plays on, without NetStream.Buffer.Empty.
        private function testAppendOnBufferFull():void {
            var appended:Boolean = false;
            var s:NetStream = stream("append on full", function(s:NetStream, code:String):void {
                if (code == "NetStream.Buffer.Full" && !appended) {
                    appended = true;
                    s.appendBytesAction(NetStreamAppendBytesAction.RESET_BEGIN);
                    s.appendBytes(a);
                }
            }, null);
            s.appendBytes(a);
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (s.time >= 1.5) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    log("append on full: played on past 1.5");
                    finish(s);
                    testSeekAfterAppend();
                }
            });
        }

        // Seeking right after appending drops the data before it plays:
        // no NetStream.Buffer.Full.
        private function testSeekAfterAppend():void {
            var s:NetStream = stream("seek after append", function(s:NetStream, code:String):void {}, null);
            s.appendBytes(a);
            s.seek(0);
            var frames:int = 0;
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (++frames == 24) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    log("seek after append: 24 frames later, time " + s.time.toFixed(1));
                    finish(s);
                    testResetSeekWithoutSeek();
                }
            });
        }

        // RESET_SEEK without seek() is like RESET_BEGIN: the appended tags
        // play after the ones already appended.
        private function testResetSeekWithoutSeek():void {
            var s:NetStream = stream("reset seek", null, testAudioFormatChange);
            s.appendBytes(a);
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (s.time >= 0.5) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    log("reset seek: appending at time " + s.time.toFixed(1));
                    s.appendBytesAction(NetStreamAppendBytesAction.RESET_SEEK);
                    s.appendBytes(aTags);
                }
            });
        }

        // A file in another audio format after RESET_BEGIN is decoded in its
        // own format: 8-bit PCM silence, then a quiet 16-bit PCM tone, which
        // would be loud noise if decoded as 8-bit samples (see the audio
        // assertions).
        private function testAudioFormatChange():void {
            var s:NetStream = stream("audio format change", null, function():void {
                log("done");
            });
            s.appendBytes(makePcmFlv(10, 8, 0));
            s.appendBytesAction(NetStreamAppendBytesAction.RESET_BEGIN);
            s.appendBytes(makePcmFlv(10, 16, 0.05));
        }

        // An FLV with `count` audio tags of 0.1 s each of a 440 Hz tone, as
        // 44 kHz mono PCM with `bits` bits per sample, at `amplitude`.
        private static function makePcmFlv(count:int, bits:int, amplitude:Number):ByteArray {
            var flv:ByteArray = new ByteArray();
            flv.writeUTFBytes("FLV");
            flv.writeByte(1);
            flv.writeByte(4);
            flv.writeUnsignedInt(9);
            flv.writeUnsignedInt(0);
            var samples:int = 4410;
            var dataSize:int = 1 + samples * bits / 8;
            for (var i:int = 0; i < count; i++) {
                var timestamp:int = i * 100;
                flv.writeByte(8);
                flv.writeByte(dataSize >> 16);
                flv.writeShort(dataSize & 0xffff);
                flv.writeByte(timestamp >> 16);
                flv.writeShort(timestamp & 0xffff);
                flv.writeByte(timestamp >>> 24);
                flv.writeByte(0);
                flv.writeShort(0);
                flv.writeByte(bits == 16 ? 0x3e : 0x3c); // PCM little endian, 44 kHz, mono
                for (var j:int = 0; j < samples; j++) {
                    var sample:Number = Math.sin((i * samples + j) * 2 * Math.PI * 440 / 44100) * amplitude;
                    if (bits == 16) {
                        var v:int = int(sample * 32767);
                        flv.writeByte(v & 0xff);
                        flv.writeByte((v >> 8) & 0xff);
                    } else {
                        flv.writeByte(128 + int(sample * 127));
                    }
                }
                flv.writeUnsignedInt(11 + dataSize);
            }
            return flv;
        }
    }
}

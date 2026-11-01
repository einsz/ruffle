package {
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.events.NetStatusEvent;
    import flash.net.NetConnection;
    import flash.net.NetStream;
    import flash.net.NetStreamAppendBytesAction;
    import flash.utils.ByteArray;

    // Appends 25 s of silent PCM audio (44.1 kHz, 16-bit stereo, about
    // 4.4 MB, tags every 0.1 s from 0 to 24.9 s) three times, the next one when the stream
    // reaches 20 s and 44 s, with appendBytesAction(RESET_BEGIN): a stream
    // that is appended to for a long time, as a looping animation does,
    // plays on.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();
        private var stream:NetStream;
        private var flv:ByteArray = makeFlv(250);
        private var appended:int = 0;
        private var lastTime:Number = -1;
        private var stoppedFrames:int = 0;

        public function Test() {
            connection.connect(null);
            stream = new NetStream(connection);
            stream.client = {};
            stream.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                trace("status " + e.info.code);
            });
            stream.play(null);
            append();
            addEventListener(Event.ENTER_FRAME, onEnterFrame);
        }

        private function append():void {
            if (appended > 0) {
                stream.appendBytesAction(NetStreamAppendBytesAction.RESET_BEGIN);
            }
            stream.appendBytes(flv);
            appended++;
            trace("appended copy " + appended + ", bytesLoaded " + stream.bytesLoaded + ", bytesTotal " + stream.bytesTotal);
        }

        private function onEnterFrame(e:Event):void {
            if (appended < 3 && stream.time >= 24.9 * appended - 4.9) {
                trace("time reached " + (24.9 * appended - 4.9).toFixed(1));
                append();
            }
            stoppedFrames = stream.time > 0 && stream.time == lastTime ? stoppedFrames + 1 : 0;
            lastTime = stream.time;
            if (stoppedFrames == 12) {
                removeEventListener(Event.ENTER_FRAME, onEnterFrame);
                trace("stopped at time " + stream.time.toFixed(0) + ", bytesLoaded " + stream.bytesLoaded + ", bytesTotal " + stream.bytesTotal);
                stream.close();
            }
        }

        // An FLV with `count` audio tags of 0.1 s of silence each.
        private static function makeFlv(count:int):ByteArray {
            var flv:ByteArray = new ByteArray();
            flv.writeUTFBytes("FLV");
            flv.writeByte(1);
            flv.writeByte(4); // audio only
            flv.writeUnsignedInt(9);
            flv.writeUnsignedInt(0);
            var dataSize:int = 1 + 4410 * 4;
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
                flv.writeByte(0x3f); // PCM little endian, 44 kHz, 16-bit, stereo
                flv.length += 4410 * 4;
                flv.position = flv.length;
                flv.writeUnsignedInt(11 + dataSize);
            }
            return flv;
        }
    }
}

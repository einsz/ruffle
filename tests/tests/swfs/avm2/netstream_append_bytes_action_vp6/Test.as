package {
    import flash.display.Sprite;
    import flash.events.Event;
    import flash.media.Video;
    import flash.net.NetConnection;
    import flash.net.NetStream;
    import flash.net.NetStreamAppendBytesAction;
    import flash.net.URLLoader;
    import flash.net.URLLoaderDataFormat;
    import flash.net.URLRequest;
    import flash.utils.ByteArray;

    // vp6.flv: 3 frames of 640x480 VP6 video, with their size only in the
    // onMetaData tag. Seeking from onMetaData and appending the video tags
    // again after RESET_SEEK, without the metadata: they play at their
    // full size.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();

        public function Test() {
            connection.connect(null);
            var loader:URLLoader = new URLLoader();
            loader.dataFormat = URLLoaderDataFormat.BINARY;
            loader.addEventListener(Event.COMPLETE, function(e:Event):void { play(loader.data as ByteArray); });
            loader.load(new URLRequest("vp6.flv"));
        }

        private function play(flv:ByteArray):void {
            // The video tags: after the file header, the first previous tag
            // size and the onMetaData tag.
            flv.position = 13 + 1;
            var metaSize:int = flv.readUnsignedByte() << 16 | flv.readUnsignedShort();
            var tags:ByteArray = new ByteArray();
            tags.writeBytes(flv, 13 + 11 + metaSize + 4);

            var s:NetStream = new NetStream(connection);
            var seeked:Boolean = false;
            s.client = {
                onMetaData: function(info:Object):void {
                    trace("onMetaData: width " + info.width + ", height " + info.height);
                    if (!seeked) {
                        seeked = true;
                        s.seek(0);
                        s.appendBytesAction(NetStreamAppendBytesAction.RESET_SEEK);
                        s.appendBytes(tags);
                    }
                }
            };
            var video:Video = new Video(320, 240);
            video.attachNetStream(s);
            addChild(video);
            s.play(null);
            s.appendBytes(flv);
        }
    }
}

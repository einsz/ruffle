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
    import flash.utils.getQualifiedClassName;

    // a.flv: 1 s of video at 10 fps. b.flv: 0.5 s. a_tags.flv: the tags of
    // a.flv, without the file header.
    public class Test extends Sprite {
        private var connection:NetConnection = new NetConnection();
        private var a:ByteArray, b:ByteArray, aTags:ByteArray;

        public function Test() {
            connection.connect(null);
            load("a.flv", function(data:ByteArray):void {
                a = data;
                load("b.flv", function(data:ByteArray):void {
                    b = data;
                    load("a_tags.flv", function(data:ByteArray):void {
                        aTags = data;
                        testBufferFull();
                    });
                });
            });
        }

        private static function load(url:String, then:Function):void {
            var loader:URLLoader = new URLLoader();
            loader.dataFormat = URLLoaderDataFormat.BINARY;
            loader.addEventListener(Event.COMPLETE, function(e:Event):void { then(loader.data as ByteArray); });
            loader.load(new URLRequest(url));
        }

        // A stream in data generation mode, which calls `then` once its time
        // has stopped: once it has run out of data.
        private function stream(name:String, onMetaData:Function, then:Function):NetStream {
            var stream:NetStream = new NetStream(connection);
            var metas:int = 0;
            stream.client = {
                onMetaData: function(info:Object):void {
                    metas++;
                    trace(name + ": onMetaData " + metas + ", duration " + info.duration +
                        (name == "seek" ? "" : ", time " + stream.time.toFixed(1)));
                    onMetaData(metas);
                }
            };
            var lastTime:Number = -1;
            var stoppedFrames:int = 0;
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                stoppedFrames = stream.time > 0 && stream.time == lastTime ? stoppedFrames + 1 : 0;
                lastTime = stream.time;
                if (stoppedFrames == 12) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    trace(name + ": stopped at time " + stream.time.toFixed(0));
                    stream.close();
                    then();
                }
            });
            stream.play(null);
            return stream;
        }

        // NetStream.Buffer.Full comes as the stream plays, after onMetaData,
        // not from appendBytes.
        private function testBufferFull():void {
            var s:NetStream = new NetStream(connection);
            s.client = {
                onMetaData: function(info:Object):void { trace("buffer full: onMetaData"); }
            };
            s.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                if (e.info.code == "NetStream.Buffer.Full") {
                    trace("buffer full: NetStream.Buffer.Full");
                    s.close();
                    testLoop();
                }
            });
            s.play(null);
            s.appendBytes(a);
            trace("buffer full: appendBytes returned");
        }

        // Appending a file again from its onMetaData, after RESET_BEGIN, plays
        // it again after the last one.
        private function testLoop():void {
            var s:NetStream = stream("loop", function(metas:int):void {
                if (metas < 3) {
                    s.appendBytesAction(NetStreamAppendBytesAction.RESET_BEGIN);
                    s.appendBytes(a);
                }
            }, testSecondFile);
            s.appendBytes(a);
        }

        // A second file without RESET_BEGIN is ignored.
        private function testSecondFile():void {
            var s:NetStream = stream("second file", function(metas:int):void {}, testSeek);
            s.appendBytes(a);
            s.appendBytes(a);
            s.appendBytesAction(NetStreamAppendBytesAction.RESET_BEGIN);
            s.appendBytes(b);
        }

        // Seeking drops the appended data; then tags play at their own
        // timestamps, from the start of a.flv.
        private function testSeek():void {
            var s:NetStream = new NetStream(connection);
            var metas:int = 0;
            s.client = {
                onMetaData: function(info:Object):void {
                    metas++;
                    trace("seek: onMetaData " + metas + ", duration " + info.duration);
                }
            };
            s.play(null);
            s.appendBytes(a);
            var seeked:Boolean = false;
            var restarted:Boolean = false;
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (!seeked && s.time >= 0.5) {
                    seeked = true;
                    s.seek(0.2);
                    s.appendBytesAction(NetStreamAppendBytesAction.RESET_SEEK);
                    s.appendBytes(aTags);
                } else if (seeked && !restarted && s.time < 0.2) {
                    restarted = true;
                    trace("seek: back at the start");
                } else if (restarted && s.time >= 0.7) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    trace("seek: played on past 0.7");
                    s.close();
                    testSeekFromCallback();
                }
            });
        }

        // Seeking from onMetaData, while the stream is processing tags.
        private function testSeekFromCallback():void {
            var s:NetStream = new NetStream(connection);
            var metas:int = 0;
            s.client = {
                onMetaData: function(info:Object):void {
                    metas++;
                    trace("seek from onMetaData: onMetaData " + metas + ", duration " + info.duration);
                    if (metas == 1) {
                        s.seek(0);
                        s.appendBytesAction(NetStreamAppendBytesAction.RESET_BEGIN);
                        s.appendBytes(b);
                    }
                }
            };
            s.play(null);
            s.appendBytes(a);
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (metas == 2 && s.time >= 0.3) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    trace("seek from onMetaData: played on past 0.3");
                    s.close();
                    testPlayFromCallback();
                }
            });
        }

        // Playing a file from onMetaData replaces the appended data.
        private function testPlayFromCallback():void {
            var s:NetStream = new NetStream(connection);
            var metas:int = 0;
            s.client = {
                onMetaData: function(info:Object):void {
                    metas++;
                    trace("play from onMetaData: onMetaData " + metas + ", duration " + info.duration);
                    if (metas == 1) {
                        s.play("b.flv");
                    }
                }
            };
            s.play(null);
            s.appendBytes(a);
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (metas == 2 && s.time >= 0.3) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    trace("play from onMetaData: played on past 0.3");
                    s.close();
                    testPlayAfterAppend();
                }
            });
        }

        // Playing a file right after appending replaces the appended data,
        // including its pending NetStream.Buffer.Full.
        private function testPlayAfterAppend():void {
            var s:NetStream = new NetStream(connection);
            s.client = {
                onMetaData: function(info:Object):void {
                    trace("play after append: onMetaData, duration " + info.duration);
                }
            };
            var bufferFulls:int = 0;
            s.addEventListener(NetStatusEvent.NET_STATUS, function(e:NetStatusEvent):void {
                if (e.info.code == "NetStream.Buffer.Full") {
                    bufferFulls++;
                }
            });
            s.play(null);
            s.appendBytes(a);
            s.play("b.flv");
            addEventListener(Event.ENTER_FRAME, function wait(e:Event):void {
                if (s.time >= 0.3) {
                    removeEventListener(Event.ENTER_FRAME, wait);
                    trace("play after append: played on past 0.3, NetStream.Buffer.Full " + bufferFulls + " time(s)");
                    s.close();
                    testErrors();
                }
            });
        }

        private function testErrors():void {
            var s:NetStream = new NetStream(connection);
            action(s, NetStreamAppendBytesAction.RESET_BEGIN, "before play(null)");
            s.play(null);
            action(s, "bogus", "unknown action");
            action(s, NetStreamAppendBytesAction.END_SEQUENCE, "endSequence");
            s.close();
            trace("done");
        }

        private static function action(s:NetStream, action:String, name:String):void {
            try {
                s.appendBytesAction(action);
                trace(name + ": ok");
            } catch (e:Error) {
                trace(name + ": " + getQualifiedClassName(e) + " " + e.errorID);
            }
        }
    }
}

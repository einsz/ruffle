// The sounds S1-S14 are DefineSound tags added to the compiled SWF, made from MP3s
// generated with FFmpeg 9.0 (libmp3lame, 32 kbit/s mono, 0.5 s of a 441 Hz sine):
//
//   ffmpeg -f lavfi -i sine=frequency=441:sample_rate=22050:duration=0.5 -ac 1 \
//       -c:a libmp3lame -b:a 32k -id3v2_version 0 -write_xing 0 -f mp3 m2.mp3
//
// m2.mp3 has 22 MPEG-2 layer III frames of 576 samples (12672). The same with the Info
// frame (without -write_xing 0) has 23, and at 44100 Hz (m1.mp3) 21 MPEG-1 frames of 1152.
// "header" is the sample count in DefineSound, "seek" its MP3 seek samples. The last
// three sounds are 22050 Hz with a sample count where n * 1000 / rate and
// n / (rate / 1000) differ in the last bit.
package {
    import flash.display.Sprite;
    import flash.media.Sound;
    import flash.text.TextField;

    public class Test extends Sprite {
        public function Test() {
            var tf:TextField = new TextField();
            tf.width = 550;
            tf.height = 400;
            addChild(tf);
            var classes:Array = [S1, S2, S3, S4, S5, S6, S7, S8, S9, S10, S11, S12, S13, S14];
            var names:Array = [
                "MPEG-2, header exact",
                "MPEG-2, header 2x",
                "MPEG-2, header shorter",
                "MPEG-2, header 2x, seek 1000",
                "MPEG-2, header 2x, last frame only its header",
                "MPEG-2, header 2x, last frame 3 bytes",
                "MPEG-2, header 2x, last frame 24 bytes",
                "MPEG-2, header 2x, 50 bytes before the first frame",
                "MPEG-2, header 2x, 37 zero bytes after frame 10",
                "MPEG-2, header 2x, Info frame",
                "MPEG-1, header 2x",
                "MPEG-2, header shorter (8018)",
                "PCM, 1003 samples",
                "ADPCM, header 1005"
            ];
            for (var i:int = 0; i < classes.length; i++) {
                var s:Sound = new classes[i]();
                var line:String = names[i] + ": " + s.length;
                trace(line);
                tf.appendText(line + "\n");
            }
        }
    }
}

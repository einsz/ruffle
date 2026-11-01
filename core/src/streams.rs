//! NetStream implementation

use crate::avm1::{
    Activation as Avm1Activation, ActivationIdentifier as Avm1ActivationIdentifier,
    ExecutionReason as Avm1ExecutionReason, FlvValueAvm1Ext, Object as Avm1Object,
};
use crate::avm2::object::NetStreamObject;
use crate::avm2::{
    Activation as Avm2Activation, Avm2, Error as Avm2Error, EventObject as Avm2EventObject,
    FlvValueAvm2Ext, FunctionArgs, Object as Avm2Object, Value as Avm2Value,
};
use crate::backend::audio::{
    DecodeError, SoundInstanceHandle, SoundStreamInfo, SoundStreamWrapping,
};
use crate::backend::navigator::Request;
use crate::context::UpdateContext;
use crate::display_object::{MovieClip, TDisplayObject};
use crate::loader::Error;
use crate::string::AvmString;
use flv_rs::{
    AudioData as FlvAudioData, AudioDataType as FlvAudioDataType, Error as FlvError, FlvReader,
    FrameType as FlvFrameType, Header as FlvHeader, ScriptData as FlvScriptData,
    SoundFormat as FlvSoundFormat, SoundRate as FlvSoundRate, SoundSize as FlvSoundSize,
    SoundType as FlvSoundType, Tag as FlvTag, TagData as FlvTagData, Value as FlvValue,
    VideoData as FlvVideoData, VideoPacket as FlvVideoPacket,
};
use gc_arena::barrier::unlock;
use gc_arena::{Collect, DynamicRoot, Gc, Lock, Mutation, Rootable};
use ruffle_common::buffer::{Buffer, Slice, Substream, SubstreamError};
use ruffle_common::duration::FloatDuration;
use ruffle_macros::istr;
use ruffle_render::bitmap::BitmapInfo;
use ruffle_video::VideoStreamHandle;
use ruffle_video::frame::EncodedFrame;
use std::cell::{Cell, RefCell};
use std::cmp::max;
use std::io::{Seek, SeekFrom};
use swf::{AudioCompression, SoundFormat, VideoCodec, VideoDeblocking};
use thiserror::Error;
use url::Url;

#[derive(Debug, Error)]
enum NetstreamError {
    #[error("Decoding failed because {0}")]
    DecodeError(DecodeError),

    #[error("Substream management error {0}")]
    SubstreamError(SubstreamError),

    #[error("Unknown codec")]
    UnknownCodec,
}

impl From<DecodeError> for NetstreamError {
    fn from(err: DecodeError) -> NetstreamError {
        NetstreamError::DecodeError(err)
    }
}

impl From<SubstreamError> for NetstreamError {
    fn from(err: SubstreamError) -> NetstreamError {
        NetstreamError::SubstreamError(err)
    }
}

/// Manager for all media streams.
///
/// This does *not* handle data transport; which is delegated to `LoadManager`.
/// `StreamManager` *only* handles decoding or encoding of relevant media
/// streams.
#[derive(Collect)]
#[collect(no_drop)]
pub struct StreamManager<'gc> {
    /// List of streams that need tick processing.
    ///
    /// This is not the total list of all created NetStreams; only the ones
    /// that have been configured to play media.
    ///
    /// A stream becomes active if it is either playing streaming media or is
    /// doing other tick-time processing such as seeking.
    active_streams: Vec<NetStream<'gc>>,
}

impl Default for StreamManager<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'gc> StreamManager<'gc> {
    pub fn new() -> Self {
        StreamManager {
            active_streams: Vec::new(),
        }
    }

    /// Activate a `NetStream`.
    ///
    /// This can be called at any time to flag that a `NetStream` has work to
    /// do, and called multiple times. The `NetStream` will determine what to
    /// do at tick time.
    pub fn activate(context: &mut UpdateContext<'gc>, stream: NetStream<'gc>) {
        if !context.stream_manager.active_streams.contains(&stream) {
            context.stream_manager.active_streams.push(stream);
        }
    }

    /// Deactivate a `NetStream`.
    ///
    /// This should only ever be called at tick time if the stream itself has
    /// determined there is no future work for it to do.
    pub fn deactivate(context: &mut UpdateContext<'gc>, stream: NetStream<'gc>) {
        let index = context
            .stream_manager
            .active_streams
            .iter()
            .position(|x| *x == stream);
        if let Some(index) = index {
            context.stream_manager.active_streams.remove(index);
        }
    }

    /// Process all active media streams.
    ///
    /// This is an unlocked timestep; the `dt` parameter indicates how many
    /// milliseconds have elapsed since the last tick. This is intended to
    /// support video framerates separate from the Stage frame rate.
    ///
    /// This does not borrow `&mut self` as we need the `UpdateContext`, too.
    pub fn tick(context: &mut UpdateContext<'gc>, dt: FloatDuration) {
        let streams = context.stream_manager.active_streams.clone();
        for stream in streams {
            stream.tick(context, dt)
        }
    }
}

#[derive(Copy, Clone, Collect, Debug)]
#[collect(no_drop)]
enum NetStreamKind<'gc> {
    Avm2(NetStreamObject<'gc>),
    Avm1(Avm1Object<'gc>),
}

/// A stream representing download of some (audiovisual) data.
///
/// `NetStream` interacts with several different parts of player
/// infrastructure:
///
///  * `LoadManager` fills individual `NetStream` buffers with data (or, in the
///    future, empties them out for media upload)
///  * `StreamManager` processes media data in the `NetStream` buffer (in the
///    future, sending it to the audio backend or `SoundManager`)
///  * `Video` display objects linked to this `NetStream` display the latest
///    decoded frame.
///
/// It corresponds directly to the AVM1 and AVM2 `NetStream` classes; it's API
/// is intended to be a VM-agnostic version of those.
#[derive(Clone, Debug, Collect, Copy)]
#[collect(no_drop)]
pub struct NetStream<'gc>(Gc<'gc, NetStreamData<'gc>>);

impl PartialEq for NetStream<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(Gc::as_ptr(self.0), Gc::as_ptr(other.0))
    }
}

impl Eq for NetStream<'_> {}

#[derive(Clone)]
pub struct NetStreamHandle(DynamicRoot<Rootable![NetStreamData<'_>]>);

impl NetStreamHandle {
    pub fn stash<'gc>(context: &UpdateContext<'gc>, this: NetStream<'gc>) -> Self {
        Self(context.dynamic_root.stash(context.gc(), this.0))
    }

    pub fn fetch<'gc>(&self, context: &UpdateContext<'gc>) -> NetStream<'gc> {
        NetStream(context.dynamic_root.fetch(&self.0))
    }
}

/// The current type of the data in the stream buffer.
#[derive(Clone, Debug)]
pub enum NetStreamType {
    /// The stream is an FLV.
    Flv {
        #[expect(dead_code)] // set but never read
        header: FlvHeader,

        /// The currently playing video track's stream instance, and its codec.
        video_stream: Option<(VideoStreamHandle, VideoCodec)>,

        /// The index of the last processed frame.
        ///
        /// FLV does not store this information directly and we are not holding
        /// onto a table of data buffers like `Video` does, so we must maintain
        /// frame IDs ourselves for various API related purposes.
        frame_id: u32,
    },
}

#[derive(Clone, Debug, Collect)]
#[collect(no_drop)]
pub struct NetStreamSource {
    /// All data currently loaded in the stream.
    buffer: RefCell<Buffer>,

    /// The buffer position that we are currently seeking to.
    offset: Cell<usize>,

    /// The expected length of the buffer once downloading is complete.
    ///
    /// `None` indicates that downloading is already complete and that the
    /// length of the associated `Buffer` is the final length.
    expected_length: Cell<Option<usize>>,

    /// The buffer position for processing incoming data.
    ///
    /// This points to the first byte that the stream has *never* processed
    /// before in the buffer. It should always be greater than or equal to the
    /// offset position.
    ///
    /// Certain data, such as the header or metadata of an FLV, should only
    /// ever be processed one time, even if we seek backwards to it later on.
    /// We call this data "preloaded", whether or not there is actually a
    /// separate preload step for that given format.
    preload_offset: Cell<usize>,

    /// The current stream type, if known.
    stream_type: RefCell<Option<NetStreamType>>,

    /// The current seek offset in the stream in milliseconds.
    stream_time: Cell<f64>,

    /// The next queued seek offset in milliseconds.
    ///
    /// Seeks are only executed on the next stream tick.
    queued_seek_time: Cell<Option<f64>>,

    /// The `Substream` associated with the currently playing audio track and
    /// the expected playback format of that audio.
    audio_stream: RefCell<Option<(Substream, SoundStreamInfo)>>,

    /// The last AAC decoder configuration (`AudioSpecificConfig`) seen.
    ///
    /// It outlives `audio_stream`, which is dropped whenever the sound stops,
    /// as the stream may go on without repeating it.
    aac_config: RefCell<Option<Box<[u8]>>>,

    /// The currently playing sound stream
    sound_instance: Cell<Option<SoundInstanceHandle>>,
}

#[derive(Clone, Debug, Collect)]
#[collect(no_drop)]
pub struct NetStreamData<'gc> {
    /// Stream source.
    source: Lock<Gc<'gc, NetStreamSource>>,

    /// The number of seconds of video data that should be buffered. This is
    /// currently unsupported and changing it has no effect.
    buffer_time: Cell<f64>,

    /// The last decoded bitmap.
    ///
    /// Any `Video`s on the stage will display the bitmap here when attached to
    /// this `NetStream`.
    last_decoded_bitmap: RefCell<Option<BitmapInfo>>,

    /// The AVM side of this stream.
    avm_object: Lock<Option<NetStreamKind<'gc>>>,

    /// The AVM2 client object, which corresponds to `NetStream.client`.
    avm2_client: Lock<Option<Avm2Object<'gc>>>,

    /// The URL of the requested FLV if one exists.
    url: RefCell<Option<String>>,

    /// The MovieClip this `NetStream` is attached to.
    attached_to: Lock<Option<MovieClip<'gc>>>,

    /// True if the stream should play when ticked.
    playing: Cell<bool>,

    /// True if this stream was put into data generation mode with play(null).
    data_generation_mode: Cell<bool>,

    /// Bytes appended while a script-data callback is running.
    pending_append_bytes: RefCell<Vec<u8>>,

    /// True while handling FLV script data that can call back into AVM.
    handling_script_data: Cell<bool>,

    /// Joins the FLV data appended in data generation mode into one stream.
    #[collect(require_static)]
    appended_flv: RefCell<AppendedFlv>,

    /// True once the buffer has been set up for appended data, after
    /// `play(null)`.
    appending: Cell<bool>,

    /// True while a stream in data generation mode has run out of data.
    out_of_data: Cell<bool>,

    /// True if a stream in data generation mode got data after running out,
    /// and is yet to report `NetStream.Buffer.Full`.
    buffer_full_pending: Cell<bool>,

    /// True after a seek in data generation mode, until data is appended:
    /// playback goes on from the timestamp of the first appended tag.
    seeking_appended: Cell<bool>,
}

/// Joins the FLV data appended with `appendBytes()` into a single FLV
/// stream, behind the header that the buffer starts with.
///
/// Like Flash, this skips the file header of new data at the start and
/// after `appendBytesAction(RESET_BEGIN)` or `RESET_SEEK`, offsets the
/// timestamps of a file appended after `RESET_BEGIN` so that it plays on
/// after the previous one, and drops data that is neither, such as a
/// second file without a reset, until the next reset.
#[derive(Clone, Debug, Default)]
struct AppendedFlv {
    /// The bytes of an incomplete header or tag, waiting for the rest.
    pending: Vec<u8>,

    /// The number of bytes to skip before the next tag: the size of the
    /// previous tag, which follows every tag and the file header.
    skip: usize,

    /// True if new data may start with a file header, to skip.
    header_allowed: bool,

    /// True if data is dropped until the next reset.
    rejecting: bool,

    /// The offset added to the timestamps of appended tags, in milliseconds.
    timestamp_offset: u32,

    /// The timestamp of the last tag output, offset included.
    last_timestamp: u32,
}

impl AppendedFlv {
    /// A placeholder FLV header for the appended data: audio and video, and
    /// no data offset beyond its own 9 bytes.
    const HEADER: [u8; 9] = [b'F', b'L', b'V', 1, 0b101, 0, 0, 0, 9];

    fn new() -> Self {
        Self {
            header_allowed: true,
            ..Default::default()
        }
    }

    /// Prepares for data that starts anew, with a file header or not.
    ///
    /// After `RESET_BEGIN` (`go_on` is true), its timestamps start where
    /// the last tag was.
    fn reset(&mut self, go_on: bool) {
        self.pending.clear();
        self.skip = 0;
        self.header_allowed = true;
        self.rejecting = false;
        if go_on {
            self.timestamp_offset = self.last_timestamp;
        }
    }

    /// Takes appended data, and writes the complete tags in it to `output`,
    /// with their timestamps offset, each behind a previous tag size as
    /// `FlvTag::parse` reads them.
    ///
    /// Returns the timestamp of the first tag written, if any.
    fn append(&mut self, data: &[u8], output: &mut Vec<u8>) -> Option<u32> {
        if self.rejecting {
            return None;
        }
        self.pending.extend_from_slice(data);

        let mut first_timestamp = None;
        let mut position = 0;
        loop {
            let rest = &self.pending[position..];
            if self.skip > 0 {
                let skipped = self.skip.min(rest.len());
                position += skipped;
                self.skip -= skipped;
                if self.skip > 0 {
                    break;
                }
                continue;
            }
            if self.header_allowed {
                if rest.len() < 3 {
                    break;
                }
                if rest.starts_with(b"FLV") {
                    let Some(&[a, b, c, d]) = rest.get(5..9) else {
                        break;
                    };
                    let header_length = u32::from_be_bytes([a, b, c, d]) as usize;
                    if rest.len() < header_length {
                        break;
                    }
                    position += header_length;
                    self.skip = 4;
                }
                self.header_allowed = false;
                continue;
            }

            // The tag header: type, data size, timestamp, extended timestamp
            // and stream ID.
            if rest.len() < 11 {
                break;
            }
            if !matches!(rest[0] & 0x1F, 8 | 9 | 18) {
                // Not a tag: Flash ignores everything up to the next reset.
                self.rejecting = true;
                self.pending.clear();
                return first_timestamp;
            }
            let data_size = u32::from_be_bytes([0, rest[1], rest[2], rest[3]]) as usize;
            let tag_length = 11 + data_size;
            if rest.len() < tag_length {
                break;
            }

            let timestamp = u32::from_be_bytes([rest[7], rest[4], rest[5], rest[6]])
                .wrapping_add(self.timestamp_offset);
            let [extended, high, middle, low] = timestamp.to_be_bytes();
            output.extend_from_slice(&(tag_length as u32).to_be_bytes());
            let start = output.len();
            output.extend_from_slice(&rest[..tag_length]);
            output[start + 4..start + 8].copy_from_slice(&[high, middle, low, extended]);

            self.last_timestamp = timestamp;
            first_timestamp.get_or_insert(timestamp);
            position += tag_length;
            self.skip = 4;
        }
        self.pending.drain(..position);
        first_timestamp
    }
}

impl Default for NetStreamSource {
    fn default() -> Self {
        Self {
            buffer: RefCell::new(Buffer::new()),
            offset: Cell::new(0),
            expected_length: Cell::new(Some(0)),
            preload_offset: Cell::new(0),
            stream_type: RefCell::new(None),
            stream_time: Cell::new(0.0),
            queued_seek_time: Cell::new(None),
            audio_stream: RefCell::new(None),
            aac_config: RefCell::new(None),
            sound_instance: Cell::new(None),
        }
    }
}

impl<'gc> NetStream<'gc> {
    /// Create a `NetStream` for use in AVM1.
    pub fn new_avm1(gc_context: &Mutation<'gc>, avm_object: Avm1Object<'gc>) -> Self {
        Self::new(gc_context, Some(NetStreamKind::Avm1(avm_object)))
    }

    /// Create a `NetStream` for use in AVM2. The caller is expected to initialize
    /// the AVM side of the `NetStream` later, by using `set_avm2_object`.
    pub fn new_avm2(gc_context: &Mutation<'gc>) -> Self {
        Self::new(gc_context, None)
    }

    fn new(gc_context: &Mutation<'gc>, avm_object: Option<NetStreamKind<'gc>>) -> Self {
        // IMPORTANT: When adding new fields consider if they need to be
        //     added here or to NetStreamSource.
        Self(Gc::new(
            gc_context,
            NetStreamData {
                source: Lock::new(Gc::new(gc_context, Default::default())),
                buffer_time: Cell::new(0.1),
                last_decoded_bitmap: RefCell::new(None),
                avm_object: Lock::new(avm_object),
                avm2_client: Lock::new(None),
                url: RefCell::new(None),
                attached_to: Lock::new(None),
                playing: Cell::new(false),
                data_generation_mode: Cell::new(false),
                pending_append_bytes: RefCell::new(Vec::new()),
                handling_script_data: Cell::new(false),
                appended_flv: RefCell::new(AppendedFlv::new()),
                appending: Cell::new(false),
                out_of_data: Cell::new(false),
                buffer_full_pending: Cell::new(false),
                seeking_appended: Cell::new(false),
            },
        ))
    }

    fn source(self) -> Gc<'gc, NetStreamSource> {
        self.0.source.get()
    }

    pub fn set_client(self, gc_context: &Mutation<'gc>, new_client: Avm2Object<'gc>) {
        unlock!(Gc::write(gc_context, self.0), NetStreamData, avm2_client).set(Some(new_client));
    }

    pub fn client(self) -> Option<Avm2Object<'gc>> {
        self.0.avm2_client.get()
    }

    pub fn set_avm2_object(self, gc_context: &Mutation<'gc>, object: NetStreamObject<'gc>) {
        let write = Gc::write(gc_context, self.0);
        unlock!(write, NetStreamData, avm_object).set(Some(NetStreamKind::Avm2(object)));
    }

    fn set_attached_to(self, gc_context: &Mutation<'gc>, attached_to: Option<MovieClip<'gc>>) {
        unlock!(Gc::write(gc_context, self.0), NetStreamData, attached_to).set(attached_to);
    }

    /// Reset the `NetStream` buffer to accept new source data.
    ///
    /// This must be done once per source change and should ideally be done
    /// immediately before the first `load_buffer` call for a particular source
    /// file.
    ///
    /// Externally visible AVM state must not be reinitialized here - i.e. the
    /// AS3 `client` doesn't go away because you played a new video file.
    fn reset_buffer(self, context: &mut UpdateContext<'gc>) {
        if let Some(instance) = self.source().sound_instance.get() {
            // We stop the sound twice because sounds may have either been
            // played through the audio manager or through the backend directly
            // depending on the attachment state at the time of first audio
            // playback.
            context.audio.stop_sound(instance);
            context.audio_manager.stop_sound(context.audio, instance);
        }

        unlock!(Gc::write(context.gc(), self.0), NetStreamData, source)
            .set(Gc::new(context.gc(), Default::default()));
    }

    /// Set the total number of bytes expected to be downloaded.
    pub fn set_expected_length(self, expected: usize) {
        let source = self.source();
        let mut buffer = source.buffer.borrow_mut();
        let len = buffer.len();

        // The subtract is to avoid reserving space for already-downloaded data.
        if expected > len {
            buffer.reserve(expected - len);
        }

        source.expected_length.set(Some(expected));
    }

    pub fn append_bytes(self, context: &mut UpdateContext<'gc>, data: &[u8]) {
        if !self.0.appending.get() {
            self.start_appending(context);
        }

        let mut tags = Vec::new();
        let first_timestamp = self.0.appended_flv.borrow_mut().append(data, &mut tags);
        if tags.is_empty() {
            return;
        }
        if let Some(timestamp) = first_timestamp
            && self.0.seeking_appended.replace(false)
        {
            self.source().stream_time.set(timestamp.into());
        }

        if self.0.handling_script_data.get() {
            self.0.pending_append_bytes.borrow_mut().append(&mut tags);
        } else {
            self.drop_played_appended_data(context);
            self.load_buffer(context, &mut tags);
        }
    }

    /// Above this many bytes, appended data that has been played is dropped
    /// before more is appended.
    const MAX_PLAYED_APPENDED_LEN: usize = 4 << 20;

    /// Drops the appended data that has been played, once there is a lot of
    /// it, so that the buffer doesn't grow without bound: content may append
    /// to a stream indefinitely, such as a loop that appends its video again
    /// and again after `RESET_BEGIN`.
    fn drop_played_appended_data(self, context: &mut UpdateContext<'gc>) {
        const HEADER_LEN: usize = AppendedFlv::HEADER.len();

        let source = self.source();
        let offset = source.offset.get();
        if !self.0.appending.get()
            || source.stream_type.borrow().is_none()
            || offset < HEADER_LEN + Self::MAX_PLAYED_APPENDED_LEN
        {
            return;
        }

        // The audio stream refers to the buffer, so it starts anew from the
        // data that is kept, cutting off what the audio backend has buffered.
        if let Some(instance) = source.sound_instance.take() {
            context.audio.stop_sound(instance);
            context.audio_manager.stop_sound(context.audio, instance);
        }
        source.audio_stream.replace(None);

        let old_buffer = source.buffer.borrow().to_full_slice();
        let mut kept = AppendedFlv::HEADER.to_vec();
        kept.extend_from_slice(&old_buffer.data()[offset..]);
        source.buffer.replace(Buffer::from(kept));
        source.offset.set(HEADER_LEN);
        source
            .preload_offset
            .set(HEADER_LEN + source.preload_offset.get().saturating_sub(offset));
    }

    /// Replaces the buffer with an empty FLV, for appended data.
    fn start_appending(self, context: &mut UpdateContext<'gc>) {
        self.reset_buffer(context);
        // Data appended by a script callback belongs to the old buffer.
        self.0.pending_append_bytes.borrow_mut().clear();
        self.source()
            .buffer
            .borrow_mut()
            .append(&mut AppendedFlv::HEADER.to_vec());
        self.0.appending.set(true);
        self.0.out_of_data.set(true);
        // Seeking right after appending drops the data before it reports
        // `NetStream.Buffer.Full`.
        self.0.buffer_full_pending.set(false);
    }

    /// Implements `appendBytesAction()`.
    ///
    /// Returns `false` for an unknown action.
    pub fn append_bytes_action(self, context: &mut UpdateContext<'gc>, action: &str) -> bool {
        match action {
            "resetBegin" => self.0.appended_flv.borrow_mut().reset(true),
            // Without a `seek()` first, Flash goes on after the data already
            // appended, as after `RESET_BEGIN`.
            "resetSeek" => self
                .0
                .appended_flv
                .borrow_mut()
                .reset(!self.0.seeking_appended.get()),
            "endSequence" => self.trigger_status_event(
                context,
                [("code", "NetStream.Buffer.Flush"), ("level", "status")],
            ),
            _ => return false,
        }
        true
    }

    /// Append data to the `NetStream`'s current internal buffer.
    ///
    /// If you are loading data from a new source, you must first initialize
    /// the buffer, otherwise existing buffer contents will remain and be
    /// incorrectly parsed.
    ///
    /// Buffer loading can be done in chunks but must be done in such a way
    /// that all data is appended in the correct order and that data from
    /// separate streams is not mixed together.
    pub fn load_buffer(self, context: &mut UpdateContext<'gc>, data: &mut Vec<u8>) {
        self.source().buffer.borrow_mut().append(data);

        StreamManager::activate(context, self);

        if self.0.data_generation_mode.get() {
            // Appended data fills the buffer when it had run out, which
            // Flash reports as the stream plays on, not from `appendBytes`:
            // content may set up its listener after appending.
            if self.0.out_of_data.replace(false) {
                self.0.buffer_full_pending.set(true);
            }
            return;
        }

        // NOTE: The onMetaData event triggers before this event in Flash due to its streaming behavior.
        self.trigger_status_event(
            context,
            [("code", "NetStream.Buffer.Full"), ("level", "status")],
        );
    }

    /// Indicate that the buffer has finished loading and that no further data
    /// is expected to be downloaded to it.
    pub fn finish_buffer(self) {
        self.source().expected_length.set(None);
    }

    pub fn report_error(self, _error: Error) {
        // TODO: Report an `asyncError` to AVM1 or 2.
    }

    pub fn bytes_loaded(self) -> usize {
        // Flash reports no bytes for appended data.
        if self.0.data_generation_mode.get() {
            return 0;
        }
        self.source().buffer.borrow().len()
    }

    pub fn bytes_total(self) -> usize {
        if self.0.data_generation_mode.get() {
            return 0;
        }
        let source = self.source();
        let buflen = source.buffer.borrow().len();
        std::cmp::max(source.expected_length.get().unwrap_or(buflen), buflen)
    }

    pub fn time(self) -> f64 {
        self.source().stream_time.get()
    }

    pub fn buffer_time(self) -> f64 {
        self.0.buffer_time.get()
    }

    pub fn set_buffer_time(self, buffer_time: f64) {
        self.0.buffer_time.set(buffer_time);
    }

    /// Queue a seek to be executed on the next frame tick.
    ///
    /// `offset` is in milliseconds.
    pub fn seek(self, context: &mut UpdateContext<'gc>, offset: f64, notify: bool) {
        if self.0.data_generation_mode.get() && self.0.appending.get() {
            // Seeking drops the appended data. Playback goes on with the next
            // data appended, at its own timestamps.
            // The FLV state carries over, with the decoders' configuration,
            // as data after `RESET_SEEK` may not repeat it.
            let stream_time = self.source().stream_time.get();
            // Copied, not taken: a script callback may be processing the old
            // source still.
            let stream_type = self.source().stream_type.borrow().clone();
            let aac_config = self.source().aac_config.borrow().clone();
            self.start_appending(context);
            let source = self.source();
            source.stream_time.set(stream_time);
            source.aac_config.replace(aac_config);
            if stream_type.is_some() {
                source.stream_type.replace(stream_type);
                source.offset.set(AppendedFlv::HEADER.len());
                source.preload_offset.set(AppendedFlv::HEADER.len());
            }
            let mut appended_flv = self.0.appended_flv.borrow_mut();
            appended_flv.pending.clear();
            appended_flv.timestamp_offset = 0;
            appended_flv.last_timestamp = 0;
            drop(appended_flv);
            self.0.seeking_appended.set(true);
        }

        self.source().queued_seek_time.set(Some(offset));
        StreamManager::activate(context, self);

        if notify {
            let trigger = format!("Start Seeking {}", offset as u64);
            self.trigger_status_event(
                context,
                [
                    ("description", trigger.as_str()),
                    ("level", "status"),
                    ("code", "NetStream.SeekStart.Notify"),
                ],
            );
        }
    }

    /// Seek to a new position in the stream.
    ///
    /// All existing audio will be paused. The stream offset will be snapped to
    /// either the prior or next keyframe depending on seek direction. If the
    /// stream is playing then new tag processing will occur when the stream
    /// ticks next.
    ///
    /// This always does an in-buffer seek. Seek-driven requests are not
    /// currently supported. When progressive download is implemented this seek
    /// algorithm will need to detect out-of-buffer seeks and trigger fresh
    /// downloads.
    ///
    /// `offset` is in milliseconds.
    ///
    /// This function should be run during stream ticks and *not* called by AVM
    /// code to service seek requests.
    pub fn execute_seek(self, context: &mut UpdateContext<'gc>, offset: f64) {
        self.trigger_status_event(
            context,
            [("code", "NetStream.Seek.Notify"), ("level", "status")],
        );

        if self.0.data_generation_mode.get() && self.0.appending.get() {
            // `seek` already dropped the appended data.
            return;
        }

        let source = self.source();

        // Ensure the container stream type is known before continuing.
        if source.stream_type.borrow().is_none() && !self.sniff_stream_type(context) {
            return;
        }

        if source.stream_time.get() == offset {
            //Don't do anything for no-op seeks.
            return;
        }

        if let Some(sound) = source.sound_instance.get() {
            context.stop_sound(sound);
            context.audio.stop_sound(sound);

            source.sound_instance.set(None);
            source.audio_stream.replace(None);
        }

        if matches!(
            &*source.stream_type.borrow(),
            Some(NetStreamType::Flv { .. })
        ) {
            let slice = source.buffer.borrow().to_full_slice();
            let buffer = slice.data();
            let mut reader = FlvReader::from_parts(&buffer, source.offset.get());
            let skipping_back = source.stream_time.get() > offset;

            loop {
                if skipping_back {
                    let res = FlvTag::skip_back(&mut reader);
                    if matches!(res, Err(FlvError::EndOfData)) {
                        //At start of video, can't skip further back
                        break;
                    }

                    if let Err(e) = res {
                        tracing::error!("FLV tag parsing failed during seek backward: {}", e);
                        break;
                    }
                }

                let old_position = reader
                    .stream_position()
                    .expect("valid stream position when seeking");

                let tag = FlvTag::parse(&mut reader);
                if matches!(tag, Err(FlvError::EndOfData)) {
                    //At end of video, can't skip further forward
                    break;
                }

                if let Err(e) = tag {
                    tracing::error!("FLV tag parsing failed during seek forward: {}", e);
                    break;
                }

                if skipping_back {
                    //Tag position won't actually move backwards if we don't do this.
                    reader
                        .seek(SeekFrom::Start(old_position))
                        .expect("valid backseek position");
                }

                let tag = tag.unwrap();
                let stream_time = tag.timestamp as f64;
                source.stream_time.set(stream_time);

                if skipping_back && stream_time > offset || !skipping_back && stream_time < offset {
                    continue;
                }

                match tag.data {
                    FlvTagData::Video(FlvVideoData {
                        frame_type: FlvFrameType::Keyframe,
                        ..
                    }) => {
                        // If we don't backseek when we find the keyframe,
                        // we will miss the keyframe.
                        reader
                            .seek(SeekFrom::Start(old_position))
                            .expect("valid backseek position");

                        break;
                    }
                    _ => continue,
                }
            }

            let offset = reader
                .stream_position()
                .expect("FLV reader stream position") as usize;
            source.offset.set(offset);
        }

        if let Some(NetStreamKind::Avm2(_)) = self.0.avm_object.get() {
            self.trigger_status_event(
                context,
                [
                    ("description", "Seek Complete -1"),
                    ("level", "status"),
                    ("code", "NetStream.Seek.Complete"),
                ],
            );
        }
    }

    pub fn is_data_generation_mode(self) -> bool {
        self.0.data_generation_mode.get()
    }

    /// Start playing media from this NetStream.
    ///
    /// If `name` is specified, this will also trigger streaming download of
    /// the given resource. Otherwise, the stream will play whatever data is
    /// available in the buffer.
    pub fn play(self, context: &mut UpdateContext<'gc>, name: Option<AvmString<'gc>>) {
        // Only AVM2 has `appendBytes()`; AVM1's `play()` resumes the stream.
        let is_avm2 = matches!(self.0.avm_object.get(), Some(NetStreamKind::Avm2(_)));
        self.0.data_generation_mode.set(name.is_none() && is_avm2);
        self.0.appending.set(false);
        self.0.seeking_appended.set(false);
        self.0.appended_flv.replace(AppendedFlv::new());
        // Nothing of the previous playback carries over.
        self.0.pending_append_bytes.borrow_mut().clear();
        self.0.buffer_full_pending.set(false);
        if let Some(name) = name {
            let request = if let Ok(stream_url) = Url::parse(context.root_swf.url())
                .and_then(|url| url.join(name.to_string().as_str()))
            {
                Request::get(stream_url.to_string())
            } else {
                Request::get(name.to_string())
            };
            self.0.url.replace(Some(request.url().to_string()));
            self.source().preload_offset.set(0);
            self.reset_buffer(context);

            let future = crate::loader::load_netstream(context, self, request);

            context.navigator.spawn_future(future);
        }

        self.0.playing.set(true);
        StreamManager::activate(context, self);

        // Flash doesn't report data generation mode as starting to play.
        if !self.0.data_generation_mode.get() {
            self.trigger_status_event(
                context,
                [("code", "NetStream.Play.Start"), ("level", "status")],
            );
        }
    }

    /// Pause stream playback.
    pub fn pause(self, context: &mut UpdateContext<'gc>, notify: bool) {
        // NOTE: We do not deactivate the stream here as there may be other
        // work to be done at tick time.
        self.0.playing.set(false);

        if notify {
            self.trigger_status_event(
                context,
                [
                    ("description", "Pausing"),
                    ("level", "status"),
                    ("code", "NetStream.Pause.Notify"),
                ],
            );
        }
    }

    /// Resume stream playback.
    pub fn resume(self, context: &mut UpdateContext<'gc>) {
        self.0.playing.set(true);
        StreamManager::activate(context, self);
    }

    /// Resume stream playback if paused, pause otherwise.
    pub fn toggle_paused(self, context: &mut UpdateContext<'gc>) {
        self.0.playing.set(!self.0.playing.get());

        if self.0.playing.get() {
            StreamManager::activate(context, self);
        }
    }

    /// Indicates that this `NetStream`'s audio was detached from a `MovieClip` (AVM1)
    pub fn was_detached(self, context: &mut UpdateContext<'gc>) {
        let source = self.source();
        if let Some(sound_instance) = source.sound_instance.get() {
            context
                .audio_manager
                .stop_sound(context.audio, sound_instance);
        }

        source.audio_stream.replace(None);
        self.set_attached_to(context.gc(), None);
    }

    /// Indicates that this `NetStream`'s audio was attached to a `MovieClip` (AVM1)
    pub fn was_attached(self, context: &mut UpdateContext<'gc>, clip: MovieClip<'gc>) {
        let source = self.source();

        // A `NetStream` cannot be attached to two `MovieClip`s at once.
        // Stop the old sound; the new one will stream at the next tag read.
        // TODO: Change this to have `audio_manager` just switch the sound
        // transforms around
        if let Some(sound_instance) = source.sound_instance.get() {
            context
                .audio_manager
                .stop_sound(context.audio, sound_instance);
        }

        source.audio_stream.replace(None);
        self.set_attached_to(context.gc(), Some(clip));
    }

    /// The SWF sound format of an FLV audio tag.
    fn flv_sound_format(audio_data: &FlvAudioData<'_>) -> Result<SoundFormat, NetstreamError> {
        Ok(SoundFormat {
            compression: match audio_data.format {
                FlvSoundFormat::LinearPCMPlatformEndian => {
                    AudioCompression::UncompressedUnknownEndian
                }
                FlvSoundFormat::Adpcm => AudioCompression::Adpcm,
                FlvSoundFormat::MP3 => AudioCompression::Mp3,
                FlvSoundFormat::LinearPCMLittleEndian => AudioCompression::Uncompressed,
                FlvSoundFormat::Nellymoser16kHz => AudioCompression::Nellymoser16Khz,
                FlvSoundFormat::Nellymoser8kHz => AudioCompression::Nellymoser8Khz,
                FlvSoundFormat::Nellymoser => AudioCompression::Nellymoser,
                FlvSoundFormat::G711ALawPCM => AudioCompression::G711ALawPCM,
                FlvSoundFormat::G711MuLawPCM => AudioCompression::G711MuLawPCM,
                FlvSoundFormat::Aac => AudioCompression::Aac,
                FlvSoundFormat::Speex => AudioCompression::Speex,
                FlvSoundFormat::MP38kHz => AudioCompression::Mp3,
                FlvSoundFormat::DeviceSpecific => return Err(NetstreamError::UnknownCodec),
            },
            sample_rate: match (audio_data.format, audio_data.rate) {
                (FlvSoundFormat::G711ALawPCM, _)
                | (FlvSoundFormat::G711MuLawPCM, _)
                | (FlvSoundFormat::MP38kHz, _) => 8_000,
                (_, FlvSoundRate::R5_500) => 5_500,
                (_, FlvSoundRate::R11_000) => 11_000,
                (_, FlvSoundRate::R22_000) => 22_000,
                (_, FlvSoundRate::R44_000) => 44_000,
            },
            is_stereo: match audio_data.sound_type {
                FlvSoundType::Mono => false,
                FlvSoundType::Stereo => true,
            },
            is_16_bit: match audio_data.size {
                FlvSoundSize::Bits8 => false,
                FlvSoundSize::Bits16 => true,
            },
        })
    }

    /// Process a parsed FLV audio tag.
    ///
    /// `write` must be an active borrow of the current `NetStream`. `slice`
    /// must reference the underlying backing buffer.
    fn flv_audio_tag(
        self,
        context: &mut UpdateContext<'gc>,
        slice: &Slice,
        audio_data: FlvAudioData<'_>,
        is_lookahead_tag: bool,
    ) -> Result<(), NetstreamError> {
        let swf_format = Self::flv_sound_format(&audio_data)?;
        let source = self.source();
        let is_new_format = matches!(
            &*source.audio_stream.borrow(),
            Some((_, info)) if info.stream_format != swf_format
        );
        if is_new_format {
            // A file appended after `RESET_BEGIN` can have another audio
            // format, which needs a decoder of its own. Switch once its first
            // tag is due, so the current sound plays up to it.
            if is_lookahead_tag {
                return Ok(());
            }
            if let Some(instance) = source.sound_instance.take() {
                context.audio.stop_sound(instance);
                context.audio_manager.stop_sound(context.audio, instance);
            }
            source.audio_stream.replace(None);
        }

        let is_aac_sequence_header =
            matches!(audio_data.data, FlvAudioDataType::AacSequenceHeader(_));
        let data = match audio_data.data {
            FlvAudioDataType::Raw(data)
            | FlvAudioDataType::AacSequenceHeader(data)
            | FlvAudioDataType::AacRaw(data) => slice.to_subslice(data),
        };
        let audio_stream = &mut *source.audio_stream.borrow_mut();
        let (substream, sound_stream_info) = match audio_stream {
            Some(audio_stream) => audio_stream,
            audio_stream => {
                // None
                let substream = Substream::new(slice.buffer().clone());
                let extra_data = if swf_format.compression == AudioCompression::Aac {
                    source.aac_config.borrow().clone()
                } else {
                    None
                };
                let sound_stream_head = SoundStreamInfo {
                    wrapping: SoundStreamWrapping::Unwrapped,
                    stream_format: swf_format,
                    num_samples_per_block: 0,
                    latency_seek: 0,
                    extra_data,
                };

                audio_stream.insert((substream, sound_stream_head))
            }
        };

        // An AAC sequence header carries the decoder configuration
        // (`AudioSpecificConfig`), not playable audio. We demux it out-of-band
        // into the stream info instead of appending it to the audio substream,
        // so the decoder only ever sees raw AAC access units and never has to
        // deal with FLV's packet framing.
        //
        // The decoder reads this config once, when it's constructed. A re-sent
        // header just refreshes the field (a no-op for the identical configs FLV
        // actually uses); a genuine mid-stream config *change* is not supported,
        // but doesn't occur in practice.
        if is_aac_sequence_header {
            let aac_config: Box<[u8]> = (*data.data()).into();
            source.aac_config.replace(Some(aac_config.clone()));
            sound_stream_info.extra_data = Some(aac_config);
            return Ok(());
        }

        if substream
            .last_chunk()
            .map(|lc| lc.end() > data.start())
            .unwrap_or(false)
        {
            // Reject repeats of existing tags.
            // We need to do this because of lookahead - we will
            // encounter the same audio tag multiple times as we buffer
            // a few ahead for the audio backend.
            // This assumes that tags are processed in-order - which
            // should always be the case. Seeks should cancel the audio
            // stream before processing new tags.
            return Ok(());
        }

        Ok(substream.append(data)?)
    }

    /// Determine if the given sound is currently playing.
    fn sound_currently_playing(
        context: &mut UpdateContext<'gc>,
        sound: Option<SoundInstanceHandle>,
    ) -> bool {
        sound
            .map(|si| context.audio.is_sound_playing(si))
            .unwrap_or(false)
    }

    /// Clean up after a sound instance that has finished playing.
    ///
    /// Generally speaking, streams are only to be used once. However, the
    /// audio backend will only retain information about sounds that are
    /// currently playing, so if the sound has finished since the last tick, we
    /// need to restart it.
    ///
    /// Intended to be called at the start of tag processing, before any new
    /// audio data has been streamed.
    fn cleanup_sound_stream(self, context: &mut UpdateContext<'gc>) {
        let source = self.source();
        if !Self::sound_currently_playing(context, source.sound_instance.get()) {
            source.audio_stream.replace(None);
            source.sound_instance.set(None);
        }
    }

    /// Ensure that if we have queued up audio into a sound stream, that said
    /// stream gets sent over to the audio backend.
    ///
    /// Intended to be called at the end of tag processing. Audio processing
    /// should occur only after a minimum number of tags have been processed to
    /// avoid audio underruns.
    fn commit_sound_stream(self, context: &mut UpdateContext<'gc>) -> Result<(), NetstreamError> {
        let source = self.source();
        if !Self::sound_currently_playing(context, source.sound_instance.get())
            && let Some((substream, sound_stream_head)) = &mut *source.audio_stream.borrow_mut()
        {
            let sound_instance = if let Some(mc) = self.0.attached_to.get() {
                context.audio_manager.start_substream(
                    context.audio,
                    substream.clone(),
                    mc,
                    sound_stream_head,
                )?
            } else {
                context
                    .audio
                    .start_substream(substream.clone(), sound_stream_head)?
            };
            source.sound_instance.set(Some(sound_instance));
        }

        Ok(())
    }

    /// Attempt to sniff the stream type from data in the buffer.
    ///
    /// Returns true if the stream type was successfully sniffed. False
    /// indicates that there is either not enough data in the buffer, or the
    /// data is of an unrecognized format. This should be used as a signal to
    /// stop stream processing until new data has been retrieved.
    pub fn sniff_stream_type(self, context: &mut UpdateContext<'gc>) -> bool {
        let source = self.source();
        let slice = source.buffer.borrow().to_full_slice();
        let buffer = slice.data();

        // A nonzero preload offset indicates that we tried and failed to
        // sniff the container format, so in that case do not process the
        // stream anymore.
        if source.preload_offset.get() > 0 {
            return false;
        }

        match buffer.get(0..3) {
            Some([0x46, 0x4C, 0x56]) => {
                let mut reader = FlvReader::from_parts(&buffer, source.offset.get());
                match FlvHeader::parse(&mut reader) {
                    Ok(header) => {
                        source.offset.set(reader.into_parts().1);
                        source.preload_offset.set(source.offset.get());
                        source.stream_type.replace(Some(NetStreamType::Flv {
                            header,
                            video_stream: None,
                            frame_id: 0,
                        }));
                        true
                    }
                    Err(FlvError::EndOfData) => false,
                    Err(e) => {
                        //TODO: Fire an error event to AS & stop playing too
                        tracing::error!("FLV header parsing failed: {}", e);
                        source.preload_offset.set(3);
                        false
                    }
                }
            }
            Some(magic) => {
                //Unrecognized signature
                //TODO: Fire an error event to AS & stop playing too
                tracing::error!("Unrecognized file signature: {:?}", magic);
                source.preload_offset.set(3);
                if let Some(url) = &*self.0.url.borrow() {
                    if url.is_empty() {
                        return false;
                    }
                    let parsed_url = match context.navigator.resolve_url(url) {
                        Ok(parsed_url) => parsed_url,
                        Err(e) => {
                            tracing::error!(
                                "Could not parse URL because of {}, the corrupt URL was: {}",
                                e,
                                url
                            );
                            return false;
                        }
                    };
                    context.ui.display_unsupported_video(parsed_url);
                }
                false
            }
            None => false, //Data not yet loaded
        }
    }

    /// Process a parsed FLV video tag.
    ///
    /// `write` must be an active borrow of the current `NetStream`. `slice`
    /// must reference the underlying backing buffer.
    ///
    /// `tag_needs_preloading` indicates that this video tag has not been
    /// encountered before.
    /// Registers a video stream as the current one of `source`, replacing
    /// any other. `source` is passed in, as a script callback may have
    /// replaced the stream's source with a new one by now.
    fn register_video_stream(
        source: &NetStreamSource,
        context: &mut UpdateContext<'gc>,
        num_frames: u32,
        size: (u16, u16),
        codec: VideoCodec,
    ) -> Option<VideoStreamHandle> {
        match context.video.register_video_stream(
            num_frames,
            size,
            codec,
            VideoDeblocking::UseVideoPacketValue,
        ) {
            Ok(handle) => {
                match &mut *source.stream_type.borrow_mut() {
                    Some(NetStreamType::Flv { video_stream, .. }) => {
                        *video_stream = Some((handle, codec))
                    }
                    _ => unreachable!(),
                }
                Some(handle)
            }
            Err(e) => {
                tracing::error!("Got error when registering FLV video stream: {}", e);
                None
            }
        }
    }

    fn flv_video_tag(
        self,
        context: &mut UpdateContext<'gc>,
        slice: &Slice,
        video_data: FlvVideoData<'_>,
        tag_needs_preloading: bool,
    ) {
        let source = self.source();
        let (video_handle, frame_id) = match *source.stream_type.borrow() {
            Some(NetStreamType::Flv {
                video_stream,
                frame_id,
                ..
            }) => (video_stream, frame_id),
            _ => unreachable!(),
        };
        let codec = VideoCodec::from_u8(video_data.codec_id as u8);
        let buffer = slice.data();

        // Appended files can each use another codec, which needs another stream.
        let video_handle = video_handle
            .filter(|&(_, stream_codec)| Some(stream_codec) == codec)
            .map(|(handle, _)| handle);
        let video_handle = match (video_handle, codec, &video_data.data) {
            (None, Some(codec), FlvVideoPacket::AvcSequenceHeader(_)) => {
                Self::register_video_stream(&source, context, 1, (8, 8), codec)
            }
            _ => video_handle,
        };

        match (video_handle, codec, video_data.data) {
            (
                maybe_video_handle,
                Some(codec),
                FlvVideoPacket::Data(mut data)
                | FlvVideoPacket::Vp6Data {
                    hadjust: _,
                    vadjust: _,
                    mut data,
                },
            ) => {
                //Some movies don't actually have metadata, so let's register a
                //dummy stream just in case. All the actual data in the registration
                //is lies, of course.
                let video_handle = match maybe_video_handle {
                    Some(stream) => stream,
                    None => match Self::register_video_stream(&source, context, 1, (8, 8), codec) {
                        Some(new_handle) => new_handle,
                        None => return, //TODO: This originally breaks and halts tag processing
                    },
                };

                if codec == VideoCodec::ScreenVideo || codec == VideoCodec::ScreenVideoV2 {
                    // ScreenVideo streams consider the FLV
                    // video data byte to be integral to their
                    // own bitstream.
                    let offset = data.as_ptr() as usize - buffer.as_ptr() as usize;
                    let len = data.len();
                    data = buffer
                        .get(offset - 1..offset + len)
                        .expect("screenvideo flvs have video data bytes");
                }

                // NOTE: Currently, no implementation of the decoder backend actually requires
                if tag_needs_preloading {
                    let encoded_frame = EncodedFrame {
                        codec,
                        data, //TODO: ScreenVideo's decoder wants the FLV header bytes
                        frame_id,
                    };

                    if let Err(e) = context
                        .video
                        .preload_video_stream_frame(video_handle, encoded_frame)
                    {
                        tracing::error!("Preloading video frame {} failed: {}", frame_id, e);
                    }
                }

                let encoded_frame = EncodedFrame {
                    codec,
                    data, //TODO: ScreenVideo's decoder wants the FLV header bytes
                    frame_id,
                };

                match context.video.decode_video_stream_frame(
                    video_handle,
                    encoded_frame,
                    context.renderer,
                ) {
                    Ok(bitmap_info) => {
                        self.0.last_decoded_bitmap.replace(Some(bitmap_info));
                        if let Some(mc) = self.0.attached_to.get() {
                            mc.invalidate_cached_bitmap();
                            *context.needs_render = true;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Decoding video frame {} failed: {}", frame_id, e);
                    }
                }
            }
            (_, _, FlvVideoPacket::CommandFrame(_command)) => {
                tracing::warn!("Stub: FLV command frame processing")
            }
            (Some(video_handle), _, FlvVideoPacket::AvcSequenceHeader(data)) => {
                match context
                    .video
                    .configure_video_stream_decoder(video_handle, data)
                {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Configuring video decoder {} failed: {}", frame_id, e);
                    }
                }
            }
            (
                Some(video_handle),
                Some(codec),
                FlvVideoPacket::AvcNalu {
                    composition_time_offset: _,
                    data,
                },
            ) => {
                let encoded_frame = EncodedFrame {
                    codec,
                    data,
                    frame_id,
                };

                match context.video.decode_video_stream_frame(
                    video_handle,
                    encoded_frame,
                    context.renderer,
                ) {
                    Ok(bitmap_info) => {
                        self.0.last_decoded_bitmap.replace(Some(bitmap_info));
                        if let Some(mc) = self.0.attached_to.get() {
                            mc.invalidate_cached_bitmap();
                            *context.needs_render = true;
                        }
                    }
                    Err(e) => {
                        tracing::error!("Decoding video frame {} failed: {}", frame_id, e);
                    }
                }
            }
            (_, _, FlvVideoPacket::AvcEndOfSequence) => {
                tracing::warn!("Stub: FLV AVC/H.264 End of Sequence processing")
            }
            (_, None, _) => {
                tracing::error!(
                    "FLV video tag has invalid codec id {}",
                    video_data.codec_id as u8
                )
            }
            (None, _, _) => {
                tracing::error!("No video handle")
            }
        }

        match &mut *source.stream_type.borrow_mut() {
            Some(NetStreamType::Flv { frame_id, .. }) => *frame_id += 1,
            _ => unreachable!(),
        };
    }

    /// Process a parsed FLV script tag.
    ///
    /// This function attempts to borrow the current `NetStream`, you must drop
    /// any existing borrows and pick them back up when you're done.
    ///
    /// `tag_needs_preloading` indicates that this script tag has not been
    /// encountered before.
    fn flv_script_tag(
        self,
        context: &mut UpdateContext<'gc>,
        script_data: FlvScriptData<'_>,
        tag_needs_preloading: bool,
    ) {
        let source = self.source();
        let stream_codec = match &*source.stream_type.borrow() {
            Some(NetStreamType::Flv { video_stream, .. }) => video_stream.map(|(_, codec)| codec),
            _ => unreachable!(),
        };

        let mut width = None;
        let mut height = None;
        let mut video_codec_id = None;
        let mut frame_rate = None;
        let mut duration = None;

        for var in &script_data.0 {
            if var.name == b"onMetaData" {
                match &var.data {
                    FlvValue::Object(subvars) | FlvValue::EcmaArray(subvars) => {
                        for subvar in subvars {
                            match (subvar.name, &subvar.data) {
                                (b"width", FlvValue::Number(val)) => width = Some(*val),
                                (b"height", FlvValue::Number(val)) => height = Some(*val),
                                (b"videocodecid", FlvValue::Number(val)) => {
                                    video_codec_id = Some(*val)
                                }
                                (b"framerate", FlvValue::Number(val)) => frame_rate = Some(*val),
                                (b"duration", FlvValue::Number(val)) => duration = Some(*val),
                                _ => {}
                            }
                        }
                    }
                    _ => tracing::error!("Invalid FLV metadata tag!"),
                }
            }
        }

        // Before the script callbacks: one may seek in data generation mode,
        // which carries the video stream over to the data appended next.
        if tag_needs_preloading
            && let (
                Some(width),
                Some(height),
                Some(video_codec_id),
                Some(frame_rate),
                Some(duration),
            ) = (width, height, video_codec_id, frame_rate, duration)
        {
            let num_frames = frame_rate * duration;
            match VideoCodec::from_u8(video_codec_id as u8) {
                // Keep the stream, e.g. for a file appended again.
                Some(video_codec) if Some(video_codec) == stream_codec => {}
                Some(video_codec) => {
                    Self::register_video_stream(
                        &source,
                        context,
                        num_frames as u32,
                        (width as u16, height as u16),
                        video_codec,
                    );
                }
                None => {
                    tracing::error!("FLV video stream has invalid codec ID {}", video_codec_id)
                }
            }
        }

        for var in script_data.0 {
            let avm_object = self.0.avm_object.get();
            // This is necessary because the script callback functions can call back into
            // these methods, (e.g. NetStream::play), so we need to avoid holding a borrow
            // while the script data is being handled.
            self.0.handling_script_data.set(true);
            let _ = self.handle_script_data(avm_object, context, var.name, var.data);
            self.0.handling_script_data.set(false);
            // Any errors while trying to lookup or call AVM2 properties are silently swallowed.
        }
    }

    /// Process stream data.
    ///
    /// `dt` is the elapsed time since the last tick.
    pub fn tick(self, context: &mut UpdateContext<'gc>, dt: FloatDuration) {
        let source = self.source();
        let seek_offset = source.queued_seek_time.take();
        if let Some(offset) = seek_offset {
            self.execute_seek(context, offset);
        }

        // Paused streams deactivate themselves after seek processing.
        if !self.0.playing.get() {
            StreamManager::deactivate(context, self);
            return;
        }

        // Ensure the container stream type is known before continuing.
        if source.stream_type.borrow().is_none() && !self.sniff_stream_type(context) {
            return;
        }

        self.cleanup_sound_stream(context);
        let slice = source.buffer.borrow().to_full_slice();
        let buffer = slice.data();

        let max_time = source.stream_time.get() + dt.as_millis();
        let mut last_tag_time = None;
        let mut buffer_underrun = false;
        let mut error = false;
        let mut max_lookahead_audio_tags = 5;
        let mut is_lookahead_tag = false;

        // At this point we should know our stream type.
        if matches!(
            &*source.stream_type.borrow(),
            Some(NetStreamType::Flv { .. })
        ) {
            let mut reader = FlvReader::from_parts(&buffer, source.offset.get());

            loop {
                let tag = FlvTag::parse(&mut reader);
                if let Err(e) = tag {
                    // `is_lookahead_tag` gets set once we start reading tags
                    // after the end & won't ever be set back. We don't want
                    // error states or playback ending to trip until we run
                    // those tags "for realsies"
                    if !is_lookahead_tag && matches!(e, FlvError::EndOfData) {
                        buffer_underrun = true;
                    } else if !is_lookahead_tag {
                        //Corrupt tag or out of data
                        tracing::error!("FLV tag parsing failed: {}", e);
                        error = true;
                    }

                    break;
                }

                let tag = tag.expect("valid tag");
                is_lookahead_tag = tag.timestamp as f64 >= max_time; //FLV timestamps are also ms
                if is_lookahead_tag && max_lookahead_audio_tags == 0 {
                    break;
                }

                let tag_needs_preloading = reader.stream_position().expect("valid position")
                    as usize
                    >= source.preload_offset.get();

                match tag.data {
                    FlvTagData::Audio(audio_data) => {
                        if is_lookahead_tag {
                            max_lookahead_audio_tags -= 1;
                        }

                        if let Err(e) =
                            self.flv_audio_tag(context, &slice, audio_data, is_lookahead_tag)
                        {
                            //TODO: Fire an error event at AS.
                            tracing::error!("Error committing sound stream: {}", e);
                        }
                    }
                    FlvTagData::Video(video_data) if !is_lookahead_tag => {
                        self.flv_video_tag(context, &slice, video_data, tag_needs_preloading)
                    }
                    FlvTagData::Script(script_data) if !is_lookahead_tag => {
                        self.flv_script_tag(context, script_data, tag_needs_preloading);
                    }
                    FlvTagData::Invalid(e) => {
                        tracing::error!("FLV data parsing failed: {}", e)
                    }
                    FlvTagData::Video(_) | FlvTagData::Script(_) => {}
                }

                if !is_lookahead_tag {
                    last_tag_time = Some(tag.timestamp as f64);
                    let offset = reader
                        .stream_position()
                        .expect("FLV reader stream position")
                        as usize;
                    source.offset.set(offset);
                    source
                        .preload_offset
                        .set(max(source.offset.get(), source.preload_offset.get()));
                }

                if !Gc::ptr_eq(source, self.source()) {
                    // A script callback replaced the source, e.g. by seeking
                    // in data generation mode: stop processing the old one.
                    break;
                }
            }
        }

        drop(buffer);

        let mut pending = std::mem::take(&mut *self.0.pending_append_bytes.borrow_mut());

        if !pending.is_empty() {
            self.drop_played_appended_data(context);
            self.load_buffer(context, &mut pending);
            // A script callback appended more, after the tags just parsed.
            buffer_underrun = false;
        }

        if !Gc::ptr_eq(source, self.source()) {
            return;
        }

        if self.0.buffer_full_pending.take() {
            self.trigger_status_event(
                context,
                [("code", "NetStream.Buffer.Full"), ("level", "status")],
            );
            if !Gc::ptr_eq(source, self.source()) {
                // The listener played something else.
                return;
            }
            if source.offset.get() < source.buffer.borrow().len() {
                // The listener appended more.
                buffer_underrun = false;
            }
        }

        if buffer_underrun && self.0.data_generation_mode.get() {
            // Out of appended data: wait at the last tag for more.
            let last_tag_time = last_tag_time.unwrap_or(source.stream_time.get());
            source
                .stream_time
                .set(source.stream_time.get().max(last_tag_time));
        } else {
            source.stream_time.set(max_time);
        }
        if let Err(e) = self.commit_sound_stream(context) {
            //TODO: Fire an error event at AS.
            tracing::error!("Error committing sound stream: {}", e);
        }

        if buffer_underrun && self.0.data_generation_mode.get() {
            // Flash waits for more data, until the stream is closed or played
            // anew, even after `appendBytesAction(END_SEQUENCE)`.
            if !self.0.out_of_data.replace(true) {
                self.trigger_status_event(
                    context,
                    [("code", "NetStream.Buffer.Empty"), ("level", "status")],
                );
            }
        } else if buffer_underrun {
            let is_end_of_video = source.expected_length.get().is_none();

            self.trigger_status_event(
                context,
                [("code", "NetStream.Buffer.Flush"), ("level", "status")],
            );

            if is_end_of_video {
                self.trigger_status_event(
                    context,
                    [("code", "NetStream.Play.Stop"), ("level", "status")],
                );
            }

            // Check if AVM code in the event handler invoked stream.play() and replaced the source.
            if Gc::ptr_eq(source, self.source()) {
                self.trigger_status_event(
                    context,
                    [("code", "NetStream.Buffer.Empty"), ("level", "status")],
                );

                if is_end_of_video {
                    self.pause(context, false);
                }
            }
        }

        if error {
            //TODO: Fire an error event at AS.
            self.pause(context, false);
        }
    }

    pub fn last_decoded_bitmap(self) -> Option<BitmapInfo> {
        self.0.last_decoded_bitmap.borrow().clone()
    }

    /// Trigger a status event on the stream.
    pub fn trigger_status_event<'a>(
        self,
        context: &mut UpdateContext<'gc>,
        values: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) {
        let object = self.0.avm_object.get();
        match object {
            Some(NetStreamKind::Avm1(object)) => {
                let root = context.stage.root_clip().expect("root");
                let mut activation = Avm1Activation::from_nothing(
                    context,
                    Avm1ActivationIdentifier::root("[NetStream Status Event]"),
                    root,
                );
                let object_proto = activation.prototypes().object;
                let info_object = Avm1Object::new(&activation.context.strings, Some(object_proto));

                for (key, value) in values {
                    let key = AvmString::new_utf8(activation.gc(), key);
                    let value = AvmString::new_utf8(activation.gc(), value);

                    info_object
                        .set(key, value, &mut activation)
                        .expect("valid set");
                }

                if let Err(e) = object.call_method(
                    istr!("onStatus"),
                    &[info_object.into()],
                    &mut activation,
                    Avm1ExecutionReason::Special,
                ) {
                    tracing::error!(
                        "Got error when dispatching AVM1 onStatus event from NetStream: {}",
                        e
                    );
                }
            }
            Some(NetStreamKind::Avm2(object)) => {
                let domain = context.avm2.stage_domain();
                let mut activation = Avm2Activation::from_domain(context, domain);
                let net_status_event = Avm2EventObject::net_status_event(&mut activation, values);
                Avm2::dispatch_event(activation.context, net_status_event, object.into());
            }
            None => {}
        }
    }

    fn handle_script_data(
        self,
        avm_object: Option<NetStreamKind<'gc>>,
        context: &mut UpdateContext<'gc>,
        variable_name: &[u8],
        variable_data: FlvValue,
    ) -> Result<(), Avm2Error<'gc>> {
        match avm_object {
            Some(NetStreamKind::Avm1(object)) => {
                let avm_string_name = AvmString::new_utf8_bytes(context.gc(), variable_name);
                let activation_name = format!("[FLV {avm_string_name}]");

                let root = context.stage.root_clip().expect("root");
                let mut activation = Avm1Activation::from_nothing(
                    context,
                    Avm1ActivationIdentifier::root(&activation_name),
                    root,
                );

                let avm1_object_value = variable_data.to_avm1_value(&mut activation);

                if let Err(e) = object.call_method(
                    avm_string_name,
                    &[avm1_object_value],
                    &mut activation,
                    Avm1ExecutionReason::Special,
                ) {
                    tracing::error!(
                        "Got error when dispatching AVM1 {} script data handler from NetStream: {}",
                        avm_string_name,
                        e,
                    );
                }
            }
            Some(NetStreamKind::Avm2(_)) => {
                let mut activation = Avm2Activation::from_nothing(context);
                let client_object = self
                    .client()
                    .expect("Client should be initialized if script data is being accessed");

                let data_object = variable_data.to_avm2_value(activation.context);
                let args = &[data_object];

                Avm2Value::from(client_object).call_public_property(
                    AvmString::new_utf8_bytes(activation.gc(), variable_name),
                    FunctionArgs::from_slice(args),
                    &mut activation,
                )?;
            }
            None => {}
        };

        Ok(())
    }
}

#![allow(dead_code)]

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const BODY: &[u8; 16] = b"0123456789abcdef";
const HLS_MASTER: &[u8] = br#"#EXTM3U
#EXT-X-VERSION:6
#EXT-X-STREAM-INF:BANDWIDTH=800000,AVERAGE-BANDWIDTH=700000,RESOLUTION=640x360,FRAME-RATE=29.970,CODECS="avc1.4d401e,mp4a.40.2",AUDIO="stereo"
low/index.m3u8
#EXT-X-STREAM-INF:BANDWIDTH=4200000,AVERAGE-BANDWIDTH=3900000,RESOLUTION=1920x1080,FRAME-RATE=60,CODECS="avc1.640028,mp4a.40.2",AUDIO="stereo"
high/index.m3u8
"#;
const HLS_EMPTY: &[u8] = b"#EXTM3U\n#EXT-X-VERSION:6\n";
const HLS_VOD: &[u8] = br#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-MEDIA-SEQUENCE:10
#EXT-X-TARGETDURATION:4
#EXTINF:4.0,
segment-10.ts
#EXTINF:4.0,
segment-11.ts
#EXTINF:2.0,
segment-12.ts
#EXT-X-ENDLIST
"#;
const HLS_SEGMENT_10: &[u8] = b"segment-ten|";
const HLS_SEGMENT_11: &[u8] = b"segment-eleven|";
const HLS_SEGMENT_12: &[u8] = b"segment-twelve";
const HLS_AES128_VOD: &[u8] = br#"#EXTM3U
#EXT-X-VERSION:5
#EXT-X-MEDIA-SEQUENCE:7794
#EXT-X-KEY:METHOD=AES-128,URI="key-one.bin"
#EXTINF:1.0,
encrypted-one.ts
#EXTINF:1.0,
encrypted-two.ts
#EXT-X-KEY:METHOD=AES-128,URI="key-two.bin",IV=0xF0F1F2F3F4F5F6F7F8F9FAFBFCFDFEFF,KEYFORMAT="identity"
#EXTINF:1.0,
rotated-three.ts
#EXT-X-KEY:METHOD=NONE
#EXTINF:1.0,
clear-four.ts
#EXT-X-ENDLIST
"#;
const HLS_AES128_BAD_KEY_VOD: &[u8] = br#"#EXTM3U
#EXT-X-KEY:METHOD=AES-128,URI="key-short.bin"
#EXTINF:1.0,
encrypted-one.ts
#EXT-X-ENDLIST
"#;
const HLS_AES128_BAD_PADDING_VOD: &[u8] = br#"#EXTM3U
#EXT-X-MEDIA-SEQUENCE:7794
#EXT-X-KEY:METHOD=AES-128,URI="key-one.bin"
#EXTINF:1.0,
invalid-padding.ts
#EXT-X-ENDLIST
"#;
const HLS_AES128_CANCEL_VOD: &[u8] = br#"#EXTM3U
#EXT-X-KEY:METHOD=AES-128,URI="key-one.bin"
#EXTINF:1.0,
cancel-ciphertext.ts
#EXT-X-ENDLIST
"#;
const HLS_AES128_KEY_ONE: &[u8] = &[
    0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
];
const HLS_AES128_KEY_TWO: &[u8] = &[
    0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0x1f,
];
const HLS_AES128_SEGMENT_ONE: &[u8] = &[
    0x66, 0xde, 0x9a, 0x0c, 0x54, 0x6f, 0xfc, 0xb6, 0x3e, 0x5e, 0x74, 0x80, 0x80, 0x1e, 0x42, 0x98,
];
const HLS_AES128_SEGMENT_TWO: &[u8] = &[
    0x40, 0xfd, 0xb1, 0x63, 0x78, 0x31, 0x73, 0x61, 0x4d, 0x47, 0x56, 0x1e, 0x5f, 0xa7, 0x7f, 0x2c,
];
const HLS_AES128_SEGMENT_THREE: &[u8] = &[
    0x64, 0x70, 0x58, 0x74, 0x9c, 0x7a, 0x33, 0x19, 0xf7, 0x0c, 0xb9, 0xbe, 0x30, 0x64, 0x88, 0x18,
];
const HLS_CONCURRENT_VOD: &[u8] = br#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-TARGETDURATION:1
#EXTINF:1.0,
segment-a.ts
#EXTINF:1.0,
segment-b.ts
#EXTINF:1.0,
segment-c.ts
#EXTINF:1.0,
segment-d.ts
#EXT-X-ENDLIST
"#;
const HLS_CONCURRENT_VOD_CHANGED: &[u8] = br#"#EXTM3U
#EXT-X-VERSION:3
#EXT-X-MEDIA-SEQUENCE:20
#EXT-X-TARGETDURATION:2
#EXTINF:2.0,
segment-a.ts
#EXTINF:2.0,
segment-b.ts
#EXTINF:2.0,
segment-c.ts
#EXTINF:2.0,
segment-d.ts
#EXT-X-ENDLIST
"#;
const HLS_RATE_LIMITED_VOD: &[u8] = b"#EXTM3U\n#EXTINF:10.0,\nsegment.ts\n#EXT-X-ENDLIST\n";
const DASH_STATIC_VOD: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" mediaPresentationDuration="PT4S">
  <BaseURL>media/</BaseURL>
  <Period duration="PT4S">
    <AdaptationSet contentType="video" mimeType="video/mp4" codecs="avc1.4d401f">
      <SegmentTemplate timescale="1000" initialization="video/$RepresentationID$/init.mp4" media="video/$RepresentationID$/$Time$.m4s">
        <SegmentTimeline><S t="0" d="2000" r="1"/></SegmentTimeline>
      </SegmentTemplate>
      <Representation id="v720" bandwidth="1800000" width="1280" height="720"/>
      <Representation id="v1080" bandwidth="4200000" width="1920" height="1080"/>
    </AdaptationSet>
    <AdaptationSet contentType="audio" mimeType="audio/mp4" codecs="mp4a.40.2" lang="zh">
      <SegmentTemplate timescale="48000" initialization="audio/$RepresentationID$/init.mp4" media="audio/$RepresentationID$/$Number%05d$.m4s" startNumber="7" duration="96000"/>
      <Representation id="a128" bandwidth="128000" audioSamplingRate="48000"/>
    </AdaptationSet>
  </Period>
</MPD>
"#;
const DASH_STATIC_VOD_CHANGED: &[u8] = br#"<?xml version="1.0" encoding="UTF-8"?>
<MPD xmlns="urn:mpeg:dash:schema:mpd:2011" type="static" mediaPresentationDuration="PT4S">
  <BaseURL>media/</BaseURL>
  <Period duration="PT4S">
    <AdaptationSet contentType="video" mimeType="video/mp4" codecs="avc1.640032">
      <SegmentTemplate timescale="1000" initialization="video/$RepresentationID$/init.mp4" media="video/$RepresentationID$/$Time$.m4s">
        <SegmentTimeline><S t="0" d="2000" r="1"/></SegmentTimeline>
      </SegmentTemplate>
      <Representation id="v720" bandwidth="1800000" width="1280" height="720"/>
      <Representation id="v1080" bandwidth="4200000" width="1920" height="1080"/>
    </AdaptationSet>
    <AdaptationSet contentType="audio" mimeType="audio/mp4" codecs="mp4a.40.2" lang="zh">
      <SegmentTemplate timescale="48000" initialization="audio/$RepresentationID$/init.mp4" media="audio/$RepresentationID$/$Number%05d$.m4s" startNumber="7" duration="96000"/>
      <Representation id="a128" bandwidth="128000" audioSamplingRate="48000"/>
    </AdaptationSet>
  </Period>
</MPD>
"#;
const DASH_TIMESCALE_VOD: &[u8] = br#"<MPD type="static" mediaPresentationDuration="PT2S"><BaseURL>media/</BaseURL><Period><AdaptationSet contentType="video" mimeType="video/mp4" codecs="avc1.4d401f"><SegmentTemplate timescale="1000" initialization="video/$RepresentationID$/init.mp4" media="video/$RepresentationID$/$Time$.m4s"><SegmentTimeline><S t="0" d="2000"/></SegmentTimeline></SegmentTemplate><Representation id="v1080" bandwidth="4200000" width="1920" height="1080"/></AdaptationSet></Period></MPD>"#;
const DASH_TIMESCALE_VOD_CHANGED: &[u8] = br#"<MPD type="static" mediaPresentationDuration="PT2S"><BaseURL>media/</BaseURL><Period><AdaptationSet contentType="video" mimeType="video/mp4" codecs="avc1.4d401f"><SegmentTemplate timescale="500" initialization="video/$RepresentationID$/init.mp4" media="video/$RepresentationID$/$Time$.m4s"><SegmentTimeline><S t="0" d="1000"/></SegmentTimeline></SegmentTemplate><Representation id="v1080" bandwidth="4200000" width="1920" height="1080"/></AdaptationSet></Period></MPD>"#;
const CONCURRENT_BODY_LEN: usize = 256 * 1024;
const IMBALANCED_BODY_LEN: usize = 8 * 1024 * 1024;

pub struct DeterministicHttpServer {
    address: SocketAddr,
    total_requests: Arc<AtomicU64>,
    revision: Arc<AtomicU64>,
    resume_transfer_started: Arc<AtomicBool>,
    active_range_transfers: Arc<AtomicU64>,
    max_active_range_transfers: Arc<AtomicU64>,
    hls_first_segment_attempts: Arc<AtomicU64>,
    hold_hls_first_segment: Arc<AtomicBool>,
    hls_second_segment_attempts: Arc<AtomicU64>,
    hold_hls_second_segment: Arc<AtomicBool>,
    transient_range_failures_remaining: Arc<AtomicU64>,
    transient_range_attempts: Arc<AtomicU64>,
    fallback_stream_failures_remaining: Arc<AtomicU64>,
    fallback_stream_attempts: Arc<AtomicU64>,
    fallback_stream_hold: Arc<AtomicBool>,
    validator_range_failures_remaining: Arc<AtomicU64>,
    validator_range_attempts: Arc<AtomicU64>,
    varying_validator_full_stream_attempts: Arc<AtomicU64>,
    dynamic_range_attempts: Arc<AtomicU64>,
    network_available: Arc<AtomicBool>,
    shutdown: Arc<AtomicBool>,
    listener_thread: Option<JoinHandle<()>>,
}

impl DeterministicHttpServer {
    pub fn start() -> Self {
        Self::start_with_connection_workers(false)
    }

    pub fn start_concurrent() -> Self {
        Self::start_with_connection_workers(true)
    }

    fn start_with_connection_workers(spawn_connection_workers: bool) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind HTTP fixture");
        listener
            .set_nonblocking(true)
            .expect("make fixture listener non-blocking");
        let address = listener.local_addr().expect("read fixture address");
        let total_requests = Arc::new(AtomicU64::new(0));
        let revision = Arc::new(AtomicU64::new(1));
        let resume_transfer_started = Arc::new(AtomicBool::new(false));
        let active_range_transfers = Arc::new(AtomicU64::new(0));
        let max_active_range_transfers = Arc::new(AtomicU64::new(0));
        let hls_first_segment_attempts = Arc::new(AtomicU64::new(0));
        let hold_hls_first_segment = Arc::new(AtomicBool::new(false));
        let hls_second_segment_attempts = Arc::new(AtomicU64::new(0));
        let hold_hls_second_segment = Arc::new(AtomicBool::new(false));
        let transient_range_failures_remaining = Arc::new(AtomicU64::new(1));
        let transient_range_attempts = Arc::new(AtomicU64::new(0));
        let fallback_stream_failures_remaining = Arc::new(AtomicU64::new(1));
        let fallback_stream_attempts = Arc::new(AtomicU64::new(0));
        let fallback_stream_hold = Arc::new(AtomicBool::new(false));
        let validator_range_failures_remaining = Arc::new(AtomicU64::new(1));
        let validator_range_attempts = Arc::new(AtomicU64::new(0));
        let varying_validator_full_stream_attempts = Arc::new(AtomicU64::new(0));
        let dynamic_range_attempts = Arc::new(AtomicU64::new(0));
        let network_available = Arc::new(AtomicBool::new(true));
        let shutdown = Arc::new(AtomicBool::new(false));
        let listener_revision = Arc::clone(&revision);
        let listener_total_requests = Arc::clone(&total_requests);
        let listener_resume_transfer_started = Arc::clone(&resume_transfer_started);
        let listener_active_range_transfers = Arc::clone(&active_range_transfers);
        let listener_max_active_range_transfers = Arc::clone(&max_active_range_transfers);
        let listener_hls_first_segment_attempts = Arc::clone(&hls_first_segment_attempts);
        let listener_hold_hls_first_segment = Arc::clone(&hold_hls_first_segment);
        let listener_hls_second_segment_attempts = Arc::clone(&hls_second_segment_attempts);
        let listener_hold_hls_second_segment = Arc::clone(&hold_hls_second_segment);
        let listener_transient_range_failures_remaining =
            Arc::clone(&transient_range_failures_remaining);
        let listener_transient_range_attempts = Arc::clone(&transient_range_attempts);
        let listener_fallback_stream_failures_remaining =
            Arc::clone(&fallback_stream_failures_remaining);
        let listener_fallback_stream_attempts = Arc::clone(&fallback_stream_attempts);
        let listener_fallback_stream_hold = Arc::clone(&fallback_stream_hold);
        let listener_validator_range_failures_remaining =
            Arc::clone(&validator_range_failures_remaining);
        let listener_validator_range_attempts = Arc::clone(&validator_range_attempts);
        let listener_varying_validator_full_stream_attempts =
            Arc::clone(&varying_validator_full_stream_attempts);
        let listener_dynamic_range_attempts = Arc::clone(&dynamic_range_attempts);
        let listener_network_available = Arc::clone(&network_available);
        let listener_shutdown = Arc::clone(&shutdown);

        let listener_thread = thread::spawn(move || {
            while !listener_shutdown.load(Ordering::Acquire) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if listener_shutdown.load(Ordering::Acquire) {
                            break;
                        }
                        let revision = Arc::clone(&listener_revision);
                        let total_requests = Arc::clone(&listener_total_requests);
                        let resume_transfer_started = Arc::clone(&listener_resume_transfer_started);
                        let active_range_transfers = Arc::clone(&listener_active_range_transfers);
                        let max_active_range_transfers =
                            Arc::clone(&listener_max_active_range_transfers);
                        let hls_first_segment_attempts =
                            Arc::clone(&listener_hls_first_segment_attempts);
                        let hold_hls_first_segment = Arc::clone(&listener_hold_hls_first_segment);
                        let hls_second_segment_attempts =
                            Arc::clone(&listener_hls_second_segment_attempts);
                        let hold_hls_second_segment = Arc::clone(&listener_hold_hls_second_segment);
                        let transient_range_failures_remaining =
                            Arc::clone(&listener_transient_range_failures_remaining);
                        let transient_range_attempts =
                            Arc::clone(&listener_transient_range_attempts);
                        let fallback_stream_failures_remaining =
                            Arc::clone(&listener_fallback_stream_failures_remaining);
                        let fallback_stream_attempts =
                            Arc::clone(&listener_fallback_stream_attempts);
                        let fallback_stream_hold = Arc::clone(&listener_fallback_stream_hold);
                        let validator_range_failures_remaining =
                            Arc::clone(&listener_validator_range_failures_remaining);
                        let validator_range_attempts =
                            Arc::clone(&listener_validator_range_attempts);
                        let varying_validator_full_stream_attempts =
                            Arc::clone(&listener_varying_validator_full_stream_attempts);
                        let dynamic_range_attempts = Arc::clone(&listener_dynamic_range_attempts);
                        let network_available = Arc::clone(&listener_network_available);
                        let serve = move || {
                            serve_connection(
                                stream,
                                &total_requests,
                                &revision,
                                &resume_transfer_started,
                                &active_range_transfers,
                                &max_active_range_transfers,
                                &hls_first_segment_attempts,
                                &hold_hls_first_segment,
                                &hls_second_segment_attempts,
                                &hold_hls_second_segment,
                                &transient_range_failures_remaining,
                                &transient_range_attempts,
                                &fallback_stream_failures_remaining,
                                &fallback_stream_attempts,
                                &fallback_stream_hold,
                                &validator_range_failures_remaining,
                                &validator_range_attempts,
                                &varying_validator_full_stream_attempts,
                                &dynamic_range_attempts,
                                &network_available,
                            );
                        };
                        if spawn_connection_workers {
                            thread::spawn(serve);
                        } else {
                            serve();
                        }
                    }
                    Err(error)
                        if matches!(
                            error.kind(),
                            std::io::ErrorKind::WouldBlock
                                | std::io::ErrorKind::Interrupted
                                | std::io::ErrorKind::ConnectionAborted
                                | std::io::ErrorKind::ConnectionReset
                        ) =>
                    {
                        thread::sleep(Duration::from_millis(2));
                    }
                    Err(_) => {
                        if listener_shutdown.load(Ordering::Acquire) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(2));
                    }
                }
            }
        });

        Self {
            address,
            total_requests,
            revision,
            resume_transfer_started,
            active_range_transfers,
            max_active_range_transfers,
            hls_first_segment_attempts,
            hold_hls_first_segment,
            hls_second_segment_attempts,
            hold_hls_second_segment,
            transient_range_failures_remaining,
            transient_range_attempts,
            fallback_stream_failures_remaining,
            fallback_stream_attempts,
            fallback_stream_hold,
            validator_range_failures_remaining,
            validator_range_attempts,
            varying_validator_full_stream_attempts,
            dynamic_range_attempts,
            network_available,
            shutdown,
            listener_thread: Some(listener_thread),
        }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub fn total_requests(&self) -> u64 {
        self.total_requests.load(Ordering::Acquire)
    }

    pub fn advance_resource_revision(&self) {
        self.revision.fetch_add(1, Ordering::AcqRel);
    }

    pub fn max_concurrent_range_transfers(&self) -> u64 {
        self.max_active_range_transfers.load(Ordering::Acquire)
    }

    pub fn hls_first_segment_attempts(&self) -> u64 {
        self.hls_first_segment_attempts.load(Ordering::Acquire)
    }

    pub fn hold_hls_first_segment(&self, hold: bool) {
        self.hold_hls_first_segment.store(hold, Ordering::Release);
    }

    pub fn hls_second_segment_attempts(&self) -> u64 {
        self.hls_second_segment_attempts.load(Ordering::Acquire)
    }

    pub fn hls_aes_first_key_attempts(&self) -> u64 {
        self.hls_first_segment_attempts()
    }

    pub fn hold_hls_aes_first_key(&self, hold: bool) {
        self.hold_hls_first_segment(hold);
    }

    pub fn hls_aes_second_key_attempts(&self) -> u64 {
        self.hls_second_segment_attempts()
    }

    pub fn hold_hls_aes_second_segment(&self, hold: bool) {
        self.hold_hls_second_segment(hold);
    }

    pub fn dash_initialization_attempts(&self) -> u64 {
        self.hls_first_segment_attempts()
    }

    pub fn dash_first_media_attempts(&self) -> u64 {
        self.hls_second_segment_attempts()
    }

    pub fn hold_dash_first_media(&self, hold: bool) {
        self.hold_hls_second_segment(hold);
    }

    pub fn hls_invalid_padding_attempts(&self) -> u64 {
        self.dynamic_range_attempts()
    }

    pub fn hold_hls_invalid_padding_retry(&self, hold: bool) {
        self.hold_hls_second_segment(hold);
    }

    pub fn hold_hls_ciphertext_body(&self, hold: bool) {
        self.hold_hls_second_segment(hold);
    }

    pub fn hold_hls_second_segment(&self, hold: bool) {
        self.hold_hls_second_segment.store(hold, Ordering::Release);
    }

    pub fn transient_range_attempts(&self) -> u64 {
        self.transient_range_attempts.load(Ordering::Acquire)
    }

    pub fn set_fallback_stream_failures(&self, failures: u64) {
        self.fallback_stream_failures_remaining
            .store(failures, Ordering::Release);
    }
    pub fn set_fallback_stream_hold(&self, hold: bool) {
        self.fallback_stream_hold.store(hold, Ordering::Release);
    }

    pub fn fallback_stream_attempts(&self) -> u64 {
        self.fallback_stream_attempts.load(Ordering::Acquire)
    }

    pub fn validator_range_attempts(&self) -> u64 {
        self.validator_range_attempts.load(Ordering::Acquire)
    }

    pub fn varying_validator_full_stream_attempts(&self) -> u64 {
        self.varying_validator_full_stream_attempts
            .load(Ordering::Acquire)
    }

    pub fn dynamic_range_attempts(&self) -> u64 {
        self.dynamic_range_attempts.load(Ordering::Acquire)
    }

    pub fn set_network_available(&self, available: bool) {
        self.network_available.store(available, Ordering::Release);
    }

    pub fn request(&self, method: &str, path: &str, headers: &[(&str, &str)]) -> RawHttpResponse {
        let mut stream = TcpStream::connect(self.address).expect("connect to HTTP fixture");
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .expect("set fixture request timeout");

        let mut request =
            format!("{method} {path} HTTP/1.1\r\nHost: fixture\r\nConnection: close\r\n");
        for (name, value) in headers {
            request.push_str(name);
            request.push_str(": ");
            request.push_str(value);
            request.push_str("\r\n");
        }
        request.push_str("\r\n");
        stream
            .write_all(request.as_bytes())
            .expect("write fixture request");

        let mut bytes = Vec::new();
        stream
            .read_to_end(&mut bytes)
            .expect("read fixture response");
        RawHttpResponse::parse(&bytes)
    }
}

impl Drop for DeterministicHttpServer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Release);
        let _ = TcpStream::connect(self.address);
        if let Some(listener_thread) = self.listener_thread.take() {
            listener_thread.join().expect("join fixture listener");
        }
    }
}

#[derive(Debug)]
pub struct RawHttpResponse {
    pub status: u16,
    headers: BTreeMap<String, String>,
    pub body: Vec<u8>,
}

impl RawHttpResponse {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    fn parse(bytes: &[u8]) -> Self {
        let header_end = bytes
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .expect("fixture response contains a header terminator");
        let head = std::str::from_utf8(&bytes[..header_end]).expect("fixture headers are UTF-8");
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .and_then(|value| value.parse::<u16>().ok())
            .expect("fixture response contains a status");
        let headers = lines
            .filter_map(|line| line.split_once(':'))
            .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
            .collect();

        Self {
            status,
            headers,
            body: bytes[header_end + 4..].to_vec(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn serve_connection(
    mut stream: TcpStream,
    total_requests: &AtomicU64,
    revision: &AtomicU64,
    resume_transfer_started: &AtomicBool,
    active_range_transfers: &AtomicU64,
    max_active_range_transfers: &AtomicU64,
    hls_first_segment_attempts: &AtomicU64,
    hold_hls_first_segment: &AtomicBool,
    hls_second_segment_attempts: &AtomicU64,
    hold_hls_second_segment: &AtomicBool,
    transient_range_failures_remaining: &AtomicU64,
    transient_range_attempts: &AtomicU64,
    fallback_stream_failures_remaining: &AtomicU64,
    fallback_stream_attempts: &AtomicU64,
    fallback_stream_hold: &AtomicBool,
    validator_range_failures_remaining: &AtomicU64,
    validator_range_attempts: &AtomicU64,
    varying_validator_full_stream_attempts: &AtomicU64,
    dynamic_range_attempts: &AtomicU64,
    network_available: &AtomicBool,
) {
    // Winsock accepted sockets inherit the listener's nonblocking mode.
    // Wait for request bytes instead of treating WSAEWOULDBLOCK as disconnect.
    stream
        .set_nonblocking(false)
        .expect("make accepted fixture connection blocking");
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .expect("bound fixture request reads");
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .expect("bound fixture response writes");
    let mut request_bytes = Vec::new();
    let mut buffer = [0_u8; 1024];

    while request_bytes.len() < 16 * 1024 {
        match stream.read(&mut buffer) {
            Ok(0) => return,
            Ok(read) => {
                request_bytes.extend_from_slice(&buffer[..read]);
                if request_bytes.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => return,
        }
    }

    let request = String::from_utf8_lossy(&request_bytes);
    let mut lines = request.split("\r\n");
    let Some(request_line) = lines.next() else {
        return;
    };
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or("");
    let request_target = request_parts.next().unwrap_or("");
    let path = request_target
        .strip_prefix("http://")
        .and_then(|target| target.find('/').map(|index| &target[index..]))
        .unwrap_or(request_target);
    total_requests.fetch_add(1, Ordering::AcqRel);
    let headers: BTreeMap<String, String> = lines
        .take_while(|line| !line.is_empty())
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim().to_ascii_lowercase(), value.trim().to_owned()))
        .collect();
    match path {
        "/hls/slow-body.m3u8" | "/dash/slow-body.mpd" => {
            let (content_type, body) = if path.ends_with(".mpd") {
                ("application/dash+xml", DASH_STATIC_VOD)
            } else {
                ("application/vnd.apple.mpegurl", HLS_MASTER)
            };
            let header = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            let _ = stream.write_all(header.as_bytes());
            if method != "HEAD" {
                thread::sleep(Duration::from_millis(500));
                let _ = stream.write_all(body);
            }
        }
        "/probe-stalled" => {
            thread::sleep(Duration::from_millis(800));
            serve_range(&mut stream, method, headers.get("range"));
        }
        "/stage6/request-context-redirect" => write_response(
            &mut stream,
            302,
            "Found",
            &[("Location", "/stage6/request-context")],
            b"",
            Some(0),
        ),
        "/stage6/request-context" => {
            let accepted = headers.get("x-nxtools-default").map(String::as_str) == Some("default")
                && headers.get("x-nxtools-explicit").map(String::as_str) == Some("explicit")
                && headers.get("x-nxtools-precedence").map(String::as_str) == Some("task")
                && headers.get("cookie").map(String::as_str) == Some("task-cookie=1")
                && headers.get("referer").map(String::as_str) == Some("https://task.example/");
            if accepted {
                serve_range_with_validator(
                    &mut stream,
                    method,
                    headers.get("range"),
                    "\"request-context\"",
                );
            } else {
                write_response(
                    &mut stream,
                    403,
                    "Forbidden",
                    &[],
                    b"missing request context",
                    None,
                );
            }
        }
        "/dash/static.mpd" => {
            let body = if revision.load(Ordering::Acquire) == 1 {
                DASH_STATIC_VOD
            } else {
                DASH_STATIC_VOD_CHANGED
            };
            write_response(
                &mut stream,
                200,
                "OK",
                &[("Content-Type", "application/dash+xml")],
                if method == "HEAD" { b"" } else { body },
                Some(body.len()),
            )
        }
        "/dash/timescale.mpd" => {
            let body = if revision.load(Ordering::Acquire) == 1 {
                DASH_TIMESCALE_VOD
            } else {
                DASH_TIMESCALE_VOD_CHANGED
            };
            write_response(
                &mut stream,
                200,
                "OK",
                &[("Content-Type", "application/dash+xml")],
                if method == "HEAD" { b"" } else { body },
                Some(body.len()),
            )
        }
        "/dash/media/video/v1080/init.mp4" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"VINIT|",
            active_range_transfers,
            max_active_range_transfers,
            Some(hls_first_segment_attempts),
            None,
        ),
        "/dash/media/video/v1080/0.m4s" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"V0|",
            active_range_transfers,
            max_active_range_transfers,
            Some(hls_second_segment_attempts),
            Some(hold_hls_second_segment),
        ),
        "/dash/media/video/v1080/2000.m4s" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"V1",
            active_range_transfers,
            max_active_range_transfers,
            None,
            None,
        ),
        "/dash/media/audio/a128/init.mp4" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"AINIT|",
            active_range_transfers,
            max_active_range_transfers,
            None,
            None,
        ),
        "/dash/media/audio/a128/00007.m4s" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"A0|",
            active_range_transfers,
            max_active_range_transfers,
            None,
            None,
        ),
        "/dash/media/audio/a128/00008.m4s" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"A1",
            active_range_transfers,
            max_active_range_transfers,
            None,
            None,
        ),
        "/hls/master.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" { b"" } else { HLS_MASTER },
            Some(HLS_MASTER.len()),
        ),
        "/hls/too-large.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            b"",
            Some(2 * 1024 * 1024 + 1),
        ),
        "/hls/empty.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" { b"" } else { HLS_EMPTY },
            Some(HLS_EMPTY.len()),
        ),
        "/hls/high/index.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" { b"" } else { HLS_VOD },
            Some(HLS_VOD.len()),
        ),
        "/hls/high/segment-10.ts" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            if method == "HEAD" {
                b""
            } else {
                HLS_SEGMENT_10
            },
            Some(HLS_SEGMENT_10.len()),
        ),
        "/hls/high/segment-11.ts" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            if method == "HEAD" {
                b""
            } else {
                HLS_SEGMENT_11
            },
            Some(HLS_SEGMENT_11.len()),
        ),
        "/hls/high/segment-12.ts" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            if method == "HEAD" {
                b""
            } else {
                HLS_SEGMENT_12
            },
            Some(HLS_SEGMENT_12.len()),
        ),
        "/hls/aes/index.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" {
                b""
            } else {
                HLS_AES128_VOD
            },
            Some(HLS_AES128_VOD.len()),
        ),
        "/hls/aes/bad-key.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" {
                b""
            } else {
                HLS_AES128_BAD_KEY_VOD
            },
            Some(HLS_AES128_BAD_KEY_VOD.len()),
        ),
        "/hls/aes/bad-padding.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" {
                b""
            } else {
                HLS_AES128_BAD_PADDING_VOD
            },
            Some(HLS_AES128_BAD_PADDING_VOD.len()),
        ),
        "/hls/aes/cancel.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" {
                b""
            } else {
                HLS_AES128_CANCEL_VOD
            },
            Some(HLS_AES128_CANCEL_VOD.len()),
        ),
        "/hls/aes/key-one.bin" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            HLS_AES128_KEY_ONE,
            active_range_transfers,
            max_active_range_transfers,
            Some(hls_first_segment_attempts),
            Some(hold_hls_first_segment),
        ),
        "/hls/aes/key-two.bin" => {
            hls_second_segment_attempts.fetch_add(1, Ordering::AcqRel);
            write_response(
                &mut stream,
                200,
                "OK",
                &[("Content-Type", "application/octet-stream")],
                if method == "HEAD" {
                    b""
                } else {
                    HLS_AES128_KEY_TWO
                },
                Some(HLS_AES128_KEY_TWO.len()),
            )
        }
        "/hls/aes/key-short.bin" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/octet-stream")],
            if method == "HEAD" {
                b""
            } else {
                &HLS_AES128_KEY_ONE[..15]
            },
            Some(15),
        ),
        "/hls/aes/encrypted-one.ts" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            if method == "HEAD" {
                b""
            } else {
                HLS_AES128_SEGMENT_ONE
            },
            Some(HLS_AES128_SEGMENT_ONE.len()),
        ),
        "/hls/aes/invalid-padding.ts" => {
            let attempt = dynamic_range_attempts.fetch_add(1, Ordering::AcqRel) + 1;
            if attempt >= 2 {
                let hold_deadline = std::time::Instant::now() + Duration::from_secs(5);
                while hold_hls_second_segment.load(Ordering::Acquire)
                    && std::time::Instant::now() < hold_deadline
                {
                    thread::sleep(Duration::from_millis(2));
                }
            }
            write_response(
                &mut stream,
                200,
                "OK",
                &[("Content-Type", "video/mp2t")],
                if method == "HEAD" {
                    b""
                } else {
                    &[
                        0x66, 0xde, 0x9a, 0x0c, 0x54, 0x6f, 0xfc, 0xb6, 0x3e, 0x5e, 0x74, 0x80,
                        0x80, 0x1e, 0x42, 0x00,
                    ]
                },
                Some(16),
            )
        }
        "/hls/aes/cancel-ciphertext.ts" => {
            serve_held_hls_ciphertext(&mut stream, method, hold_hls_second_segment)
        }
        "/hls/aes/encrypted-two.ts" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            HLS_AES128_SEGMENT_TWO,
            active_range_transfers,
            max_active_range_transfers,
            None,
            Some(hold_hls_second_segment),
        ),
        "/hls/aes/rotated-three.ts" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            if method == "HEAD" {
                b""
            } else {
                HLS_AES128_SEGMENT_THREE
            },
            Some(HLS_AES128_SEGMENT_THREE.len()),
        ),
        "/hls/aes/clear-four.ts" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            if method == "HEAD" { b"" } else { b"clear-four" },
            Some(b"clear-four".len()),
        ),
        "/hls/concurrent/index.m3u8" => {
            let body = if revision.load(Ordering::Acquire) == 1 {
                HLS_CONCURRENT_VOD
            } else {
                HLS_CONCURRENT_VOD_CHANGED
            };
            write_response(
                &mut stream,
                200,
                "OK",
                &[("Content-Type", "application/vnd.apple.mpegurl")],
                if method == "HEAD" { b"" } else { body },
                Some(body.len()),
            )
        }
        "/hls/concurrent/segment-a.ts" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"segment-a|",
            active_range_transfers,
            max_active_range_transfers,
            Some(hls_first_segment_attempts),
            Some(hold_hls_first_segment),
        ),
        "/hls/concurrent/segment-b.ts" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"segment-b|",
            active_range_transfers,
            max_active_range_transfers,
            Some(hls_second_segment_attempts),
            Some(hold_hls_second_segment),
        ),
        "/hls/concurrent/segment-c.ts" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"segment-c|",
            active_range_transfers,
            max_active_range_transfers,
            None,
            None,
        ),
        "/hls/concurrent/segment-d.ts" => serve_concurrent_hls_segment(
            &mut stream,
            method,
            b"segment-d",
            active_range_transfers,
            max_active_range_transfers,
            None,
            None,
        ),
        "/hls/rate-limited/index.m3u8" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Content-Type", "application/vnd.apple.mpegurl")],
            if method == "HEAD" {
                b""
            } else {
                HLS_RATE_LIMITED_VOD
            },
            Some(HLS_RATE_LIMITED_VOD.len()),
        ),
        "/hls/rate-limited/segment.ts" => {
            let body = vec![b'x'; 64 * 1024];
            write_response(
                &mut stream,
                200,
                "OK",
                &[("Content-Type", "video/mp2t")],
                if method == "HEAD" { b"" } else { &body },
                Some(body.len()),
            )
        }
        "/range" => serve_range(&mut stream, method, headers.get("range")),
        "/range-if-range-incompatible" => serve_if_range_incompatible(
            &mut stream,
            method,
            headers.get("range"),
            headers.get("if-range"),
        ),
        "/range-resume-required" => serve_resume_required(
            &mut stream,
            method,
            headers.get("range"),
            headers.get("if-range"),
            resume_transfer_started,
        ),
        "/range-probe-only" => serve_probe_only_range(
            &mut stream,
            method,
            headers.get("range"),
            fallback_stream_failures_remaining,
            fallback_stream_attempts,
            fallback_stream_hold,
            false,
        ),
        "/range-probe-only-unavailable" => serve_probe_only_range(
            &mut stream,
            method,
            headers.get("range"),
            fallback_stream_failures_remaining,
            fallback_stream_attempts,
            fallback_stream_hold,
            true,
        ),
        "/range-mutable" => {
            serve_mutable_range(&mut stream, method, headers.get("range"), revision)
        }
        "/range-concurrent" => serve_concurrent_range(
            &mut stream,
            method,
            headers.get("range"),
            active_range_transfers,
            max_active_range_transfers,
        ),
        "/range-concurrent-varying-validator" => serve_concurrent_varying_validator(
            &mut stream,
            method,
            headers.get("range"),
            active_range_transfers,
            max_active_range_transfers,
            varying_validator_full_stream_attempts,
        ),
        "/range-progress-then-ignored" => {
            serve_range_progress_then_ignored(&mut stream, method, headers.get("range"))
        }
        "/range-concurrent-if-range-incompatible" => {
            if headers.contains_key("if-range") {
                let body = concurrent_fixture_bytes();
                write_response(
                    &mut stream,
                    200,
                    "OK",
                    &[
                        ("Accept-Ranges", "bytes"),
                        ("ETag", "\"conditional-representation\""),
                    ],
                    if method == "HEAD" { b"" } else { &body },
                    Some(body.len()),
                );
            } else {
                serve_concurrent_range(
                    &mut stream,
                    method,
                    headers.get("range"),
                    active_range_transfers,
                    max_active_range_transfers,
                );
            }
        }
        "/range-disconnect" => {
            serve_disconnectable_range(&mut stream, method, headers.get("range"), network_available)
        }
        "/range-imbalanced" => serve_imbalanced_range(
            &mut stream,
            method,
            headers.get("range"),
            dynamic_range_attempts,
        ),
        "/range-transient" => serve_transient_range(
            &mut stream,
            method,
            headers.get("range"),
            transient_range_failures_remaining,
            transient_range_attempts,
        ),
        "/range-validator-replica" => serve_validator_replica_range(
            &mut stream,
            method,
            headers.get("range"),
            validator_range_failures_remaining,
            validator_range_attempts,
        ),
        "/range-always-transient" => serve_always_range_error(
            &mut stream,
            method,
            headers.get("range"),
            transient_range_attempts,
            503,
            "Service Unavailable",
        ),
        "/range-permanent" => serve_always_range_error(
            &mut stream,
            method,
            headers.get("range"),
            transient_range_attempts,
            404,
            "Not Found",
        ),
        "/head-no-metadata" if method == "HEAD" => {
            let _ = stream.write_all(b"HTTP/1.1 200 OK\r\nConnection: close\r\n\r\n");
        }
        "/head-no-metadata" => {
            thread::sleep(Duration::from_millis(600));
            serve_range(&mut stream, method, headers.get("range"));
        }
        "/head-slow" if method == "HEAD" => {
            thread::sleep(Duration::from_secs(1));
            write_response(&mut stream, 200, "OK", &[], b"", Some(BODY.len()));
        }
        "/head-slow" => serve_range(&mut stream, method, headers.get("range")),
        "/range-slow" if method != "HEAD" => {
            thread::sleep(Duration::from_secs(1));
            serve_range(&mut stream, method, headers.get("range"));
        }
        "/range-slow" => serve_range(&mut stream, method, headers.get("range")),
        "/head-delayed-name" if method == "HEAD" => {
            thread::sleep(Duration::from_millis(200));
            write_response(
                &mut stream,
                200,
                "OK",
                &[(
                    "Content-Disposition",
                    "attachment; filename=\"delayed-head.bin\"",
                )],
                b"",
                Some(BODY.len()),
            );
        }
        "/head-delayed-name" => serve_range(&mut stream, method, headers.get("range")),
        "/head-names-the-file" if method == "HEAD" => write_response(
            &mut stream,
            200,
            "OK",
            &[
                ("Accept-Ranges", "bytes"),
                (
                    "Content-Disposition",
                    "attachment; filename=\"from-head.bin\"",
                ),
            ],
            b"",
            Some(BODY.len()),
        ),
        "/head-names-the-file" => serve_range(&mut stream, method, headers.get("range")),
        "/head-rejected" if method == "HEAD" => {
            write_response(&mut stream, 405, "Method Not Allowed", &[], b"", None);
        }
        "/head-rejected" => serve_range(&mut stream, method, headers.get("range")),
        "/head-only" if method == "HEAD" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"head-only\"")],
            b"",
            Some(BODY.len()),
        ),
        "/head-only" if headers.contains_key("range") => {
            write_response(&mut stream, 416, "Range Not Satisfiable", &[], b"", None);
        }
        "/head-only" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"head-only\"")],
            BODY,
            Some(BODY.len()),
        ),
        "/get-only" if method == "HEAD" => {
            write_response(&mut stream, 405, "Method Not Allowed", &[], b"", None);
        }
        "/get-only" if headers.contains_key("range") => {
            write_response(&mut stream, 416, "Range Not Satisfiable", &[], b"", None);
        }
        "/get-only" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"get-only\"")],
            BODY,
            Some(BODY.len()),
        ),
        "/no-range" => write_response(
            &mut stream,
            200,
            "OK",
            &[("ETag", "\"no-range\"")],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        ),
        "/advertised-range-ignored" => write_response(
            &mut stream,
            200,
            "OK",
            &[("Accept-Ranges", "bytes"), ("ETag", "\"ignored-range\"")],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        ),
        "/mismatched-content-range" => write_response(
            &mut stream,
            206,
            "Partial Content",
            &[
                ("Accept-Ranges", "bytes"),
                ("Content-Range", "bytes 5-8/16"),
            ],
            if method == "HEAD" { b"" } else { &BODY[4..=7] },
            Some(4),
        ),
        "/missing-content-range" => write_response(
            &mut stream,
            206,
            "Partial Content",
            &[("Accept-Ranges", "bytes")],
            if method == "HEAD" { b"" } else { &BODY[4..=7] },
            Some(4),
        ),
        "/mismatched-content-length" => write_response(
            &mut stream,
            206,
            "Partial Content",
            &[
                ("Accept-Ranges", "bytes"),
                ("Content-Range", "bytes 4-7/16"),
            ],
            if method == "HEAD" { b"" } else { &BODY[4..=7] },
            Some(5),
        ),
        "/range-validator-changed" if method == "HEAD" => write_response(
            &mut stream,
            200,
            "OK",
            &[
                ("Accept-Ranges", "bytes"),
                ("ETag", "\"fixture-v1\""),
                ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
            ],
            b"",
            Some(BODY.len()),
        ),
        "/range-validator-changed" => {
            serve_range_with_validator(&mut stream, method, headers.get("range"), "\"fixture-v2\"")
        }
        "/redirect" => write_response(
            &mut stream,
            302,
            "Found",
            &[("Location", "/range")],
            b"",
            None,
        ),
        "/drop" => {
            write_response(&mut stream, 200, "OK", &[], &BODY[..5], Some(BODY.len()));
        }
        "/stall" => serve_stall(&mut stream),
        "/chunked" => serve_chunked(&mut stream, method),
        "/status/429" => write_response(
            &mut stream,
            429,
            "Too Many Requests",
            &[("Retry-After", "1")],
            b"retry",
            None,
        ),
        "/status/500" => {
            write_response(
                &mut stream,
                500,
                "Internal Server Error",
                &[],
                b"error",
                None,
            );
        }
        "/mutable" => {
            let revision = revision.load(Ordering::Acquire);
            let etag = format!("\"fixture-v{revision}\"");
            let last_modified = format!("Mon, 27 Jul 2026 00:00:{revision:02} GMT");
            write_response(
                &mut stream,
                200,
                "OK",
                &[
                    ("Accept-Ranges", "bytes"),
                    ("ETag", &etag),
                    ("Last-Modified", &last_modified),
                ],
                if method == "HEAD" { b"" } else { BODY },
                Some(BODY.len()),
            );
        }
        "/named/from-url.bin" => write_response(
            &mut stream,
            200,
            "OK",
            &[
                (
                    "Content-Disposition",
                    "attachment; filename=\"server-name.bin\"",
                ),
                ("Content-Type", "application/octet-stream"),
            ],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        ),
        _ => write_response(&mut stream, 404, "Not Found", &[], b"missing", None),
    }
    let _ = stream.shutdown(Shutdown::Write);
}

fn serve_range(stream: &mut TcpStream, method: &str, range: Option<&String>) {
    serve_range_with_validator(stream, method, range, "\"fixture-static\"");
}

fn serve_held_hls_ciphertext(stream: &mut TcpStream, method: &str, hold: &AtomicBool) {
    const CIPHERTEXT_BYTES: usize = 64 * 1024;
    if method == "HEAD" {
        write_response(
            stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            b"",
            Some(CIPHERTEXT_BYTES),
        );
        return;
    }
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: video/mp2t\r\nContent-Length: {CIPHERTEXT_BYTES}\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(head.as_bytes()).is_err()
        || stream.write_all(&[0_u8; 4 * 1024]).is_err()
        || stream.flush().is_err()
    {
        return;
    }
    let hold_deadline = std::time::Instant::now() + Duration::from_secs(5);
    while hold.load(Ordering::Acquire) && std::time::Instant::now() < hold_deadline {
        thread::sleep(Duration::from_millis(2));
    }
    for _ in 1..16 {
        if stream.write_all(&[0_u8; 4 * 1024]).is_err() {
            return;
        }
    }
    let _ = stream.flush();
}

fn serve_concurrent_hls_segment(
    stream: &mut TcpStream,
    method: &str,
    body: &[u8],
    active: &AtomicU64,
    max_active: &AtomicU64,
    attempts: Option<&AtomicU64>,
    hold: Option<&AtomicBool>,
) {
    if method == "HEAD" {
        write_response(
            stream,
            200,
            "OK",
            &[("Content-Type", "video/mp2t")],
            b"",
            Some(body.len()),
        );
        return;
    }

    if let Some(attempts) = attempts {
        attempts.fetch_add(1, Ordering::AcqRel);
    }

    let hold_deadline = std::time::Instant::now() + Duration::from_secs(5);
    while hold.is_some_and(|hold| hold.load(Ordering::Acquire))
        && std::time::Instant::now() < hold_deadline
    {
        thread::sleep(Duration::from_millis(2));
    }

    let current = active.fetch_add(1, Ordering::AcqRel) + 1;
    max_active.fetch_max(current, Ordering::AcqRel);
    thread::sleep(Duration::from_millis(80));
    write_response(
        stream,
        200,
        "OK",
        &[("Content-Type", "video/mp2t")],
        body,
        Some(body.len()),
    );
    active.fetch_sub(1, Ordering::AcqRel);
}

fn serve_if_range_incompatible(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    if_range: Option<&String>,
) {
    if if_range.is_some() {
        write_response(
            stream,
            200,
            "OK",
            &[
                ("Accept-Ranges", "bytes"),
                ("ETag", "\"conditional-representation\""),
            ],
            if method == "HEAD" { b"" } else { BODY },
            Some(BODY.len()),
        );
        return;
    }
    serve_range_with_validator(stream, method, range, "\"fixture-static\"");
}

fn serve_resume_required(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    if_range: Option<&String>,
    transfer_started: &AtomicBool,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"resume-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }

    let parsed_range = range.and_then(|value| parse_range(value));
    if parsed_range == Some((0, 0)) {
        let mut headers = common.to_vec();
        headers.push(("Content-Range", "bytes 0-0/16"));
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }

    if let Some((start, end)) = parsed_range
        && start <= end
        && end < BODY.len()
    {
        if start > 0
            && transfer_started.load(Ordering::Acquire)
            && if_range.map(String::as_str) != Some("\"resume-static\"")
        {
            write_response(
                stream,
                428,
                "Precondition Required",
                &common,
                b"resume validator required",
                None,
            );
            return;
        }
        if start == 0 && transfer_started.swap(true, Ordering::AcqRel) {
            write_response(stream, 409, "Conflict", &common, b"restart rejected", None);
            return;
        }
        let content_range = format!("bytes {start}-{end}/{}", BODY.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        write_slow_response(stream, 206, "Partial Content", &headers, &BODY[start..=end]);
        return;
    }

    if transfer_started.swap(true, Ordering::AcqRel) {
        write_response(stream, 409, "Conflict", &common, b"restart rejected", None);
    } else {
        write_slow_response(stream, 200, "OK", &common, BODY);
    }
}

fn serve_probe_only_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    fallback_failures_remaining: &AtomicU64,
    fallback_attempts: &AtomicU64,
    fallback_stream_hold: &AtomicBool,
    respond_unavailable: bool,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"probe-only\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    if range.and_then(|value| parse_range(value)) == Some((0, 0)) {
        let mut headers = common.to_vec();
        headers.push(("Content-Range", "bytes 0-0/16"));
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }
    if range.is_none() {
        fallback_attempts.fetch_add(1, Ordering::AcqRel);
        if fallback_failures_remaining
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
                remaining.checked_sub(1)
            })
            .is_ok()
        {
            while fallback_stream_hold.load(Ordering::Acquire) {
                thread::sleep(Duration::from_millis(2));
            }
            if respond_unavailable {
                write_response(stream, 503, "Service Unavailable", &common, b"", Some(0));
            } else {
                let _ = stream.shutdown(Shutdown::Both);
            }
            return;
        }
    }
    write_response(stream, 200, "OK", &common, BODY, Some(BODY.len()));
}

fn serve_mutable_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    revision: &AtomicU64,
) {
    let revision = revision.load(Ordering::Acquire);
    let etag = format!("\"mutable-v{revision}\"");
    let last_modified = format!("Mon, 27 Jul 2026 00:00:{revision:02} GMT");
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", etag.as_str()),
        ("Last-Modified", last_modified.as_str()),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < BODY.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", BODY.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &BODY[..1],
                Some(1),
            );
        } else {
            write_slow_response(stream, 206, "Partial Content", &headers, &BODY[start..=end]);
        }
        return;
    }
    write_response(stream, 200, "OK", &common, BODY, Some(BODY.len()));
}

pub fn concurrent_fixture_bytes() -> Vec<u8> {
    (0..CONCURRENT_BODY_LEN)
        .map(|index| (index % 251) as u8)
        .collect()
}

pub fn imbalanced_fixture_bytes() -> Vec<u8> {
    (0..IMBALANCED_BODY_LEN)
        .map(|index| (index % 251) as u8)
        .collect()
}

fn serve_concurrent_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    active: &AtomicU64,
    max_active: &AtomicU64,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"concurrent-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }
    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        let current = active.fetch_add(1, Ordering::AcqRel) + 1;
        max_active.fetch_max(current, Ordering::AcqRel);
        write_throttled_response(stream, 206, "Partial Content", &headers, &body[start..=end]);
        active.fetch_sub(1, Ordering::AcqRel);
        return;
    }
    write_response(stream, 200, "OK", &common, &body, Some(CONCURRENT_BODY_LEN));
}

fn serve_concurrent_varying_validator(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    active: &AtomicU64,
    max_active: &AtomicU64,
    full_stream_attempts: &AtomicU64,
) {
    let base_headers = [
        ("Accept-Ranges", "bytes"),
        ("Last-Modified", "Sat, 01 Aug 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        let mut headers = base_headers.to_vec();
        headers.push(("ETag", "\"probe-replica\""));
        write_response(stream, 200, "OK", &headers, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }

    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let validator = if (start / 4096) % 2 == 0 {
            "\"replica-a\""
        } else {
            "\"replica-b\""
        };
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = base_headers.to_vec();
        headers.push(("ETag", validator));
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            headers[2] = ("ETag", "\"probe-replica\"");
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        let current = active.fetch_add(1, Ordering::AcqRel) + 1;
        max_active.fetch_max(current, Ordering::AcqRel);
        write_throttled_response(stream, 206, "Partial Content", &headers, &body[start..=end]);
        active.fetch_sub(1, Ordering::AcqRel);
        return;
    }

    full_stream_attempts.fetch_add(1, Ordering::AcqRel);
    let mut headers = base_headers.to_vec();
    headers.push(("ETag", "\"full-replica\""));
    write_response(
        stream,
        200,
        "OK",
        &headers,
        &body,
        Some(CONCURRENT_BODY_LEN),
    );
}

fn serve_range_progress_then_ignored(stream: &mut TcpStream, method: &str, range: Option<&String>) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"mixed-range-replica\""),
        ("Last-Modified", "Sat, 01 Aug 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }

    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) || start == 0 {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[start..=end],
                Some(end - start + 1),
            );
            return;
        }

        // Let the leading segment become observable before another CDN replica ignores Range.
        // This reproduces the production failure where fallback was incorrectly gated on zero
        // in-flight progress.
        thread::sleep(Duration::from_millis(250));
        write_response(stream, 200, "OK", &common, &body, Some(CONCURRENT_BODY_LEN));
        return;
    }

    write_response(stream, 200, "OK", &common, &body, Some(CONCURRENT_BODY_LEN));
}

fn serve_disconnectable_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    network_available: &AtomicBool,
) {
    if !network_available.load(Ordering::Acquire) {
        let _ = stream.shutdown(Shutdown::Both);
        return;
    }
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"disconnect-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(CONCURRENT_BODY_LEN));
        return;
    }
    let body = concurrent_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        write_disconnectable_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &body[start..=end],
            network_available,
        );
        return;
    }
    write_disconnectable_response(stream, 200, "OK", &common, &body, network_available);
}

fn serve_imbalanced_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    attempts: &AtomicU64,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"imbalanced-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(IMBALANCED_BODY_LEN));
        return;
    }
    let body = imbalanced_fixture_bytes();
    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < body.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", body.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        if (start, end) == (0, 0) {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[..1],
                Some(1),
            );
            return;
        }
        attempts.fetch_add(1, Ordering::AcqRel);
        if end < body.len() / 2 {
            write_response(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[start..=end],
                Some(end - start + 1),
            );
        } else {
            write_throttled_response_with_delay(
                stream,
                206,
                "Partial Content",
                &headers,
                &body[start..=end],
                Duration::from_millis(1),
            );
        }
        return;
    }
    write_response(stream, 200, "OK", &common, &body, Some(IMBALANCED_BODY_LEN));
}

fn serve_transient_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    failures_remaining: &AtomicU64,
    attempts: &AtomicU64,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"transient-static\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    let Some((start, end)) = range.and_then(|value| parse_range(value)) else {
        write_response(
            stream,
            503,
            "Service Unavailable",
            &[("Retry-After", "0")],
            b"retry",
            None,
        );
        return;
    };
    if start > end || end >= BODY.len() {
        write_response(stream, 416, "Range Not Satisfiable", &[], b"", None);
        return;
    }
    let content_range = format!("bytes {start}-{end}/{}", BODY.len());
    let mut headers = common.to_vec();
    headers.push(("Content-Range", &content_range));
    if (start, end) == (0, 0) {
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }

    attempts.fetch_add(1, Ordering::AcqRel);
    let should_fail = failures_remaining
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
            remaining.checked_sub(1)
        })
        .is_ok();
    if should_fail {
        write_response(
            stream,
            503,
            "Service Unavailable",
            &[("Retry-After", "0")],
            b"retry",
            None,
        );
    } else {
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[start..=end],
            Some(end - start + 1),
        );
    }
}

fn serve_validator_replica_range(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    failures_remaining: &AtomicU64,
    attempts: &AtomicU64,
) {
    if method == "HEAD" || range.and_then(|value| parse_range(value)) == Some((0, 0)) {
        serve_range_with_validator(stream, method, range, "\"validator-stable\"");
        return;
    }

    attempts.fetch_add(1, Ordering::AcqRel);
    let mismatching_replica = failures_remaining
        .fetch_update(Ordering::AcqRel, Ordering::Acquire, |remaining| {
            remaining.checked_sub(1)
        })
        .is_ok();
    let validator = if mismatching_replica {
        "\"validator-stale-replica\""
    } else {
        "\"validator-stable\""
    };
    serve_range_with_validator(stream, method, range, validator);
}

fn serve_always_range_error(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    attempts: &AtomicU64,
    status: u16,
    reason: &str,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", "\"always-transient\""),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }
    if range.and_then(|value| parse_range(value)) == Some((0, 0)) {
        let headers = [
            ("Accept-Ranges", "bytes"),
            ("ETag", "\"always-transient\""),
            ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
            ("Content-Range", "bytes 0-0/16"),
        ];
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[..1],
            Some(1),
        );
        return;
    }
    attempts.fetch_add(1, Ordering::AcqRel);
    write_response(
        stream,
        status,
        reason,
        &[("Retry-After", "0")],
        b"retry",
        None,
    );
}

fn serve_range_with_validator(
    stream: &mut TcpStream,
    method: &str,
    range: Option<&String>,
    etag: &str,
) {
    let common = [
        ("Accept-Ranges", "bytes"),
        ("ETag", etag),
        ("Last-Modified", "Mon, 27 Jul 2026 00:00:00 GMT"),
    ];
    if method == "HEAD" {
        write_response(stream, 200, "OK", &common, b"", Some(BODY.len()));
        return;
    }

    if let Some((start, end)) = range.and_then(|value| parse_range(value))
        && start <= end
        && end < BODY.len()
    {
        let content_range = format!("bytes {start}-{end}/{}", BODY.len());
        let mut headers = common.to_vec();
        headers.push(("Content-Range", &content_range));
        write_response(
            stream,
            206,
            "Partial Content",
            &headers,
            &BODY[start..=end],
            None,
        );
        return;
    }

    write_response(stream, 200, "OK", &common, BODY, None);
}

fn parse_range(value: &str) -> Option<(usize, usize)> {
    let range = value.strip_prefix("bytes=")?;
    let (start, end) = range.split_once('-')?;
    Some((start.parse().ok()?, end.parse().ok()?))
}

fn serve_stall(stream: &mut TcpStream) {
    let head = format!(
        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        BODY.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(&BODY[..1]);
    let _ = stream.flush();
    thread::sleep(Duration::from_millis(250));
    let _ = stream.write_all(&BODY[1..]);
}

fn write_slow_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream.write_all(head.as_bytes());
    if let Some((first, rest)) = body.split_first() {
        let _ = stream.write_all(std::slice::from_ref(first));
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(250));
        let _ = stream.write_all(rest);
    }
    let _ = stream.flush();
}

fn write_throttled_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
) {
    write_throttled_response_with_delay(
        stream,
        status,
        reason,
        headers,
        body,
        Duration::from_millis(20),
    );
}

fn write_throttled_response_with_delay(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    delay: Duration,
) {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    let _ = stream.write_all(head.as_bytes());
    for chunk in body.chunks(4 * 1024) {
        let _ = stream.write_all(chunk);
        let _ = stream.flush();
        thread::sleep(delay);
    }
}

fn write_disconnectable_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    network_available: &AtomicBool,
) {
    let mut head = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        head.push_str(name);
        head.push_str(": ");
        head.push_str(value);
        head.push_str("\r\n");
    }
    head.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    ));
    if stream.write_all(head.as_bytes()).is_err() {
        return;
    }
    for chunk in body.chunks(4 * 1024) {
        if !network_available.load(Ordering::Acquire) {
            let _ = stream.shutdown(Shutdown::Both);
            return;
        }
        if stream.write_all(chunk).is_err() || stream.flush().is_err() {
            return;
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn serve_chunked(stream: &mut TcpStream, method: &str) {
    let head = "HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
    let _ = stream.write_all(head.as_bytes());
    if method != "HEAD" {
        let _ = stream.write_all(b"4\r\n0123\r\nc\r\n456789abcdef\r\n0\r\n\r\n");
    }
    let _ = stream.flush();
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    reason: &str,
    headers: &[(&str, &str)],
    body: &[u8],
    declared_length: Option<usize>,
) {
    let mut response = format!("HTTP/1.1 {status} {reason}\r\n");
    for (name, value) in headers {
        response.push_str(name);
        response.push_str(": ");
        response.push_str(value);
        response.push_str("\r\n");
    }
    response.push_str(&format!(
        "Content-Length: {}\r\nConnection: close\r\n\r\n",
        declared_length.unwrap_or(body.len())
    ));
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

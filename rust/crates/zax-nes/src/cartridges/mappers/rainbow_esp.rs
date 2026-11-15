//! The ESP Wi-Fi coprocessor of mapper 682 (Rainbow), as the NES sees it through the board's
//! `$4190-$4194` registers: phase 3 of `docs/progress/MAPPERS-N.md`'s plan.
//!
//! Written from Broke Studio's `rainbow-net/rainbow-net.md` (the message protocol) and
//! `NES/mapper-doc.md` (the registers); no emulator source was read for this part. A message is
//! `[length][command][parameters]`, the length counting the bytes after it; the NES writes one in
//! the FPGA RAM page `$4194` selects and sends it with a write to `$4192`; the ESP's answers are
//! queued and delivered one at a time to the page `$4193` selects, `$4191` bit 7 telling the game
//! one is there and a write to `$4191` acknowledging it.
//!
//! Everything that does not leave the console is emulated whatever the option says: the status,
//! the debug level, the random numbers, the server and network settings, and the file system
//! (in memory, in the savestate — FIDELITY-LEADS J36). **The network itself is behind the
//! emulation option `emu.rainbow_network`, off by default** (the owner's request): off, Wi-Fi
//! reports "not connected" and a server connection never comes up; on, TCP and UDP go through
//! the host's sockets on worker threads, and `FILE_DOWNLOAD` fetches plain HTTP.
use std::collections::{BTreeMap, VecDeque};
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::time::Duration;

use crate::state::{InvalidState, StateReader, StateWriter};

/// The answers' identifiers (ESP to NES).
mod to_nes {
    pub const READY: u8 = 0;
    pub const DEBUG_LEVEL: u8 = 1;
    pub const ESP_FIRMWARE_VERSION: u8 = 2;
    pub const ESP_FACTORY_RESET: u8 = 3;
    pub const WIFI_STATUS: u8 = 4;
    pub const SSID: u8 = 5;
    pub const IP_ADDRESS: u8 = 6;
    pub const WIFI_CONFIG: u8 = 7;
    pub const RND_BYTE: u8 = 8;
    pub const RND_WORD: u8 = 9;
    pub const SERVER_STATUS: u8 = 10;
    pub const SERVER_PING: u8 = 11;
    pub const SERVER_SETTINGS: u8 = 12;
    pub const MESSAGE_FROM_SERVER: u8 = 13;
    pub const NETWORK_SCAN_RESULT: u8 = 14;
    pub const NETWORK_SCANNED_DETAILS: u8 = 15;
    pub const NETWORK_REGISTERED_DETAILS: u8 = 16;
    pub const NETWORK_REGISTERED: u8 = 17;
    pub const FILE_STATUS: u8 = 18;
    pub const FILE_EXISTS: u8 = 19;
    pub const FILE_DELETE: u8 = 20;
    pub const FILE_LIST: u8 = 21;
    pub const FILE_DATA: u8 = 22;
    pub const FILE_COUNT: u8 = 23;
    pub const FILE_ID: u8 = 24;
    pub const FILE_FS_INFO: u8 = 25;
    pub const FILE_INFO: u8 = 26;
    pub const FILE_DOWNLOAD: u8 = 27;
    pub const MAC_ADDRESS: u8 = 28;
    pub const PING_STATUS: u8 = 29;
    pub const PING_RESULT: u8 = 30;
}

/// The SSID the emulated access point reports when the network is on.
const SSID: &[u8] = b"ZAXNES";
/// The firmware string `ESP_GET_FIRMWARE_VERSION` answers.
const FIRMWARE: &[u8] = b"zaxnes-esp-1";
/// What `FILE_GET_FS_INFO` reports as the size of each drive.
const DRIVE_SIZE: u64 = 4 * 1024 * 1024;
/// Auto-mode file slots per path.
const AUTO_FILES: u8 = 64;
/// The most queued answers kept; older ones are dropped first (a game that never reads its
/// messages must not grow the queue without bound).
const QUEUE_LIMIT: usize = 64;

/// A server connection and what it has received, behind worker threads.
struct Link {
    /// Bytes to send, one framed message each; closing it ends the writer.
    outgoing: Option<Sender<Vec<u8>>>,
    incoming: Receiver<NetEvent>,
    /// The connected socket, kept to shut it down on disconnect.
    stream: std::sync::Arc<std::sync::Mutex<Option<TcpStream>>>,
}

enum NetEvent {
    Connected,
    Message(Vec<u8>),
    Closed,
    Download(u8, u16, Option<Vec<u8>>, String),
}

/// The host side of the network: nothing at all while the option is off.
#[derive(Default)]
struct Network {
    tcp: Option<Link>,
    udp: Option<UdpSocket>,
    downloads: Vec<Receiver<NetEvent>>,
}

impl std::fmt::Debug for Network {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("Network")
    }
}

/// Server settings: port and host name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Server {
    port: u16,
    host: Vec<u8>,
}

/// One registered Wi-Fi network.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Registered {
    active: bool,
    ssid: Vec<u8>,
    password: Vec<u8>,
}

/// The open file: its key, and how `FILE_STATUS` describes it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OpenFile {
    key: String,
    descriptor: Vec<u8>,
    cursor: usize,
}

/// The ESP model.
#[derive(Debug)]
pub(crate) struct Esp {
    network_enabled: bool,
    rx: VecDeque<Vec<u8>>,
    debug_level: u8,
    wifi_config: u8,
    rng: u64,
    protocol: u8,
    server: Server,
    saved_server: Server,
    connected: bool,
    networks: [Registered; 3],
    udp_pool: Vec<(Vec<u8>, u16)>,
    files: BTreeMap<String, Vec<u8>>,
    open: Option<OpenFile>,
    net: Network,
}

impl Clone for Esp {
    /// A copy without the host connection (`Rainbow::cpu_peek_override` reads a copy).
    fn clone(&self) -> Self {
        Self {
            network_enabled: self.network_enabled,
            rx: self.rx.clone(),
            debug_level: self.debug_level,
            wifi_config: self.wifi_config,
            rng: self.rng,
            protocol: self.protocol,
            server: self.server.clone(),
            saved_server: self.saved_server.clone(),
            connected: self.connected,
            networks: self.networks.clone(),
            udp_pool: self.udp_pool.clone(),
            files: self.files.clone(),
            open: self.open.clone(),
            net: Network::default(),
        }
    }
}

impl PartialEq for Esp {
    /// The emulated state; the host connection is not compared.
    fn eq(&self, other: &Self) -> bool {
        self.network_enabled == other.network_enabled
            && self.rx == other.rx
            && self.debug_level == other.debug_level
            && self.wifi_config == other.wifi_config
            && self.rng == other.rng
            && self.protocol == other.protocol
            && self.server == other.server
            && self.saved_server == other.saved_server
            && self.connected == other.connected
            && self.networks == other.networks
            && self.udp_pool == other.udp_pool
            && self.files == other.files
            && self.open == other.open
    }
}

impl Eq for Esp {}

/// CRC-32 (IEEE), for `FILE_GET_INFO`.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// A message: its length byte, then `id` and `body`. Bodies are cut at 254 bytes.
fn message(id: u8, body: &[u8]) -> Vec<u8> {
    let body = &body[..body.len().min(254)];
    let mut out = Vec::with_capacity(body.len() + 2);
    out.push(u8::try_from(body.len() + 1).unwrap_or(0xFF));
    out.push(id);
    out.extend_from_slice(body);
    out
}

/// A length-prefixed string.
fn pstr(text: &[u8]) -> Vec<u8> {
    let text = &text[..text.len().min(250)];
    let mut out = vec![u8::try_from(text.len()).unwrap_or(0)];
    out.extend_from_slice(text);
    out
}

impl Esp {
    pub(crate) fn new() -> Self {
        Self {
            network_enabled: false,
            rx: VecDeque::new(),
            debug_level: 0,
            wifi_config: 0x01,
            rng: 0x2545_F491_4F6C_DD1D,
            protocol: 0,
            server: Server::default(),
            saved_server: Server::default(),
            connected: false,
            networks: Default::default(),
            udp_pool: Vec::new(),
            files: BTreeMap::new(),
            open: None,
            net: Network::default(),
        }
    }

    /// The emulation option: on, Wi-Fi is connected and the server commands use the host's
    /// network; off, nothing leaves the emulator.
    pub(crate) fn set_network_enabled(&mut self, enabled: bool) {
        self.network_enabled = enabled;
        if !enabled {
            self.disconnect();
        }
    }

    /// Whether answers wait to be delivered (`$4191` bit 6).
    pub(crate) fn has_answer(&self) -> bool {
        !self.rx.is_empty()
    }

    /// The next answer to deliver, with its length byte.
    pub(crate) fn pop_answer(&mut self) -> Option<Vec<u8>> {
        self.rx.pop_front()
    }

    /// Whether the host network is in use, so that the board polls it.
    pub(crate) fn polls(&self) -> bool {
        self.net.tcp.is_some() || self.net.udp.is_some() || !self.net.downloads.is_empty()
    }

    fn push(&mut self, answer: Vec<u8>) {
        if self.rx.len() >= QUEUE_LIMIT {
            self.rx.pop_front();
        }
        self.rx.push_back(answer);
    }

    fn reply(&mut self, id: u8, body: &[u8]) {
        let answer = message(id, body);
        self.push(answer);
    }

    /// xorshift64*: deterministic, so a replay of a session without network repeats itself.
    fn next_random(&mut self) -> u64 {
        let mut x = self.rng;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    fn random_in(&mut self, low: u32, high: u32) -> u32 {
        let (low, high) = if low > high { (high, low) } else { (low, high) };
        let span = u64::from(high - low) + 1;
        low + u32::try_from(self.next_random() % span).unwrap_or(0)
    }

    /// One message from the NES: `bytes` is the command and its parameters (the length byte
    /// already removed).
    #[allow(clippy::too_many_lines)] // The protocol's command table.
    pub(crate) fn command(&mut self, bytes: &[u8]) {
        let Some((&id, args)) = bytes.split_first() else {
            return;
        };
        let arg = |index: usize| args.get(index).copied();
        match id {
            0 => self.reply(to_nes::READY, &[0x00]),
            1 => self.reply(to_nes::DEBUG_LEVEL, &[self.debug_level]),
            2 => self.debug_level = arg(0).unwrap_or(0),
            // Deprecated: answered without a measurement.
            21 => self.reply(to_nes::SERVER_PING, &[]),
            4 => self.rx.clear(),
            5 => {
                if let (Some(kind), Some(keep)) = (arg(0), arg(1)) {
                    let total = self.rx.iter().filter(|m| m.get(1) == Some(&kind)).count();
                    let mut drop = total.saturating_sub(usize::from(keep));
                    self.rx.retain(|m| {
                        if drop > 0 && m.get(1) == Some(&kind) {
                            drop -= 1;
                            false
                        } else {
                            true
                        }
                    });
                }
            }
            6 => self.reply(to_nes::ESP_FIRMWARE_VERSION, &pstr(FIRMWARE)),
            7 => {
                self.wifi_config = 0x01;
                self.saved_server = Server::default();
                self.networks = Default::default();
                self.reply(to_nes::ESP_FACTORY_RESET, &[0]);
            }
            8 => {
                self.disconnect();
                self.rx.clear();
                self.open = None;
            }
            9 => {
                let status = if self.network_enabled { 2 } else { 0 };
                self.reply(to_nes::WIFI_STATUS, &[status, 0, 0]);
            }
            10 | 14 => {
                let ssid = if self.network_enabled { SSID } else { b"" };
                self.reply(to_nes::SSID, &pstr(ssid));
            }
            11 | 15 => {
                let ip = if self.network_enabled {
                    [127, 0, 0, 1]
                } else {
                    [0; 4]
                };
                if arg(0) == Some(1) {
                    self.reply(to_nes::IP_ADDRESS, &ip);
                } else {
                    let text = format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3]);
                    self.reply(to_nes::IP_ADDRESS, &pstr(text.as_bytes()));
                }
            }
            59 => {
                let mac = [0x02, 0x5A, 0x41, 0x58, 0x4E, 0x53];
                if arg(0) == Some(1) {
                    self.reply(to_nes::MAC_ADDRESS, &mac);
                } else {
                    let text: Vec<String> = mac.iter().map(|b| format!("{b:02X}")).collect();
                    self.reply(to_nes::MAC_ADDRESS, &pstr(text.join(":").as_bytes()));
                }
            }
            12 => self.reply(to_nes::WIFI_CONFIG, &[self.wifi_config]),
            13 => self.wifi_config = arg(0).unwrap_or(self.wifi_config) & 0x07,
            16 => {
                let byte = self.next_random().to_le_bytes()[0];
                self.reply(to_nes::RND_BYTE, &[byte]);
            }
            17 => {
                let value = match (arg(0), arg(1)) {
                    (Some(low), Some(high)) => self
                        .random_in(u32::from(low), u32::from(high))
                        .to_le_bytes()[0],
                    _ => 0,
                };
                self.reply(to_nes::RND_BYTE, &[value]);
            }
            18 => {
                let word = self.next_random().to_le_bytes();
                self.reply(to_nes::RND_WORD, &[word[1], word[0]]);
            }
            19 => {
                let value = if args.len() >= 4 {
                    let low = u32::from(u16::from_be_bytes([args[0], args[1]]));
                    let high = u32::from(u16::from_be_bytes([args[2], args[3]]));
                    u16::try_from(self.random_in(low, high)).unwrap_or(0)
                } else {
                    0
                };
                self.reply(to_nes::RND_WORD, &value.to_be_bytes());
            }
            20 => self.reply(to_nes::SERVER_STATUS, &[u8::from(self.connected)]),
            22 => self.protocol = arg(0).unwrap_or(0).min(3),
            23 => {
                let body = Self::settings_body(&self.server);
                self.reply(to_nes::SERVER_SETTINGS, &body);
            }
            24 => self.server = Self::parse_settings(args),
            25 => {
                let body = Self::settings_body(&self.saved_server);
                self.reply(to_nes::SERVER_SETTINGS, &body);
            }
            26 => {
                self.saved_server = Self::parse_settings(args);
                self.server = self.saved_server.clone();
            }
            27 => self.server = self.saved_server.clone(),
            28 => self.connect(),
            29 => self.disconnect(),
            30 => self.send_to_server(args),
            31 => {
                if arg(0) != Some(1) {
                    let count = u8::from(self.network_enabled);
                    self.reply(to_nes::NETWORK_SCAN_RESULT, &[count]);
                }
            }
            32 => {
                let count = u8::from(self.network_enabled);
                self.reply(to_nes::NETWORK_SCAN_RESULT, &[count]);
            }
            33 => {
                if self.network_enabled && arg(0) == Some(0) {
                    let mut body = vec![0, 0xC4, 0, 0, 0, 1, 0];
                    body.extend_from_slice(&pstr(SSID));
                    self.reply(to_nes::NETWORK_SCANNED_DETAILS, &body);
                } else {
                    self.reply(to_nes::NETWORK_SCANNED_DETAILS, &[]);
                }
            }
            34 => {
                let slots = self.networks.clone().map(|n| u8::from(!n.ssid.is_empty()));
                self.reply(to_nes::NETWORK_REGISTERED, &slots);
            }
            35 => {
                if let Some(network) = arg(0).and_then(|slot| self.networks.get(usize::from(slot)))
                {
                    let mut body = vec![u8::from(network.active)];
                    body.extend_from_slice(&pstr(&network.ssid));
                    body.extend_from_slice(&pstr(&network.password));
                    self.reply(to_nes::NETWORK_REGISTERED_DETAILS, &body);
                } else {
                    self.reply(to_nes::NETWORK_REGISTERED_DETAILS, &[]);
                }
            }
            36 => self.register_network(args),
            37 => {
                if let Some(network) =
                    arg(0).and_then(|slot| self.networks.get_mut(usize::from(slot)))
                {
                    *network = Registered::default();
                }
            }
            38 => {
                if let Some(slot) = arg(0).map(usize::from).filter(|slot| *slot < 3) {
                    for (index, network) in self.networks.iter_mut().enumerate() {
                        network.active = index == slot;
                    }
                }
            }
            39..=54 => self.file_command(id, args),
            55 => self.udp_pool.clear(),
            56 | 57 => {
                if args.len() >= 3 {
                    let port = u16::from_be_bytes([args[0], args[1]]);
                    let length = usize::from(args[2]);
                    if let Some(ip) = args.get(3..3 + length) {
                        let entry = (ip.to_vec(), port);
                        if id == 56 {
                            if !self.udp_pool.contains(&entry) && self.udp_pool.len() < 16 {
                                self.udp_pool.push(entry);
                            }
                        } else {
                            self.udp_pool.retain(|e| *e != entry);
                        }
                    }
                }
            }
            58 => self.send_to_pool(args),
            // ICMP is not reachable from a user process without privileges: "start failed".
            61 | 63 => self.reply(to_nes::PING_STATUS, &[3]),
            62 => self.reply(to_nes::PING_RESULT, &[0]),
            // DEBUG_LOG (3) and WIFI_CONNECT (60, the connection is permanent here) do nothing.
            _ => {}
        }
    }

    fn settings_body(server: &Server) -> Vec<u8> {
        let mut body = server.port.to_be_bytes().to_vec();
        body.extend_from_slice(&pstr(&server.host));
        body
    }

    /// `SERVER_SET_SETTINGS`: port, then a length-prefixed host; a message of the identifier
    /// alone clears them.
    fn parse_settings(args: &[u8]) -> Server {
        if args.len() < 3 {
            return Server::default();
        }
        let length = usize::from(args[2]).min(64);
        Server {
            port: u16::from_be_bytes([args[0], args[1]]),
            host: args.get(3..3 + length).unwrap_or(&args[3..]).to_vec(),
        }
    }

    fn register_network(&mut self, args: &[u8]) {
        let (Some(&slot), Some(&active), Some(&ssid_length)) =
            (args.first(), args.get(1), args.get(2))
        else {
            return;
        };
        let ssid_end = 3 + usize::from(ssid_length).min(32);
        let Some(ssid) = args.get(3..ssid_end) else {
            return;
        };
        let password = args.get(ssid_end).map_or(&[][..], |&length| {
            let end = ssid_end + 1 + usize::from(length).min(64);
            args.get(ssid_end + 1..end).unwrap_or(&[])
        });
        if usize::from(slot) >= 3 {
            return;
        }
        if active != 0 {
            for network in &mut self.networks {
                network.active = false;
            }
        }
        self.networks[usize::from(slot)] = Registered {
            active: active != 0,
            ssid: ssid.to_vec(),
            password: password.to_vec(),
        };
    }

    // ------------------------------------------------------------------ files

    /// The key of the file a FILE_OPEN-shaped request names, and the bytes `FILE_STATUS` echoes:
    /// `config`, then the auto-mode path and index or the manual-mode string.
    fn file_key(args: &[u8]) -> Option<(String, Vec<u8>)> {
        let config = *args.first()?;
        let drive = (config >> 1) & 0x01;
        if config & 0x01 == 0 {
            let path = *args.get(1)?;
            let index = *args.get(2)?;
            if path > 2 || index >= AUTO_FILES {
                return None;
            }
            Some((
                format!("{drive}/auto/{path}/{index}"),
                vec![config, path, index],
            ))
        } else {
            let length = usize::from(*args.get(1)?);
            let name = args.get(2..2 + length)?;
            let text = String::from_utf8_lossy(name)
                .trim_matches('/')
                .to_ascii_uppercase();
            let mut descriptor = vec![config, u8::try_from(length).ok()?];
            descriptor.extend_from_slice(name);
            Some((format!("{drive}/manual/{text}"), descriptor))
        }
    }

    /// The directory prefix a FILE_COUNT / FILE_GET_LIST request names.
    fn folder_key(config: u8, args: &[u8]) -> Option<String> {
        let drive = (config >> 1) & 0x01;
        if config & 0x01 == 0 {
            let path = *args.first()?;
            Some(format!("{drive}/auto/{path}/"))
        } else {
            let length = usize::from(*args.first()?);
            let name = args.get(1..1 + length)?;
            let text = String::from_utf8_lossy(name)
                .trim_matches('/')
                .to_ascii_uppercase();
            Some(if text.is_empty() {
                format!("{drive}/manual/")
            } else {
                format!("{drive}/manual/{text}/")
            })
        }
    }

    #[allow(clippy::too_many_lines)] // One arm per file command.
    fn file_command(&mut self, id: u8, args: &[u8]) {
        match id {
            39 => {
                if let Some((key, descriptor)) = Self::file_key(args) {
                    self.files.entry(key.clone()).or_default();
                    self.open = Some(OpenFile {
                        key,
                        descriptor,
                        cursor: 0,
                    });
                }
            }
            40 => self.open = None,
            41 => {
                let mut body = vec![u8::from(self.open.is_some())];
                if let Some(open) = &self.open {
                    body.extend_from_slice(&open.descriptor);
                }
                self.reply(to_nes::FILE_STATUS, &body);
            }
            42 => {
                let exists =
                    Self::file_key(args).is_some_and(|(key, _)| self.files.contains_key(&key));
                self.reply(to_nes::FILE_EXISTS, &[u8::from(exists)]);
            }
            43 => {
                let result = match Self::file_key(args) {
                    None => 3,
                    Some((key, _)) => {
                        if self.files.remove(&key).is_some() {
                            if self.open.as_ref().is_some_and(|open| open.key == key) {
                                self.open = None;
                            }
                            0
                        } else {
                            2
                        }
                    }
                };
                self.reply(to_nes::FILE_DELETE, &[result]);
            }
            44 => {
                let mut offset = [0u8; 4];
                for (slot, byte) in offset.iter_mut().zip(args) {
                    *slot = *byte;
                }
                let offset = usize::try_from(u32::from_le_bytes(offset)).unwrap_or(0);
                if let Some(open) = &mut self.open {
                    open.cursor = offset;
                    let file = self.files.entry(open.key.clone()).or_default();
                    if file.len() < offset {
                        file.resize(offset, 0);
                    }
                }
            }
            45 => {
                let wanted = usize::from(args.first().copied().unwrap_or(0)).min(253);
                let mut data = Vec::new();
                if let Some(open) = &mut self.open {
                    if let Some(file) = self.files.get(&open.key) {
                        let start = open.cursor.min(file.len());
                        let end = (start + wanted).min(file.len());
                        data.extend_from_slice(&file[start..end]);
                        open.cursor = end;
                    }
                }
                let mut body = vec![u8::try_from(data.len()).unwrap_or(0)];
                body.extend_from_slice(&data);
                self.reply(to_nes::FILE_DATA, &body);
            }
            46 | 47 => {
                if let Some(open) = &mut self.open {
                    let file = self.files.entry(open.key.clone()).or_default();
                    if id == 46 {
                        let end = open.cursor + args.len();
                        if file.len() < end {
                            file.resize(end, 0);
                        }
                        file[open.cursor..end].copy_from_slice(args);
                        open.cursor = end;
                    } else {
                        file.extend_from_slice(args);
                    }
                }
            }
            48 => {
                let count = args
                    .split_first()
                    .and_then(|(&config, rest)| Self::folder_key(config, rest))
                    .map_or(0, |prefix| self.children(&prefix).0.len());
                self.reply(to_nes::FILE_COUNT, &[u8::try_from(count).unwrap_or(0xFF)]);
            }
            49 => self.list_files(args),
            50 => {
                let drive = args.first().copied().unwrap_or(0).min(1);
                let path = args.get(1).copied().unwrap_or(0);
                let free = (0..AUTO_FILES).find(|index| {
                    !self
                        .files
                        .contains_key(&format!("{drive}/auto/{path}/{index}"))
                });
                match free {
                    Some(index) => self.reply(to_nes::FILE_ID, &[index]),
                    None => self.reply(to_nes::FILE_ID, &[]),
                }
            }
            51 => {
                let drive = (args.first().copied().unwrap_or(0) >> 1) & 0x01;
                let prefix = format!("{drive}/");
                let used: u64 = self
                    .files
                    .iter()
                    .filter(|(key, _)| key.starts_with(&prefix))
                    .map(|(_, data)| u64::try_from(data.len()).unwrap_or(0))
                    .sum();
                let used = used.min(DRIVE_SIZE);
                let free = DRIVE_SIZE - used;
                let percent = |part: u64| u8::try_from(part * 100 / DRIVE_SIZE).unwrap_or(100);
                let mut body = DRIVE_SIZE.to_be_bytes().to_vec();
                body.extend_from_slice(&free.to_be_bytes());
                body.push(percent(free));
                body.extend_from_slice(&used.to_be_bytes());
                body.push(percent(used));
                self.reply(to_nes::FILE_FS_INFO, &body);
            }
            52 => {
                let found = Self::file_key(args).and_then(|(key, _)| self.files.get(&key));
                match found {
                    Some(data) => {
                        let mut body = crc32(data).to_be_bytes().to_vec();
                        body.extend_from_slice(
                            &u32::try_from(data.len()).unwrap_or(u32::MAX).to_be_bytes(),
                        );
                        self.reply(to_nes::FILE_INFO, &body);
                    }
                    None => self.reply(to_nes::FILE_INFO, &[]),
                }
            }
            53 => self.download(args),
            54 => {
                let prefix = format!("{}/", args.first().copied().unwrap_or(0).min(1));
                self.files.retain(|key, _| !key.starts_with(&prefix));
                if self
                    .open
                    .as_ref()
                    .is_some_and(|open| open.key.starts_with(&prefix))
                {
                    self.open = None;
                }
            }
            _ => {}
        }
    }

    /// The direct children of `prefix`: files and folders.
    fn children(&self, prefix: &str) -> (Vec<String>, Vec<String>) {
        let mut files = Vec::new();
        let mut folders: Vec<String> = Vec::new();
        for key in self.files.keys().filter(|key| key.starts_with(prefix)) {
            let rest = &key[prefix.len()..];
            match rest.split_once('/') {
                Some((folder, _)) => {
                    if folders.last().map(String::as_str) != Some(folder) {
                        folders.push(folder.to_owned());
                    }
                }
                None => files.push(rest.to_owned()),
            }
        }
        (files, folders)
    }

    fn list_files(&mut self, args: &[u8]) {
        let Some((&config, rest)) = args.split_first() else {
            self.reply(to_nes::FILE_LIST, &[0]);
            return;
        };
        if config & 0x01 == 0 {
            let Some(prefix) = Self::folder_key(config, rest) else {
                self.reply(to_nes::FILE_LIST, &[0]);
                return;
            };
            let mut indices: Vec<u8> = self
                .children(&prefix)
                .0
                .iter()
                .filter_map(|name| name.parse().ok())
                .collect();
            indices.sort_unstable();
            let indices = Self::page(&indices, rest.get(1), rest.get(2), 64);
            let mut body = vec![u8::try_from(indices.len()).unwrap_or(0)];
            body.extend_from_slice(&indices);
            self.reply(to_nes::FILE_LIST, &body);
            return;
        }
        let kind = rest.first().copied().unwrap_or(0);
        let Some(prefix) = Self::folder_key(config, rest.get(1..).unwrap_or(&[])) else {
            self.reply(to_nes::FILE_LIST, &[0]);
            return;
        };
        let length = usize::from(rest.get(1).copied().unwrap_or(0));
        let (files, folders) = self.children(&prefix);
        let mut items: Vec<(bool, String)> = Vec::new();
        if kind != 0 {
            items.extend(folders.into_iter().map(|name| (true, name)));
        }
        if kind != 1 {
            items.extend(files.into_iter().map(|name| (false, name)));
        }
        let items = Self::page(&items, rest.get(2 + length), rest.get(3 + length), 19);
        let mut body = vec![u8::try_from(items.len()).unwrap_or(0)];
        for (folder, name) in items {
            let name = &name.as_bytes()[..name.len().min(127)];
            body.push(u8::try_from(name.len()).unwrap_or(0) | if folder { 0x80 } else { 0 });
            body.extend_from_slice(name);
        }
        self.reply(to_nes::FILE_LIST, &body);
    }

    fn page<T: Clone>(items: &[T], size: Option<&u8>, page: Option<&u8>, cap: usize) -> Vec<T> {
        let size = size.map_or(cap, |size| usize::from(*size).clamp(1, cap));
        let start = page.map_or(0, |page| usize::from(*page) * size);
        items.iter().skip(start).take(size).cloned().collect()
    }

    // ------------------------------------------------------------------ network

    /// `SERVER_CONNECT`: TCP connects to the server, UDP opens a socket on a random port of
    /// 49152-65535. Nothing happens while the option is off.
    fn connect(&mut self) {
        self.disconnect();
        if !self.network_enabled || self.server.host.is_empty() {
            return;
        }
        let host = String::from_utf8_lossy(&self.server.host).into_owned();
        let port = self.server.port;
        match self.protocol {
            0 => self.net.tcp = Some(Self::open_tcp(host, port)),
            2 | 3 => {
                let local = u16::try_from(self.random_in(49152, 65535)).unwrap_or(49152);
                let socket = UdpSocket::bind(("0.0.0.0", local))
                    .or_else(|_| UdpSocket::bind(("0.0.0.0", 0)));
                if let Ok(socket) = socket {
                    if socket.set_nonblocking(true).is_ok() {
                        self.net.udp = Some(socket);
                        self.connected = true;
                    }
                }
            }
            // TCP_S needs TLS, which this build does not carry (FIDELITY-LEADS J36).
            _ => {}
        }
    }

    fn open_tcp(host: String, port: u16) -> Link {
        let (event_tx, incoming) = mpsc::channel();
        let (outgoing, out_rx) = mpsc::channel::<Vec<u8>>();
        let stream = std::sync::Arc::new(std::sync::Mutex::new(None));
        let shared = std::sync::Arc::clone(&stream);
        std::thread::spawn(move || {
            let connected = (host.as_str(), port)
                .to_socket_addrs()
                .ok()
                .and_then(|mut addresses| addresses.next())
                .and_then(|address| {
                    TcpStream::connect_timeout(&address, Duration::from_secs(5)).ok()
                });
            let Some(mut writer) = connected else {
                let _ = event_tx.send(NetEvent::Closed);
                return;
            };
            let _ = writer.set_nodelay(true);
            let Ok(mut reader) = writer.try_clone() else {
                let _ = event_tx.send(NetEvent::Closed);
                return;
            };
            if let Ok(mut slot) = shared.lock() {
                *slot = writer.try_clone().ok();
            }
            let _ = event_tx.send(NetEvent::Connected);
            let reader_tx = event_tx.clone();
            std::thread::spawn(move || {
                let mut length = [0u8; 1];
                loop {
                    if reader.read_exact(&mut length).is_err() {
                        let _ = reader_tx.send(NetEvent::Closed);
                        return;
                    }
                    let mut payload = vec![0u8; usize::from(length[0])];
                    if reader.read_exact(&mut payload).is_err() {
                        let _ = reader_tx.send(NetEvent::Closed);
                        return;
                    }
                    if reader_tx.send(NetEvent::Message(payload)).is_err() {
                        return;
                    }
                }
            });
            while let Ok(data) = out_rx.recv() {
                if writer.write_all(&data).is_err() {
                    return;
                }
            }
        });
        Link {
            outgoing: Some(outgoing),
            incoming,
            stream,
        }
    }

    fn disconnect(&mut self) {
        if let Some(mut link) = self.net.tcp.take() {
            link.outgoing = None;
            if let Ok(slot) = link.stream.lock() {
                if let Some(stream) = slot.as_ref() {
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                }
            }
        }
        self.net.udp = None;
        self.connected = false;
    }

    /// `SERVER_SEND_MESSAGE`: framed `[length][payload]` on TCP, one datagram on UDP.
    fn send_to_server(&mut self, data: &[u8]) {
        if let Some(link) = &self.net.tcp {
            if let Some(sender) = &link.outgoing {
                let mut framed = vec![u8::try_from(data.len()).unwrap_or(0xFF)];
                framed.extend_from_slice(data);
                let _ = sender.send(framed);
            }
        } else if let Some(socket) = &self.net.udp {
            let host = String::from_utf8_lossy(&self.server.host).into_owned();
            let _ = socket.send_to(data, (host.as_str(), self.server.port));
        }
    }

    fn send_to_pool(&mut self, data: &[u8]) {
        if let Some(socket) = &self.net.udp {
            for (ip, port) in &self.udp_pool {
                let ip = String::from_utf8_lossy(ip).into_owned();
                let _ = socket.send_to(data, (ip.as_str(), *port));
            }
        }
    }

    /// `FILE_DOWNLOAD`: plain HTTP on a worker thread, the answer when it completes. HTTPS and
    /// everything with the option off answer at once.
    fn download(&mut self, args: &[u8]) {
        let Some((&config, rest)) = args.split_first() else {
            self.reply(to_nes::FILE_DOWNLOAD, &[6, 0, 0]);
            return;
        };
        let length = usize::from(rest.first().copied().unwrap_or(0));
        let (Some(url), Some(destination)) = (rest.get(1..1 + length), rest.get(1 + length..))
        else {
            self.reply(to_nes::FILE_DOWNLOAD, &[6, 0, 0]);
            return;
        };
        let mut request = vec![config];
        request.extend_from_slice(destination);
        let Some((key, _)) = Self::file_key(&request) else {
            self.reply(to_nes::FILE_DOWNLOAD, &[1, 0, 0]);
            return;
        };
        let url = String::from_utf8_lossy(url).into_owned();
        if !self.network_enabled {
            self.reply(to_nes::FILE_DOWNLOAD, &[8, 0, 0]);
            return;
        }
        let Some(rest) = url.strip_prefix("http://") else {
            self.reply(to_nes::FILE_DOWNLOAD, &[3, 0, 0]);
            return;
        };
        let (authority, path) = rest
            .split_once('/')
            .map_or((rest, "/".to_owned()), |(a, p)| (a, format!("/{p}")));
        let (host, port) = authority
            .rsplit_once(':')
            .and_then(|(h, p)| p.parse().ok().map(|p| (h.to_owned(), p)))
            .unwrap_or_else(|| (authority.to_owned(), 80));
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = Self::http_get(&host, port, &path);
            let event = match result {
                Some((status, body)) => {
                    let code = if matches!(status, 200 | 203 | 204) {
                        0
                    } else {
                        5
                    };
                    NetEvent::Download(code, status, Some(body), key)
                }
                None => NetEvent::Download(8, 0, None, key),
            };
            let _ = tx.send(event);
        });
        self.net.downloads.push(rx);
    }

    fn http_get(host: &str, port: u16, path: &str) -> Option<(u16, Vec<u8>)> {
        let address = (host, port).to_socket_addrs().ok()?.next()?;
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_secs(5)).ok()?;
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .ok()?;
        let request = format!("GET {path} HTTP/1.0\r\nHost: {host}\r\nConnection: close\r\n\r\n");
        stream.write_all(request.as_bytes()).ok()?;
        let mut response = Vec::new();
        stream.read_to_end(&mut response).ok()?;
        let split = response.windows(4).position(|w| w == b"\r\n\r\n")?;
        let head = String::from_utf8_lossy(&response[..split]);
        let status = head.split_whitespace().nth(1)?.parse().ok()?;
        Some((status, response[split + 4..].to_vec()))
    }

    /// Collects what the host network has received; the board calls it every few thousand CPU
    /// cycles while [`Esp::polls`].
    pub(crate) fn poll(&mut self) {
        let mut events = Vec::new();
        if let Some(link) = &self.net.tcp {
            loop {
                match link.incoming.try_recv() {
                    Ok(event) => events.push(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        events.push(NetEvent::Closed);
                        break;
                    }
                }
            }
        }
        if let Some(socket) = &self.net.udp {
            let mut buffer = [0u8; 254];
            while let Ok((length, _)) = socket.recv_from(&mut buffer) {
                events.push(NetEvent::Message(buffer[..length].to_vec()));
            }
        }
        self.net.downloads.retain(|rx| match rx.try_recv() {
            Ok(event) => {
                events.push(event);
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => false,
        });
        for event in events {
            match event {
                NetEvent::Connected => self.connected = true,
                NetEvent::Message(payload) => self.reply(to_nes::MESSAGE_FROM_SERVER, &payload),
                NetEvent::Closed => {
                    self.net.tcp = None;
                    self.connected = false;
                }
                NetEvent::Download(code, status, body, key) => {
                    if let Some(body) = body {
                        self.files.insert(key, body);
                    }
                    let [high, low] = status.to_be_bytes();
                    self.reply(to_nes::FILE_DOWNLOAD, &[code, high, low]);
                }
            }
        }
    }

    // ------------------------------------------------------------------ state

    pub(crate) fn save_state(&self, writer: &mut StateWriter) {
        let bytes = |writer: &mut StateWriter, data: &[u8]| {
            writer.write_u32(u32::try_from(data.len()).unwrap_or(0));
            writer.write_raw(data);
        };
        writer.write_u32(u32::try_from(self.rx.len()).unwrap_or(0));
        for answer in &self.rx {
            bytes(writer, answer);
        }
        writer.write_u8(self.debug_level);
        writer.write_u8(self.wifi_config);
        writer.write_u64(self.rng);
        writer.write_u8(self.protocol);
        for server in [&self.server, &self.saved_server] {
            writer.write_u16(server.port);
            bytes(writer, &server.host);
        }
        for network in &self.networks {
            writer.write_bool(network.active);
            bytes(writer, &network.ssid);
            bytes(writer, &network.password);
        }
        writer.write_u32(u32::try_from(self.udp_pool.len()).unwrap_or(0));
        for (ip, port) in &self.udp_pool {
            bytes(writer, ip);
            writer.write_u16(*port);
        }
        writer.write_u32(u32::try_from(self.files.len()).unwrap_or(0));
        for (key, data) in &self.files {
            bytes(writer, key.as_bytes());
            bytes(writer, data);
        }
        match &self.open {
            Some(open) => {
                writer.write_bool(true);
                bytes(writer, open.key.as_bytes());
                bytes(writer, &open.descriptor);
                writer.write_u32(u32::try_from(open.cursor).unwrap_or(0));
            }
            None => writer.write_bool(false),
        }
    }

    /// The connection is not part of a state: loading one disconnects.
    pub(crate) fn load_state(&mut self, reader: &mut StateReader) -> Result<(), InvalidState> {
        fn bytes(reader: &mut StateReader) -> Result<Vec<u8>, InvalidState> {
            let length = usize::try_from(reader.read_u32()?).map_err(|_| {
                InvalidState::corrupt("Corrupt savestate: Rainbow ESP block.".to_owned())
            })?;
            if length > 16 * 1024 * 1024 {
                return Err(InvalidState::corrupt(
                    "Corrupt savestate: Rainbow ESP block.".to_owned(),
                ));
            }
            let mut data = vec![0u8; length];
            reader.read_into(&mut data)?;
            Ok(data)
        }
        let text = |data: Vec<u8>| {
            String::from_utf8(data).map_err(|_| {
                InvalidState::corrupt("Corrupt savestate: Rainbow ESP block.".to_owned())
            })
        };
        self.disconnect();
        self.rx.clear();
        for _ in 0..reader.read_u32()? {
            self.rx.push_back(bytes(reader)?);
        }
        self.debug_level = reader.read_u8()?;
        self.wifi_config = reader.read_u8()?;
        self.rng = reader.read_u64()?;
        self.protocol = reader.read_u8()?.min(3);
        for index in 0..2 {
            let port = reader.read_u16()?;
            let host = bytes(reader)?;
            let server = Server { port, host };
            if index == 0 {
                self.server = server;
            } else {
                self.saved_server = server;
            }
        }
        for network in &mut self.networks {
            network.active = reader.read_bool()?;
            network.ssid = bytes(reader)?;
            network.password = bytes(reader)?;
        }
        self.udp_pool.clear();
        for _ in 0..reader.read_u32()? {
            let ip = bytes(reader)?;
            let port = reader.read_u16()?;
            self.udp_pool.push((ip, port));
        }
        self.files.clear();
        for _ in 0..reader.read_u32()? {
            let key = text(bytes(reader)?)?;
            let data = bytes(reader)?;
            self.files.insert(key, data);
        }
        self.open = if reader.read_bool()? {
            let key = text(bytes(reader)?)?;
            let descriptor = bytes(reader)?;
            let cursor = usize::try_from(reader.read_u32()?).map_err(|_| {
                InvalidState::corrupt("Corrupt savestate: Rainbow ESP block.".to_owned())
            })?;
            Some(OpenFile {
                key,
                descriptor,
                cursor,
            })
        } else {
            None
        };
        Ok(())
    }
}

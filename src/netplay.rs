use std::collections::VecDeque;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

// Packet magic bytes
const MAGIC_INPUT: [u8; 2] = [0x4E, 0x50]; // "NP" - input packet
const MAGIC_HOST: [u8; 2] = [0x4E, 0x48];  // "NH" - host welcome
const MAGIC_JOIN: [u8; 2] = [0x4E, 0x43];  // "NC" - client join request
const MAGIC_ACCEPT: [u8; 2] = [0x4E, 0x41]; // "NA" - host accept
const MAGIC_KEEPALIVE: [u8; 2] = [0x4E, 0x4B]; // "NK" - keepalive heartbeat
const KEEPALIVE_INTERVAL: Duration = Duration::from_secs(2);

const INPUT_PACKET_SIZE: usize = 15;// 2 magic + 8 frame + 1 input + 4 checksum

#[derive(Debug, PartialEq)]
pub enum NetplayState {
    Disconnected,
    Hosting { port: u16 },
    Connecting,
    Connected,
    Desynced,
}

pub struct NetplaySession {
    socket: Option<UdpSocket>,
    peer_addr: Option<SocketAddr>,
    pub port: u16,
    pub local_player: u8,
    pub input_delay: u8,
    local_inputs: VecDeque<u8>,
    remote_inputs: VecDeque<u8>,
    pub frame_num: u64,
    pub state: NetplayState,
    last_recv_frame: u64,
    pub ping_ms: u32,
    ping_sent: Option<Instant>,
    last_remote_input: u8,
    pub last_keepalive: Instant,
}

impl NetplaySession {
    pub fn new() -> Self {
        Self {
            socket: None,
            peer_addr: None,
            port: 7777,
            local_player: 0,
            input_delay: 2,
            local_inputs: VecDeque::new(),
            remote_inputs: VecDeque::new(),
            frame_num: 0,
            state: NetplayState::Disconnected,
            last_recv_frame: 0,
            ping_ms: 0,
            ping_sent: None,
            last_remote_input: 0,
            last_keepalive: Instant::now(),
        }
    }
}

impl Default for NetplaySession {
    fn default() -> Self {
        Self::new()
    }
}

impl NetplaySession {
    pub fn host(&mut self) -> Result<(), String> {
        let addr: SocketAddr = format!("0.0.0.0:{}", self.port)
            .parse()
            .map_err(|e| format!("Invalid address: {}", e))?;
        let socket = UdpSocket::bind(addr).map_err(|e| format!("Bind failed: {}", e))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| format!("Non-blocking failed: {}", e))?;
        self.socket = Some(socket);
        self.local_player = 0; // Host is P1
        self.state = NetplayState::Hosting { port: self.port };
        self.frame_num = 0;
        self.local_inputs.clear();
        self.remote_inputs.clear();
        Ok(())
    }

    pub fn join(&mut self, addr: &str) -> Result<(), String> {
        let peer: SocketAddr = addr.parse().map_err(|e| format!("Invalid address: {}", e))?;
        let socket =
            UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Bind failed: {}", e))?;
        socket
            .set_nonblocking(true)
            .map_err(|e| format!("Non-blocking failed: {}", e))?;
        // Send join request
        socket
            .send_to(&MAGIC_JOIN, peer)
            .map_err(|e| format!("Send failed: {}", e))?;
        self.socket = Some(socket);
        self.peer_addr = Some(peer);
        self.local_player = 1; // Client is P2
        self.state = NetplayState::Connecting;
        self.frame_num = 0;
        self.ping_sent = Some(Instant::now());
        self.local_inputs.clear();
        self.remote_inputs.clear();
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.socket = None;
        self.peer_addr = None;
        self.state = NetplayState::Disconnected;
        self.frame_num = 0;
        self.last_recv_frame = 0;
        self.ping_ms = 0;
        self.ping_sent = None;
        self.local_inputs.clear();
        self.remote_inputs.clear();
        self.last_remote_input = 0;
    }

    pub fn send_input(&mut self, frame: u64, input: u8, checksum: u32) {
        let socket = match &self.socket {
            Some(s) => s,
            None => return,
        };
        let peer = match &self.peer_addr {
            Some(a) => *a,
            None => return,
        };

        let mut buf = [0u8; INPUT_PACKET_SIZE];
        buf[0] = MAGIC_INPUT[0];
        buf[1] = MAGIC_INPUT[1];
        buf[2..10].copy_from_slice(&frame.to_le_bytes());
        buf[10] = input;
        buf[11..15].copy_from_slice(&checksum.to_le_bytes());

        let _ = socket.send_to(&buf, peer);
        self.local_inputs.push_back(input);
        // Keep buffer bounded
        if self.local_inputs.len() > 120 {
            self.local_inputs.pop_front();
        }
    }

    pub fn receive_input(&mut self) -> Option<u8> {
        // Return buffered remote input first (one per frame)
        if let Some(input) = self.remote_inputs.pop_front() {
            self.last_remote_input = input;
            return Some(input);
        }

        self.socket.as_ref()?;

        let mut buf = [0u8; 64];
        let mut packets: Vec<(Vec<u8>, SocketAddr)> = Vec::new();

        // Drain all pending packets (borrow socket briefly)
        if let Some(ref socket) = self.socket {
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((len, src)) => {
                        packets.push((buf[..len].to_vec(), src));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }

        // Process collected packets (now safe to mutably borrow self)
        for (data, src) in packets {
            // Always process handshake packets
            self.handle_packet(&data, src);

            if data.len() == INPUT_PACKET_SIZE
                && data[0] == MAGIC_INPUT[0]
                && data[1] == MAGIC_INPUT[1]
            {
                // Reject input from unknown sources after handshake
                if let Some(expected) = &self.peer_addr {
                    if src != *expected {
                        continue;
                    }
                }
                self.remote_inputs.push_back(data[10]);
            }
        }

        // Return first buffered input
        if let Some(input) = self.remote_inputs.pop_front() {
            self.last_remote_input = input;
            Some(input)
        } else {
            None
        }
    }

    /// Returns the last known remote input for prediction when no new data arrives.
    pub fn last_remote_input(&self) -> u8 {
        self.last_remote_input
    }

    pub fn should_send_keepalive(&self) -> bool {
        self.last_keepalive.elapsed() >= KEEPALIVE_INTERVAL
    }

    pub fn send_keepalive(&mut self) {
        if self.is_connected() || matches!(self.state, NetplayState::Hosting { .. }) {
            if let (Some(socket), Some(peer)) = (&self.socket, &self.peer_addr) {
                let _ = socket.send_to(&MAGIC_KEEPALIVE, peer);
            }
        }
        self.last_keepalive = Instant::now();
    }

    fn handle_packet(&mut self, data: &[u8], src: SocketAddr) {
        if data.len() < 2 {
            return;
        }

        if data[0] == MAGIC_KEEPALIVE[0] && data[1] == MAGIC_KEEPALIVE[1] {
            return; // Keepalive acknowledged, no action needed
        }

        match (data[0], data[1]) {
            // Client join request (host receives this)
            (0x4E, 0x43) => {
                if let NetplayState::Hosting { .. } = &self.state {
                    self.peer_addr = Some(src);
                    // Send accept
                    if let Some(ref socket) = self.socket {
                        let _ = socket.send_to(&MAGIC_ACCEPT, src);
                        // Also send host welcome
                        let _ = socket.send_to(&MAGIC_HOST, src);
                    }
                    self.state = NetplayState::Connected;
                    self.frame_num = 0;
                    // Measure initial ping
                    self.ping_sent = Some(Instant::now());
                }
            }
            // Host accept (client receives this)
            (0x4E, 0x41) => {
                if self.state == NetplayState::Connecting {
                    self.state = NetplayState::Connected;
                    self.frame_num = 0;
                    if let Some(sent) = self.ping_sent.take() {
                        self.ping_ms = sent.elapsed().as_millis() as u32;
                    }
                }
            }
            // Host welcome (client receives this)
            (0x4E, 0x48) => {
                // Already handled by accept above, ignore duplicates
            }
            // Input packet
            (0x4E, 0x50) => {
                if data.len() >= INPUT_PACKET_SIZE {
                    let frame = u64::from_le_bytes([
                        data[2], data[3], data[4], data[5], data[6], data[7], data[8], data[9],
                    ]);
                    if frame > self.last_recv_frame {
                        self.last_recv_frame = frame;
                        // Update ping estimate from frame cadence
                        if let Some(sent) = self.ping_sent.take() {
                            self.ping_ms = sent.elapsed().as_millis() as u32;
                        }
                        self.ping_sent = Some(Instant::now());
                    }
                }
            }
            _ => {}
        }
    }

    pub fn is_connected(&self) -> bool {
        self.state == NetplayState::Connected
    }

    /// Returns a status string for display.
    pub fn status_text(&self) -> &str {
        match &self.state {
            NetplayState::Disconnected => "DISCONNECTED",
            NetplayState::Hosting { .. } => "HOSTING... WAITING",
            NetplayState::Connecting => "CONNECTING...",
            NetplayState::Connected => "CONNECTED",
            NetplayState::Desynced => "DESYNCED!",
        }
    }

    /// Encode local input bits from individual button booleans.
    /// Bit layout matches NES joypad: A B Sel St U D L R
    #[allow(clippy::too_many_arguments)]
    pub fn encode_input(a: bool, b: bool, select: bool, start: bool, up: bool, down: bool, left: bool, right: bool) -> u8 {
        let mut bits = 0u8;
        if a { bits |= 1 << 0; }
        if b { bits |= 1 << 1; }
        if select { bits |= 1 << 2; }
        if start { bits |= 1 << 3; }
        if up { bits |= 1 << 4; }
        if down { bits |= 1 << 5; }
        if left { bits |= 1 << 6; }
        if right { bits |= 1 << 7; }
        bits
    }

    /// Decode input bits back to individual booleans.
    /// Returns (a, b, select, start, up, down, left, right)
    pub fn decode_input(bits: u8) -> (bool, bool, bool, bool, bool, bool, bool, bool) {
        (
            bits & (1 << 0) != 0,
            bits & (1 << 1) != 0,
            bits & (1 << 2) != 0,
            bits & (1 << 3) != 0,
            bits & (1 << 4) != 0,
            bits & (1 << 5) != 0,
            bits & (1 << 6) != 0,
            bits & (1 << 7) != 0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_session() {
        let session = NetplaySession::new();
        assert_eq!(session.state, NetplayState::Disconnected);
        assert_eq!(session.local_player, 0);
        assert_eq!(session.input_delay, 2);
        assert!(!session.is_connected());
    }

    #[test]
    fn test_encode_decode_input() {
        let bits = NetplaySession::encode_input(true, false, true, false, true, false, true, false);
        let (a, b, sel, st, u, d, l, r) = NetplaySession::decode_input(bits);
        assert!(a);
        assert!(!b);
        assert!(sel);
        assert!(!st);
        assert!(u);
        assert!(!d);
        assert!(l);
        assert!(!r);
    }

    #[test]
    fn test_encode_all_buttons() {
        let bits = NetplaySession::encode_input(true, true, true, true, true, true, true, true);
        assert_eq!(bits, 0xFF);
        let (a, b, sel, st, u, d, l, r) = NetplaySession::decode_input(0xFF);
        assert!(a && b && sel && st && u && d && l && r);
    }

    #[test]
    fn test_encode_no_buttons() {
        let bits = NetplaySession::encode_input(false, false, false, false, false, false, false, false);
        assert_eq!(bits, 0x00);
    }

    #[test]
    fn test_host_and_disconnect() {
        let mut session = NetplaySession::new();
        // Host on a random high port
        session.port = 0;
        let result = session.host(); // port 0 = OS picks
        assert!(result.is_ok());
        assert!(matches!(session.state, NetplayState::Hosting { .. }));
        assert_eq!(session.local_player, 0);

        session.disconnect();
        assert_eq!(session.state, NetplayState::Disconnected);
        assert!(session.socket.is_none());
    }

    #[test]
    fn test_status_text() {
        let mut session = NetplaySession::new();
        assert_eq!(session.status_text(), "DISCONNECTED");
        session.port = 0;
        let _ = session.host();
        assert_eq!(session.status_text(), "HOSTING... WAITING");
        session.disconnect();
    }

    #[test]
    fn test_loopback_connection() {
        // Host on random port
        let mut host = NetplaySession::new();
        host.port = 0;
        host.host().unwrap();
        let host_port = match host.socket.as_ref().unwrap().local_addr() {
            Ok(addr) => addr.port(),
            Err(_) => return, // Skip if we can't get port
        };

        // Client joins
        let mut client = NetplaySession::new();
        client.join(&format!("127.0.0.1:{}", host_port)).unwrap();
        assert_eq!(client.state, NetplayState::Connecting);

        // Give the OS a moment, then have host receive the join
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = host.receive_input();
        assert_eq!(host.state, NetplayState::Connected);

        // Client receives accept
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = client.receive_input();
        assert_eq!(client.state, NetplayState::Connected);

        // Exchange an input packet
        host.peer_addr = Some(format!("127.0.0.1:{}", client.socket.as_ref().unwrap().local_addr().unwrap().port()).parse().unwrap());
        host.send_input(1, 0b00000001, 0x12345678);

        std::thread::sleep(std::time::Duration::from_millis(50));
        let input = client.receive_input();
        assert_eq!(input, Some(0b00000001));

        host.disconnect();
        client.disconnect();
    }

    #[test]
    fn test_keepalive_timing() {
        let mut session = NetplaySession::new();
        assert!(!session.should_send_keepalive());
        session.last_keepalive = Instant::now() - Duration::from_secs(3);
        assert!(session.should_send_keepalive());
    }

    #[test]
    fn test_send_keepalive_resets_timer() {
        let mut session = NetplaySession::new();
        session.last_keepalive = Instant::now() - Duration::from_secs(3);
        assert!(session.should_send_keepalive());
        session.send_keepalive();
        assert!(!session.should_send_keepalive());
    }

    #[test]
    fn test_configurable_port() {
        let mut session = NetplaySession::new();
        assert_eq!(session.port, 7777);
        session.port = 8080;
        assert_eq!(session.port, 8080);
    }

    #[test]
    fn test_host_uses_configured_port() {
        let mut session = NetplaySession::new();
        session.port = 0;
        session.host().unwrap();
        match &session.state {
            NetplayState::Hosting { port } => assert_ne!(*port, 7777),
            other => panic!("Expected Hosting, got {:?}", other),
        }
        session.disconnect();
    }

}

use std::io::{Read, Write};

pub struct InputRecording {
    pub rom_hash: [u8; 32],
    pub frames: Vec<(u8, u8)>,
    pub state: RecordingState,
}

#[derive(PartialEq)]
pub enum RecordingState {
    Idle,
    Recording,
    Playing { frame_index: usize },
}

const MAGIC: &[u8; 4] = b"NREC";
const VERSION: u32 = 1;

impl InputRecording {
    pub fn new(rom_hash: [u8; 32]) -> Self {
        Self {
            rom_hash,
            frames: Vec::new(),
            state: RecordingState::Idle,
        }
    }

    pub fn start_recording(&mut self) {
        self.frames.clear();
        self.state = RecordingState::Recording;
    }

    pub fn stop_recording(&mut self) {
        self.state = RecordingState::Idle;
    }

    pub fn record_frame(&mut self, p1: u8, p2: u8) {
        if self.state == RecordingState::Recording {
            self.frames.push((p1, p2));
        }
    }

    pub fn start_playback(&mut self) {
        self.state = RecordingState::Playing { frame_index: 0 };
    }

    pub fn next_frame(&mut self) -> Option<(u8, u8)> {
        if let RecordingState::Playing { ref mut frame_index } = self.state {
            if *frame_index < self.frames.len() {
                let frame = self.frames[*frame_index];
                *frame_index += 1;
                Some(frame)
            } else {
                self.state = RecordingState::Idle;
                None
            }
        } else {
            None
        }
    }

    pub fn save_to_file(&self, path: &str) -> Result<(), String> {
        let mut file = std::fs::File::create(path).map_err(|e| format!("Create failed: {}", e))?;

        file.write_all(MAGIC).map_err(|e| format!("Write failed: {}", e))?;
        file.write_all(&VERSION.to_le_bytes()).map_err(|e| format!("Write failed: {}", e))?;
        file.write_all(&self.rom_hash).map_err(|e| format!("Write failed: {}", e))?;

        if self.frames.len() > u32::MAX as usize {
            return Err("Recording too long (exceeds u32 frame count)".to_string());
        }
        let frame_count = self.frames.len() as u32;
        file.write_all(&frame_count.to_le_bytes()).map_err(|e| format!("Write failed: {}", e))?;

        for &(p1, p2) in &self.frames {
            file.write_all(&[p1, p2]).map_err(|e| format!("Write failed: {}", e))?;
        }

        Ok(())
    }

    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let mut file = std::fs::File::open(path).map_err(|e| format!("Open failed: {}", e))?;

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic).map_err(|e| format!("Read failed: {}", e))?;
        if &magic != MAGIC {
            return Err("Invalid recording file (bad magic)".to_string());
        }

        let mut ver_bytes = [0u8; 4];
        file.read_exact(&mut ver_bytes).map_err(|e| format!("Read failed: {}", e))?;
        let version = u32::from_le_bytes(ver_bytes);
        if version != VERSION {
            return Err(format!("Unsupported recording version: {}", version));
        }

        let mut rom_hash = [0u8; 32];
        file.read_exact(&mut rom_hash).map_err(|e| format!("Read failed: {}", e))?;

        let mut count_bytes = [0u8; 4];
        file.read_exact(&mut count_bytes).map_err(|e| format!("Read failed: {}", e))?;
        let frame_count = u32::from_le_bytes(count_bytes);
        const MAX_RECORDING_FRAMES: u32 = 10_000_000; // ~46 hours at 60fps
        if frame_count > MAX_RECORDING_FRAMES {
            return Err(format!("Recording too large: {} frames", frame_count));
        }
        let frame_count = frame_count as usize;

        let mut frames = Vec::with_capacity(frame_count);
        for _ in 0..frame_count {
            let mut pair = [0u8; 2];
            file.read_exact(&mut pair).map_err(|e| format!("Read failed: {}", e))?;
            frames.push((pair[0], pair[1]));
        }

        Ok(Self {
            rom_hash,
            frames,
            state: RecordingState::Idle,
        })
    }

    /// Export to FCEUX .fm2 format.
    pub fn export_fm2(&self, path: &str, rom_name: &str) -> Result<(), String> {
        let mut file = std::fs::File::create(path).map_err(|e| format!("Create failed: {}", e))?;

        // FM2 header
        writeln!(file, "version 3").map_err(|e| format!("Write failed: {}", e))?;
        writeln!(file, "emuVersion 20500").map_err(|e| format!("Write failed: {}", e))?;
        writeln!(file, "romFilename {}", rom_name).map_err(|e| format!("Write failed: {}", e))?;

        // Each frame: |0|RLDUTSBA|........||
        // Button order: R L D U T(Start) S(Select) B A
        // Bit layout in our joypad byte: A=0, B=1, Select=2, Start=3, Up=4, Down=5, Left=6, Right=7
        for &(p1, p2) in &self.frames {
            let p1_str = buttons_to_fm2(p1);
            let p2_str = buttons_to_fm2(p2);
            writeln!(file, "|0|{}|{}||", p1_str, p2_str).map_err(|e| format!("Write failed: {}", e))?;
        }

        Ok(())
    }

    pub fn is_recording(&self) -> bool {
        self.state == RecordingState::Recording
    }

    pub fn is_playing(&self) -> bool {
        matches!(self.state, RecordingState::Playing { .. })
    }

    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }
}

/// Convert a joypad byte to FM2 button string (RLDUTSBA order).
/// Bit layout: A=0, B=1, Select=2, Start=3, Up=4, Down=5, Left=6, Right=7
fn buttons_to_fm2(buttons: u8) -> String {
    let mut s = String::with_capacity(8);
    s.push(if buttons & 0x80 != 0 { 'R' } else { '.' }); // Right = bit 7
    s.push(if buttons & 0x40 != 0 { 'L' } else { '.' }); // Left  = bit 6
    s.push(if buttons & 0x20 != 0 { 'D' } else { '.' }); // Down  = bit 5
    s.push(if buttons & 0x10 != 0 { 'U' } else { '.' }); // Up    = bit 4
    s.push(if buttons & 0x08 != 0 { 'T' } else { '.' }); // Start = bit 3
    s.push(if buttons & 0x04 != 0 { 'S' } else { '.' }); // Select= bit 2
    s.push(if buttons & 0x02 != 0 { 'B' } else { '.' }); // B     = bit 1
    s.push(if buttons & 0x01 != 0 { 'A' } else { '.' }); // A     = bit 0
    s
}

/// Compute SHA-256 of a byte slice (minimal implementation, no dependency).
pub fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
        0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
        0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
        0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
        0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
        0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
        0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
        0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let orig_len_bits = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len_bits.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(K[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut result = [0u8; 32];
    for (i, &val) in h.iter().enumerate() {
        result[i * 4..i * 4 + 4].copy_from_slice(&val.to_be_bytes());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_empty() {
        let hash = sha256(b"");
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
    }

    #[test]
    fn test_sha256_abc() {
        let hash = sha256(b"abc");
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        assert_eq!(hex, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn test_buttons_to_fm2() {
        // A pressed = bit 0
        assert_eq!(buttons_to_fm2(0x01), ".......A");
        // Right+A = bits 7+0
        assert_eq!(buttons_to_fm2(0x81), "R......A");
        // All buttons
        assert_eq!(buttons_to_fm2(0xFF), "RLDUTSBA");
        // Nothing
        assert_eq!(buttons_to_fm2(0x00), "........");
    }

    #[test]
    fn test_recording_lifecycle() {
        let mut rec = InputRecording::new([0u8; 32]);
        assert!(!rec.is_recording());
        assert!(!rec.is_playing());

        rec.start_recording();
        assert!(rec.is_recording());

        rec.record_frame(0x01, 0x00); // A pressed
        rec.record_frame(0x81, 0x00); // Right+A
        rec.record_frame(0x00, 0x00); // nothing
        rec.stop_recording();

        assert_eq!(rec.frame_count(), 3);

        rec.start_playback();
        assert!(rec.is_playing());
        assert_eq!(rec.next_frame(), Some((0x01, 0x00)));
        assert_eq!(rec.next_frame(), Some((0x81, 0x00)));
        assert_eq!(rec.next_frame(), Some((0x00, 0x00)));
        assert_eq!(rec.next_frame(), None);
        assert!(!rec.is_playing()); // Auto-stops
    }

    #[test]
    fn test_save_load_roundtrip() {
        let hash = sha256(b"test rom data");
        let mut rec = InputRecording::new(hash);
        rec.start_recording();
        rec.record_frame(0x01, 0x02);
        rec.record_frame(0x80, 0x40);
        rec.stop_recording();

        let dir = std::env::temp_dir().join("nes_test_recording");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.nrec");
        let path_str = path.to_str().unwrap();

        rec.save_to_file(path_str).unwrap();
        let loaded = InputRecording::load_from_file(path_str).unwrap();

        assert_eq!(loaded.rom_hash, hash);
        assert_eq!(loaded.frames.len(), 2);
        assert_eq!(loaded.frames[0], (0x01, 0x02));
        assert_eq!(loaded.frames[1], (0x80, 0x40));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_export_fm2() {
        let mut rec = InputRecording::new([0u8; 32]);
        rec.start_recording();
        rec.record_frame(0x01, 0x00); // A only
        rec.record_frame(0xFF, 0x00); // All buttons
        rec.stop_recording();

        let dir = std::env::temp_dir().join("nes_test_fm2");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("test.fm2");
        let path_str = path.to_str().unwrap();

        rec.export_fm2(path_str, "TestRom").unwrap();

        let content = std::fs::read_to_string(path_str).unwrap();
        assert!(content.contains("version 3"));
        assert!(content.contains("romFilename TestRom"));
        assert!(content.contains("|0|.......A|........||"));
        assert!(content.contains("|0|RLDUTSBA|........||"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_record_frame_ignored_when_idle() {
        let mut rec = InputRecording::new([0u8; 32]);
        rec.record_frame(0x01, 0x00);
        assert_eq!(rec.frame_count(), 0);
    }
}

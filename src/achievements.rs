use std::path::PathBuf;
use serde::{Serialize, Deserialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: u32,
    pub title: String,
    pub description: String,
    pub points: u32,
    pub conditions: String,
    pub unlocked: bool,
}

pub struct AchievementNotification {
    pub title: String,
    pub points: u32,
    pub frames_remaining: u32,
}

#[derive(Serialize, Deserialize)]
struct AchievementFile {
    game_title: String,
    achievements: Vec<AchievementDef>,
}

#[derive(Serialize, Deserialize)]
struct AchievementDef {
    id: u32,
    title: String,
    description: String,
    points: u32,
    conditions: String,
}

pub struct AchievementEngine {
    pub achievements: Vec<Achievement>,
    pub game_title: String,
    pub game_id: Option<u32>,
    pub notifications: Vec<AchievementNotification>,
    pub total_points: u32,
    pub unlocked_count: usize,
    enabled: bool,
    prev_ram: Vec<u8>,
    cache_path: Option<PathBuf>,
}

impl AchievementEngine {
    pub fn new() -> Self {
        Self {
            achievements: Vec::new(),
            game_title: String::new(),
            game_id: None,
            notifications: Vec::new(),
            total_points: 0,
            unlocked_count: 0,
            enabled: false,
            prev_ram: Vec::new(),
            cache_path: None,
        }
    }

    /// Load achievement definitions for a ROM identified by its MD5 hash.
    /// Looks for `~/.nes-emulator/achievements/{hash}.json`.
    pub fn load_for_rom(rom_md5: &str) -> Self {
        let mut engine = Self::new();

        let base = match dirs_base() {
            Some(p) => p.join("achievements"),
            None => return engine,
        };
        let path = base.join(format!("{}.json", rom_md5));

        let data = match std::fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return engine,
        };

        let file: AchievementFile = match serde_json::from_str(&data) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("[achievements] Failed to parse {}: {}", path.display(), e);
                return engine;
            }
        };

        engine.game_title = file.game_title;
        engine.achievements = file.achievements.into_iter().map(|a| Achievement {
            id: a.id,
            title: a.title,
            description: a.description,
            points: a.points,
            conditions: a.conditions,
            unlocked: false,
        }).collect();

        // Load unlock state from separate cache file
        let state_path = base.join(format!("{}_unlocked.json", rom_md5));
        if let Ok(state_data) = std::fs::read_to_string(&state_path) {
            if let Ok(unlocked_ids) = serde_json::from_str::<Vec<u32>>(&state_data) {
                let set: std::collections::HashSet<u32> = unlocked_ids.into_iter().collect();
                for ach in &mut engine.achievements {
                    if set.contains(&ach.id) {
                        ach.unlocked = true;
                    }
                }
            }
        }

        engine.unlocked_count = engine.achievements.iter().filter(|a| a.unlocked).count();
        engine.total_points = engine.achievements.iter().filter(|a| a.unlocked).map(|a| a.points).sum();
        engine.enabled = !engine.achievements.is_empty();
        engine.cache_path = Some(state_path);
        engine
    }

    /// Evaluate all locked achievements against current RAM state.
    pub fn check_frame(&mut self, ram: &[u8]) {
        if !self.enabled {
            self.prev_ram = ram.to_vec();
            return;
        }

        let mut to_unlock: Vec<u32> = Vec::new();

        for ach in &self.achievements {
            if ach.unlocked {
                continue;
            }
            if evaluate_conditions(&ach.conditions, ram, &self.prev_ram) {
                to_unlock.push(ach.id);
            }
        }

        for id in to_unlock {
            self.unlock(id);
        }

        self.prev_ram = ram.to_vec();
    }

    /// Mark achievement as unlocked, create notification, persist.
    pub fn unlock(&mut self, id: u32) {
        let ach = match self.achievements.iter_mut().find(|a| a.id == id) {
            Some(a) => a,
            None => return,
        };
        if ach.unlocked {
            return;
        }
        ach.unlocked = true;
        self.unlocked_count += 1;
        self.total_points += ach.points;

        self.notifications.push(AchievementNotification {
            title: ach.title.clone(),
            points: ach.points,
            frames_remaining: 180, // 3 seconds at 60fps
        });

        self.save_unlock_state();
    }

    /// Return active notifications, decrement timers, remove expired.
    pub fn tick_notifications(&mut self) {
        for n in &mut self.notifications {
            if n.frames_remaining > 0 {
                n.frames_remaining -= 1;
            }
        }
        self.notifications.retain(|n| n.frames_remaining > 0);
    }

    /// Get a snapshot of active notifications for rendering.
    pub fn active_notifications(&self) -> impl Iterator<Item = &AchievementNotification> {
        self.notifications.iter().filter(|n| n.frames_remaining > 0)
    }

    fn save_unlock_state(&self) {
        if let Some(ref path) = self.cache_path {
            let unlocked_ids: Vec<u32> = self.achievements.iter()
                .filter(|a| a.unlocked)
                .map(|a| a.id)
                .collect();
            if let Ok(json) = serde_json::to_string_pretty(&unlocked_ids) {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(path, json);
            }
        }
    }
}

/// Evaluate a RetroAchievements-style condition string against RAM.
/// Supports basic conditions joined by `_S_` (AND):
///   `0xHADDR=VALUE`  — byte equals value
///   `0xHADDR>VALUE`  — byte greater than value
///   `0xHADDR<VALUE`  — byte less than value
///   `d0xHADDR!=0xHADDR` — value changed from previous frame (delta)
fn evaluate_conditions(conditions: &str, ram: &[u8], prev_ram: &[u8]) -> bool {
    if conditions.is_empty() {
        return false;
    }
    let parts: Vec<&str> = conditions.split("_S_").collect();
    for part in parts {
        let part = part.trim();
        if !evaluate_single(part, ram, prev_ram) {
            return false;
        }
    }
    true
}

fn evaluate_single(cond: &str, ram: &[u8], prev_ram: &[u8]) -> bool {
    // Delta condition: d0xHADDR!=0xHADDR
    if cond.starts_with("d0xH") || cond.starts_with("d0xh") {
        if let Some(pos) = cond.find("!=") {
            let addr_str = &cond[4..pos];
            if let Ok(addr) = u16::from_str_radix(addr_str, 16) {
                let a = addr as usize;
                let cur = ram.get(a).copied().unwrap_or(0);
                let prev = prev_ram.get(a).copied().unwrap_or(0);
                return cur != prev;
            }
        }
        return false; // Unrecognized delta format
    }

    // Standard conditions: 0xHADDR op VALUE
    if cond.starts_with("0xH") || cond.starts_with("0xh") {
        // Find operator (check multi-char operators first, longest-to-shortest)
        let (op_pos, op, op_len) = if let Some(p) = cond.find(">=") {
            (p, ">=", 2)
        } else if let Some(p) = cond.find("<=") {
            (p, "<=", 2)
        } else if let Some(p) = cond.find("!=") {
            (p, "!=", 2)
        } else if let Some(p) = cond.find('>') {
            (p, ">", 1)
        } else if let Some(p) = cond.find('<') {
            (p, "<", 1)
        } else if let Some(p) = cond.find('=') {
            (p, "=", 1)
        } else {
            return false; // No operator found
        };

        let addr_str = &cond[3..op_pos];
        let val_str = &cond[op_pos + op_len..];

        let addr = match u16::from_str_radix(addr_str, 16) {
            Ok(a) => a as usize,
            Err(_) => return false,
        };
        let expected = if val_str.starts_with("0x") || val_str.starts_with("0X") {
            u8::from_str_radix(&val_str[2..], 16).unwrap_or(0)
        } else {
            val_str.parse::<u8>().unwrap_or(0)
        };

        let actual = ram.get(addr).copied().unwrap_or(0);

        match op {
            "=" => actual == expected,
            "!=" => actual != expected,
            ">" => actual > expected,
            "<" => actual < expected,
            ">=" => actual >= expected,
            "<=" => actual <= expected,
            _ => false,
        }
    } else {
        false // Unsupported condition type, skip gracefully
    }
}

/// Return base config directory: ~/.nes-emulator
fn dirs_base() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(|p| PathBuf::from(p).join(".nes-emulator"))
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(|p| PathBuf::from(p).join(".nes-emulator"))
    }
}

/// Compute MD5 hex digest of a byte slice (minimal implementation, no dependency).
pub fn md5_hex(data: &[u8]) -> String {
    // Minimal MD5 implementation for ROM hashing
    let mut state: [u32; 4] = [0x67452301, 0xefcdab89, 0x98badcfe, 0x10325476];

    let orig_len_bits = (data.len() as u64) * 8;

    // Padding
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&orig_len_bits.to_le_bytes());

    // Constants
    const S: [u32; 64] = [
        7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22,
        5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20,
        4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23,
        6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
    ];
    const K: [u32; 64] = [
        0xd76aa478, 0xe8c7b756, 0x242070db, 0xc1bdceee,
        0xf57c0faf, 0x4787c62a, 0xa8304613, 0xfd469501,
        0x698098d8, 0x8b44f7af, 0xffff5bb1, 0x895cd7be,
        0x6b901122, 0xfd987193, 0xa679438e, 0x49b40821,
        0xf61e2562, 0xc040b340, 0x265e5a51, 0xe9b6c7aa,
        0xd62f105d, 0x02441453, 0xd8a1e681, 0xe7d3fbc8,
        0x21e1cde6, 0xc33707d6, 0xf4d50d87, 0x455a14ed,
        0xa9e3e905, 0xfcefa3f8, 0x676f02d9, 0x8d2a4c8a,
        0xfffa3942, 0x8771f681, 0x6d9d6122, 0xfde5380c,
        0xa4beea44, 0x4bdecfa9, 0xf6bb4b60, 0xbebfbc70,
        0x289b7ec6, 0xeaa127fa, 0xd4ef3085, 0x04881d05,
        0xd9d4d039, 0xe6db99e5, 0x1fa27cf8, 0xc4ac5665,
        0xf4292244, 0x432aff97, 0xab9423a7, 0xfc93a039,
        0x655b59c3, 0x8f0ccc92, 0xffeff47d, 0x85845dd1,
        0x6fa87e4f, 0xfe2ce6e0, 0xa3014314, 0x4e0811a1,
        0xf7537e82, 0xbd3af235, 0x2ad7d2bb, 0xeb86d391,
    ];

    for chunk in msg.chunks(64) {
        let mut m = [0u32; 16];
        for (i, c) in chunk.chunks(4).enumerate() {
            m[i] = u32::from_le_bytes([c[0], c[1], c[2], c[3]]);
        }

        let (mut a, mut b, mut c, mut d) = (state[0], state[1], state[2], state[3]);

        for i in 0..64 {
            let (f, g) = match i {
                0..=15 => ((b & c) | ((!b) & d), i),
                16..=31 => ((d & b) | ((!d) & c), (5 * i + 1) % 16),
                32..=47 => (b ^ c ^ d, (3 * i + 5) % 16),
                _ => (c ^ (b | (!d)), (7 * i) % 16),
            };
            let temp = d;
            d = c;
            c = b;
            b = b.wrapping_add(
                (a.wrapping_add(f).wrapping_add(K[i]).wrapping_add(m[g]))
                    .rotate_left(S[i]),
            );
            a = temp;
        }

        state[0] = state[0].wrapping_add(a);
        state[1] = state[1].wrapping_add(b);
        state[2] = state[2].wrapping_add(c);
        state[3] = state[3].wrapping_add(d);
    }

    let mut result = String::with_capacity(32);
    for &s in &state {
        for b in s.to_le_bytes() {
            result.push_str(&format!("{:02x}", b));
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hex() {
        // MD5("") = d41d8cd98f00b204e9800998ecf8427e
        assert_eq!(md5_hex(b""), "d41d8cd98f00b204e9800998ecf8427e");
        // MD5("abc") = 900150983cd24fb0d6963f7d28e17f72
        assert_eq!(md5_hex(b"abc"), "900150983cd24fb0d6963f7d28e17f72");
    }

    #[test]
    fn test_evaluate_equals() {
        let ram = vec![0u8; 2048];
        let prev = vec![0u8; 2048];
        assert!(evaluate_conditions("0xH0000=0", &ram, &prev));
        assert!(!evaluate_conditions("0xH0000=1", &ram, &prev));
    }

    #[test]
    fn test_evaluate_greater_less() {
        let mut ram = vec![0u8; 2048];
        ram[0x75] = 5;
        let prev = vec![0u8; 2048];
        assert!(evaluate_conditions("0xH0075>0", &ram, &prev));
        assert!(evaluate_conditions("0xH0075>4", &ram, &prev));
        assert!(!evaluate_conditions("0xH0075>5", &ram, &prev));
        assert!(evaluate_conditions("0xH0075<6", &ram, &prev));
        assert!(!evaluate_conditions("0xH0075<5", &ram, &prev));
    }

    #[test]
    fn test_evaluate_and_conditions() {
        let mut ram = vec![0u8; 2048];
        ram[0x75] = 5;
        ram[0x76] = 10;
        let prev = vec![0u8; 2048];
        assert!(evaluate_conditions("0xH0075>0_S_0xH0076=10", &ram, &prev));
        assert!(!evaluate_conditions("0xH0075>0_S_0xH0076=11", &ram, &prev));
    }

    #[test]
    fn test_evaluate_delta() {
        let mut ram = vec![0u8; 2048];
        ram[0x10] = 5;
        let mut prev = vec![0u8; 2048];
        prev[0x10] = 3;
        assert!(evaluate_conditions("d0xH0010!=0xH0010", &ram, &prev));

        prev[0x10] = 5;
        assert!(!evaluate_conditions("d0xH0010!=0xH0010", &ram, &prev));
    }

    #[test]
    fn test_empty_conditions() {
        let ram = vec![0u8; 2048];
        let prev = vec![0u8; 2048];
        assert!(!evaluate_conditions("", &ram, &prev));
    }

    #[test]
    fn test_hex_value() {
        let mut ram = vec![0u8; 2048];
        ram[0x10] = 0xFF;
        let prev = vec![0u8; 2048];
        assert!(evaluate_conditions("0xH0010=0xFF", &ram, &prev));
    }

    #[test]
    fn test_engine_new() {
        let engine = AchievementEngine::new();
        assert!(engine.achievements.is_empty());
        assert_eq!(engine.total_points, 0);
        assert_eq!(engine.unlocked_count, 0);
    }

    #[test]
    fn test_engine_check_frame_unlocks() {
        let mut engine = AchievementEngine::new();
        engine.enabled = true;
        engine.achievements.push(Achievement {
            id: 1,
            title: "Test".to_string(),
            description: "Test achievement".to_string(),
            points: 10,
            conditions: "0xH0075>0".to_string(),
            unlocked: false,
        });
        engine.prev_ram = vec![0u8; 2048];

        let mut ram = vec![0u8; 2048];
        ram[0x75] = 1;
        engine.check_frame(&ram);

        assert!(engine.achievements[0].unlocked);
        assert_eq!(engine.unlocked_count, 1);
        assert_eq!(engine.total_points, 10);
        assert_eq!(engine.notifications.len(), 1);
    }
}

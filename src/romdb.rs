use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Serialize, Deserialize)]
pub struct RomEntry {
    pub title: String,
    pub region: String,
    pub mapper: u16,
    pub mirroring: String,
    pub prg_size: usize,
    pub chr_size: usize,
    pub battery: bool,
}

pub struct RomDatabase {
    entries: HashMap<String, RomEntry>,
}

// Built-in entries are limited to factual compatibility metadata for
// user-provided ROMs. Do not add descriptions, artwork, publisher copy, or
// other promotional fields here. User metadata is loaded after this table and
// intentionally overrides matching CRCs.
const BUILTIN_DB: &str = r#"
{
  "3337EC46": { "title": "Super Mario Bros.", "region": "US", "mapper": 0, "mirroring": "vertical", "prg_size": 32768, "chr_size": 8192, "battery": false },
  "E0B72AAE": { "title": "Super Mario Bros. 2", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 131072, "chr_size": 131072, "battery": false },
  "A03B44F0": { "title": "Super Mario Bros. 3", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 262144, "chr_size": 131072, "battery": false },
  "A12D74C1": { "title": "The Legend of Zelda", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 0, "battery": true },
  "0FCFC32D": { "title": "Mega Man 2", "region": "US", "mapper": 1, "mirroring": "vertical", "prg_size": 262144, "chr_size": 32768, "battery": false },
  "53F78B15": { "title": "Castlevania", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 16384, "battery": false },
  "92A4B955": { "title": "Castlevania III: Dracula's Curse", "region": "US", "mapper": 5, "mirroring": "vertical", "prg_size": 262144, "chr_size": 131072, "battery": false },
  "60FC5E96": { "title": "Contra", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 0, "battery": false },
  "A3B0B73E": { "title": "Metroid", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 0, "battery": false },
  "A9B201B2": { "title": "Kirby's Adventure", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 262144, "chr_size": 262144, "battery": false },
  "A6216AA1": { "title": "Jackal", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 0, "battery": false },
  "D029F841": { "title": "DuckTales", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 32768, "battery": false },
  "370C7E86": { "title": "Double Dragon II: The Revenge", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 131072, "chr_size": 131072, "battery": false },
  "A98A3B01": { "title": "Tecmo Super Bowl", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 262144, "chr_size": 131072, "battery": true },
  "CEBD2A31": { "title": "Final Fantasy", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 262144, "chr_size": 0, "battery": true },
  "09A40C2A": { "title": "Dragon Warrior", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 65536, "chr_size": 32768, "battery": true },
  "A78AEC53": { "title": "Punch-Out!!", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 0, "battery": false },
  "C6C9D8B5": { "title": "Ninja Gaiden", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 32768, "battery": false },
  "6D72C53A": { "title": "Tetris", "region": "US", "mapper": 0, "mirroring": "horizontal", "prg_size": 32768, "chr_size": 8192, "battery": false },
  "3A94FA0B": { "title": "Excitebike", "region": "US", "mapper": 0, "mirroring": "vertical", "prg_size": 16384, "chr_size": 8192, "battery": false },
  "B2F93B8A": { "title": "Zelda II: The Adventure of Link", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 131072, "battery": true },
  "D68A6F33": { "title": "Mega Man", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 16384, "battery": false },
  "7E2770B4": { "title": "Mega Man 3", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 262144, "chr_size": 131072, "battery": false },
  "5ED6F221": { "title": "Mega Man 4", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 524288, "chr_size": 262144, "battery": false },
  "89B89560": { "title": "Mega Man 5", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 524288, "chr_size": 262144, "battery": false },
  "E4DC7875": { "title": "Mega Man 6", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 524288, "chr_size": 262144, "battery": false },
  "1B2BAE66": { "title": "Ninja Gaiden II: The Dark Sword of Chaos", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 131072, "battery": false },
  "96CE586E": { "title": "Ninja Gaiden III: The Ancient Ship of Doom", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 262144, "chr_size": 131072, "battery": false },
  "AB862073": { "title": "Donkey Kong", "region": "US", "mapper": 0, "mirroring": "horizontal", "prg_size": 16384, "chr_size": 8192, "battery": false },
  "4A7E19B5": { "title": "Pac-Man", "region": "US", "mapper": 0, "mirroring": "horizontal", "prg_size": 16384, "chr_size": 8192, "battery": false },
  "662E9F03": { "title": "Gradius", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 0, "battery": false },
  "7474AC92": { "title": "Life Force", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 0, "battery": false },
  "B1B16B8A": { "title": "Blaster Master", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 131072, "battery": false },
  "2E6301ED": { "title": "Battletoads", "region": "US", "mapper": 7, "mirroring": "horizontal", "prg_size": 262144, "chr_size": 0, "battery": false },
  "932A077A": { "title": "Ice Climber", "region": "US", "mapper": 0, "mirroring": "horizontal", "prg_size": 32768, "chr_size": 8192, "battery": false },
  "A4DCEA7B": { "title": "Kid Icarus", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 0, "battery": true },
  "1DC0F740": { "title": "Balloon Fight", "region": "US", "mapper": 0, "mirroring": "horizontal", "prg_size": 16384, "chr_size": 8192, "battery": false },
  "82413D04": { "title": "River City Ransom", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 131072, "battery": true },
  "87BB1E07": { "title": "Bionic Commando", "region": "US", "mapper": 3, "mirroring": "vertical", "prg_size": 131072, "chr_size": 131072, "battery": false },
  "C20F16FC": { "title": "Ghosts 'n Goblins", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 8192, "battery": false },
  "3B3F88F0": { "title": "Gauntlet", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 131072, "chr_size": 0, "battery": false },
  "79F688BC": { "title": "R.C. Pro-Am", "region": "US", "mapper": 3, "mirroring": "vertical", "prg_size": 32768, "chr_size": 32768, "battery": false },
  "DEE05DA6": { "title": "Bubble Bobble", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 131072, "chr_size": 131072, "battery": false },
  "C8EAEC28": { "title": "Dragon Warrior II", "region": "US", "mapper": 1, "mirroring": "horizontal", "prg_size": 131072, "chr_size": 131072, "battery": true },
  "FDDEE33C": { "title": "Dragon Warrior III", "region": "US", "mapper": 4, "mirroring": "horizontal", "prg_size": 262144, "chr_size": 131072, "battery": true },
  "E9BC8522": { "title": "Dragon Warrior IV", "region": "US", "mapper": 4, "mirroring": "horizontal", "prg_size": 524288, "chr_size": 262144, "battery": true },
  "0B363CCA": { "title": "Final Fantasy II (US)", "region": "US", "mapper": 4, "mirroring": "horizontal", "prg_size": 262144, "chr_size": 262144, "battery": true },
  "A4C05919": { "title": "Final Fantasy III (US)", "region": "US", "mapper": 4, "mirroring": "horizontal", "prg_size": 524288, "chr_size": 262144, "battery": true },
  "5084461E": { "title": "Mike Tyson's Punch-Out!!", "region": "US", "mapper": 2, "mirroring": "vertical", "prg_size": 262144, "chr_size": 131072, "battery": false },
  "1F6EA423": { "title": "Teenage Mutant Ninja Turtles", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 131072, "chr_size": 131072, "battery": false },
  "83766340": { "title": "TMNT II: The Arcade Game", "region": "US", "mapper": 4, "mirroring": "vertical", "prg_size": 262144, "chr_size": 131072, "battery": false }
}
"#;

impl RomDatabase {
    pub fn new() -> Self {
        let mut db = RomDatabase {
            entries: HashMap::new(),
        };
        db.load_builtin();
        db.load_user_db();
        db
    }
}

impl Default for RomDatabase {
    fn default() -> Self {
        Self::new()
    }
}

impl RomDatabase {
    pub fn lookup(&self, crc32: u32) -> Option<&RomEntry> {
        let key = format!("{:08X}", crc32);
        self.entries.get(&key)
    }

    fn load_builtin(&mut self) {
        self.load_json_entries(BUILTIN_DB);
    }

    fn load_json_entries(&mut self, data: &str) -> bool {
        let Ok(map) = serde_json::from_str::<HashMap<String, RomEntry>>(data) else {
            return false;
        };

        for (crc, entry) in map {
            if let Some(key) = normalize_crc_key(&crc) {
                self.entries.insert(key, entry);
            }
        }
        true
    }

    fn load_user_db(&mut self) {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_else(|_| ".".to_string());
        let path = std::path::Path::new(&home)
            .join(".nes-emulator")
            .join("romdb.json");
        if let Ok(data) = std::fs::read_to_string(&path) {
            self.load_json_entries(&data);
        }
    }
}

fn normalize_crc_key(crc: &str) -> Option<String> {
    let trimmed = crc.trim();
    if trimmed.len() == 8 && trimmed.chars().all(|ch| ch.is_ascii_hexdigit()) {
        Some(trimmed.to_ascii_uppercase())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_json_entries_rejects_malformed_metadata_without_panic() {
        let mut db = RomDatabase {
            entries: HashMap::new(),
        };

        assert!(!db.load_json_entries("not json"));
        assert!(db.entries.is_empty());
    }

    #[test]
    fn load_json_entries_accepts_valid_user_metadata() {
        let mut db = RomDatabase {
            entries: HashMap::new(),
        };
        let json = r#"{
            "1234ABCD": {
                "title": "Homebrew Test",
                "region": "US",
                "mapper": 0,
                "mirroring": "horizontal",
                "prg_size": 32768,
                "chr_size": 8192,
                "battery": false
            }
        }"#;

        assert!(db.load_json_entries(json));
        let entry = db.lookup(0x1234_ABCD).expect("entry should load");
        assert_eq!(entry.title, "Homebrew Test");
        assert_eq!(entry.mapper, 0);
    }

    #[test]
    fn load_json_entries_normalizes_crc_keys_for_user_overrides() {
        let mut db = RomDatabase {
            entries: HashMap::new(),
        };
        let builtin_json = r#"{
            "1234ABCD": {
                "title": "Original Metadata",
                "region": "US",
                "mapper": 0,
                "mirroring": "horizontal",
                "prg_size": 32768,
                "chr_size": 8192,
                "battery": false
            }
        }"#;
        let user_json = r#"{
            "1234abcd": {
                "title": "User Metadata",
                "region": "PAL",
                "mapper": 2,
                "mirroring": "vertical",
                "prg_size": 131072,
                "chr_size": 0,
                "battery": true
            }
        }"#;

        assert!(db.load_json_entries(builtin_json));
        assert!(db.load_json_entries(user_json));
        let entry = db.lookup(0x1234_ABCD).expect("entry should load");

        assert_eq!(entry.title, "User Metadata");
        assert_eq!(entry.mapper, 2);
        assert_eq!(entry.region, "PAL");
        assert!(entry.battery);
    }

    #[test]
    fn load_json_entries_ignores_invalid_crc_keys() {
        let mut db = RomDatabase {
            entries: HashMap::new(),
        };
        let json = r#"{
            "not-a-crc": {
                "title": "Invalid Key",
                "region": "US",
                "mapper": 0,
                "mirroring": "horizontal",
                "prg_size": 32768,
                "chr_size": 8192,
                "battery": false
            }
        }"#;

        assert!(db.load_json_entries(json));
        assert!(db.entries.is_empty());
    }
}

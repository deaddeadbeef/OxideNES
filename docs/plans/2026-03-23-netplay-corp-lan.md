# Netplay Corp-LAN Fix Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Make OxideNES netplay work on corporate LANs by adding configurable port and keepalive heartbeat.

**Architecture:** Two surgical changes to `netplay.rs` + `main.rs`: (1) Replace hardcoded port 7777 with a configurable `port` field on `NetplaySession`, exposed via UI port picker and passed through `host(port)`; (2) Add a periodic keepalive packet (every 2 seconds) so stateful firewalls don't drop the UDP flow. Both changes are backward-compatible — the default port remains 7777.

**Tech Stack:** Rust, UDP sockets (`std::net::UdpSocket`), `std::time::Instant`

---

## Task 1: Add configurable port to NetplaySession

**Files:**
- Modify: `src/netplay.rs:22-35` (struct fields)
- Modify: `src/netplay.rs:37-60` (constructor defaults)
- Test: `src/netplay.rs:301-401` (inline tests)

**Step 1: Write the failing test**

Add to the inline test module in `src/netplay.rs` (after the existing tests, before the closing `}`):

```rust
#[test]
fn test_configurable_port() {
    let mut session = NetplaySession::new();
    assert_eq!(session.port, 7777); // default
    session.port = 8080;
    assert_eq!(session.port, 8080);
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p oxidenes test_configurable_port -- --nocapture`
Expected: FAIL — `no field 'port' on type 'NetplaySession'`

**Step 3: Add `port` field to NetplaySession**

In `src/netplay.rs`, add to the `NetplaySession` struct (around line 23):

```rust
pub port: u16,
```

And in `new()` (around line 43), add the default:

```rust
port: 7777,
```

**Step 4: Run test to verify it passes**

Run: `cargo test -p oxidenes test_configurable_port -- --nocapture`
Expected: PASS

**Step 5: Commit**

```bash
git add src/netplay.rs
git commit -m "feat(netplay): add configurable port field with default 7777"
```

---

## Task 2: Use configurable port in host()

**Files:**
- Modify: `src/netplay.rs:63-78` (`host()` method)
- Modify: `src/main.rs:3661` (call site)
- Modify: `src/main.rs:3663` (overlay message)
- Test: `src/netplay.rs` (inline tests)

**Step 1: Write the failing test**

Add to inline tests in `src/netplay.rs`:

```rust
#[test]
fn test_host_uses_configured_port() {
    let mut session = NetplaySession::new();
    session.port = 0; // OS picks ephemeral port
    session.host();
    match &session.state {
        NetplayState::Hosting { port } => assert_ne!(*port, 7777),
        other => panic!("Expected Hosting, got {:?}", other),
    }
    session.disconnect();
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p oxidenes test_host_uses_configured_port -- --nocapture`
Expected: FAIL — `host()` takes 1 argument (the port), signature mismatch

**Step 3: Change `host()` to use `self.port` instead of parameter**

In `src/netplay.rs`, change the `host()` method signature from:

```rust
pub fn host(&mut self, port: u16) {
```

to:

```rust
pub fn host(&mut self) {
```

And change the bind address inside `host()` from using the `port` parameter to `self.port`:

```rust
let addr = format!("0.0.0.0:{}", self.port);
```

Also update the `Hosting` state assignment to use `self.port`:

```rust
self.state = NetplayState::Hosting { port: self.port };
```

**Step 4: Update call site in main.rs**

In `src/main.rs` around line 3661, change:

```rust
netplay.host(7777);
```

to:

```rust
netplay.host();
```

Around line 3663, change the overlay message from the hardcoded string to use the actual port:

```rust
format!("HOSTING ON PORT {}", netplay.port)
```

(Adapt to however the overlay string is constructed — it may be a direct string literal or formatted.)

**Step 5: Fix existing test `test_host_and_disconnect`**

The existing test at ~line 343 calls `session.host(0)`. Change it to:

```rust
session.port = 0;
session.host();
```

**Step 6: Fix existing test `test_loopback_connection`**

The loopback test at ~line 366 calls `host.host(0)` and joins to the host port. Update it:

```rust
host.port = 0;
host.host();
```

The join address extraction from `Hosting { port }` should still work since `self.port` is stored there.

**Step 7: Run all tests**

Run: `cargo test -p oxidenes -- --nocapture`
Expected: ALL PASS

**Step 8: Commit**

```bash
git add src/netplay.rs src/main.rs
git commit -m "feat(netplay): host() uses configurable self.port instead of parameter"
```

---

## Task 3: Add port picker to netplay UI menu

**Files:**
- Modify: `src/main.rs:5478-5479` (menu label)
- Modify: `src/main.rs:3661` area (menu action handler)
- Modify: `src/main.rs:2295-2298` area (state variables)

**Step 1: Add port editing state variable**

Near line 2295 in `src/main.rs` where `netplay_ip_input` is defined, add:

```rust
let mut netplay_port_input: String = "7777".to_string();
```

**Step 2: Update the HOST menu label**

Around line 5478-5479, change the menu item from the hardcoded `"HOST (PORT 7777)"` to:

```rust
format!("HOST (PORT {})", netplay_port_input)
```

**Step 3: Add port parsing before host()**

Around line 3661 where `netplay.host()` is called, parse the port from the input string:

```rust
if let Ok(p) = netplay_port_input.parse::<u16>() {
    netplay.port = p;
}
netplay.host();
```

**Step 4: Add PORT menu item with keyboard editing**

Add a new menu item `"PORT: {}"` to the netplay submenu (alongside HOST, JOIN, DISCONNECT, INPUT DELAY). When selected, allow the same keyboard digit entry pattern used for the IP input (keys 0-9, backspace, enter to confirm). Wire it to modify `netplay_port_input`.

Look at how `netplay_ip_input` keyboard handling works (~line 3600-3630) and replicate the pattern for `netplay_port_input`, but only accept digits (no dots or colons).

**Step 5: Build and manually verify**

Run: `cargo build --release`
Expected: Compiles without errors

**Step 6: Commit**

```bash
git add src/main.rs
git commit -m "feat(netplay): add port picker to netplay menu"
```

---

## Task 4: Add keepalive heartbeat

**Files:**
- Modify: `src/netplay.rs:5-11` (add new magic constant)
- Modify: `src/netplay.rs:22-35` (add `last_keepalive` field)
- Modify: `src/netplay.rs:37-60` (initialize field)
- Add new method: `src/netplay.rs` (send_keepalive + check in receive)
- Modify: `src/main.rs:4059` area (call keepalive in game loop)
- Test: `src/netplay.rs` (inline tests)

**Step 1: Write the failing test**

Add to inline tests in `src/netplay.rs`:

```rust
#[test]
fn test_keepalive_sent_when_due() {
    use std::time::{Duration, Instant};
    let mut session = NetplaySession::new();
    // Pretend we're connected with a socket
    session.port = 0;
    session.host();
    // Force last_keepalive to 3 seconds ago
    session.last_keepalive = Instant::now() - Duration::from_secs(3);
    assert!(session.should_send_keepalive());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test -p oxidenes test_keepalive_sent_when_due -- --nocapture`
Expected: FAIL — `no field 'last_keepalive'` / `no method 'should_send_keepalive'`

**Step 3: Add keepalive constant and fields**

In `src/netplay.rs` constants section (~line 5-11), add:

```rust
const MAGIC_KEEPALIVE: [u8; 2] = [0x4E, 0x4B]; // "NK"
const KEEPALIVE_INTERVAL_SECS: u64 = 2;
```

In the `NetplaySession` struct, add:

```rust
pub last_keepalive: Instant,
```

In `new()`, initialize:

```rust
last_keepalive: Instant::now(),
```

Add `use std::time::{Duration, Instant};` at the top if not already imported.

**Step 4: Add `should_send_keepalive()` and `send_keepalive()` methods**

```rust
pub fn should_send_keepalive(&self) -> bool {
    self.last_keepalive.elapsed() >= Duration::from_secs(KEEPALIVE_INTERVAL_SECS)
}

pub fn send_keepalive(&mut self) {
    if !self.is_connected() && !matches!(self.state, NetplayState::Hosting { .. }) {
        return;
    }
    if let (Some(socket), Some(peer)) = (&self.socket, &self.peer_addr) {
        let _ = socket.send_to(&MAGIC_KEEPALIVE, peer);
    }
    self.last_keepalive = Instant::now();
}
```

**Step 5: Handle keepalive packets in receive path**

In `handle_packet()` (~line 198-252), add a match arm for `MAGIC_KEEPALIVE`:

```rust
if data[0..2] == MAGIC_KEEPALIVE {
    // Keepalive received — just update last_recv timestamp, no action needed
    return;
}
```

**Step 6: Run test to verify it passes**

Run: `cargo test -p oxidenes test_keepalive_sent_when_due -- --nocapture`
Expected: PASS

**Step 7: Integrate into main game loop**

In `src/main.rs` around line 4059 (the `if netplay.is_connected()` block), add near the top of the per-frame netplay section:

```rust
if netplay.should_send_keepalive() {
    netplay.send_keepalive();
}
```

Also add the same check in the hosting/connecting polling section (~line 4132) so keepalives flow during the handshake wait too.

**Step 8: Run all tests**

Run: `cargo test -p oxidenes -- --nocapture`
Expected: ALL PASS

**Step 9: Build release**

Run: `cargo build --release`
Expected: Compiles clean

**Step 10: Commit**

```bash
git add src/netplay.rs src/main.rs
git commit -m "feat(netplay): add keepalive heartbeat every 2 seconds"
```

---

## Task 5: Update default join address to use configured port

**Files:**
- Modify: `src/main.rs:2295-2298` area

**Step 1: Update default join address**

The default `netplay_ip_input` is currently `"127.0.0.1:7777"`. Change it to dynamically use the configured port when the JOIN menu is opened, or at minimum document that the user should update the port portion when using a non-default port.

Simplest approach: when the user changes `netplay_port_input`, also update the port portion of `netplay_ip_input` if it still ends with the old port. Or just initialize both from the same default.

**Step 2: Build and verify**

Run: `cargo build --release`
Expected: Compiles clean

**Step 3: Commit**

```bash
git add src/main.rs
git commit -m "feat(netplay): sync default join address with configured port"
```

---

## Task 6: Final integration test and release prep

**Step 1: Run full test suite**

Run: `cargo test -p oxidenes -- --nocapture`
Expected: ALL PASS (including all 7 original + 3 new tests)

**Step 2: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No warnings

**Step 3: Update CHANGELOG.md**

Add under a new `## [Unreleased]` section at the top:

```markdown
## [Unreleased]

### Added
- Configurable netplay port (default remains 7777, changeable via in-game menu)
- Keepalive heartbeat every 2 seconds to prevent firewall UDP timeout
- Port picker in netplay submenu
```

**Step 4: Commit**

```bash
git add CHANGELOG.md
git commit -m "docs: add unreleased changelog for netplay corp-LAN features"
```

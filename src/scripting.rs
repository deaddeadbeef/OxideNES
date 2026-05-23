use mlua::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

/// Lua scripting engine for the NES emulator.
///
/// Exposes a `nes` API table to Lua scripts with memory read, frame count,
/// overlay drawing, and debug logging. Overlay pixels and messages are
/// collected each frame and returned to the renderer.
pub struct ScriptEngine {
    lua: Lua,
    pub active: bool,
    pub script_path: Option<String>,
    pub overlay_pixels: Vec<(usize, usize, u32)>,
    pub messages: Vec<(String, u32)>,
    // Shared state between Rust and Lua callbacks
    pixel_sink: Rc<RefCell<Vec<(usize, usize, u32)>>>,
    message_sink: Rc<RefCell<Vec<(String, u32)>>>,
}

impl ScriptEngine {
    pub fn new() -> Self {
        let lua = Lua::new();
        // Sandbox: remove dangerous standard library globals
        lua.globals().set("io", mlua::Value::Nil).ok();
        lua.globals().set("os", mlua::Value::Nil).ok();
        lua.globals().set("require", mlua::Value::Nil).ok();
        lua.globals().set("loadfile", mlua::Value::Nil).ok();
        lua.globals().set("dofile", mlua::Value::Nil).ok();
        lua.globals().set("debug", mlua::Value::Nil).ok();
        lua.globals().set("package", mlua::Value::Nil).ok();

        let pixel_sink = Rc::new(RefCell::new(Vec::new()));
        let message_sink = Rc::new(RefCell::new(Vec::new()));

        // Register static nes API functions (pixel, message, log)
        {
            let nes = lua.create_table().expect("create nes table");

            // nes.pixel(x, y, color)
            let ps = pixel_sink.clone();
            let pixel_fn = lua
                .create_function(move |_, (x, y, color): (usize, usize, u32)| {
                    ps.borrow_mut().push((x, y, color));
                    Ok(())
                })
                .expect("create pixel fn");
            nes.set("pixel", pixel_fn).expect("set pixel");

            // nes.message(text)
            let ms = message_sink.clone();
            let message_fn = lua
                .create_function(move |_, text: String| {
                    ms.borrow_mut().push((text, 60));
                    Ok(())
                })
                .expect("create message fn");
            nes.set("message", message_fn).expect("set message");

            // nes.log(text)
            let log_fn = lua
                .create_function(|_, text: String| {
                    eprintln!("[lua] {}", text);
                    Ok(())
                })
                .expect("create log fn");
            nes.set("log", log_fn).expect("set log");

            lua.globals().set("nes", nes).expect("set nes global");
        }

        ScriptEngine {
            lua,
            active: false,
            script_path: None,
            overlay_pixels: Vec::new(),
            messages: Vec::new(),
            pixel_sink,
            message_sink,
        }
    }
}

impl Default for ScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ScriptEngine {
    /// Load and execute a Lua script file. The script should register an
    /// `on_frame` callback via `nes.onframe(fn)`.
    pub fn load_script(&mut self, path: &str) -> Result<(), String> {
        let source = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read '{}': {}", path, e))?;

        self.lua
            .load(&source)
            .exec()
            .map_err(|e| format!("Lua error: {}", e))?;

        self.active = true;
        self.script_path = Some(path.to_string());
        eprintln!("[scripting] Loaded: {}", path);
        Ok(())
    }

    /// Called once per emulated frame. Snapshots RAM into Lua, then invokes
    /// the script's registered `on_frame` callback (if any).
    pub fn on_frame(&mut self, ram: &[u8], frame_count: u64) -> Result<(), String> {
        if !self.active {
            return Ok(());
        }

        // Clear sinks from previous frame
        self.pixel_sink.borrow_mut().clear();
        self.message_sink.borrow_mut().clear();

        // Use scope so the read function borrows ram for this call only
        self.lua
            .scope(|scope| {
                // nes.read(addr) — scoped, captures ram slice
                let read_fn = scope.create_function(|_, addr: u16| {
                    Ok(if (addr as usize) < ram.len() {
                        ram[addr as usize]
                    } else {
                        0
                    })
                })?;

                let nes: LuaTable = self.lua.globals().get("nes")?;
                nes.set("read", read_fn)?;

                // nes.framecount()
                let fc_fn = scope.create_function(|_, ()| Ok(frame_count))?;
                nes.set("framecount", fc_fn)?;

                // Call the registered on_frame callback (if any)
                let callback: Option<LuaFunction> = self.lua.globals().get("_nes_on_frame").ok();
                if let Some(cb) = callback {
                    cb.call::<()>(())?;
                }

                Ok(())
            })
            .map_err(|e| format!("on_frame: {}", e))?;

        // Drain sinks into public vecs for renderer
        self.overlay_pixels
            .append(&mut self.pixel_sink.borrow_mut());
        self.messages.append(&mut self.message_sink.borrow_mut());

        Ok(())
    }

    /// Unload the current script and reset state.
    pub fn unload(&mut self) {
        self.active = false;
        self.script_path = None;
        self.overlay_pixels.clear();
        self.messages.clear();
        // Remove the callback
        let _ = self.lua.globals().set("_nes_on_frame", LuaValue::Nil);
        eprintln!("[scripting] Script unloaded");
    }
}

/// Register the `nes.onframe` plumbing. Called once during engine setup
/// to allow scripts to register their frame callback via `nes.onframe(fn)`.
pub fn register_onframe(lua: &Lua) -> Result<(), LuaError> {
    let onframe_fn = lua.create_function(|lua, func: LuaFunction| {
        lua.globals().set("_nes_on_frame", func)?;
        Ok(())
    })?;
    let nes: LuaTable = lua.globals().get("nes")?;
    nes.set("onframe", onframe_fn)?;
    Ok(())
}

impl ScriptEngine {
    /// Full initialization: create engine + register onframe hook.
    pub fn init() -> Self {
        let engine = Self::new();
        register_onframe(&engine.lua).expect("register onframe");
        engine
    }
}

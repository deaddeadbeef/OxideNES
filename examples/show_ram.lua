-- show_ram.lua: Display first 16 bytes of CPU RAM as overlay
-- Usage: nes-emulator game.nes --script examples/show_ram.lua

function on_frame()
    local fc = nes.framecount()

    -- Only update display every 15 frames (~4 times/sec)
    if fc % 15 ~= 0 then return end

    -- Build a string showing the first 16 RAM bytes
    local parts = {}
    for addr = 0, 15 do
        local val = nes.read(addr)
        parts[#parts + 1] = string.format("%02X", val)
    end

    nes.message("RAM: " .. table.concat(parts, " "))
end

nes.onframe(on_frame)

use crate::bus::Bus;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AddressingMode {
    Implicit,
    Accumulator,
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Relative,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
}

pub struct Cpu {
    pub a: u8,
    pub x: u8,
    pub y: u8,
    pub sp: u8,
    pub pc: u16,
    pub status: u8,
    pub cycles: u8,
    total_cycles: usize,
}

// Status flags
const CARRY: u8 = 0x01;
const ZERO: u8 = 0x02;
const INTERRUPT_DISABLE: u8 = 0x04;
const DECIMAL: u8 = 0x08;
const BREAK: u8 = 0x10;
const UNUSED: u8 = 0x20;
const OVERFLOW: u8 = 0x40;
const NEGATIVE: u8 = 0x80;

impl Cpu {
    pub fn new() -> Self {
        Cpu {
            a: 0,
            x: 0,
            y: 0,
            sp: 0xFD,
            pc: 0,
            status: 0x24,
            cycles: 0,
            total_cycles: 0,
        }
    }

    // ── Flag helpers ────────────────────────────────────────────────

    pub fn get_flag(&self, flag: u8) -> bool {
        (self.status & flag) != 0
    }

    pub fn set_flag(&mut self, flag: u8, value: bool) {
        if value {
            self.status |= flag;
        } else {
            self.status &= !flag;
        }
    }

    fn update_zero_negative(&mut self, value: u8) {
        self.set_flag(ZERO, value == 0);
        self.set_flag(NEGATIVE, value & 0x80 != 0);
    }

    // ── Stack operations ────────────────────────────────────────────

    fn push(&mut self, bus: &mut Bus, value: u8) {
        bus.cpu_write(0x0100 | self.sp as u16, value);
        self.sp = self.sp.wrapping_sub(1);
    }

    fn pull(&mut self, bus: &mut Bus) -> u8 {
        self.sp = self.sp.wrapping_add(1);
        bus.cpu_read(0x0100 | self.sp as u16)
    }

    fn push16(&mut self, bus: &mut Bus, value: u16) {
        self.push(bus, (value >> 8) as u8);
        self.push(bus, (value & 0xFF) as u8);
    }

    fn pull16(&mut self, bus: &mut Bus) -> u16 {
        let lo = self.pull(bus) as u16;
        let hi = self.pull(bus) as u16;
        (hi << 8) | lo
    }

    // ── Interrupts / reset ──────────────────────────────────────────

    pub fn reset(&mut self, bus: &mut Bus) {
        let lo = bus.cpu_read(0xFFFC) as u16;
        let hi = bus.cpu_read(0xFFFD) as u16;
        self.pc = (hi << 8) | lo;
        self.a = 0;
        self.x = 0;
        self.y = 0;
        self.sp = 0xFD;
        self.status = 0x24;
        self.cycles = 8;
        self.total_cycles = 0;
    }

    pub fn nmi(&mut self, bus: &mut Bus) {
        self.push16(bus, self.pc);
        self.set_flag(BREAK, false);
        self.set_flag(UNUSED, true);
        self.push(bus, self.status);
        self.set_flag(INTERRUPT_DISABLE, true);
        let lo = bus.cpu_read(0xFFFA) as u16;
        let hi = bus.cpu_read(0xFFFB) as u16;
        self.pc = (hi << 8) | lo;
        self.cycles = 7;
    }

    pub fn irq(&mut self, bus: &mut Bus) {
        if self.get_flag(INTERRUPT_DISABLE) {
            return;
        }
        self.push16(bus, self.pc);
        self.set_flag(BREAK, false);
        self.set_flag(UNUSED, true);
        self.push(bus, self.status);
        self.set_flag(INTERRUPT_DISABLE, true);
        let lo = bus.cpu_read(0xFFFE) as u16;
        let hi = bus.cpu_read(0xFFFF) as u16;
        self.pc = (hi << 8) | lo;
        self.cycles = 7;
    }

    // ── Addressing modes ────────────────────────────────────────────

    fn get_operand_address(&mut self, bus: &mut Bus, mode: AddressingMode) -> (u16, bool) {
        match mode {
            AddressingMode::Immediate => {
                let addr = self.pc;
                self.pc = self.pc.wrapping_add(1);
                (addr, false)
            }
            AddressingMode::ZeroPage => {
                let addr = bus.cpu_read(self.pc) as u16;
                self.pc = self.pc.wrapping_add(1);
                (addr, false)
            }
            AddressingMode::ZeroPageX => {
                let addr = bus.cpu_read(self.pc).wrapping_add(self.x) as u16;
                self.pc = self.pc.wrapping_add(1);
                (addr, false)
            }
            AddressingMode::ZeroPageY => {
                let addr = bus.cpu_read(self.pc).wrapping_add(self.y) as u16;
                self.pc = self.pc.wrapping_add(1);
                (addr, false)
            }
            AddressingMode::Absolute => {
                let lo = bus.cpu_read(self.pc) as u16;
                let hi = bus.cpu_read(self.pc.wrapping_add(1)) as u16;
                self.pc = self.pc.wrapping_add(2);
                ((hi << 8) | lo, false)
            }
            AddressingMode::AbsoluteX => {
                let lo = bus.cpu_read(self.pc) as u16;
                let hi = bus.cpu_read(self.pc.wrapping_add(1)) as u16;
                self.pc = self.pc.wrapping_add(2);
                let base = (hi << 8) | lo;
                let addr = base.wrapping_add(self.x as u16);
                (addr, (base & 0xFF00) != (addr & 0xFF00))
            }
            AddressingMode::AbsoluteY => {
                let lo = bus.cpu_read(self.pc) as u16;
                let hi = bus.cpu_read(self.pc.wrapping_add(1)) as u16;
                self.pc = self.pc.wrapping_add(2);
                let base = (hi << 8) | lo;
                let addr = base.wrapping_add(self.y as u16);
                (addr, (base & 0xFF00) != (addr & 0xFF00))
            }
            AddressingMode::Indirect => {
                let lo = bus.cpu_read(self.pc) as u16;
                let hi = bus.cpu_read(self.pc.wrapping_add(1)) as u16;
                self.pc = self.pc.wrapping_add(2);
                let ptr = (hi << 8) | lo;
                // 6502 page boundary bug
                let addr = if lo == 0xFF {
                    let a_lo = bus.cpu_read(ptr) as u16;
                    let a_hi = bus.cpu_read(ptr & 0xFF00) as u16;
                    (a_hi << 8) | a_lo
                } else {
                    let a_lo = bus.cpu_read(ptr) as u16;
                    let a_hi = bus.cpu_read(ptr + 1) as u16;
                    (a_hi << 8) | a_lo
                };
                (addr, false)
            }
            AddressingMode::IndirectX => {
                let base = bus.cpu_read(self.pc);
                self.pc = self.pc.wrapping_add(1);
                let ptr = base.wrapping_add(self.x);
                let lo = bus.cpu_read(ptr as u16) as u16;
                let hi = bus.cpu_read(ptr.wrapping_add(1) as u16) as u16;
                ((hi << 8) | lo, false)
            }
            AddressingMode::IndirectY => {
                let ptr = bus.cpu_read(self.pc);
                self.pc = self.pc.wrapping_add(1);
                let lo = bus.cpu_read(ptr as u16) as u16;
                let hi = bus.cpu_read(ptr.wrapping_add(1) as u16) as u16;
                let base = (hi << 8) | lo;
                let addr = base.wrapping_add(self.y as u16);
                (addr, (base & 0xFF00) != (addr & 0xFF00))
            }
            AddressingMode::Relative => {
                let offset = bus.cpu_read(self.pc) as i8;
                self.pc = self.pc.wrapping_add(1);
                let addr = self.pc.wrapping_add(offset as u16);
                (addr, (self.pc & 0xFF00) != (addr & 0xFF00))
            }
            _ => (0, false),
        }
    }

    // ── Clock ───────────────────────────────────────────────────────

    pub fn clock(&mut self, bus: &mut Bus) {
        if bus.dma_active() {
            bus.dma_tick(self.total_cycles % 2 == 1);
            self.total_cycles = self.total_cycles.wrapping_add(1);
            return;
        }

        if self.cycles == 0 {
            if bus.poll_nmi() {
                self.nmi(bus);
            }

            let opcode = bus.cpu_read(self.pc);
            self.pc = self.pc.wrapping_add(1);
            self.set_flag(UNUSED, true);

            self.execute(bus, opcode);
        }

        self.cycles = self.cycles.saturating_sub(1);
        self.total_cycles = self.total_cycles.wrapping_add(1);
    }

    // ── Instruction helpers ─────────────────────────────────────────

    fn adc(&mut self, value: u8) {
        let carry = if self.get_flag(CARRY) { 1u16 } else { 0u16 };
        let sum = self.a as u16 + value as u16 + carry;
        self.set_flag(CARRY, sum > 0xFF);
        let result = sum as u8;
        self.set_flag(OVERFLOW, (!(self.a ^ value) & (self.a ^ result)) & 0x80 != 0);
        self.a = result;
        self.update_zero_negative(self.a);
    }

    fn sbc(&mut self, value: u8) {
        self.adc(!value);
    }

    fn compare(&mut self, reg: u8, value: u8) {
        let result = reg.wrapping_sub(value);
        self.set_flag(CARRY, reg >= value);
        self.update_zero_negative(result);
    }

    fn asl_acc(&mut self) {
        self.set_flag(CARRY, self.a & 0x80 != 0);
        self.a <<= 1;
        self.update_zero_negative(self.a);
    }

    fn asl_mem(&mut self, bus: &mut Bus, addr: u16) {
        let mut value = bus.cpu_read(addr);
        self.set_flag(CARRY, value & 0x80 != 0);
        value <<= 1;
        bus.cpu_write(addr, value);
        self.update_zero_negative(value);
    }

    fn lsr_acc(&mut self) {
        self.set_flag(CARRY, self.a & 0x01 != 0);
        self.a >>= 1;
        self.update_zero_negative(self.a);
    }

    fn lsr_mem(&mut self, bus: &mut Bus, addr: u16) {
        let mut value = bus.cpu_read(addr);
        self.set_flag(CARRY, value & 0x01 != 0);
        value >>= 1;
        bus.cpu_write(addr, value);
        self.update_zero_negative(value);
    }

    fn rol_acc(&mut self) {
        let old_carry = self.get_flag(CARRY) as u8;
        self.set_flag(CARRY, self.a & 0x80 != 0);
        self.a = (self.a << 1) | old_carry;
        self.update_zero_negative(self.a);
    }

    fn rol_mem(&mut self, bus: &mut Bus, addr: u16) {
        let old_carry = self.get_flag(CARRY) as u8;
        let mut value = bus.cpu_read(addr);
        self.set_flag(CARRY, value & 0x80 != 0);
        value = (value << 1) | old_carry;
        bus.cpu_write(addr, value);
        self.update_zero_negative(value);
    }

    fn ror_acc(&mut self) {
        let old_carry = self.get_flag(CARRY) as u8;
        self.set_flag(CARRY, self.a & 0x01 != 0);
        self.a = (self.a >> 1) | (old_carry << 7);
        self.update_zero_negative(self.a);
    }

    fn ror_mem(&mut self, bus: &mut Bus, addr: u16) {
        let old_carry = self.get_flag(CARRY) as u8;
        let mut value = bus.cpu_read(addr);
        self.set_flag(CARRY, value & 0x01 != 0);
        value = (value >> 1) | (old_carry << 7);
        bus.cpu_write(addr, value);
        self.update_zero_negative(value);
    }

    fn branch(&mut self, condition: bool, addr: u16, page_crossed: bool) {
        if condition {
            self.cycles += 1;
            if page_crossed {
                self.cycles += 1;
            }
            self.pc = addr;
        }
    }

    // ── Execute ─────────────────────────────────────────────────────

    pub fn execute(&mut self, bus: &mut Bus, opcode: u8) {
        match opcode {
            // ── LDA ─────────────────────────────────────────────
            0xA9 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 2;
            }
            0xA5 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 3;
            }
            0xB5 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0xAD => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0xBD => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0xB9 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0xA1 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0xB1 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.a = bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 5 + pc as u8;
            }

            // ── LDX ─────────────────────────────────────────────
            0xA2 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                self.x = bus.cpu_read(addr);
                self.update_zero_negative(self.x);
                self.cycles = 2;
            }
            0xA6 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.x = bus.cpu_read(addr);
                self.update_zero_negative(self.x);
                self.cycles = 3;
            }
            0xB6 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageY);
                self.x = bus.cpu_read(addr);
                self.update_zero_negative(self.x);
                self.cycles = 4;
            }
            0xAE => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.x = bus.cpu_read(addr);
                self.update_zero_negative(self.x);
                self.cycles = 4;
            }
            0xBE => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.x = bus.cpu_read(addr);
                self.update_zero_negative(self.x);
                self.cycles = 4 + pc as u8;
            }

            // ── LDY ─────────────────────────────────────────────
            0xA0 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                self.y = bus.cpu_read(addr);
                self.update_zero_negative(self.y);
                self.cycles = 2;
            }
            0xA4 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.y = bus.cpu_read(addr);
                self.update_zero_negative(self.y);
                self.cycles = 3;
            }
            0xB4 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.y = bus.cpu_read(addr);
                self.update_zero_negative(self.y);
                self.cycles = 4;
            }
            0xAC => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.y = bus.cpu_read(addr);
                self.update_zero_negative(self.y);
                self.cycles = 4;
            }
            0xBC => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.y = bus.cpu_read(addr);
                self.update_zero_negative(self.y);
                self.cycles = 4 + pc as u8;
            }

            // ── STA ─────────────────────────────────────────────
            0x85 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                bus.cpu_write(addr, self.a);
                self.cycles = 3;
            }
            0x95 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                bus.cpu_write(addr, self.a);
                self.cycles = 4;
            }
            0x8D => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                bus.cpu_write(addr, self.a);
                self.cycles = 4;
            }
            0x9D => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                bus.cpu_write(addr, self.a);
                self.cycles = 5;
            }
            0x99 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                bus.cpu_write(addr, self.a);
                self.cycles = 5;
            }
            0x81 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                bus.cpu_write(addr, self.a);
                self.cycles = 6;
            }
            0x91 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectY);
                bus.cpu_write(addr, self.a);
                self.cycles = 6;
            }

            // ── STX ─────────────────────────────────────────────
            0x86 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                bus.cpu_write(addr, self.x);
                self.cycles = 3;
            }
            0x96 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageY);
                bus.cpu_write(addr, self.x);
                self.cycles = 4;
            }
            0x8E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                bus.cpu_write(addr, self.x);
                self.cycles = 4;
            }

            // ── STY ─────────────────────────────────────────────
            0x84 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                bus.cpu_write(addr, self.y);
                self.cycles = 3;
            }
            0x94 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                bus.cpu_write(addr, self.y);
                self.cycles = 4;
            }
            0x8C => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                bus.cpu_write(addr, self.y);
                self.cycles = 4;
            }

            // ── ADC ─────────────────────────────────────────────
            0x69 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 2;
            }
            0x65 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 3;
            }
            0x75 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 4;
            }
            0x6D => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 4;
            }
            0x7D => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 4 + pc as u8;
            }
            0x79 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 4 + pc as u8;
            }
            0x61 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 6;
            }
            0x71 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 5 + pc as u8;
            }

            // ── SBC ─────────────────────────────────────────────
            0xE9 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 2;
            }
            0xE5 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 3;
            }
            0xF5 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 4;
            }
            0xED => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 4;
            }
            0xFD => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 4 + pc as u8;
            }
            0xF9 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 4 + pc as u8;
            }
            0xE1 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 6;
            }
            0xF1 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                let val = bus.cpu_read(addr);
                self.sbc(val);
                self.cycles = 5 + pc as u8;
            }

            // ── CMP ─────────────────────────────────────────────
            0xC9 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 2;
            }
            0xC5 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 3;
            }
            0xD5 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 4;
            }
            0xCD => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 4;
            }
            0xDD => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 4 + pc as u8;
            }
            0xD9 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 4 + pc as u8;
            }
            0xC1 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 6;
            }
            0xD1 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                let val = bus.cpu_read(addr);
                self.compare(self.a, val);
                self.cycles = 5 + pc as u8;
            }

            // ── CPX ─────────────────────────────────────────────
            0xE0 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                let val = bus.cpu_read(addr);
                self.compare(self.x, val);
                self.cycles = 2;
            }
            0xE4 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr);
                self.compare(self.x, val);
                self.cycles = 3;
            }
            0xEC => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr);
                self.compare(self.x, val);
                self.cycles = 4;
            }

            // ── CPY ─────────────────────────────────────────────
            0xC0 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                let val = bus.cpu_read(addr);
                self.compare(self.y, val);
                self.cycles = 2;
            }
            0xC4 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr);
                self.compare(self.y, val);
                self.cycles = 3;
            }
            0xCC => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr);
                self.compare(self.y, val);
                self.cycles = 4;
            }

            // ── INC ─────────────────────────────────────────────
            0xE6 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 5;
            }
            0xF6 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 6;
            }
            0xEE => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 6;
            }
            0xFE => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 7;
            }

            // ── INX ─────────────────────────────────────────────
            0xE8 => {
                self.x = self.x.wrapping_add(1);
                self.update_zero_negative(self.x);
                self.cycles = 2;
            }

            // ── INY ─────────────────────────────────────────────
            0xC8 => {
                self.y = self.y.wrapping_add(1);
                self.update_zero_negative(self.y);
                self.cycles = 2;
            }

            // ── DEC ─────────────────────────────────────────────
            0xC6 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 5;
            }
            0xD6 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 6;
            }
            0xCE => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 6;
            }
            0xDE => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.update_zero_negative(val);
                self.cycles = 7;
            }

            // ── DEX ─────────────────────────────────────────────
            0xCA => {
                self.x = self.x.wrapping_sub(1);
                self.update_zero_negative(self.x);
                self.cycles = 2;
            }

            // ── DEY ─────────────────────────────────────────────
            0x88 => {
                self.y = self.y.wrapping_sub(1);
                self.update_zero_negative(self.y);
                self.cycles = 2;
            }

            // ── AND ─────────────────────────────────────────────
            0x29 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 2;
            }
            0x25 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 3;
            }
            0x35 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0x2D => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0x3D => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0x39 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0x21 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x31 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 5 + pc as u8;
            }

            // ── ORA ─────────────────────────────────────────────
            0x09 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 2;
            }
            0x05 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 3;
            }
            0x15 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0x0D => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0x1D => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0x19 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0x01 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x11 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 5 + pc as u8;
            }

            // ── EOR ─────────────────────────────────────────────
            0x49 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Immediate);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 2;
            }
            0x45 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 3;
            }
            0x55 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0x4D => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }
            0x5D => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0x59 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 4 + pc as u8;
            }
            0x41 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x51 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 5 + pc as u8;
            }

            // ── BIT ─────────────────────────────────────────────
            0x24 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr);
                self.set_flag(ZERO, (self.a & val) == 0);
                self.set_flag(NEGATIVE, val & 0x80 != 0);
                self.set_flag(OVERFLOW, val & 0x40 != 0);
                self.cycles = 3;
            }
            0x2C => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr);
                self.set_flag(ZERO, (self.a & val) == 0);
                self.set_flag(NEGATIVE, val & 0x80 != 0);
                self.set_flag(OVERFLOW, val & 0x40 != 0);
                self.cycles = 4;
            }

            // ── ASL ─────────────────────────────────────────────
            0x0A => {
                self.asl_acc();
                self.cycles = 2;
            }
            0x06 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.asl_mem(bus, addr);
                self.cycles = 5;
            }
            0x16 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.asl_mem(bus, addr);
                self.cycles = 6;
            }
            0x0E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.asl_mem(bus, addr);
                self.cycles = 6;
            }
            0x1E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.asl_mem(bus, addr);
                self.cycles = 7;
            }

            // ── LSR ─────────────────────────────────────────────
            0x4A => {
                self.lsr_acc();
                self.cycles = 2;
            }
            0x46 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.lsr_mem(bus, addr);
                self.cycles = 5;
            }
            0x56 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.lsr_mem(bus, addr);
                self.cycles = 6;
            }
            0x4E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.lsr_mem(bus, addr);
                self.cycles = 6;
            }
            0x5E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.lsr_mem(bus, addr);
                self.cycles = 7;
            }

            // ── ROL ─────────────────────────────────────────────
            0x2A => {
                self.rol_acc();
                self.cycles = 2;
            }
            0x26 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.rol_mem(bus, addr);
                self.cycles = 5;
            }
            0x36 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.rol_mem(bus, addr);
                self.cycles = 6;
            }
            0x2E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.rol_mem(bus, addr);
                self.cycles = 6;
            }
            0x3E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.rol_mem(bus, addr);
                self.cycles = 7;
            }

            // ── ROR ─────────────────────────────────────────────
            0x6A => {
                self.ror_acc();
                self.cycles = 2;
            }
            0x66 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.ror_mem(bus, addr);
                self.cycles = 5;
            }
            0x76 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.ror_mem(bus, addr);
                self.cycles = 6;
            }
            0x6E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.ror_mem(bus, addr);
                self.cycles = 6;
            }
            0x7E => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.ror_mem(bus, addr);
                self.cycles = 7;
            }

            // ── Branches ────────────────────────────────────────
            0x90 => { // BCC
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(!self.get_flag(CARRY), addr, pc);
            }
            0xB0 => { // BCS
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(self.get_flag(CARRY), addr, pc);
            }
            0xF0 => { // BEQ
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(self.get_flag(ZERO), addr, pc);
            }
            0x30 => { // BMI
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(self.get_flag(NEGATIVE), addr, pc);
            }
            0xD0 => { // BNE
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(!self.get_flag(ZERO), addr, pc);
            }
            0x10 => { // BPL
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(!self.get_flag(NEGATIVE), addr, pc);
            }
            0x50 => { // BVC
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(!self.get_flag(OVERFLOW), addr, pc);
            }
            0x70 => { // BVS
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::Relative);
                self.cycles = 2;
                self.branch(self.get_flag(OVERFLOW), addr, pc);
            }

            // ── JMP ─────────────────────────────────────────────
            0x4C => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.pc = addr;
                self.cycles = 3;
            }
            0x6C => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Indirect);
                self.pc = addr;
                self.cycles = 5;
            }

            // ── JSR ─────────────────────────────────────────────
            0x20 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.push16(bus, self.pc.wrapping_sub(1));
                self.pc = addr;
                self.cycles = 6;
            }

            // ── RTS ─────────────────────────────────────────────
            0x60 => {
                let addr = self.pull16(bus);
                self.pc = addr.wrapping_add(1);
                self.cycles = 6;
            }

            // ── RTI ─────────────────────────────────────────────
            0x40 => {
                self.status = self.pull(bus);
                self.set_flag(BREAK, false);
                self.set_flag(UNUSED, true);
                self.pc = self.pull16(bus);
                self.cycles = 6;
            }

            // ── PHA ─────────────────────────────────────────────
            0x48 => {
                self.push(bus, self.a);
                self.cycles = 3;
            }

            // ── PHP ─────────────────────────────────────────────
            0x08 => {
                self.push(bus, self.status | BREAK | UNUSED);
                self.cycles = 3;
            }

            // ── PLA ─────────────────────────────────────────────
            0x68 => {
                self.a = self.pull(bus);
                self.update_zero_negative(self.a);
                self.cycles = 4;
            }

            // ── PLP ─────────────────────────────────────────────
            0x28 => {
                let val = self.pull(bus);
                self.status = (val & !(BREAK | UNUSED)) | (self.status & (BREAK | UNUSED));
                self.cycles = 4;
            }

            // ── BRK ─────────────────────────────────────────────
            0x00 => {
                self.pc = self.pc.wrapping_add(1);
                self.push16(bus, self.pc);
                self.push(bus, self.status | BREAK | UNUSED);
                self.set_flag(INTERRUPT_DISABLE, true);
                let lo = bus.cpu_read(0xFFFE) as u16;
                let hi = bus.cpu_read(0xFFFF) as u16;
                self.pc = (hi << 8) | lo;
                self.cycles = 7;
            }

            // ── NOP ─────────────────────────────────────────────
            0xEA => {
                self.cycles = 2;
            }

            // ── Flag instructions ───────────────────────────────
            0x38 => { self.set_flag(CARRY, true); self.cycles = 2; }             // SEC
            0x18 => { self.set_flag(CARRY, false); self.cycles = 2; }            // CLC
            0x78 => { self.set_flag(INTERRUPT_DISABLE, true); self.cycles = 2; } // SEI
            0x58 => { self.set_flag(INTERRUPT_DISABLE, false); self.cycles = 2; }// CLI
            0xF8 => { self.set_flag(DECIMAL, true); self.cycles = 2; }           // SED
            0xD8 => { self.set_flag(DECIMAL, false); self.cycles = 2; }          // CLD
            0xB8 => { self.set_flag(OVERFLOW, false); self.cycles = 2; }         // CLV

            // ── Transfer instructions ───────────────────────────
            0xAA => { self.x = self.a; self.update_zero_negative(self.x); self.cycles = 2; } // TAX
            0xA8 => { self.y = self.a; self.update_zero_negative(self.y); self.cycles = 2; } // TAY
            0x8A => { self.a = self.x; self.update_zero_negative(self.a); self.cycles = 2; } // TXA
            0x98 => { self.a = self.y; self.update_zero_negative(self.a); self.cycles = 2; } // TYA
            0xBA => { self.x = self.sp; self.update_zero_negative(self.x); self.cycles = 2; }// TSX
            0x9A => { self.sp = self.x; self.cycles = 2; }                                   // TXS

            // ═════════════════════════════════════════════════════
            // ILLEGAL / UNOFFICIAL OPCODES
            // ═════════════════════════════════════════════════════

            // ── LAX (LDA + LDX) ─────────────────────────────────
            0xA7 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr);
                self.a = val; self.x = val;
                self.update_zero_negative(val);
                self.cycles = 3;
            }
            0xB7 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageY);
                let val = bus.cpu_read(addr);
                self.a = val; self.x = val;
                self.update_zero_negative(val);
                self.cycles = 4;
            }
            0xAF => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr);
                self.a = val; self.x = val;
                self.update_zero_negative(val);
                self.cycles = 4;
            }
            0xBF => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                let val = bus.cpu_read(addr);
                self.a = val; self.x = val;
                self.update_zero_negative(val);
                self.cycles = 4 + pc as u8;
            }
            0xA3 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                let val = bus.cpu_read(addr);
                self.a = val; self.x = val;
                self.update_zero_negative(val);
                self.cycles = 6;
            }
            0xB3 => {
                let (addr, pc) = self.get_operand_address(bus, AddressingMode::IndirectY);
                let val = bus.cpu_read(addr);
                self.a = val; self.x = val;
                self.update_zero_negative(val);
                self.cycles = 5 + pc as u8;
            }

            // ── SAX (store A & X) ───────────────────────────────
            0x87 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                bus.cpu_write(addr, self.a & self.x);
                self.cycles = 3;
            }
            0x97 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageY);
                bus.cpu_write(addr, self.a & self.x);
                self.cycles = 4;
            }
            0x8F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                bus.cpu_write(addr, self.a & self.x);
                self.cycles = 4;
            }
            0x83 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                bus.cpu_write(addr, self.a & self.x);
                self.cycles = 6;
            }

            // ── DCP (DEC + CMP) ─────────────────────────────────
            0xC7 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.compare(self.a, val);
                self.cycles = 5;
            }
            0xD7 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.compare(self.a, val);
                self.cycles = 6;
            }
            0xCF => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.compare(self.a, val);
                self.cycles = 6;
            }
            0xDF => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.compare(self.a, val);
                self.cycles = 7;
            }
            0xDB => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.compare(self.a, val);
                self.cycles = 7;
            }
            0xC3 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.compare(self.a, val);
                self.cycles = 8;
            }
            0xD3 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectY);
                let val = bus.cpu_read(addr).wrapping_sub(1);
                bus.cpu_write(addr, val);
                self.compare(self.a, val);
                self.cycles = 8;
            }

            // ── ISB / ISC (INC + SBC) ───────────────────────────
            0xE7 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.sbc(val);
                self.cycles = 5;
            }
            0xF7 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.sbc(val);
                self.cycles = 6;
            }
            0xEF => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.sbc(val);
                self.cycles = 6;
            }
            0xFF => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.sbc(val);
                self.cycles = 7;
            }
            0xFB => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.sbc(val);
                self.cycles = 7;
            }
            0xE3 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.sbc(val);
                self.cycles = 8;
            }
            0xF3 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectY);
                let val = bus.cpu_read(addr).wrapping_add(1);
                bus.cpu_write(addr, val);
                self.sbc(val);
                self.cycles = 8;
            }

            // ── SLO (ASL + ORA) ─────────────────────────────────
            0x07 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.asl_mem(bus, addr);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 5;
            }
            0x17 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.asl_mem(bus, addr);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x0F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.asl_mem(bus, addr);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x1F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.asl_mem(bus, addr);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 7;
            }
            0x1B => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.asl_mem(bus, addr);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 7;
            }
            0x03 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.asl_mem(bus, addr);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 8;
            }
            0x13 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.asl_mem(bus, addr);
                self.a |= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 8;
            }

            // ── RLA (ROL + AND) ─────────────────────────────────
            0x27 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.rol_mem(bus, addr);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 5;
            }
            0x37 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.rol_mem(bus, addr);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x2F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.rol_mem(bus, addr);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x3F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.rol_mem(bus, addr);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 7;
            }
            0x3B => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.rol_mem(bus, addr);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 7;
            }
            0x23 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.rol_mem(bus, addr);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 8;
            }
            0x33 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.rol_mem(bus, addr);
                self.a &= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 8;
            }

            // ── SRE (LSR + EOR) ─────────────────────────────────
            0x47 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.lsr_mem(bus, addr);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 5;
            }
            0x57 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.lsr_mem(bus, addr);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x4F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.lsr_mem(bus, addr);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 6;
            }
            0x5F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.lsr_mem(bus, addr);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 7;
            }
            0x5B => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.lsr_mem(bus, addr);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 7;
            }
            0x43 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.lsr_mem(bus, addr);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 8;
            }
            0x53 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.lsr_mem(bus, addr);
                self.a ^= bus.cpu_read(addr);
                self.update_zero_negative(self.a);
                self.cycles = 8;
            }

            // ── RRA (ROR + ADC) ─────────────────────────────────
            0x67 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPage);
                self.ror_mem(bus, addr);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 5;
            }
            0x77 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::ZeroPageX);
                self.ror_mem(bus, addr);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 6;
            }
            0x6F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::Absolute);
                self.ror_mem(bus, addr);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 6;
            }
            0x7F => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteX);
                self.ror_mem(bus, addr);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 7;
            }
            0x7B => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::AbsoluteY);
                self.ror_mem(bus, addr);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 7;
            }
            0x63 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectX);
                self.ror_mem(bus, addr);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 8;
            }
            0x73 => {
                let (addr, _) = self.get_operand_address(bus, AddressingMode::IndirectY);
                self.ror_mem(bus, addr);
                let val = bus.cpu_read(addr);
                self.adc(val);
                self.cycles = 8;
            }

            // ── Illegal NOPs ────────────────────────────────────
            // 1-byte NOPs (implicit)
            0x1A | 0x3A | 0x5A | 0x7A | 0xDA | 0xFA => {
                self.cycles = 2;
            }
            // 2-byte NOPs (skip one byte)
            0x04 | 0x14 | 0x34 | 0x44 | 0x54 | 0x64 | 0x74 |
            0x80 | 0x82 | 0x89 | 0xC2 | 0xD4 | 0xE2 | 0xF4 => {
                self.pc = self.pc.wrapping_add(1);
                self.cycles = 2;
            }
            // 3-byte NOPs (skip two bytes)
            0x0C | 0x1C | 0x3C | 0x5C | 0x7C | 0xDC | 0xFC => {
                self.pc = self.pc.wrapping_add(2);
                self.cycles = 2;
            }

            // ── Catch-all: unknown opcodes → 1-byte NOP ─────────
            _ => {
                self.cycles = 2;
            }
        }
    }
}

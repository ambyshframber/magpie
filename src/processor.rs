use super::Memory;

const SP: usize = 13;
const LINK_REG: usize = 14;
const PC: usize = 15;

const IRQ_VEC: u16 = 0xfffa; // fa and fb
const NMI_VEC: u16 = 0xfffc; // fc and fd
const RESET_VEC: u16 = 0xfffe; // fe and ff

pub struct Processor {
    registers: [u16; 16],

    zero: bool,
    negative: bool,
    carry: bool,

    interrupts: bool,

    delay_vals: [u16; 2], // intentional branch delay slots tee hee
    delay_regs: [u8; 2],

    delay_ctr: usize,
}
impl Processor {
    pub fn new() -> Processor {
        Processor {
            registers: [0; 16],
            carry: false, zero: false, negative: false,
            interrupts: false,
            delay_vals: [0; 2],
            delay_regs: [0; 2],
            delay_ctr: 0
        }
    }
    pub fn reset<M: Memory>(&mut self, mem: &mut M) {
        let pc = mem.read(RESET_VEC);
        self.registers[PC] = pc
    }

    fn get_flags(&self) -> u16 {
        let mut ret = self.zero as u16;
        ret |= (self.negative as u16) << 1;
        ret |= (self.carry as u16) << 2;

        ret
    }
    fn set_flags(&mut self, f: u16) {
        self.zero = f & 0b1 != 0;
        self.negative = f & 0b10 != 0;
        self.carry = f & 0b100 != 0;
    }

    fn set_delay(&mut self, reg: u8, val: u16) {
        self.delay_vals[self.delay_ctr] = val;
        self.delay_regs[self.delay_ctr] = reg;
    }
    fn read_reg(&self, id: u8) -> u16 {
        if id == 0 {
            0
        }
        else {
            self.registers[(id & 0xf) as usize]
        }
    }
    fn write_reg(&mut self, id: u8, val: u16) {
        if id == PC as u8 {
            self.set_delay(PC as u8, val) // can't bypass the delay slot using a move
        }
        else {
            self.registers[(id & 0xf) as usize] = val;
            self.zero = val == 0;
            self.negative = (val as i16) < 0;
        }
    }

    fn write_delays(&mut self) {
        // flip delay count before moving values
        // so last time's values don't move
        self.delay_ctr ^= 1;
        self.registers[(self.delay_regs[self.delay_ctr] & 0xf) as usize] = self.delay_vals[self.delay_ctr];
        // set reg to zero such that it does nothing next time
        self.delay_regs[self.delay_ctr] = 0;
    }

    pub fn clock<M: Memory>(&mut self, mem: &mut M) {
        self.write_delays();
        let instr = mem.read(self.registers[PC]);

        if instr & 0b1000 == 0 { // short opcode
            self.short_op(mem, instr)
        }
        else { // long opcode
            let r1 = ((instr & 0xf000) >> 12) as u8;
            let r2 = ((instr & 0x0f00) >> 8) as u8;

            match instr & 0x8f {
                0x8 => self.jump(instr, r1, r2),
                0x88 => self.movement(instr, r1, r2, mem),
                0x9 | 0x89 => self.arithmetic(instr, r1, r2),
                _ => {}
            }
        }

        self.registers[PC] += 2
    }

    fn jump(&mut self, instr: u16, ra: u8, rl: u8) {
        if match (instr & 0b111_0000) >> 4 {
            0 => true,
            1 => self.zero,
            2 => !self.zero,
            3 => self.negative,
            _ => false
        } {
            let link = self.registers[PC] + 4;
            self.write_reg(rl, link);

            let address = self.read_reg(ra);
            self.set_delay(PC as u8, address);
        }
    }
    fn arithmetic(&mut self, instr: u16, rs: u8, rd: u8) {
        let src = self.read_reg(rs);
        let dest = self.read_reg(rd);
        let op = (instr & 0xf0) >> 4;
        let result = match op {
            0x0..=0x3 => {
                let (val, carry) = self.add_sub(op, src, dest);
                self.carry = carry;
                val
            }
            0x4 => src & dest,
            0x5 => !dest,
            0x6 => src | dest,
            0x7 => src ^ dest,
            0x8 => src.overflowing_shl(dest as u32).0,
            0x9 => src.overflowing_shr(dest as u32).0,
            0xa => (src as i16).overflowing_shl(dest as u32).0 as u16,
            0xb => (src as i16).overflowing_shr(dest as u32).0 as u16,
            0xc => src.wrapping_shl(dest as u32),
            0xd => src.wrapping_shr(dest as u32),
            0xe => self.get_flags(),
            0xf => {
                self.set_flags(src);
                dest
            }
            _ => unreachable!()
        };
        self.write_reg(rd, result)
    }
    fn movement<M: Memory>(&mut self, instr: u16, rs: u8, rd: u8, mem: &mut M) {
        match (instr & 0b0011_0000) >> 4 {
            0 => { // push
                let ptr = self.read_reg(rs);
                mem.write(ptr, self.read_reg(rd));
                self.write_reg(rs, ptr.wrapping_add(2))
            }
            1 => {
                let ptr = self.read_reg(rs).wrapping_sub(2);
                let val = mem.read(ptr);
                self.set_delay(rd, val);
                self.write_reg(rs, ptr);
            }
            2 => {
                let val = self.read_reg(rs);
                self.write_reg(rd, val)
            }
            3 => {
                let val = self.read_reg(rs) as i8 as i16 as u16; // EXTEND
                self.write_reg(rd, val)
            }
            _ => unreachable!()
        }
    }

    fn add_sub(&mut self, op: u16, lhs: u16, rhs: u16) -> (u16, bool) {
        let carry = (op & 1) != 0 && self.carry;
        if op & 0b10 == 0 {
            lhs.carrying_add(rhs, carry)    
        }
        else {
            let (val, borrow) = lhs.borrowing_sub(rhs, !carry);
            (val, !borrow)
        }
    }

    fn short_op<M: Memory>(&mut self, mem: &mut M, instr: u16) {
        let rd = ((instr & 0xf0) >> 4) as u8;
        match instr & 0b110 {
            0b100 => { // ld/st
                let ra = ((instr & 0xf000) >> 12) as u8;
                let addr = self.read_reg(ra);
                let ro = ((instr & 0x0f00) >> 8) as u8;
                let offset = self.read_reg(ro);

                if instr & 1 == 0 {
                    let val = mem.read(addr.wrapping_add(offset));
                    self.set_delay(rd, val)
                }
                else {
                    mem.write(addr, self.read_reg(rd))
                }
            }
            0b110 => { // rjmp/rjal
                let offset_ek = ((instr & 0xfff0) >> 3) as i16; // shift 3 because bit 0 is always 0
                let offset_corrected = offset_ek - 2i16.pow(12); // excess k, where k is 2**13
                let new_pc = self.registers[PC].wrapping_add_signed(offset_corrected);
                if instr & 1 != 0 { // link
                    self.registers[LINK_REG] = self.registers[PC] + 4;
                }

                self.set_delay(PC as u8, new_pc)
            }
            _ => { // imm-reg
                let mut val = (instr & 0xff00) >> 8;
                match instr & 0b11 {
                    0 => val = val as i8 as i16 as u16,
                    1 => { // ldh
                        let old = self.read_reg(rd) & 0xff;
                        val = old | instr & 0xff00
                    }
                    2 => { // adi
                        let old = self.read_reg(rd);
                        let (new, carry) = old.carrying_add(val, false);
                        self.carry = carry;
                        val = new
                    }
                    3 => { // sbi
                        let old = self.read_reg(rd);
                        let (new, borrow) = old.borrowing_sub(val, false);
                        self.carry = !borrow;
                        val = new
                    }
                    _ => unreachable!()
                }
                self.write_reg(rd, val)
            }
        }
    }
}
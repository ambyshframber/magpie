use super::Memory;

const LINK_REG: usize = 14;
const PC: usize = 15;

const IRQ_VEC: u16 = 0xfffa; // fa and fb
const NMI_VEC: u16 = 0xfffc; // fc and fd
const RESET_VEC: u16 = 0xfffe; // fe and ff

#[derive(Copy, Clone, PartialEq)]
enum ShouldWriteFlags {
    No, No2, No3, Yes
}
impl ShouldWriteFlags {
    pub fn cycle(self) -> Self {
        match self {
            Self::No => Self::No3,
            Self::No2 => Self::No3,
            Self::No3 => Self::Yes,
            Self::Yes => Self::Yes
        }
    }
}

pub struct Processor {
    registers: [u16; 16],

    zero: bool,
    negative: bool,
    carry: bool,

    interrupts: bool,
    iret: u16,

    fault: bool,
    should_write_flags: ShouldWriteFlags
}
impl Processor {
    pub fn new() -> Processor {
        Processor {
            registers: [0; 16],
            carry: false, zero: false, negative: false,
            interrupts: false, iret: 0,
            fault: false,
            should_write_flags: ShouldWriteFlags::Yes
        }
    }
    pub fn reset<M: Memory>(&mut self, mem: &mut M) {
        let pc = u16::from_le_bytes(mem.read(RESET_VEC));
        self.registers[PC] = pc;
    }

    fn get_flags(&self) -> u16 {
        let mut ret = self.zero as u16;
        ret |= (self.negative as u16) << 1;
        ret |= (self.carry as u16) << 2;
        ret |= (self.interrupts as u16) << 3;
        ret |= (self.fault as u16) << 4;

        ret
    }
    fn set_flags(&mut self, f: u16) {
        self.zero = f & 0b1 != 0;
        self.negative = f & 0b10 != 0;
        self.carry = f & 0b100 != 0;
        self.interrupts = f & 0b1000 != 0;
        self.fault = f & 0b1_0000 != 0;
        self.should_write_flags = ShouldWriteFlags::No;
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
        self.write_reg_no_flags(id, val);
        if self.should_write_flags == ShouldWriteFlags::Yes {
            self.zero = val == 0;
            self.negative = (val as i16) < 0;
            //eprintln!("zero {} neg {}", self.zero, self.negative)
        }
    }
    fn write_reg_no_flags(&mut self, id: u8, val: u16) {
        self.registers[(id & 0xf) as usize] = val;
    }

    pub fn clock<M: Memory>(&mut self, mem: &mut M) {
        let instr = u16::from_be_bytes(mem.read(self.registers[PC]));

        #[cfg(debug_assertions)] {
            eprintln!("pc: {:04x}; cur: {:04x}", self.registers[PC], instr);
            for v in self.registers {
                eprint!("{:04x} ", v)
            }
            eprintln!();
        }

        self.should_write_flags = self.should_write_flags.cycle();

        self.do_instruction(instr, mem);

        self.registers[PC] = self.registers[PC].wrapping_add(2)
    }

    fn do_instruction<M: Memory>(&mut self, instr: u16, mem: &mut M) {
        if instr & 0b1000 == 0 || instr & 0b1100 == 0b1100 { // short opcode
            self.short_op(mem, instr)
        }
        else { // long opcode
            let r1 = ((instr & 0xf000) >> 12) as u8;
            let r2 = ((instr & 0x0f00) >> 8) as u8;

            match instr & 0xf {
                0x8 => {
                    match (instr & 0b1100_0000) >> 6 {
                        0 => self.jump(instr, r1, r2),
                        1 => self.misc(instr, r1, r2, mem),
                        2 => self.movement(instr, r1, r2, mem),
                        _ => {
                            if instr & 0xf0 == 0xc0 { // int
                                self.nmi(mem)
                            }
                            // else nop
                        }
                    }
                }
                0x9 => self.arithmetic(instr, r1, r2),
                _ => {}
            }
        }
    }

    fn misc<M: Memory>(&mut self, instr: u16, r1: u8, r2: u8, mem: &mut M) { // interrupts etc
        //eprintln!("misc");
        match (instr & 0b11_0000) >> 4 {
            0 => { // psr
                let iret = self.iret;
                //eprintln!("psr {:04x}", iret);
                self.push(r1, iret, mem)
            }
            1 => { // iret
                let iret = self.pop(r1, mem);
                self.registers[PC] = iret;
                self.interrupts = true;
            }
            2 => self.write_reg(r2, self.get_flags()),
            _ => {
                self.set_flags(self.read_reg(r1));
                self.should_write_flags = ShouldWriteFlags::No;
            }
        }
    }

    fn nmi<M: Memory>(&mut self, mem: &mut M) {
        self.iret = self.registers[PC];
        let new_addr = u16::from_le_bytes(mem.read(NMI_VEC)).wrapping_sub(2);
        self.registers[PC] = new_addr;
        self.interrupts = false;
    }
    pub fn irq<M: Memory>(&mut self, mem: &mut M) {
        //eprintln!("{}", self.interrupts);
        if self.interrupts && self.should_write_flags == ShouldWriteFlags::Yes {
            #[cfg(debug_assertions)] {
                eprintln!("irq!");
            }
            self.iret = self.registers[PC].wrapping_sub(2);
            let new_addr = u16::from_le_bytes(mem.read(IRQ_VEC));
            self.registers[PC] = new_addr;
            self.interrupts = false;
            //eprintln!("program counter: {:04x}", self.registers[PC]);
        }
    }

    fn jump(&mut self, instr: u16, ra: u8, rl: u8) {
        if match (instr & 0b111_0000) >> 4 {
            0 => true,
            1 => self.zero,
            2 => !self.zero,
            3 => self.negative,
            _ => false
        } {
            let link = self.registers[PC] + 2;
            self.write_reg(rl, link);
            
            let address = self.read_reg(ra);
            self.registers[PC] = address.wrapping_sub(2);
            //eprintln!("jumping to {:04x}", address);
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
            0x8 => dest.overflowing_shl(src as u32).0,
            0x9 => dest.overflowing_shr(src as u32).0,
            0xa => (dest as i16).overflowing_shl(src as u32).0 as u16,
            0xb => (dest as i16).overflowing_shr(src as u32).0 as u16,
            0xc => dest.wrapping_shl(src as u32),
            0xd => dest.wrapping_shr(src as u32),
            0xe => self.get_flags(),
            0xf => {
                self.set_flags(src);
                dest
            }
            _ => dest
        };
        self.write_reg(rd, result)
    }

    fn push<M: Memory>(&mut self, rs: u8, val: u16, mem: &mut M) {
        let ptr = self.read_reg(rs);
        //eprintln!("push {:04x} to {:04x}", val, ptr);
        mem.write(ptr, val.to_le_bytes());
        self.write_reg_no_flags(rs, ptr.wrapping_sub(2)) // stacks grow down
    }
    fn pop<M: Memory>(&mut self, rs: u8, mem: &mut M) -> u16 {
        let ptr = self.read_reg(rs).wrapping_add(2);
        let val = u16::from_le_bytes(mem.read(ptr));
        //eprintln!("pop {:04x} from {:04x}", val, ptr);
        self.write_reg_no_flags(rs, ptr);
        val
    }

    fn movement<M: Memory>(&mut self, instr: u16, rs: u8, rd: u8, mem: &mut M) {
        match (instr & 0b0011_0000) >> 4 {
            0 => { // push
                self.push(rs, self.read_reg(rd), mem)
            }
            1 => { // pop
                let val = self.pop(rs, mem);
                self.write_reg(rd, val);
            }
            2 => { // mov
                let val = self.read_reg(rs);
                self.write_reg(rd, val)
            }
            3 => { // msx
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

    fn ld_st<M: Memory>(&mut self, mem: &mut M, instr: u16, ra: u8, ro: u8, rd: u8) {
        let addr = self.read_reg(ra);
        let offset = self.read_reg(ro);
        let eaddr = addr.wrapping_add(offset);
        if instr & 0b10 == 0 { // 16 bit
            if instr & 1 == 0 { // load
                let val = u16::from_le_bytes(mem.read(eaddr));
                self.write_reg(rd, val)
            }
            else {
                mem.write(eaddr, self.read_reg(rd).to_le_bytes())
            }
        }
        else {
            if instr & 1 == 0 { // load
                let val = mem.read_8(eaddr) as u16;
                self.write_reg(rd, val)
            }
            else {
                mem.write_8(eaddr, self.read_reg(rd).to_le_bytes()[0])
            }
        }
    }

    fn short_op<M: Memory>(&mut self, mem: &mut M, instr: u16) {
        let rd = ((instr & 0xf0) >> 4) as u8;
        match instr & 0b1100 {
            0b0100 => { // ld/st
                let ra = ((instr & 0xf000) >> 12) as u8;
                let ro = ((instr & 0x0f00) >> 8) as u8;
                let rd = ((instr & 0x00f0) >> 4) as u8;
                self.ld_st(mem, instr, ra, ro, rd)
            }
            0b1100 => { // rjmp
                let (jump, link) = match instr & 0b11 {
                    0b00 => (true, false),
                    0b01 => (true, true),
                    0b10 => (self.zero, false),
                    _ => (self.negative, false)
                };
                if jump {
                    let offset_ek = ((instr & 0xfff0) >> 3) as i16; // shift 3 because bit 0 is always 0
                    let offset_corrected = offset_ek - 2i16.pow(12); // excess k, where k is 2**13
                    let new_pc = self.registers[PC].wrapping_add_signed(offset_corrected);
                    if link { // link
                        self.registers[LINK_REG] = self.registers[PC] + 2;
                    }
                    self.registers[PC] = new_pc.wrapping_sub(2)
                }
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

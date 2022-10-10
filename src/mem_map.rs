use termion::{raw::*, AsyncReader, async_stdin};
use std::io::{stdout, Stdout, Read, Write};
use std::collections::VecDeque;
use super::Memory;

const MAIN_MEM_SIZE: usize = 2usize.pow(15);

const ROM_START: usize = 0xf000;
const ROM_SIZE: usize = 0x1000;

const SERIAL_TX: usize = 0xe000;
const SERIAL_RX: usize = 0xe002;

const EXIT: usize = 0xe100;

pub struct MemoryMap {
    main_mem: [u8; MAIN_MEM_SIZE],
    serial: Serial,
    rom: [u8; ROM_SIZE],
    should_exit: bool,
}
impl MemoryMap {
    pub fn new(rom: [u8; ROM_SIZE]) -> Self {
        MemoryMap {
            main_mem: [0; MAIN_MEM_SIZE],
            serial: Serial::new(),
            rom,
            should_exit: false
        }
    }
}
impl Memory for MemoryMap {
    fn read(&mut self, addr: u16) -> [u8; 2] {
        //eprintln!("read from {:04x}", addr);
        let addr = addr as usize;
        if addr < MAIN_MEM_SIZE {
            let lo = self.main_mem[addr];
            let hi = self.main_mem.get(addr + 1).unwrap_or(&0);
            [lo, *hi]
        }
        else if addr >= ROM_START {
            let addr = addr - ROM_START;
            let lo = self.rom[addr];
            let hi = self.rom.get(addr + 1).unwrap_or(&0);
            [lo, *hi]
        }
        else if addr == SERIAL_RX {
            self.serial.read()
        }
        else {
            [0; 2]
        }
    }
    fn read_8(&mut self, addr: u16) -> u8 {
        //eprintln!("read from {:04x}\r", addr);
        let addr = addr as usize;
        if addr < MAIN_MEM_SIZE {
            let lo = self.main_mem[addr];
            lo
        }
        else if addr >= ROM_START {
            let addr = addr - ROM_START;
            let lo = self.rom[addr];
            lo
        }
        else if addr == SERIAL_RX {
            self.serial.read()[0]
        }
        else {
            0
        }
    }
    fn write(&mut self, addr: u16, val: [u8; 2]) {
        //eprintln!("write {:02x}{:02x} to {:04x}\r", val[0], val[1], addr);
        let [lo, high] = val;
        let addr = addr as usize;
        if addr < MAIN_MEM_SIZE {
            self.main_mem[addr] = lo;
            if addr + 1 < MAIN_MEM_SIZE {
                self.main_mem[addr + 1] = high
            }
        }
        else if addr == SERIAL_TX {
            self.serial.write(val)
        }
        else if addr == EXIT {
            self.should_exit = true
        }
    }
    fn write_8(&mut self, addr: u16, val: u8) {
        //eprintln!("write {:02x} to {:04x}\r", val, addr);
        let addr = addr as usize;
        if addr < MAIN_MEM_SIZE {
            self.main_mem[addr] = val;
        }
        else if addr == SERIAL_TX {
            self.serial.write([val, 0])
        }
        else if addr == EXIT {
            self.should_exit = true
        }
    }
    fn clock(&mut self) -> bool {
        //eprintln!("clock");
        self.serial.clock()
    }
    fn should_exit(&self) -> bool {
        self.should_exit
    }
}

enum SerialOut {
    Raw(RawTerminal<Stdout>),
    Regular(Stdout)
}
impl Write for SerialOut {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Self::Raw(t) => t.write(buf),
            Self::Regular(t) => t.write(buf)
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Self::Raw(t) => t.flush(),
            Self::Regular(t) => t.flush()
        }
    }
}

struct Serial {
    buf: VecDeque<u8>,
    term: SerialOut,
    term_in: AsyncReader,
    cycles_since_first_byte: usize
}
impl Serial {
    fn new() -> Serial {
        let term = stdout().into_raw_mode().map(|t| SerialOut::Raw(t)).unwrap_or(SerialOut::Regular(stdout()));
        Serial { buf: VecDeque::new(), term, term_in: async_stdin(), cycles_since_first_byte: 0 }
    }

    fn clock(&mut self) -> bool {
        let mut buf = [0; 16];
        let len = self.term_in.read(&mut buf).unwrap(); // just panic, no way to recover
        for idx in 0..len {
            //eprintln!("got serial byte {:02x}\r", buf[idx]);
            if self.buf.len() >= 16 {
                break
            }
            else {
                self.buf.push_back(buf[idx])
            }
        }
        if !self.buf.is_empty() {
            self.cycles_since_first_byte += 1
        }
        self.buf.len() >= 4 || self.cycles_since_first_byte >= 16
    }
    fn read(&mut self) -> [u8; 2] {
        let v = self.buf.pop_front().map(|b| b as u16).unwrap_or(-1i16 as u16);
        if self.buf.is_empty() {
            self.cycles_since_first_byte = 0
        }
        //seprintln!("serial read {:02x}\r", v);
        v.to_le_bytes()
    }
    fn write(&mut self, b: [u8; 2]) {
        let lb = b[0];
        //eprintln!("serial write {:02x}\r", lb);
        self.term.write(&[lb]).unwrap();
        self.term.flush().unwrap();
    }
}

use super::Memory;
use std::io::stdin;

pub struct MemShell {
    
}
impl MemShell {
    pub fn new() -> MemShell {
        MemShell { }
    }
}

impl Memory for MemShell {
    fn read(&mut self, addr: u16) -> [u8; 2] {
        println!("read from {:04x}", addr);
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
        u16::from_str_radix(&buf.trim(), 16).unwrap().to_be_bytes()
    }
    fn write(&mut self, addr: u16, val: [u8; 2]) {
        let val = u16::from_be_bytes(val);
        println!("wrote {:04x} to {:04x}", val, addr);
        let mut buf = String::new();
        stdin().read_line(&mut buf).unwrap();
    }
}

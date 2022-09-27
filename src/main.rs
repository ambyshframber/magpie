#![feature(bigint_helper_methods)]
#![feature(mixed_integer_ops)]

use processor::Processor;

mod processor;
mod debug_mem;

fn main() {
    println!("Hello, world!");
}

struct Computer<M: Memory> {
    mem: M,
    processor: Processor,
}
impl<M: Memory> Computer<M> {
    pub fn new(mem: M) -> Computer<M> {
        Computer {
            mem,
            processor: Processor::new(),
        }
    }
    pub fn run(&mut self) {
        self.processor.reset(&mut self.mem);
        loop {
            self.processor.clock(&mut self.mem);
            if self.mem.clock() {
                self.processor.irq(&mut self.mem)
            }
        }
    }
}

pub trait Memory {
    fn read(&mut self, addr: u16) -> u16;
    fn write(&mut self, addr: u16, val: u16);
    fn clock(&mut self) -> bool { false } // returned value is irq
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn mem_shell() {
        let shell = super::debug_mem::MemShell::new();
        let mut c = Computer::new(shell);
        c.run()
    }
}

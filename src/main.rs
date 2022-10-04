#![feature(bigint_helper_methods)]
#![feature(mixed_integer_ops)]

use processor::Processor;
use std::env::args;
use std::thread::sleep;
use std::time::{Instant, Duration};

mod mem_map;
mod processor;
#[cfg(test)]
mod debug_mem;

fn main() {
    let rom_name = args().skip(1).next().unwrap();
    let rom = std::fs::read(rom_name).unwrap();
    let mem = mem_map::MemoryMap::new(rom.try_into().unwrap());
    let mut c = Computer::new(mem);
    c.run()
}

struct Computer<M: Memory> {
    mem: M,
    processor: Processor,
    clock: Clock
}
impl<M: Memory> Computer<M> {
    pub fn new(mem: M) -> Computer<M> {
        Computer {
            mem,
            processor: Processor::new(),
            clock: Clock::new(1000f64)
        }
    }
    pub fn run(&mut self) {
        self.processor.reset(&mut self.mem);
        //let mut now = Instant::now();
        
        loop {
            self.processor.clock(&mut self.mem);
            if self.mem.clock() {
                //eprintln!("irq on board");
                self.processor.irq(&mut self.mem)
            }
            if self.mem.should_exit() {
                break
            }
            self.clock.wait();
            //let iter_time = Instant::now().duration_since(now);
            //eprintln!("{:?}\r", iter_time);
            //now = Instant::now();
        }
    }
}

pub trait Memory {
    fn read(&mut self, addr: u16) -> [u8; 2];
    fn read_8(&mut self, addr: u16) -> u8;
    fn write(&mut self, addr: u16, val: [u8; 2]);
    fn write_8(&mut self, addr: u16, val: u8);
    fn clock(&mut self) -> bool { false } // returned value is irq
    fn should_exit(&self) -> bool { false }
}

struct Clock {
    prev: Instant,
    period: Duration
}
impl Clock {
    pub fn new(frequency: f64) -> Clock {
        let prev = Instant::now();
        let period = Duration::from_secs_f64(1.0 / frequency);
        Clock {
            prev, period
        }
    }
    pub fn wait(&mut self) {
        let now = Instant::now();
        let next = self.prev + self.period;
        let wait_dur = next.checked_duration_since(now).unwrap_or_else(|| {
            //eprintln!("clock saturated!\r");
            Duration::ZERO
        });
        sleep(wait_dur);
        self.prev = next
    }
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

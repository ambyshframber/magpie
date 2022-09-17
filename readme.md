# magpie cpu

a 16-bit RISCish virtual cpu

## development status

currently, the core emulator is in place, but no functionality exists to load or execute roms, or to do I/O. you can pretend to be the memory by running `cargo test mem_shell -- --ignored --nocapture`. this gives you a "shell" where all reads and writes from the cpu are displayed, and you can decide what values it sees.

## documentation

the documentation needs fancier tables than github will let me add (specifically, merged cells and colours) so the full documentation is hosted on [my website](ambylastname.xyz/magpie)

    .org 0xf000 ; start of rom
    .set char_i 0x69

start:
    ldl 0x68 r1
    ldi 0xe000 r2
    st r2 0 r1
    rjal test_fn
    ldl 0x0a r1
    st r2 0 r1
    ldl 0x0d r1
    st r2 0 r1
    ldi 0xe100 r2
    st r2 0 r1

    .spaceto 0xf800
test_fn:
    push r13 r14
    ldl char_i/l r3
    st r2 0 r3
    pop r13 r14
    ldl 0 0
    jmp r14 0
    ldl 0 0

    
    .spaceto 0xfffe
    .data start

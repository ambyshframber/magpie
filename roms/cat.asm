    .org 0xf000 ; start of rom
    
    .set serial_front 0x200
    .set serial_back 0x201
    .set serial_buf 0x100 ; 256 bytes of buffer

    .set TX 0xe000
    .set RX 0xe002
    .set EXIT 0xe100

start:
    ldi 0x1fe sp
    ldi RX r2
    ldi TX r3
    ldl 0b1000 r1 ; set interrupt flag
    sf r1

    .spaceto 0xf050
_wait_loop:
    ldl 0 0
    rjmp _wait_loop ; spin until interrupted

    .spaceto 0xf100

irq:
    ; make space
    psr r13
    push r13 r12
    gf r12
    push r13 r1


_irq_getser:
    ; get serial bytes and echo back out
    ld r2 0 r1 ; get next byte
    rjn _irq_ret ; jump to iret if negative
    ldl 03 r4
    xor r1 r4
    rjz exit
    stb r3 0 r1
    rjmp _irq_getser

_irq_ret:
    ; return from interrupt
    pop r13 r1
    sf r12
    pop r13 r12
    iret r13

exit:
    ldi 0xe100 r2
    st r2 0 0
    
    .spaceto 0xfffa
    .data irq
    .spaceto 0xfffe
    .data start

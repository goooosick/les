use super::addressing::AddrMode;
use super::Cpu;
use crate::Bus;

type Op = fn(&mut Cpu, &mut Bus);

#[rustfmt::skip]
pub const OP_FUNCS: [Op; 256] = [
//     00        01        02        03        04        05        06        07        08        09        0a        0b        0c        0d        0e        0f
    Cpu::brk, Cpu::ora, Cpu::stp, Cpu::slo, Cpu::nop, Cpu::ora, Cpu::asl, Cpu::slo, Cpu::php, Cpu::ora, Cpu::asl, Cpu::stp, Cpu::nop, Cpu::ora, Cpu::asl, Cpu::slo, // 00
    Cpu::bpl, Cpu::ora, Cpu::stp, Cpu::slo, Cpu::nop, Cpu::ora, Cpu::asl, Cpu::slo, Cpu::clc, Cpu::ora, Cpu::nop, Cpu::slo, Cpu::top, Cpu::ora, Cpu::asl, Cpu::slo, // 01
    Cpu::jsr, Cpu::and, Cpu::stp, Cpu::rla, Cpu::bit, Cpu::and, Cpu::rol, Cpu::rla, Cpu::plp, Cpu::and, Cpu::rol, Cpu::stp, Cpu::bit, Cpu::and, Cpu::rol, Cpu::rla, // 02
    Cpu::bmi, Cpu::and, Cpu::stp, Cpu::rla, Cpu::nop, Cpu::and, Cpu::rol, Cpu::rla, Cpu::sec, Cpu::and, Cpu::nop, Cpu::rla, Cpu::top, Cpu::and, Cpu::rol, Cpu::rla, // 03
    Cpu::rti, Cpu::eor, Cpu::stp, Cpu::sre, Cpu::nop, Cpu::eor, Cpu::lsr, Cpu::sre, Cpu::pha, Cpu::eor, Cpu::lsr, Cpu::stp, Cpu::jmp, Cpu::eor, Cpu::lsr, Cpu::sre, // 04
    Cpu::bvc, Cpu::eor, Cpu::stp, Cpu::sre, Cpu::nop, Cpu::eor, Cpu::lsr, Cpu::sre, Cpu::cli, Cpu::eor, Cpu::nop, Cpu::sre, Cpu::top, Cpu::eor, Cpu::lsr, Cpu::sre, // 05
    Cpu::rts, Cpu::adc, Cpu::stp, Cpu::rra, Cpu::nop, Cpu::adc, Cpu::ror, Cpu::rra, Cpu::pla, Cpu::adc, Cpu::ror, Cpu::stp, Cpu::jmp, Cpu::adc, Cpu::ror, Cpu::rra, // 06
    Cpu::bvs, Cpu::adc, Cpu::stp, Cpu::rra, Cpu::nop, Cpu::adc, Cpu::ror, Cpu::rra, Cpu::sei, Cpu::adc, Cpu::nop, Cpu::rra, Cpu::top, Cpu::adc, Cpu::ror, Cpu::rra, // 07
    Cpu::nop, Cpu::sta, Cpu::nop, Cpu::sax, Cpu::sty, Cpu::sta, Cpu::stx, Cpu::sax, Cpu::dey, Cpu::nop, Cpu::txa, Cpu::stp, Cpu::sty, Cpu::sta, Cpu::stx, Cpu::sax, // 08
    Cpu::bcc, Cpu::sta, Cpu::stp, Cpu::stp, Cpu::sty, Cpu::sta, Cpu::stx, Cpu::sax, Cpu::tya, Cpu::sta, Cpu::txs, Cpu::stp, Cpu::stp, Cpu::sta, Cpu::stp, Cpu::stp, // 09
    Cpu::ldy, Cpu::lda, Cpu::ldx, Cpu::lax, Cpu::ldy, Cpu::lda, Cpu::ldx, Cpu::lax, Cpu::tay, Cpu::lda, Cpu::tax, Cpu::stp, Cpu::ldy, Cpu::lda, Cpu::ldx, Cpu::lax, // 0a
    Cpu::bcs, Cpu::lda, Cpu::stp, Cpu::lax, Cpu::ldy, Cpu::lda, Cpu::ldx, Cpu::lax, Cpu::clv, Cpu::lda, Cpu::tsx, Cpu::stp, Cpu::ldy, Cpu::lda, Cpu::ldx, Cpu::lax, // 0b
    Cpu::cpy, Cpu::cmp, Cpu::nop, Cpu::dcp, Cpu::cpy, Cpu::cmp, Cpu::dec, Cpu::dcp, Cpu::iny, Cpu::cmp, Cpu::dex, Cpu::stp, Cpu::cpy, Cpu::cmp, Cpu::dec, Cpu::dcp, // 0c
    Cpu::bne, Cpu::cmp, Cpu::stp, Cpu::dcp, Cpu::nop, Cpu::cmp, Cpu::dec, Cpu::dcp, Cpu::cld, Cpu::cmp, Cpu::nop, Cpu::dcp, Cpu::top, Cpu::cmp, Cpu::dec, Cpu::dcp, // 0d
    Cpu::cpx, Cpu::sbc, Cpu::nop, Cpu::isb, Cpu::cpx, Cpu::sbc, Cpu::inc, Cpu::isb, Cpu::inx, Cpu::sbc, Cpu::nop, Cpu::sbc, Cpu::cpx, Cpu::sbc, Cpu::inc, Cpu::isb, // 0e
    Cpu::beq, Cpu::sbc, Cpu::stp, Cpu::isb, Cpu::nop, Cpu::sbc, Cpu::inc, Cpu::isb, Cpu::sed, Cpu::sbc, Cpu::nop, Cpu::isb, Cpu::top, Cpu::sbc, Cpu::inc, Cpu::isb, // 0f
];

#[rustfmt::skip]
pub const OP_NAMES: [&'static str; 256] = [
//   00     01     02     03     04     05     06     07     08     09     0a     0b     0c     0d     0e     0f
    "BRK", "ORA", "STP", "SLO", "NOP", "ORA", "ASL", "SLO", "PHP", "ORA", "ASL", "STP", "NOP", "ORA", "ASL", "SLO", // 00
    "BPL", "ORA", "STP", "SLO", "NOP", "ORA", "ASL", "SLO", "CLC", "ORA", "NOP", "SLO", "NOP", "ORA", "ASL", "SLO", // 01
    "JSR", "AND", "STP", "RLA", "BIT", "AND", "ROL", "RLA", "PLP", "AND", "ROL", "STP", "BIT", "AND", "ROL", "RLA", // 02
    "BMI", "AND", "STP", "RLA", "NOP", "AND", "ROL", "RLA", "SEC", "AND", "NOP", "RLA", "NOP", "AND", "ROL", "RLA", // 03
    "RTI", "EOR", "STP", "SRE", "NOP", "EOR", "LSR", "SRE", "PHA", "EOR", "LSR", "STP", "JMP", "EOR", "LSR", "SRE", // 04
    "BVC", "EOR", "STP", "SRE", "NOP", "EOR", "LSR", "SRE", "CLI", "EOR", "NOP", "SRE", "NOP", "EOR", "LSR", "SRE", // 05
    "RTS", "ADC", "STP", "RRA", "NOP", "ADC", "ROR", "RRA", "PLA", "ADC", "ROR", "STP", "JMP", "ADC", "ROR", "RRA", // 06
    "BVS", "ADC", "STP", "RRA", "NOP", "ADC", "ROR", "RRA", "SEI", "ADC", "NOP", "RRA", "NOP", "ADC", "ROR", "RRA", // 07
    "NOP", "STA", "NOP", "SAX", "STY", "STA", "STX", "SAX", "DEY", "NOP", "TXA", "STP", "STY", "STA", "STX", "SAX", // 08
    "BCC", "STA", "STP", "STP", "STY", "STA", "STX", "SAX", "TYA", "STA", "TXS", "STP", "STP", "STA", "STP", "STP", // 09
    "LDY", "LDA", "LDX", "LAX", "LDY", "LDA", "LDX", "LAX", "TAY", "LDA", "TAX", "STP", "LDY", "LDA", "LDX", "LAX", // 0a
    "BCS", "LDA", "STP", "LAX", "LDY", "LDA", "LDX", "LAX", "CLV", "LDA", "TSX", "STP", "LDY", "LDA", "LDX", "LAX", // 0b
    "CPY", "CMP", "NOP", "DCP", "CPY", "CMP", "DEC", "DCP", "INY", "CMP", "DEX", "STP", "CPY", "CMP", "DEC", "DCP", // 0c
    "BNE", "CMP", "STP", "DCP", "NOP", "CMP", "DEC", "DCP", "CLD", "CMP", "NOP", "DCP", "NOP", "CMP", "DEC", "DCP", // 0d
    "CPX", "SBC", "NOP", "ISB", "CPX", "SBC", "INC", "ISB", "INX", "SBC", "NOP", "SBC", "CPX", "SBC", "INC", "ISB", // 0e
    "BEQ", "SBC", "STP", "ISB", "NOP", "SBC", "INC", "ISB", "SED", "SBC", "NOP", "ISB", "NOP", "SBC", "INC", "ISB", // 0f
];

#[rustfmt::skip]
pub const OP_EXTRA_CYCLES: [u8; 256] = [
//  0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f
    0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0, // 00
    0, 0, 0, 0, 1, 1, 2, 1, 1, 0, 0, 0, 0, 0, 2, 0, // 01
    1, 1, 0, 1, 0, 0, 1, 0, 2, 0, 1, 0, 0, 0, 1, 0, // 02
    0, 0, 0, 0, 1, 1, 2, 1, 1, 0, 0, 0, 0, 0, 2, 0, // 03
    2, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0, // 04
    0, 0, 0, 0, 1, 1, 2, 1, 1, 0, 0, 0, 0, 0, 2, 0, // 05
    3, 1, 0, 1, 0, 0, 1, 0, 2, 0, 1, 0, 0, 0, 1, 0, // 06
    0, 0, 0, 0, 1, 1, 2, 1, 1, 0, 0, 0, 0, 0, 2, 0, // 07
    0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, // 08
    0, 1, 0, 0, 1, 1, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, // 09
    0, 1, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, // 0a
    0, 0, 0, 0, 1, 1, 1, 1, 1, 0, 1, 0, 0, 0, 0, 0, // 0b
    0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1, 0, // 0c
    0, 0, 0, 0, 1, 1, 2, 1, 1, 0, 0, 0, 0, 0, 2, 0, // 0d
    0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 0, 0, 0, 1, 0, // 0e
    0, 0, 0, 0, 1, 1, 2, 1, 1, 0, 0, 0, 0, 0, 2, 0, // 0f
];

impl Cpu {
    fn get_operand(&self, bus: &mut Bus) -> u8 {
        match self.op_mode {
            AddrMode::IMP => unreachable!(),
            AddrMode::ACC => self.a,
            _ => bus.read(self.op_address),
        }
    }

    fn tick_cross_page(&self, bus: &mut Bus) {
        if self.cross_page {
            bus.tick();
        }
    }

    fn stp(&mut self, _: &mut Bus) {
        panic!("invalid op: {}", self.op);
    }

    fn nop(&mut self, bus: &mut Bus) {
        bus.tick();
    }
}

/// arith
impl Cpu {
    fn adc(&mut self, bus: &mut Bus) {
        self._adc(self.get_operand(bus));

        self.tick_cross_page(bus);
    }

    fn sbc(&mut self, bus: &mut Bus) {
        self._adc(!self.get_operand(bus));

        self.tick_cross_page(bus);
    }

    fn and(&mut self, bus: &mut Bus) {
        self.a &= self.get_operand(bus);
        self.p.set_zn(self.a);

        self.tick_cross_page(bus);
    }

    fn ora(&mut self, bus: &mut Bus) {
        self.a |= self.get_operand(bus);
        self.p.set_zn(self.a);

        self.tick_cross_page(bus);
    }

    fn eor(&mut self, bus: &mut Bus) {
        let op = self.get_operand(bus);
        self.a ^= op;
        self.p.set_zn(self.a);

        self.tick_cross_page(bus);
    }

    fn inc(&mut self, bus: &mut Bus) {
        let op = self.get_operand(bus).wrapping_add(1);
        bus.write(self.op_address, op);
        self.p.set_zn(op);
    }

    fn inx(&mut self, _: &mut Bus) {
        self.x = self.x.wrapping_add(1);
        self.p.set_zn(self.x);
    }

    fn iny(&mut self, _: &mut Bus) {
        self.y = self.y.wrapping_add(1);
        self.p.set_zn(self.y);
    }

    fn dec(&mut self, bus: &mut Bus) {
        let op = self.get_operand(bus).wrapping_sub(1);
        bus.write(self.op_address, op);
        self.p.set_zn(op);
    }

    fn dex(&mut self, _: &mut Bus) {
        self.x = self.x.wrapping_sub(1);
        self.p.set_zn(self.x);
    }

    fn dey(&mut self, _: &mut Bus) {
        self.y = self.y.wrapping_sub(1);
        self.p.set_zn(self.y);
    }

    fn rol(&mut self, bus: &mut Bus) {
        let c = self.p.c as u8;
        let op = self.get_operand(bus);
        self.p.c = (op & 0x80) != 0;

        let op = (op << 1) | c;
        self.p.set_zn(op);

        if self.op_mode == AddrMode::ACC {
            self.a = op;
        } else {
            bus.write(self.op_address, op);
        }
    }

    fn ror(&mut self, bus: &mut Bus) {
        let c = self.p.c as u8;
        let op = self.get_operand(bus);
        self.p.c = (op & 0x01) != 0;

        let op = (op >> 1) | (c << 7);
        self.p.set_zn(op);

        if self.op_mode == AddrMode::ACC {
            self.a = op;
        } else {
            bus.write(self.op_address, op);
        }
    }

    fn asl(&mut self, bus: &mut Bus) {
        let op = self.get_operand(bus);
        self.p.c = (op & 0x80) != 0;

        let op = op << 1;
        self.p.set_zn(op);

        if self.op_mode == AddrMode::ACC {
            self.a = op;
        } else {
            bus.write(self.op_address, op);
        }
    }

    fn lsr(&mut self, bus: &mut Bus) {
        let op = self.get_operand(bus);
        self.p.c = (op & 0x01) != 0;

        let op = op >> 1;
        self.p.set_zn(op);

        if self.op_mode == AddrMode::ACC {
            self.a = op;
        } else {
            bus.write(self.op_address, op);
        }
    }
}

/// branch and jump
impl Cpu {
    fn brk(&mut self, bus: &mut Bus) {
        self.p.b = true;

        self.push_word(self.pc, bus);
        self.push_byte(self.p.to_u8(), bus);
        self.pc = self.read_word(0xfffe, bus);
    }

    fn bcc(&mut self, bus: &mut Bus) {
        self._branch(!self.p.c, bus);
    }

    fn bcs(&mut self, bus: &mut Bus) {
        self._branch(self.p.c, bus);
    }

    fn beq(&mut self, bus: &mut Bus) {
        self._branch(self.p.z, bus);
    }

    fn bmi(&mut self, bus: &mut Bus) {
        self._branch(self.p.n, bus);
    }

    fn bne(&mut self, bus: &mut Bus) {
        self._branch(!self.p.z, bus);
    }

    fn bpl(&mut self, bus: &mut Bus) {
        self._branch(!self.p.n, bus);
    }

    fn bvc(&mut self, bus: &mut Bus) {
        self._branch(!self.p.v, bus);
    }

    fn bvs(&mut self, bus: &mut Bus) {
        self._branch(self.p.v, bus);
    }

    fn jmp(&mut self, _: &mut Bus) {
        self.pc = self.op_address;
    }

    fn jsr(&mut self, bus: &mut Bus) {
        self.push_word(self.pc.wrapping_sub(1), bus);
        self.pc = self.op_address;
    }

    fn rti(&mut self, bus: &mut Bus) {
        self.p = self.pop_byte(bus).into();
        self.pc = self.pop_word(bus);
    }

    fn rts(&mut self, bus: &mut Bus) {
        let addr = self.pop_word(bus);
        let addr = addr.wrapping_add(1);
        self.pc = addr;
    }
}

/// move
impl Cpu {
    fn lda(&mut self, bus: &mut Bus) {
        self.a = self.get_operand(bus);
        self.p.set_zn(self.a);

        self.tick_cross_page(bus);
    }

    fn ldx(&mut self, bus: &mut Bus) {
        self.x = self.get_operand(bus);
        self.p.set_zn(self.x);

        self.tick_cross_page(bus);
    }

    fn ldy(&mut self, bus: &mut Bus) {
        self.y = self.get_operand(bus);
        self.p.set_zn(self.y);

        self.tick_cross_page(bus);
    }

    fn pha(&mut self, bus: &mut Bus) {
        self.push_byte(self.a, bus);
    }

    fn php(&mut self, bus: &mut Bus) {
        self.push_byte(self.p.to_u8() | 0b0001_0000u8, bus);
    }

    fn pla(&mut self, bus: &mut Bus) {
        let b = self.pop_byte(bus);
        self.a = b;
        self.p.set_zn(self.a);
    }

    fn plp(&mut self, bus: &mut Bus) {
        let b = self.pop_byte(bus);
        self.p = b.into();
    }

    fn sta(&mut self, bus: &mut Bus) {
        bus.write(self.op_address, self.a);
    }

    fn stx(&mut self, bus: &mut Bus) {
        bus.write(self.op_address, self.x);
    }

    fn sty(&mut self, bus: &mut Bus) {
        bus.write(self.op_address, self.y);
    }

    fn tax(&mut self, _: &mut Bus) {
        self.x = self.a;
        self.p.set_zn(self.x);
    }

    fn tay(&mut self, _: &mut Bus) {
        self.y = self.a;
        self.p.set_zn(self.y);
    }

    fn tsx(&mut self, _: &mut Bus) {
        self.x = self.sp;
        self.p.set_zn(self.x);
    }

    fn txs(&mut self, _: &mut Bus) {
        self.sp = self.x;
    }

    fn txa(&mut self, _: &mut Bus) {
        self.a = self.x;
        self.p.set_zn(self.a);
    }

    fn tya(&mut self, _: &mut Bus) {
        self.a = self.y;
        self.p.set_zn(self.a);
    }
}

/// flags
impl Cpu {
    fn bit(&mut self, bus: &mut Bus) {
        let op = self.get_operand(bus);
        self.p.z = (self.a & op) == 0;
        self.p.n = (op & 0x80) != 0;
        self.p.v = (op & 0x40) != 0;
    }

    fn cmp(&mut self, bus: &mut Bus) {
        self._cmp(self.a, self.get_operand(bus));

        self.tick_cross_page(bus);
    }

    fn cpx(&mut self, bus: &mut Bus) {
        self._cmp(self.x, self.get_operand(bus));
    }

    fn cpy(&mut self, bus: &mut Bus) {
        self._cmp(self.y, self.get_operand(bus));
    }

    fn clc(&mut self, _: &mut Bus) {
        self.p.c = false;
    }

    fn cld(&mut self, _: &mut Bus) {
        self.p.d = false;
    }

    fn cli(&mut self, _: &mut Bus) {
        self.p.i = false;
    }

    fn clv(&mut self, _: &mut Bus) {
        self.p.v = false;
    }

    fn sec(&mut self, _: &mut Bus) {
        self.p.c = true;
    }

    fn sed(&mut self, _: &mut Bus) {
        self.p.d = true;
    }

    fn sei(&mut self, _: &mut Bus) {
        self.p.i = true;
    }
}

/// unofficial
impl Cpu {
    fn top(&mut self, bus: &mut Bus) {
        bus.tick();
        self.tick_cross_page(bus);
    }

    fn lax(&mut self, bus: &mut Bus) {
        self.lda(bus);
        self.x = self.a;
    }

    fn sax(&mut self, bus: &mut Bus) {
        bus.write(self.op_address, self.a & self.x);
    }

    fn dcp(&mut self, bus: &mut Bus) {
        self.dec(bus);
        self.cmp(bus);
    }

    fn isb(&mut self, bus: &mut Bus) {
        self.inc(bus);
        self.sbc(bus);
    }

    fn slo(&mut self, bus: &mut Bus) {
        self.asl(bus);
        self.ora(bus);
    }

    fn rla(&mut self, bus: &mut Bus) {
        self.rol(bus);
        self.and(bus);
    }

    fn sre(&mut self, bus: &mut Bus) {
        self.lsr(bus);
        self.eor(bus);
    }

    fn rra(&mut self, bus: &mut Bus) {
        self.ror(bus);
        self.adc(bus);
    }
}

impl Cpu {
    fn _adc(&mut self, op: u8) {
        let sum = self.a as u16 + op as u16 + self.p.c as u16;

        self.p.c = sum > 0xff;
        self.p.v = (!(self.a ^ op) & (self.a ^ sum as u8) & 0x80) != 0;
        self.a = sum as u8;
        self.p.set_zn(self.a);
    }

    fn _branch(&mut self, b: bool, bus: &mut Bus) {
        if b {
            bus.tick();
            self.pc = self.op_address;
            self.tick_cross_page(bus);
        }
    }

    fn _cmp(&mut self, a: u8, b: u8) {
        self.p.c = a >= b;
        self.p.z = a == b;
        self.p.n = (a.wrapping_sub(b) & 0x80) != 0;
    }
}

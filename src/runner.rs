//! Browser p-code VM — ported from pv24t (sw-cor24-pcode/tracer/src/main.rs).
//! Runs .p24 binaries directly in WASM without the COR24 emulator layer.

const MASK24: i32 = 0x00FF_FFFF;

fn sign_extend_24(v: i32) -> i32 {
    let v = v & MASK24;
    if v & 0x0080_0000 != 0 { v | !MASK24 } else { v }
}

fn wrap24(v: i32) -> i32 {
    sign_extend_24(v)
}

const WORD: usize = 3;

pub struct Session {
    mem: Vec<u8>,
    pc: usize,
    esp: usize,
    csp: usize,
    fp_vm: usize,
    gp: usize,
    hp: usize,
    code_size: usize,
    eval_stack_base: usize,
    heap_base: usize,
    status: u8,
    trap_code: u8,
    instruction_count: u64,
    stdin_buf: Vec<u8>,
    stdin_pos: usize,
    stdout_buf: String,
    interactive: bool,
    awaiting_input: bool,
}

const BATCH_SIZE: u64 = 200_000;

impl Session {
    fn dummy() -> Self {
        Session {
            mem: vec![0u8; 1],
            pc: 0,
            esp: 0,
            csp: 0,
            fp_vm: 0,
            gp: 0,
            hp: 0,
            code_size: 0,
            eval_stack_base: 0,
            heap_base: 0,
            status: 1,
            trap_code: 0,
            instruction_count: 0,
            stdin_buf: Vec::new(),
            stdin_pos: 0,
            stdout_buf: String::new(),
            interactive: false,
            awaiting_input: false,
        }
    }

    pub fn new(basic_source: &str) -> Self {
        Self::new_with_mode(basic_source, false)
    }

    pub fn new_interactive(basic_source: &str) -> Self {
        Self::new_with_mode(basic_source, true)
    }

    fn new_with_mode(basic_source: &str, interactive: bool) -> Self {
        let p24_bytes = crate::config::BASIC_P24;
        let image = match pa24r::load_p24(p24_bytes) {
            Ok(img) => img,
            Err(e) => {
                web_sys::console::error_1(&format!("p24 load error: {:?}", e).into());
                return Self::dummy();
            }
        };

        let code_size = image.code.len();
        let data_size = image.data.len();
        let global_count = image.global_count as usize;
        let globals_base = code_size + data_size;
        let globals_size = global_count * WORD;
        let call_stack_base = globals_base + globals_size;
        let call_stack_size = 256 * WORD;
        let eval_stack_base = call_stack_base + call_stack_size;
        let eval_stack_size = 256 * WORD;
        let heap_base = eval_stack_base + eval_stack_size;
        let heap_size = 4096 * WORD;
        let total = heap_base + heap_size;

        let mut mem = vec![0u8; total];
        mem[..code_size].copy_from_slice(&image.code);
        mem[code_size..code_size + data_size].copy_from_slice(&image.data);

        let has_line_numbers = basic_source.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.chars().next().is_some_and(|c| c.is_ascii_digit())
        });

        let mut stdin_buf = Vec::new();
        for b in basic_source.bytes() {
            stdin_buf.push(b);
        }
        if interactive {
            // Interactive mode: load source and start RUN, but leave the stream
            // open so the user can supply INPUT bytes (and eventually type BYE).
            if has_line_numbers {
                stdin_buf.extend_from_slice(b"RUN\n");
            } else {
                stdin_buf.push(b'\n');
            }
        } else if has_line_numbers {
            stdin_buf.extend_from_slice(b"RUN\nBYE\n");
            stdin_buf.push(0x04);
        } else {
            stdin_buf.push(b'\n');
            stdin_buf.push(0x04);
        }

        Session {
            mem,
            pc: image.entry_point as usize,
            esp: eval_stack_base,
            csp: call_stack_base,
            fp_vm: call_stack_base,
            gp: globals_base,
            hp: heap_base,
            code_size,
            eval_stack_base,
            heap_base,
            status: 0,
            trap_code: 0,
            instruction_count: 0,
            stdin_buf,
            stdin_pos: 0,
            stdout_buf: String::new(),
            interactive,
            awaiting_input: false,
        }
    }

    pub fn is_awaiting_input(&self) -> bool {
        self.awaiting_input
    }

    /// Append a line of user input (a newline is appended automatically) and
    /// resume execution. No-op outside interactive mode.
    pub fn feed_input(&mut self, line: &str) {
        if !self.interactive {
            return;
        }
        for b in line.bytes() {
            self.stdin_buf.push(b);
        }
        if !line.ends_with('\n') {
            self.stdin_buf.push(b'\n');
        }
        self.awaiting_input = false;
    }

    pub fn tick(&mut self) -> TickResult {
        if self.status != 0 {
            return TickResult { done: true };
        }
        if self.awaiting_input {
            return TickResult { done: false };
        }

        let budget = BATCH_SIZE;

        let mut ran = 0u64;
        while self.status == 0 && !self.awaiting_input && ran < budget {
            let op_byte = self.fetch_u8();
            self.execute(op_byte);
            if self.awaiting_input {
                // GETC pulled an empty interactive stream and rewound PC.
                // Don't count this as an executed instruction.
                break;
            }
            self.instruction_count += 1;
            ran += 1;
        }

        TickResult {
            done: self.status != 0,
        }
    }

    pub fn is_done(&self) -> bool {
        self.status != 0
    }

    pub fn is_halted(&self) -> bool {
        self.status == 1
    }

    pub fn stop_reason(&self) -> String {
        if self.status == 1 {
            "halted".into()
        } else {
            format!("trap {}", self.trap_code)
        }
    }

    pub fn instructions(&self) -> u64 {
        self.instruction_count
    }

    pub fn output(&self) -> String {
        self.stdout_buf.chars().filter(|&c| c != '>').collect()
    }

    // --- memory access ---

    fn read_word(&self, addr: usize) -> i32 {
        if addr + 2 >= self.mem.len() {
            return 0;
        }
        let lo = self.mem[addr] as i32;
        let mid = self.mem[addr + 1] as i32;
        let hi = self.mem[addr + 2] as i32;
        sign_extend_24(lo | (mid << 8) | (hi << 16))
    }

    fn write_word(&mut self, addr: usize, val: i32) {
        let v = val & MASK24;
        if addr + 2 >= self.mem.len() {
            self.trap(2);
            return;
        }
        self.mem[addr] = v as u8;
        self.mem[addr + 1] = (v >> 8) as u8;
        self.mem[addr + 2] = (v >> 16) as u8;
    }

    fn read_byte(&self, addr: usize) -> i32 {
        if addr >= self.mem.len() {
            0
        } else {
            self.mem[addr] as i32
        }
    }

    fn write_byte(&mut self, addr: usize, val: i32) {
        if addr >= self.mem.len() {
            self.trap(2);
            return;
        }
        self.mem[addr] = val as u8;
    }

    // --- fetch ---

    fn fetch_u8(&mut self) -> u8 {
        let v = if self.pc < self.code_size {
            self.mem[self.pc]
        } else {
            0
        };
        self.pc += 1;
        v
    }

    fn fetch_i8(&mut self) -> i32 {
        self.fetch_u8() as i8 as i32
    }

    fn fetch_u24(&mut self) -> u32 {
        let lo = self.fetch_u8() as u32;
        let mid = self.fetch_u8() as u32;
        let hi = self.fetch_u8() as u32;
        lo | (mid << 8) | (hi << 16)
    }

    // --- eval stack ---

    fn push_eval(&mut self, val: i32) {
        if self.esp >= self.heap_base {
            self.trap(2);
            return;
        }
        self.write_word(self.esp, val);
        self.esp += WORD;
    }

    fn pop_eval(&mut self) -> i32 {
        if self.esp <= self.eval_stack_base {
            self.trap(3);
            return 0;
        }
        self.esp -= WORD;
        self.read_word(self.esp)
    }

    fn peek_eval(&self) -> i32 {
        if self.esp <= self.eval_stack_base {
            0
        } else {
            self.read_word(self.esp - WORD)
        }
    }

    fn trap(&mut self, code: u8) {
        self.status = 2;
        self.trap_code = code;
    }

    fn follow_static_links(&self, depth: usize) -> usize {
        let mut frame = self.fp_vm;
        for _ in 0..depth {
            frame = self.read_word(frame + 2 * WORD) as usize;
        }
        frame
    }

    // --- execute ---

    fn execute(&mut self, op: u8) {
        match op {
            0x00 => {
                self.status = 1;
            }
            0x01 => {
                let v = self.fetch_u24() as i32;
                self.push_eval(sign_extend_24(v));
            }
            0x02 => {
                let v = self.fetch_i8();
                self.push_eval(wrap24(v));
            }
            0x03 => {
                let v = self.peek_eval();
                self.push_eval(v);
            }
            0x04 => {
                self.pop_eval();
            }
            0x05 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(b);
                self.push_eval(a);
            }
            0x06 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(a);
                self.push_eval(b);
                self.push_eval(a);
            }
            0x10 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a.wrapping_add(b)));
            }
            0x11 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a.wrapping_sub(b)));
            }
            0x12 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a.wrapping_mul(b)));
            }
            0x13 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                if b == 0 {
                    self.trap(1);
                    return;
                }
                self.push_eval(wrap24(a / b));
            }
            0x14 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                if b == 0 {
                    self.trap(1);
                    return;
                }
                self.push_eval(wrap24(a % b));
            }
            0x15 => {
                let a = self.pop_eval();
                self.push_eval(wrap24(-a));
            }
            0x16 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a & b));
            }
            0x17 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a | b));
            }
            0x18 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a ^ b));
            }
            0x19 => {
                let a = self.pop_eval();
                self.push_eval(wrap24(!a));
            }
            0x1A => {
                let n = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a << (n & 0x1F)));
            }
            0x1B => {
                let n = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(wrap24(a >> (n & 0x1F)));
            }
            0x20 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a == b { 1 } else { 0 });
            }
            0x21 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a != b { 1 } else { 0 });
            }
            0x22 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a < b { 1 } else { 0 });
            }
            0x23 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a <= b { 1 } else { 0 });
            }
            0x24 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a > b { 1 } else { 0 });
            }
            0x25 => {
                let b = self.pop_eval();
                let a = self.pop_eval();
                self.push_eval(if a >= b { 1 } else { 0 });
            }
            0x30 => {
                let addr = self.fetch_u24() as usize;
                self.pc = addr;
            }
            0x31 => {
                let addr = self.fetch_u24() as usize;
                let flag = self.pop_eval();
                if flag == 0 {
                    self.pc = addr;
                }
            }
            0x32 => {
                let addr = self.fetch_u24() as usize;
                let flag = self.pop_eval();
                if flag != 0 {
                    self.pc = addr;
                }
            }
            0x33 => {
                let addr = self.fetch_u24() as usize;
                self.write_word(self.csp, self.pc as i32);
                self.write_word(self.csp + WORD, self.fp_vm as i32);
                self.write_word(self.csp + 2 * WORD, self.fp_vm as i32);
                self.write_word(self.csp + 3 * WORD, self.esp as i32);
                self.csp += 4 * WORD;
                self.pc = addr;
            }
            0x34 => {
                let nargs = self.fetch_u8() as usize;
                let return_pc = self.read_word(self.fp_vm) as usize;
                let dynamic_link = self.read_word(self.fp_vm + WORD) as usize;
                let saved_esp = self.read_word(self.fp_vm + 3 * WORD) as usize;
                let has_return = self.esp > saved_esp;
                let return_val = if has_return {
                    Some(self.pop_eval())
                } else {
                    None
                };
                self.csp = self.fp_vm;
                self.fp_vm = dynamic_link;
                self.esp = saved_esp - nargs * WORD;
                if let Some(rv) = return_val {
                    self.push_eval(rv);
                }
                self.pc = return_pc;
            }
            0x35 => {
                let depth = self.fetch_u8();
                let addr = self.fetch_u24() as usize;
                let mut sl = self.fp_vm;
                for _ in 0..depth {
                    sl = self.read_word(sl + 2 * WORD) as usize;
                }
                self.write_word(self.csp, self.pc as i32);
                self.write_word(self.csp + WORD, self.fp_vm as i32);
                self.write_word(self.csp + 2 * WORD, sl as i32);
                self.write_word(self.csp + 3 * WORD, self.esp as i32);
                self.csp += 4 * WORD;
                self.pc = addr;
            }
            0x36 => {
                let code = self.fetch_u8();
                self.trap(code);
            }
            0x40 => {
                let nlocals = self.fetch_u8() as usize;
                self.fp_vm = self.csp - 4 * WORD;
                for _ in 0..nlocals {
                    self.write_word(self.csp, 0);
                    self.csp += WORD;
                }
            }
            0x41 => {
                self.csp = self.fp_vm + 4 * WORD;
            }
            0x42 => {
                let off = self.fetch_u8() as usize;
                let addr = self.fp_vm + 4 * WORD + off * WORD;
                self.push_eval(self.read_word(addr));
            }
            0x43 => {
                let off = self.fetch_u8() as usize;
                let val = self.pop_eval();
                let addr = self.fp_vm + 4 * WORD + off * WORD;
                self.write_word(addr, val);
            }
            0x44 => {
                let off = self.fetch_u24() as usize;
                let addr = self.gp + off * WORD;
                self.push_eval(self.read_word(addr));
            }
            0x45 => {
                let off = self.fetch_u24() as usize;
                let val = self.pop_eval();
                let addr = self.gp + off * WORD;
                self.write_word(addr, val);
            }
            0x46 => {
                let off = self.fetch_u8() as usize;
                let addr = self.fp_vm + 4 * WORD + off * WORD;
                self.push_eval(addr as i32);
            }
            0x47 => {
                let off = self.fetch_u24() as usize;
                let addr = self.gp + off * WORD;
                self.push_eval(addr as i32);
            }
            0x48 => {
                let idx = self.fetch_u8() as usize;
                let saved_esp = self.read_word(self.fp_vm + 3 * WORD) as usize;
                let addr = saved_esp - (idx + 1) * WORD;
                self.push_eval(self.read_word(addr));
            }
            0x49 => {
                let idx = self.fetch_u8() as usize;
                let val = self.pop_eval();
                let saved_esp = self.read_word(self.fp_vm + 3 * WORD) as usize;
                let addr = saved_esp - (idx + 1) * WORD;
                self.write_word(addr, val);
            }
            0x4A => {
                let depth = self.fetch_u8() as usize;
                let off = self.fetch_u8() as usize;
                let frame = self.follow_static_links(depth);
                let addr = frame + 4 * WORD + off * WORD;
                self.push_eval(self.read_word(addr));
            }
            0x4B => {
                let depth = self.fetch_u8() as usize;
                let off = self.fetch_u8() as usize;
                let val = self.pop_eval();
                let frame = self.follow_static_links(depth);
                let addr = frame + 4 * WORD + off * WORD;
                self.write_word(addr, val);
            }
            0x50 => {
                let addr = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                }
                self.push_eval(self.read_word(addr as usize));
            }
            0x51 => {
                let addr = self.pop_eval();
                let val = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                }
                self.write_word(addr as usize, val);
            }
            0x52 => {
                let addr = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                }
                self.push_eval(self.read_byte(addr as usize));
            }
            0x53 => {
                let addr = self.pop_eval();
                let val = self.pop_eval();
                if addr == 0 {
                    self.trap(6);
                    return;
                }
                self.write_byte(addr as usize, val);
            }
            0x60 => {
                let id = self.fetch_u8();
                self.sys_call(id);
            }
            0x70 => {
                let len = self.pop_eval() as usize;
                let dst = self.pop_eval() as usize;
                let src = self.pop_eval() as usize;
                if len > 0 {
                    if src < dst {
                        for i in (0..len).rev() {
                            let b = self.read_byte(src + i);
                            self.write_byte(dst + i, b);
                        }
                    } else {
                        for i in 0..len {
                            let b = self.read_byte(src + i);
                            self.write_byte(dst + i, b);
                        }
                    }
                }
            }
            0x71 => {
                let len = self.pop_eval() as usize;
                let val = self.pop_eval();
                let dst = self.pop_eval() as usize;
                if len > 0 {
                    for i in 0..len {
                        self.write_byte(dst + i, val);
                    }
                }
            }
            0x72 => {
                let len = self.pop_eval() as usize;
                let b = self.pop_eval() as usize;
                let a = self.pop_eval() as usize;
                let mut result: i32 = 0;
                for i in 0..len {
                    let ba = self.read_byte(a + i) & 0xFF;
                    let bb = self.read_byte(b + i) & 0xFF;
                    if ba != bb {
                        result = if ba < bb { -1 } else { 1 };
                        break;
                    }
                }
                self.push_eval(result);
            }
            0x73 => {
                let addr = self.pop_eval() as usize;
                self.pc = addr;
            }
            0x74 => {
                let _ = self.fetch_u8();
                let _ = self.fetch_u8();
                self.trap(4);
            }
            0x75 | 0x76 => {
                let _ = self.fetch_u8();
                let _ = self.fetch_u8();
                self.trap(4);
            }
            _ => {
                self.trap(4);
            }
        }
    }

    fn sys_call(&mut self, id: u8) {
        match id {
            0 => {
                self.status = 1;
            }
            1 => {
                let ch = self.pop_eval() as u8;
                self.stdout_buf.push(ch as char);
            }
            2 => {
                if self.stdin_pos < self.stdin_buf.len() {
                    let c = self.stdin_buf[self.stdin_pos];
                    self.stdin_pos += 1;
                    self.push_eval(c as i32);
                } else if self.interactive {
                    // Pause execution until the host calls feed_input().
                    // Rewind PC past the SYSCALL opcode (0x60) and its 1-byte
                    // id so the syscall re-executes on resume.
                    self.pc -= 2;
                    self.awaiting_input = true;
                } else {
                    self.push_eval(-1);
                }
            }
            3 => {
                self.pop_eval();
            }
            4 => {
                let size = self.pop_eval() as usize;
                let ptr = self.hp;
                self.hp += size;
                if self.hp > self.mem.len() {
                    self.mem.resize(self.hp, 0);
                }
                self.push_eval(ptr as i32);
            }
            5 => {
                self.pop_eval();
            }
            6 => {
                self.push_eval(0);
            }
            7 => {
                self.pop_eval();
            }
            8 => {}
            _ => {
                self.trap(4);
            }
        }
    }
}

pub struct TickResult {
    pub done: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run_to_done(src: &str) -> Session {
        let mut s = Session::new(src);
        for _ in 0..1000 {
            let r = s.tick();
            if r.done {
                break;
            }
        }
        s
    }

    #[test]
    fn calc_runs() {
        let src = include_str!("../examples/calc.bas");
        let mut s = Session::new(src);
        for _ in 0..100000 {
            let r = s.tick();
            if r.done {
                break;
            }
        }
        eprintln!(
            "status={} trap={} instrs={}",
            s.status, s.trap_code, s.instruction_count
        );
        eprintln!("output={:?}", s.stdout_buf);
        assert!(
            s.is_done() && s.is_halted(),
            "calc failed: {}",
            s.stop_reason()
        );
    }

    #[test]
    fn interactive_pauses_for_input() {
        // Tiny interactive program: read a number with INPUT, print 2*it.
        let src = "10 INPUT \"N\";N\n20 PRINT N*2\n30 END\n";
        let mut s = Session::new_interactive(src);
        // Spin until either done or awaiting_input.
        for _ in 0..2000 {
            let r = s.tick();
            if r.done || s.is_awaiting_input() {
                break;
            }
        }
        assert!(
            s.is_awaiting_input(),
            "expected pause at INPUT, status={} out={:?}",
            s.status,
            s.stdout_buf
        );
        s.feed_input("7");
        assert!(!s.is_awaiting_input());
        // Now resume — program should print 14 then return to REPL prompt and
        // block waiting for the next interactive input. Send BYE to halt.
        for _ in 0..200_000 {
            let r = s.tick();
            if r.done || s.is_awaiting_input() {
                break;
            }
        }
        assert!(
            s.is_awaiting_input() || s.is_done(),
            "expected another input pause or done"
        );
        if s.is_awaiting_input() {
            s.feed_input("BYE");
            for _ in 0..200_000 {
                let r = s.tick();
                if r.done {
                    break;
                }
            }
        }
        assert!(s.is_halted(), "expected halt: {}", s.stop_reason());
        assert!(
            s.stdout_buf.contains("14"),
            "missing computed output: {:?}",
            s.stdout_buf
        );
    }

    #[test]
    fn hello_runs() {
        let src = include_str!("../examples/hello.bas");
        let s = run_to_done(src);
        eprintln!(
            "status={} trap={} instrs={}",
            s.status, s.trap_code, s.instruction_count
        );
        eprintln!("output={:?}", s.stdout_buf);
        assert!(s.is_done(), "did not finish");
        assert!(s.is_halted(), "trapped: {}", s.stop_reason());
        assert!(
            s.stdout_buf.contains("HELLO WORLD"),
            "no hello: {:?}",
            s.stdout_buf
        );
    }
}

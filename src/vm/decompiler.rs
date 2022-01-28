use gc::Gc;

use super::{chunk_parser::{LuaChunk, FunctionBlock, LuaConstant}, instruction::{VMInst, VMOpcode, MASK_CBIT}, LuaFunction, LuaValue};

pub struct InstStream {
    pub idx: usize,
    pub func: FunctionBlock,
}
impl InstStream {
    pub fn new(func: FunctionBlock) -> Self {
        Self { idx: 0, func }
    }
    pub fn next(&mut self) -> Option<VMInst> {
        let x = self.func.list_instructions.get(self.idx).cloned()?;    
        self.idx += 1;
        Some(x)
    }
    pub fn peek_ahead(&self, amount: usize) -> Option<VMInst> {
        self.func.list_instructions.get((self.idx - 1) + amount).cloned()
    }
    pub fn next_inst_is(&self, opcode: VMOpcode) -> bool {
        let v = self.peek_ahead(1);
        if let Some(v) = v {
            if v.opcode == opcode {
                return true;
            }
        }
        false
    }
    pub fn get_const(&self, idx: u32) -> Gc<LuaConstant> {
        self.func.list_const.get(idx as usize).unwrap().clone()
    }
}
pub struct LuaDecompiler {
    chunk: LuaChunk,
    iter: InstStream,
    lv_idx: usize
}
impl LuaDecompiler {
    pub fn new(chunk: LuaChunk) -> Self {
        let chunk_func = chunk.func.clone();
        Self { chunk, lv_idx: 0, iter: InstStream::new(chunk_func) }
    }
    fn get_lv_name(&mut self) -> String {
        let s = format!("lv_{}", self.lv_idx);
        self.lv_idx += 1;
        s
    }
    pub fn run(&mut self) -> String {
        let mut out = String::new();
        while let Some(inst) = self.iter.next() {
            match inst.opcode {
                VMOpcode::LOADK => {
                    let loadk_reg = inst.params[0].get_num_val();
                    let const_loc = inst.params[1].get_num_val();
                    let mut builder = String::new();
                    let mut local = true;
                    if let Some(inst) = self.iter.peek_ahead(1) {
                        if inst.opcode == VMOpcode::SETGLOBAL && loadk_reg == inst.params[0].get_num_val() {
                            builder.push_str(&self.iter.get_const(inst.params[1].get_num_val()).non_gc_asvalue().as_string(false));
                            local = false;
                        }
                    }
                    if local {
                        builder.push_str("local ");
                        builder.push_str(&self.get_lv_name());
                        builder.push_str(" = ");
                        builder.push_str(&self.iter.get_const(const_loc).non_gc_asvalue().as_string(true))
                    } else {
                        builder.push_str(" = ");
                        builder.push_str(&self.iter.get_const(const_loc).non_gc_asvalue().as_string(true))
                    }
                    out.push_str(&format!("{}\n", builder));
                }
                VMOpcode::ADD => {

                }
                _ => ()
            }
        }
        out
    }
}
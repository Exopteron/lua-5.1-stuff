use std::{any::Any, borrow::Borrow, fmt::Debug, sync::Arc, collections::HashMap, panic::{catch_unwind, AssertUnwindSafe}};

use ahash::AHashMap;
use gc::{Finalize, Gc, GcCell, GcCellRef, Trace, GcCellRefMut};

use self::{
    chunk_parser::{FunctionBlock, LuaChunk, LuaConstant},
    instruction::{InstParam, VMInst, VMOpcode, MASK_CBIT},
};

pub mod chunk_parser;
pub mod instruction;
pub mod decompiler;
#[derive(Debug, Trace, Finalize)]
pub enum LuaValue {
    Nil,
    Number(f64),
    Boolean(bool),
    String(String),
    Function(GCLuaFunction),
}
impl LuaValue {
    pub fn to_gc(self) -> GCLuaValue {
        GCLuaValue::new(self)
    }
    pub fn as_string(&self, fmts: bool) -> String {
        match self {
            LuaValue::Nil => "nil".to_string(),
            LuaValue::Number(v) => v.to_string(),
            LuaValue::Boolean(b) => b.to_string(),
            LuaValue::String(s) => {
                if fmts {
                    format!("\"{}\"", s.clone())
                } else {
                    s.clone()
                }
            },
            _ => String::new(),
        }
    }
}
#[derive(Debug, Clone, Trace, Finalize)]
pub struct GCLuaValue(Gc<GcCell<LuaValue>>);
impl GCLuaValue {
    pub fn new(v: LuaValue) -> Self {
        Self(Gc::new(GcCell::new(v)))
    }
    pub fn borrow(&self) -> GcCellRef<LuaValue> {
        self.0.try_borrow().unwrap()
    }
}
#[derive(Debug, Clone, Trace, Finalize)]
pub struct GCLuaFunction(Gc<GcCell<LuaFunction>>);
impl GCLuaFunction {
    pub fn new(v: LuaFunction) -> Self {
        Self(Gc::new(GcCell::new(v)))
    }
    pub fn borrow(&self) -> GcCellRef<LuaFunction> {
        self.0.try_borrow().unwrap()
    }
    pub fn borrow_mut(&self) -> GcCellRefMut<LuaFunction> {
        self.0.try_borrow_mut().unwrap()
    }
}
#[derive(Debug, Clone, Trace, Finalize)]
pub struct LuaFunction {
    prototype: Gc<FunctionBlock>,
    upvalues: HashMap<u32, GCLuaValue>,
    idx: usize,
}
impl LuaFunction {
    pub fn new(prototype: Gc<FunctionBlock>, upvalues: HashMap<u32, GCLuaValue>) -> Self {
        Self { prototype, idx: 0, upvalues }
    }
    pub fn to_gc(self) -> GCLuaFunction {
        GCLuaFunction::new(self)
    }
}
pub struct LuaVM {
    registers: AHashMap<u32, GCLuaValue>,
    globals: AHashMap<u32, GCLuaValue>,
    constants: AHashMap<u32, Gc<LuaConstant>>,
    currently_executing: Option<GCLuaFunction>,
    base: u32,
    top: Option<u32>,
    last_op: Option<VMInst>,
}
impl LuaVM {
    pub fn new() -> Self {
        let mut constants = AHashMap::new();
        Self {
            registers: AHashMap::new(),
            globals: AHashMap::new(),
            constants,
            currently_executing: None,
            base: 0,
            top: None,
            last_op: None
        }
    }
    pub fn process_chunk(&mut self, chunk: LuaChunk) -> Option<Vec<GCLuaValue>> {
        self.process_func(GCLuaFunction::new(LuaFunction::new(Gc::new(chunk.func), HashMap::new())))
    }
    fn call_func(&mut self, func: GCLuaFunction, base_set: u32, param_count: Option<u32>) -> Option<Vec<GCLuaValue>> {
        println!("Calling");
        let base = self.base;
        self.base = base_set;
        // if let Some(c) = param_count {
        //     for i in 0..c {
        //         if self.try_copy_register(i).is_none() {
        //             self.set_register(i, LuaValue::Nil.to_gc());
        //         }
        //     }
        // }
        let prev = self.currently_executing.clone().expect("Not executing? How");
        let v = self.process_func(func);
        println!("V: {:?}", v);
        self.set_consts(&prev);
        self.currently_executing = Some(prev);
        //self.base = base;
        v
    }
    fn set_consts(&mut self, func: &GCLuaFunction) {
        self.constants = AHashMap::new();
        for (idx, c) in func.borrow().prototype.list_const.iter().enumerate() {
            self.constants.insert(idx as u32, c.clone());
        }
    }
    fn process_func(&mut self, func: GCLuaFunction) -> Option<Vec<GCLuaValue>> {
        self.currently_executing = Some(func.clone());
        self.set_consts(&func);
        let i = func.borrow().idx;
        let len = func.borrow().prototype.list_instructions.len();
        for i in i..len {
            func.borrow_mut().idx = i;
            let inst = func.borrow().prototype.list_instructions[i].clone();
            let x = catch_unwind(AssertUnwindSafe(|| {
                match inst.opcode {
                    VMOpcode::LOADK => {
                        let params = &inst.params;
                        self.set_register(
                            params[0].get_num_val(),
                            self.get_constant(params[1].get_num_val()),
                        );
                    }
                    VMOpcode::RETURN => {
                        let b = inst.params[1].get_num_val();
                        if b > 1 {
                            println!("GR 1");
                            let mut ra = inst.params[0].get_num_val();
                            let mut returnval = Vec::new();
                            println!("RA {} b-1 {}", ra, b - 1);
                            if ra < b - 1 {
                                while ra < b - 1 {
                                    println!("Starting with register R({})", ra);
                                    returnval.push(self.copy_register(ra));
                                    ra += 1;
                                }
                            } else {
                                returnval.push(self.copy_register(ra));
                            }
                            // for r in ra..(b - 1) + 1 {
                            //     returnval.push(self.copy_register(r));
                            // }
                            return Some(returnval);
                        } else if b == 0 {
                            println!("Base: {}", self.base);
                            let mut i = inst.params[0].get_num_val();
                            let mut returnval = Vec::new();
                            //i += 1;
                            for i in i..self.top.unwrap() {
                                returnval.push(self.copy_register(i));
                            }
                            // while let Some(v) = self.try_copy_register(i) {
    
                            //     i += 1;
                            // }
                            return Some(returnval);
                        }
                    }
                    VMOpcode::ADD => {
                        let (out, p1, p2) = self.get_abc(&inst);
                        println!("p1 {:?} p2 {:?}", p1, p2);
                        if let LuaValue::Number(a) = *p1.borrow() {
                            if let LuaValue::Number(b) = *p2.borrow() {
                                println!("Setting R({}) to {}", out, a + b);
                                self.set_register(out, LuaValue::Number(a + b).to_gc());
                            }
                        };
                    }
                    VMOpcode::SUB => {
                        let (out, p1, p2) = self.get_abc(&inst);
                        if let LuaValue::Number(a) = *p1.borrow() {
                            if let LuaValue::Number(b) = *p2.borrow() {
                                self.set_register(out, LuaValue::Number(a - b).to_gc());
                            }
                        };
                    }
                    VMOpcode::MUL => {
                        let (out, p1, p2) = self.get_abc(&inst);
                        if let LuaValue::Number(a) = *p1.borrow() {
                            if let LuaValue::Number(b) = *p2.borrow() {
                                self.set_register(out, LuaValue::Number(a * b).to_gc());
                            }
                        };   
                    }
                    VMOpcode::MOD => {
                        let (out, p1, p2) = self.get_abc(&inst);
                        if let LuaValue::Number(a) = *p1.borrow() {
                            if let LuaValue::Number(b) = *p2.borrow() {
                                self.set_register(out, LuaValue::Number(a % b).to_gc());
                            }
                        };   
                    }
                    VMOpcode::POW => {
                        let (out, p1, p2) = self.get_abc(&inst);
                        if let LuaValue::Number(a) = *p1.borrow() {
                            if let LuaValue::Number(b) = *p2.borrow() {
                                self.set_register(out, LuaValue::Number(a.powf(b)).to_gc());
                            }
                        };     
                    }
                    VMOpcode::DIV => {
                        let (out, p1, p2) = self.get_abc(&inst);
                        if let LuaValue::Number(a) = *p1.borrow() {
                            if let LuaValue::Number(b) = *p2.borrow() {
                                self.set_register(out, LuaValue::Number(a / b).to_gc());
                            }
                        };     
                    }
                    VMOpcode::CLOSURE => {
                        let reg = inst.params[0].get_num_val();
                        let closure =
                            func.borrow().prototype.list_fnproto[inst.params[1].get_num_val() as usize].clone();
                        self.set_register(reg, LuaValue::Function(LuaFunction::new(closure, HashMap::new()).to_gc()).to_gc());
                    }
                    VMOpcode::MOVE => {
                        let last_op = self.last_op.as_ref().unwrap().clone();
                        if let VMOpcode::CLOSURE = self.last_op.as_ref().unwrap().opcode {
                            let reg = last_op.params[0].get_num_val();
                            let val = self.copy_register(reg);
                            if let LuaValue::Function(f) = &*val.borrow() {
                                let upvalue_to_set = inst.params[0].get_num_val();
                                let mut fmut = f.borrow_mut(); 
                                fmut.upvalues.remove(&upvalue_to_set);
                                fmut.upvalues.insert(upvalue_to_set, self.copy_register(inst.params[1].get_num_val()));
                            };
                        }
                        let b = self.copy_register(inst.params[1].get_num_val());
                        self.set_register(inst.params[0].get_num_val(), b);
                    }
                    VMOpcode::SETGLOBAL => {
                        let reg = inst.params[0].get_num_val();
                        let global_idx = inst.params[1].get_num_val();
                        let v = self.copy_register(reg);
                        self.set_global(global_idx, v);
                    }
                    VMOpcode::GETGLOBAL => {
                        let reg = inst.params[0].get_num_val();
                        let global_idx = inst.params[1].get_num_val();
                        let v = self.copy_global(global_idx);
                        self.set_register(reg, v);
                    }
                    VMOpcode::CALL => {
                        let reg_idx = inst.params[0].get_num_val();
                        let b = inst.params[1].get_num_val();
                        let c = inst.params[2].get_num_val();
                        let mut last_result = 0;
                        if b >= 2 {
                            let reg = self.copy_register(reg_idx);
                            let reg = reg.borrow();
                            if let LuaValue::Function(f) = &*reg {
                                if let Some(params) = self.call_func(f.clone(), reg_idx + 1, Some(b - 1)) {
                                    let mut idx = reg_idx;
                                    for p in params {
                                        last_result = idx;
                                        self.set_register(idx, p);
                                        idx += 1;
                                    }
                                }
                            }
                        } else {
                            let reg = self.copy_register(reg_idx);
                            let reg = reg.borrow();
                            if let LuaValue::Function(f) = &*reg {
                                if let Some(params) = self.call_func(f.clone(), self.base, None) {
                                    let mut idx = reg_idx;
                                    for p in params {
                                        last_result = idx;
                                        self.set_register(idx, p);
                                        idx += 1;
                                    }
                                }
                            }
                        }
                        if c == 0 {
                            self.top = Some(last_result + 1);
                        }
                    }
                    VMOpcode::TAILCALL => {
                        let reg_idx = inst.params[0].get_num_val();
                        let b = inst.params[1].get_num_val();
                        let mut last_result = 0;
                        if b >= 2 {
                            let reg = self.copy_register(reg_idx);
                            let reg = reg.borrow();
                            if let LuaValue::Function(f) = &*reg {
                                if let Some(params) = self.call_func(f.clone(), reg_idx + 1, Some(b - 1)) {
                                    let mut idx = reg_idx;
                                    for p in params {
                                        self.set_register(idx, p);
                                        last_result = idx;
                                        idx += 1;
                                    }
                                }
                            }
                        } else {
                            println!("LSTH {}", reg_idx);
                            let reg = self.copy_register(reg_idx);
                            println!("AF");
                            let reg = reg.borrow();
                            if let LuaValue::Function(f) = &*reg {
                                if let Some(params) = self.call_func(f.clone(), self.base, None) {
                                    let mut idx = reg_idx;
                                    for p in params {
                                        self.set_register(idx, p);
                                        last_result = idx;
                                        idx += 1;
                                    }
                                }
                            }
                        }
                        self.top = Some(last_result + 1);
                    }
                    VMOpcode::GETUPVAL => {
                        let upvalue_num = inst.params[1].get_num_val();
                        let register_num = inst.params[0].get_num_val();
                        let upvalue = func.borrow().upvalues.get(&upvalue_num).unwrap().clone();
                        self.set_register(register_num, upvalue);
                    }
                    VMOpcode::SETUPVAL => {
                        let upvalue_num = inst.params[1].get_num_val();
                        let register_num = inst.params[0].get_num_val();
                        let upvalue = &mut func.borrow_mut().upvalues;
                        upvalue.remove(&upvalue_num);
                        upvalue.insert(upvalue_num, self.copy_register(register_num));
                    }
                    _ => (),
                }
                None
            }));
            if x.is_err() {
                panic!("Panicked on {:?} instruction", inst.opcode);
            }
            self.last_op = Some(inst.clone());
            if let Some(x) = x.unwrap() {
                return Some(x);
            }
        }
        None
    }
    fn get_abc(&mut self, inst: &VMInst) -> (u32, GCLuaValue, GCLuaValue) {
        let out = inst.params[0].get_num_val();

        let p1loc = inst.params[1].get_num_val();
        let p2loc = inst.params[2].get_num_val();
        let p1_const = ((p1loc & MASK_CBIT) >> 8) == 1;
        let p2_const = ((p2loc & MASK_CBIT) >> 8) == 1;

        let p1 = if p1_const {
            self.get_constant(p1loc & !MASK_CBIT)
        } else {
            println!("P1 is reg {}", p1loc & !MASK_CBIT);
            self.copy_register(p1loc & !MASK_CBIT)
        };

        let p2 = if p2_const {
            self.get_constant(p2loc & !MASK_CBIT)
        } else {
            self.copy_register(p2loc & !MASK_CBIT)
        };
        (out, p1, p2)
    }
    fn cast<'a, T: 'static>(&self, val: &'a LuaValue) -> &'a T {
        let any = val as &dyn Any;
        any.downcast_ref::<T>().unwrap()
    }
    fn get_constant(&self, idx: u32) -> GCLuaValue {
        println!("getting constant: {}", idx);
        self.constants.get(&idx).unwrap().clone().as_value()
    }
    fn copy_global(&self, idx: u32) -> GCLuaValue {
        self.globals.get(&idx).unwrap().clone()
    }
    fn set_register(&mut self, mut idx: u32, val: GCLuaValue) {
        idx += self.base;
        self.registers.remove(&idx);
        self.registers.insert(idx, val);
    }
    fn get_register(&self, mut idx: u32) -> &GCLuaValue {
        idx += self.base;
        println!("getting register: {}", idx);
        self.registers.get(&idx).as_ref().unwrap()
    }
    fn take_register(&mut self, mut idx: u32) -> GCLuaValue {
        idx += self.base;
        self.registers.remove(&idx).unwrap()
    }
    fn try_take_register(&mut self, mut idx: u32) -> Option<GCLuaValue> {
        idx += self.base;
        self.registers.remove(&idx)
    }
    fn copy_register(&mut self, mut idx: u32) -> GCLuaValue {
        idx += self.base;
        self.registers.get(&idx).unwrap().clone()
    }
    fn try_copy_register(&mut self, mut idx: u32) -> Option<GCLuaValue> {
        idx += self.base;
        self.registers.get(&idx).cloned()
    }
    fn set_global(&mut self, idx: u32, v: GCLuaValue) {
        self.globals.remove(&idx);
        self.globals.insert(idx, v);
    }
}

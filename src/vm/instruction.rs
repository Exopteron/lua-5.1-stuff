use crate::def_enum;
use bitflags::bitflags;
use gc::{Finalize, Trace};
def_enum! {
    VMOpcode (u32) {
        0 = MOVE,
        1 = LOADK,
        2 = LOADBOOL,
        3 = LOADNIL,
        4 = GETUPVAL,
        5 = GETGLOBAL,
        6 = GETTABLE,
        7 = SETGLOBAL,
        8 = SETUPVAL,
        9 = SETTABLE,
        10 = NEWTABLE,
        11 = SELF,
        12 = ADD,
        13 = SUB,
        14 = MUL,
        15 = DIV,
        16 = MOD,
        17 = POW,
        18 = UNM,
        19 = NOT,
        20 = LEN,
        21 = CONCAT,
        22 = JMP,
        23 = EQ,
        24 = LT,
        25 = LE,
        26 = TEST,
        27 = TESTSET,
        28 = CALL,
        29 = TAILCALL,
        30 = RETURN,
        31 = FORLOOP,
        32 = FORPREP,
        33 = TFORLOOP,
        34 = SETLIST,
        35 = CLOSE,
        36 = CLOSURE,
        37 = VARARG,
    }
}
impl VMOpcode {
    pub fn param_types(&self) -> Vec<InstParamType> {
        match self {
            VMOpcode::MOVE => vec![InstParamType::A, InstParamType::B],
            VMOpcode::LOADK => vec![InstParamType::A, InstParamType::Bx],
            VMOpcode::LOADBOOL => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::LOADNIL => vec![InstParamType::A, InstParamType::B],
            VMOpcode::GETUPVAL => vec![InstParamType::A, InstParamType::B],
            VMOpcode::GETGLOBAL => vec![InstParamType::A, InstParamType::Bx],
            VMOpcode::GETTABLE => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::SETGLOBAL => vec![InstParamType::A, InstParamType::Bx],
            VMOpcode::SETUPVAL => vec![InstParamType::A, InstParamType::B],
            VMOpcode::SETTABLE => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::NEWTABLE => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::SELF => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::ADD => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::SUB => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::MUL => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::DIV => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::MOD => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::POW => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::UNM => vec![InstParamType::A, InstParamType::B],
            VMOpcode::NOT => vec![InstParamType::A, InstParamType::B],
            VMOpcode::LEN => vec![InstParamType::A, InstParamType::B],
            VMOpcode::CONCAT => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::JMP => vec![InstParamType::sBx],
            VMOpcode::EQ => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::LT => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::LE => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::TEST => vec![InstParamType::A, InstParamType::C],
            VMOpcode::TESTSET => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::CALL => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::TAILCALL => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::RETURN => vec![InstParamType::A, InstParamType::B],
            VMOpcode::FORLOOP => vec![InstParamType::A, InstParamType::sBx],
            VMOpcode::FORPREP => vec![InstParamType::A, InstParamType::sBx],
            VMOpcode::TFORLOOP => vec![InstParamType::A, InstParamType::C],
            VMOpcode::SETLIST => vec![InstParamType::A, InstParamType::B, InstParamType::C],
            VMOpcode::CLOSE => vec![InstParamType::A],
            VMOpcode::CLOSURE => vec![InstParamType::A, InstParamType::Bx],
            VMOpcode::VARARG => vec![InstParamType::A, InstParamType::B],
        }
    }
}
pub enum InstParamType {
    A,
    B,
    C,
    Bx,
    sBx,
}
#[derive(Debug, Clone, Finalize, Trace)]
pub enum InstParam {
    A(u32),
    B(u32),
    C(u32),
    Bx(u32),
    sBx(i32),
}
pub const MASK_CBIT: u32 = 0b00000000000000000000000100000000u32;
impl InstParam {
    const MASK_B: u32 = 0b11111111100000000000000000000000u32;
    const MASK_C: u32 = 0b00000000011111111100000000000000u32;
    const MASK_Bx: u32 = 0b11111111111111111100000000000000u32;
    const MASK_A: u32 = 0b00000000000000000011111111000000u32;
    const MASK_Op: u32 = 0b00000000000000000000000000111111u32;
    const A_SHIFT: u32 = 6;
    const B_SHIFT: u32 = 23;
    const C_SHIFT: u32 = 14;
    const Bx_SHIFT: u32 = 14;
    pub fn parse(t: InstParamType, num: u32) -> Self {
        match t {
            InstParamType::A => Self::A((num & InstParam::MASK_A) >> InstParam::A_SHIFT),
            InstParamType::B => Self::B((num & InstParam::MASK_B) >> InstParam::B_SHIFT),
            InstParamType::C => Self::C((num & InstParam::MASK_C) >> InstParam::C_SHIFT),
            InstParamType::Bx => Self::Bx((num & InstParam::MASK_Bx) >> InstParam::Bx_SHIFT),
            InstParamType::sBx => {
                Self::sBx((((num & InstParam::MASK_Bx) >> InstParam::Bx_SHIFT) as i32) - 131071)
            }
        }
    }
    /// DO NOT CALL ON SBX
    pub fn get_num_val(&self) -> u32 {
        match self {
            InstParam::A(v) => *v,
            InstParam::B(v) => *v,
            InstParam::C(v) => *v,
            InstParam::Bx(v) => *v,
            InstParam::sBx(_) => panic!("BAD"),
        }
    }
}
#[derive(Debug, Clone, Finalize, Trace)]
pub struct VMInst {
    pub opcode: VMOpcode,
    pub params: Vec<InstParam>,
}
impl VMInst {
    pub fn from_u32(num: u32) -> anyhow::Result<Self> {
        //println!("Num {}", num & 0b00000000000000000011111111000000);
        let opcode = VMOpcode::from_num(num.to_le() & 0b00000000000000000000000000111111)?;
        let types = opcode.param_types();
        let mut params = Vec::new();
        for t in types {
            params.push(InstParam::parse(t, num));
        }
        Ok(Self { opcode, params })
    }
}

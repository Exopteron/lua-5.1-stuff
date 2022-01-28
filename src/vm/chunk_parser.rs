use std::{io::Read, sync::Arc};

use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};
use gc::{Finalize, Gc, Trace};
use rand::RngCore;

use super::{
    instruction::{VMInst, VMOpcode},
    GCLuaValue, LuaValue,
};
pub struct ChunkReader<'a> {
    reader: &'a mut dyn Read,
}
impl<'a> ChunkReader<'a> {
    pub fn new(reader: &'a mut dyn Read) -> Self {
        Self { reader }
    }
    pub fn read_bytes(&mut self, len: usize) -> anyhow::Result<Vec<u8>> {
        let mut bytes = vec![0; len];
        self.reader.read_exact(&mut bytes)?;
        Ok(bytes)
    }
    pub fn read_byte(&mut self) -> anyhow::Result<u8> {
        Ok(self.read_bytes(1)?[0])
    }
    pub fn read_int(&mut self) -> anyhow::Result<u32> {
        let mut bytes: &[u8] = &self.read_bytes(4)?;
        Ok(bytes.read_u32::<LittleEndian>()?)
    }
    pub fn read_sizet(&mut self) -> anyhow::Result<usize> {
        Ok(self.reader.read_u64::<LittleEndian>()? as usize)
    }
    pub fn read_string(&mut self) -> anyhow::Result<String> {
        let len = self.read_sizet()?;
        //println!("Len: {}", len);
        if len == 0 {
            return Ok(String::new());
        }
        let mut bytes = self.read_bytes(len)?;
        bytes.pop();
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }
    pub fn read_boolean(&mut self) -> anyhow::Result<bool> {
        Ok(self.reader.read_u8()? != 0)
    }
    pub fn read_number(&mut self) -> anyhow::Result<f64> {
        Ok(self.reader.read_f64::<LittleEndian>()?)
    }
}
impl Read for ChunkReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.reader.read(buf)
    }
}
trait FromChunkReader: Sized {
    fn from_reader(reader: &mut ChunkReader, info: Option<&str>) -> anyhow::Result<Self>;
}
pub struct LuaChunk {
    pub header: ChunkHeader,
    pub func: FunctionBlock,
}
impl LuaChunk {
    pub fn from_reader(reader: &mut dyn Read) -> anyhow::Result<Self> {
        let mut reader = ChunkReader::new(reader);
        let header = ChunkHeader::from_reader(&mut reader, None)?;
        if header.version != 81 {
            bail!(
                "Wrong lua version {}, we are on 81 (Lua 5.1)",
                header.version
            );
        }
        println!("Header: {:?}", header);
        let func = FunctionBlock::from_reader(&mut reader, None)?;
        Ok(Self { header, func })
    }
}
#[derive(Debug)]
pub struct ChunkHeader {
    version: u8,
    format_version: u8,
    endianness: u8,
    size_int: u8,
    size_t: u8,
    size_Inst: u8,
    size_luaNum: u8,
    integral_flag: u8,
}
impl FromChunkReader for ChunkHeader {
    fn from_reader(reader: &mut ChunkReader, info: Option<&str>) -> anyhow::Result<Self> {
        let mut bytes = [0; 4];
        reader.read_exact(&mut bytes)?;
        let h = u32::from_be_bytes(bytes);
        if h != 0x1B4C7561 {
            bail!("Header mismatch");
        }
        let mut bytes = [0; 8];
        reader.read_exact(&mut bytes)?;
        Ok(Self {
            version: bytes[0],
            format_version: bytes[1],
            endianness: bytes[2],
            size_int: bytes[3],
            size_t: bytes[4],
            size_Inst: bytes[5],
            size_luaNum: bytes[6],
            integral_flag: bytes[7],
        })
    }
}
#[derive(Debug, Clone, Finalize, Trace)]
pub struct FunctionBlock {
    pub source_name: String,
    pub line_def: u32,
    pub last_line_def: u32,
    pub num_upval: u8,
    pub num_param: u8,
    pub is_vararg: u8,
    pub max_stack_size: u8,
    pub list_instructions: Vec<VMInst>,
    pub list_const: Vec<Gc<LuaConstant>>,
    pub list_fnproto: Vec<Gc<FunctionBlock>>,
}
impl FromChunkReader for FunctionBlock {
    fn from_reader(reader: &mut ChunkReader, info: Option<&str>) -> anyhow::Result<Self> {
        let source_name = reader.read_string().unwrap();
        println!("Source name: {}", source_name);
        let line_def = reader.read_int().unwrap();
        //println!("Line def: {}", line_def);
        let last_line_def = reader.read_int().unwrap();
        //println!("Last line def: {}", last_line_def);
        let num_upval = reader.read_byte().unwrap();
        //println!("Num upval: {}", num_upval);
        let num_param = reader.read_byte().unwrap();
        //println!("Num parm: {}", num_param);
        let is_vararg = reader.read_byte().unwrap();
        //println!("is varg: {}", is_vararg);
        let max_stack_size = reader.read_byte().unwrap();
        //println!("mas stack: {}", max_stack_size);
        let list_instructions: Vec<VMInst> = FromChunkReader::from_reader(reader, None).unwrap();
        for inst in list_instructions.iter() {
            println!("INST: {:?} {:?}", inst.opcode, inst.params);
        }
        let list_const: Vec<LuaConstant> = FromChunkReader::from_reader(reader, None).unwrap();
        for (idx, c) in list_const.iter().enumerate() {
            println!("CONST #{}: {:?}", idx, c);
        }
        //println!("Reading FN prototypes");
        let list_fnproto: Vec<FunctionBlock> = FromChunkReader::from_reader(reader, Some("s")).unwrap();
        let mut list_arc_const: Vec<Gc<LuaConstant>> = Vec::new();
        for c in list_const {
            list_arc_const.push(Gc::new(c));
        }
        let mut list_arc_fnproto: Vec<Gc<FunctionBlock>> = Vec::new();
        for c in list_fnproto {
            list_arc_fnproto.push(Gc::new(c));
        }
        reader.read_int();
        reader.read_int();
        reader.read_int();
        Ok(Self {
            source_name,
            line_def,
            last_line_def,
            num_upval,
            num_param,
            is_vararg,
            max_stack_size,
            list_instructions,
            list_const: list_arc_const,
            list_fnproto: list_arc_fnproto,
        })
    }
}
impl FromChunkReader for LuaConstant {
    fn from_reader(reader: &mut ChunkReader, info: Option<&str>) -> anyhow::Result<Self> {
        let x = reader.read_byte()?;
        //println!("Num: {}", x);
        Ok(match x {
            0 => LuaConstant::LUA_TNIL,
            1 => LuaConstant::LUA_TBOOLEAN(reader.read_boolean()?),
            3 => LuaConstant::LUA_TNUMBER(reader.read_number()?),
            4 => LuaConstant::LUA_TSTRING(reader.read_string()?),
            _ => bail!("Unknown const {}", x),
        })
    }
}
impl<T> FromChunkReader for Vec<T>
where
    T: FromChunkReader,
{
    fn from_reader(reader: &mut ChunkReader, info: Option<&str>) -> anyhow::Result<Self> {
        let x = rand::thread_rng().next_u32();
        //println!("Genning {}", x);
        let size = reader.read_int()?;
        //println!("Size: {:?} on x {} with info {:?}", size, x, info);
        if size == 0 {
            return Ok(Vec::new());
        }
        //println!("Size here {}", size);
        let mut vec: Vec<T> = Vec::new();
        for _ in 0..size {
            //println!("Size in {} on x {} with info {:?}", size, x, info);
            vec.push(T::from_reader(reader, None).unwrap());
        }
        Ok(vec)
    }
}
// impl FromChunkReader for Vec<(VMOpcode, u32)> {
//     fn from_reader(reader: &mut ChunkReader) -> anyhow::Result<Self> {
//         let mut vec = Vec::new();
//         let size = reader.read_int()?;
//         println!("Size {}", size);
//         let vals = reader.read_bytes(8)?;
//         println!("VALS: {:?}", vals);
//         for _ in 0..size {
//             let param = reader.read_int()?;
//             vec.push((VMOpcode::from_reader(reader)?, param));
//         }
//         Ok(vec)
//     }
// }
impl FromChunkReader for VMInst {
    fn from_reader(reader: &mut ChunkReader, info: Option<&str>) -> anyhow::Result<Self> {
        let v = reader.read_int()?;
        //println!("V: {}", v);
        VMInst::from_u32(v)
    }
}
// impl FromChunkReader for Vec<VMOpcode> {
//     fn from_reader(reader: &mut ChunkReader) -> anyhow::Result<Self> {
//         let size = reader.read_int()?;
//         let mut vec = Vec::new();
//         for _ in 0..size {
//             let val = reader.read_int()?;
//             vec.push(VMOpcode::NUL);
//         }
//         Ok(vec)
//     }
// }
#[derive(Debug, Clone, Finalize, Trace)]
pub enum LuaConstant {
    LUA_TNIL,
    LUA_TBOOLEAN(bool),
    LUA_TNUMBER(f64),
    LUA_TSTRING(String),
}

impl LuaConstant {
    pub fn non_gc_asvalue(&self) -> LuaValue {
        match self {
            LuaConstant::LUA_TNIL => LuaValue::Nil,
            LuaConstant::LUA_TBOOLEAN(v) => LuaValue::Boolean(*v),
            LuaConstant::LUA_TNUMBER(v) => LuaValue::Number(*v),
            LuaConstant::LUA_TSTRING(v) => LuaValue::String(v.clone()),
        }
    }
    pub fn as_value(&self) -> GCLuaValue {
        GCLuaValue::new(self.non_gc_asvalue())
    }
}

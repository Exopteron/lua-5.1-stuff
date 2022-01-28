use std::fs::File;

use vm::{chunk_parser::LuaChunk, LuaVM, decompiler::LuaDecompiler};

use crate::vm::chunk_parser::ChunkHeader;

pub mod vm;
#[macro_use]
pub mod macros;
fn main() {
    let main = LuaChunk::from_reader(&mut File::open("luac.out").unwrap()).unwrap();
    // let mut decomp = LuaDecompiler::new(main);
    // println!("{}", decomp.run());
    let mut vm = LuaVM::new();
    println!("Output: {:#?}", vm.process_chunk(main));
    //let header = ChunkHeader::from_reader(&mut File::open("luac.out").unwrap()).unwrap();
    //println!("Hello, world! {:?}", header);
}

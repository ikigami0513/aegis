use crate::chunk::Chunk;
use crate::opcode::OpCode;

pub fn disassemble_chunk(chunk: &Chunk, name: &str) {
    println!("== {} ==", name);

    let mut offset = 0;
    while offset < chunk.code.len() {
        offset = disassemble_instruction(chunk, offset);
    }
}

pub fn disassemble_instruction(chunk: &Chunk, offset: usize) -> usize {
    print!("{:04} ", offset); // Affiche l'adresse (ex: 0000)

    let instruction: OpCode = chunk.code[offset].into();

    match instruction {
        OpCode::Return => simple_instruction("RETURN", offset),
        OpCode::Print => simple_instruction("PRINT", offset),
        OpCode::Add => simple_instruction("ADD", offset),
        OpCode::Sub => simple_instruction("SUB", offset),
        OpCode::Mul => simple_instruction("MUL", offset),
        OpCode::Div => simple_instruction("DIV", offset),

        OpCode::Pop => simple_instruction("POP", offset),
        
        // Instructions avec opérandes (1 octet de plus)
        OpCode::LoadConst => constant_instruction("LOAD_CONST", chunk, offset),

        // --- Affichage des Globales ---
        OpCode::GetGlobal => byte_instruction("GET_GLOBAL", chunk, offset),
        OpCode::SetGlobal => byte_instruction("SET_GLOBAL", chunk, offset),
        OpCode::GetLocal => byte_instruction("GET_LOCAL", chunk, offset),
        OpCode::SetLocal => byte_instruction("SET_LOCAL", chunk, offset),

        OpCode::Jump => jump_instruction("JUMP", 1, chunk, offset),
        OpCode::JumpIfFalse => jump_instruction("JUMP_IF_FALSE", 1, chunk, offset),
        OpCode::Loop => jump_instruction("LOOP", -1, chunk, offset), // -1 pour indiquer arrière
        OpCode::Call => byte_instruction("CALL", chunk, offset),

        OpCode::Modulo => simple_instruction("MOD", offset),
        OpCode::Equal => simple_instruction("EQUAL", offset),
        OpCode::NotEqual => simple_instruction("NOT_EQUAL", offset),
        OpCode::Greater => simple_instruction("GREATER", offset),
        OpCode::GreaterEqual => simple_instruction("GREATER_EQUAL", offset),
        OpCode::Less => simple_instruction("LESS", offset),
        OpCode::LessEqual => simple_instruction("LESS_EQUAL", offset),
        OpCode::Not => simple_instruction("NOT", offset),
        
        OpCode::BitAnd => simple_instruction("BIT_AND", offset),
        OpCode::BitOr => simple_instruction("BIT_OR", offset),
        OpCode::BitXor => simple_instruction("BIT_XOR", offset),
        OpCode::ShiftLeft => simple_instruction("SHIFT_LEFT", offset),
        OpCode::ShiftRight => simple_instruction("SHIFT_RIGHT", offset),

        OpCode::MakeList => byte_instruction("MAKE_LIST", chunk, offset),
        OpCode::MakeDict => byte_instruction("MAKE_DICT", chunk, offset),
        
        OpCode::Class => constant_instruction("CLASS", chunk, offset),
        OpCode::Method => constant_instruction("METHOD", chunk, offset),
        OpCode::GetAttr => constant_instruction("GET_ATTR", chunk, offset),
        OpCode::SetAttr => constant_instruction("SET_ATTR", chunk, offset),
        OpCode::Super => {
            let method_idx = chunk.code[offset + 1];
            let arg_count = chunk.code[offset + 2];
            let parent_idx = chunk.code[offset + 3];

            let method_name = &chunk.constants[method_idx as usize];
            let parent_name = &chunk.constants[parent_idx as usize];

            println!("{:-16} '{}' ({} args) super-> '{}'", "SUPER", method_name, arg_count, parent_name);
            
            // On avance de 4 (1 OpCode + 3 Args)
            offset + 4
        },
        
        OpCode::Input => simple_instruction("INPUT", offset),

        OpCode::MakeClosure => simple_instruction("MAKE_CLOSURE", offset),
        OpCode::GetFreeVar => { constant_instruction("GET_FREE_VAR", chunk, offset) },
        OpCode::Dup => simple_instruction("DUP", offset),

        OpCode::SetupExcept => jump_instruction("SETUP_EXCEPT", 1, chunk, offset),
        OpCode::PopExcept => simple_instruction("POP_EXCEPT", offset),
        OpCode::Throw => simple_instruction("THROW", offset),

        OpCode::Import => constant_instruction("IMPORT", chunk, offset),
        OpCode::CheckType => constant_instruction("CHECK_TYPE", chunk, offset),
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{}", name);
    offset + 1
}

fn constant_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    // L'octet suivant contient l'index de la constante
    let constant_idx = chunk.code[offset + 1];
    print!("{:<16} {:4} '", name, constant_idx);
    print!("{}", chunk.constants[constant_idx as usize]);
    println!("'");
    offset + 2 // On a lu l'opcode + l'index
}

fn byte_instruction(name: &str, chunk: &Chunk, offset: usize) -> usize {
    let slot = chunk.code[offset + 1];
    println!("{:<16} {:4}", name, slot);
    offset + 2
}

fn jump_instruction(name: &str, sign: i8, chunk: &Chunk, offset: usize) -> usize {
    // On lit 2 octets pour former un u16
    let jump = (chunk.code[offset + 1] as u16) << 8 | chunk.code[offset + 2] as u16;
    
    // On calcule la destination absolue pour l'affichage
    let dest = offset as isize + 3 + (sign as isize * jump as isize);
    
    println!("{:<16} {:4} -> {}", name, offset, dest);
    offset + 3 // Opcode + 2 bytes
}

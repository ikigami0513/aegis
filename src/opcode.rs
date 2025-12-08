#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum OpCode {
    // --- Chargement de données ---
    LoadConst,  // Pousse une constante sur la pile
    
    // --- Variables ---
    GetGlobal,  // Récupère une variable par son index
    SetGlobal,  // Définit une variable
    GetLocal, // Récupère une variable sur la pile (argument/locale)
    SetLocal, // Modifie une variable sur la pile
    
    // --- Arithmétique ---
    Add,
    Sub,
    Mul,
    Div,

    // Math & Logic
    Modulo,
    NotEqual, Equal, Greater, GreaterEqual, Less, LessEqual,
    Not,
    BitAnd, BitOr, BitXor, ShiftLeft, ShiftRight,
    
    // --- Contrôle de flux ---
    JumpIfFalse,
    Jump,
    Loop,
    
    // --- Système ---
    Print,
    Return,
    Call,

    // Structures
    MakeList, // operand: u8 (count)
    MakeDict, // operand: u8 (count * 2)

    // OOP
    Class,    // operand: const_idx (nom)
    SetAttr,  // operand: const_idx (nom attribut)
    GetAttr,  // operand: const_idx (nom attribut)
    Method,   // operand: const_idx (nom méthode)

    // Scopes / Namespaces
    Pop,      // Pour nettoyer la pile (ExpressionStatement)
    
    // I/O
    Input,

    MakeClosure,
    GetFreeVar,
    Dup,

    // Exception
    SetupExcept, // Démarre un bloc Try (pousse un handler)
    PopExcept,   // Fin du bloc Try avec succès (retire le handler)
    Throw,
}

impl From<u8> for OpCode {
    fn from(b: u8) -> Self {
        unsafe { std::mem::transmute(b) }
    }
}

# Aegis Architecture (VM vs Tree-Walk)

Aegis v0.2 marks a paradigm shift from a Tree-Walk Interpreter to a **Bytecode Virtual Machine**. This architecture is designed for speed, cache locality, and portability.

[Image of Aegis compiler pipeline source to bytecode]

## The Pipeline

When you run a script (`aegis run script.aeg`), the following steps occur:

1.  **Lexing (Scanner)**: The source code string is broken down into a stream of **Tokens** (Keywords, Identifiers, Literals).
2.  **Parsing**: The tokens are analyzed to build an **Abstract Syntax Tree (AST)**. This represents the logical structure of the code.
3.  **Compilation**: The AST is traversed once to generate a flat **Chunk** of Bytecode. Control flow (if/while) is converted into Jump instructions.
4.  **Execution (VM)**: The Virtual Machine executes the Bytecode linearly using a central **Stack**.

## Stack-Based Virtual Machine

Aegis uses a Stack-Based architecture (similar to Java JVM or Python). There are no registers. Instructions push values onto the stack or pop them off to perform operations.

**Example: `10 + 20`**

| Instruction | Stack State | Description |
| :--- | :--- | :--- |
| `LOAD_CONST 10` | `[10]` | Pushes 10 onto the stack. |
| `LOAD_CONST 20` | `[10, 20]` | Pushes 20 onto the stack. |
| `ADD` | `[30]` | Pops 20 and 10, adds them, pushes 30. |

### Memory Model

* **Values**: Aegis uses a compact `Value` enum (~24 bytes). Heavy objects (Functions, Classes, Lists) are stored on the Heap using Reference Counting (`Rc<RefCell>`), allowing for cheap copies and automatic memory management.
* **Call Frames**: When a function is called, a new Frame is pushed. It tracks the function's instruction pointer and the offset for its local variables on the global stack.

## Performance

The transition to v0.2 resulted in a massive performance boost (approx. **12x faster** on heavy recursion).

* **CPU Cache Friendly**: Instructions are stored in a contiguous `Vec<u8>`, reducing cache misses compared to traversing a pointer-heavy tree.
* **Fast-Path Optimization**: Common operations (like Integer addition) are optimized to occur in-place on the stack without memory allocation.

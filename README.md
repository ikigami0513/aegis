# 🛡️ Aegis Language

Aegis is a modern, dynamic, high-performance scripting language written in Rust.

Previously an AST interpreter, **Aegis v0.2.0** introduces a brand new Stack-Based Bytecode Virtual Machine, making it exponentially faster and suitable for real-world applications, game scripting, and system automation.

## ⚡ Performance Benchmark (Fibonacci 30)

The move to a Bytecode VM resulted in a massive performance leap.
| Language | Engine | Execution Time |
|--- |--- |--- |
| Rust | Native (Optimized) | ~2 ms |
| Python | CPython 3.11 | ~147 ms |
| Aegis v0.2 | Bytecode VM (Rust) | ~290 ms |
| Aegis v0.1 | Tree-Walk Interpreter | ~7286 ms |

## 📘 Documentation

The complete documentation, including tutorials and API reference, is available here: 👉 (The Aegis Book)[https://aegisprogramminglanguage.github.io/AegisProgrammingLanguage/getting_started/cheatsheet.html]

## 🚀 Installation & Usage

### Building from Source

You need Rust installed.

```bash
git clone https://github.com/your-username/aegis.git
cd aegis
cargo install --path .
```

Running a Script

```bash
aegis run path/to/script.aeg
```

Interactive Mode (REPL)

```bash
aegis repl
```

## ✨ Features Overview

### 1️⃣ Typing System (Gradual Typing)

Aegis allows you to mix dynamic freedom with static safety.

```aegis
// Dynamic
var x = 10
var name = "Aegis"

// Typed (Runtime checked)
var age: int = 25
var price: float = 19.99
```

### 2️⃣ Modern Data Structures

Native support for Lists and Dictionaries with functional methods.

```aegis
var users = [
    { name: "Alice", admin: true },
    { name: "Bob", admin: false }
]

// Functional chaining (Map/Filter)
var admins = users.filter(func(u) { return u.get("admin") })
                  .map(func(u) { return u.get("name") })

print admins // ["Alice"]
```

### 3️⃣ Control Flow

Includes `if`, `while`, `for` (range-based), and `switch`. Supports `break` and `continue`.

```aegis
for (i, 0, 10, 1) {
    if (i % 2 == 0) { continue }
    print i
}
```

### 4️⃣ Object-Oriented Programming

Class-based OOP with Single Inheritance, Methods, and `super` calls.

```aegis
class Entity(name) {
    init() { print this.name + " spawned." }
}

class Hero(name, hp) extends Entity {
    init() {
        super.init()
        this.hp = hp
    }
}

var p = new Hero("Link", 100)
p.init()
```

### 5️⃣ Modularity

Code organization with **Namespaces** and **Imports**.

```aegis
// file: math_utils.aeg
namespace MathUtils {
    func add(a, b) { return a + b }
}

// main.aeg
import "math_utils.aeg"
print MathUtils.add(10, 20)
```

### 🔋 Standard Library ("Batteries Included")

Aegis v0.2.0 ships with a rich set of modules built into the VM or available as native extensions.

| Module | Purpose | Example |
|--- |--- |--- |
| System | OS interaction (Args, Env, Clear) | `System.env("PATH")` |
| File | Read/Write files | `File.read("config.json")` |
| Http | Web Client (GET/POST) | `Http.get("https://api.com")` |
| Socket | TCP Networking (Server/Client) | `Socket.listen("127.0.0.1", 8080)` |
| Json | Parsing & Serialization | `Json.parse(data)` |
| Regex | Pattern Matching | `Regex.match(re, text)` |
| Math | Advanced Math & Vector2 | `Math.sin(x)` |

## 🛠️ Tooling

- **VS Code Extension**: Syntax highlighting is available for `.aeg` files.
- **Package Manager**: Use `aegis add <package>` to install dependencies (WIP).

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

- Fork the project
- Create your feature branch (`git checkout -b feature/AmazingFeature`)
- Commit your changes (`git commit -m 'Add some AmazingFeature'`)
- Push to the branch (`git push origin feature/AmazingFeature`)
- Open a Pull Request

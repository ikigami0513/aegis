# 🛡️ Aegis Language Documentation

Aegis is a modern, dynamic (with gradual typing), object-oriented interpreted scripting language written in Rust. It combines the simplicity of Python/JavaScript with powerful features like first-class functions, pattern matching, and built-in system tools.

---

## 🚀 Quick Start
### Interactive Mode (REPL)

To start the Aegis shell and type code directly:

```bash
cargo run
```

### Run a Script

To execute an Aegis file:

```bash
cargo run path/to/script.aeg
```

---

## 1️⃣ Variables & Types
### Declaration & Types

Variables are declared using var. Aegis supports Gradual Typing: you can specify types for safety, or leave them dynamic.

```
// Dynamic typing
var x = 10
var name = "Aegis"

// Static typing (Gradual)
var age: int = 25
var pi: float = 3.14159
var active: bool = true
```

### Destructuring

You can unpack lists directly into variables.

```
var point = [10, 20]
var [x, y] = point
print x // 10
```

### String Interpolation & Formatting

Embed expressions directly into strings using ${...}. You can also specify format precision for floats.

```
var item = "Apple"
var price = 1.256
print "The ${item} costs ${price:.2f} dollars." 
// Output: "The Apple costs 1.26 dollars."
```

---

## 2️⃣ Data Structures
### Lists

Ordered collections of items.

```
var list = [1, 2, 3]
list.push(4)
print list.at(0) // Access element
```

### Dictionaries

Key-value pairs. Keys can be strings. Access values via methods or dot notation.

```
var user = {
    nom: "Alice",
    role: "Admin"
}

// Access
print user.get("nom") // Method style
print user.role       // Property style (Syntactic sugar)
```

---

## 3️⃣ Operators
### Arithmetic & Assignment

Standard: `+`, `-`, `*`, `/`, `%`. Shorthands: `+=`, `-=`, `*=`, `/=`, `++`, `--`.

```
var i = 0
i++       // i is 1
i += 10   // i is 11
```

### Bitwise Operators

Low-level bit manipulation. `&` (AND), `|` (OR), `^` (XOR), `<<` (Left Shift), `>>` (Right Shift).

```
var a = 12 // 1100
var b = 5  // 0101
print a & b // 4 (0100)
```

---

## 4️⃣ Control Structures
### Conditionals

Standard `if`, `else`. Parentheses are required.

```
if (x > 10) { ... } else { ... }
```

### Loops

- While: `while (condition) { ... }`

- For Range: `for (var_name, start, end, step) { ... }`

### Switch / Case

Clean control flow with implicit break.

```
switch (val) {
    case 1:
        print "One"
    case 2:
        print "Two"
    default:
        print "Other"
}
```

### Error Handling (Try / Catch)

Safely handle runtime errors.

```
try {
    var res = 10 / 0
} catch (e) {
    print "Error caught: " + e
}
```

---

## 5️⃣ Functions & Lambdas
### Named Functions (Typed)

You can optionally type arguments and return values.

```
func add(a: int, b: int) -> int {
    return a + b
}
```

### Anonymous Functions (Lambdas)

Functions are first-class citizens. They can be stored in variables and passed as arguments.

```
var greet = func(name) {
    print "Hello " + name
}
greet("World")
```

### Decorators

Wrap functions with logic using @.

```
@log
func compute(x) { return x * x }
```

---

## 6️⃣ Functional Programming

Lists support functional methods using lambdas.

- map: Transform elements.

- filter: Select elements.

- for_each: Iterate over elements.

```
var nums = [1, 2, 3, 4]

// Square numbers > 2
var res = nums.filter(func(n) { return n > 2 })
              .map(func(n) { return n * n })

print res // [9, 16]
```

## 7️⃣ Object-Oriented Programming
### Classes & Namespaces

Classes define objects. Namespaces organize code.

```
namespace Game {
    class Player(name) {
        hello() { print "Hi, I am " + this.name }
    }
}

var p = new Game.Player("Mario")
p.hello()
```

### Inheritance

Use extends to inherit methods.

```
class Dog(n) extends Animal { ... }
```

---

## 8️⃣ Standard Library (Modules)

Aegis comes with a powerful standard library organized in Namespaces.

`Math`
| Function | Description |
| :--- | :--- |
| Math.PI | Constant (3.14159...) |
| Math.abs(n) | Absolute value |
| Math.pow(b, e) | Power |
| Math.sqrt(n) | Square root |
| Math.max(a, b) | Maximum |
| Math.Vector2(x,y) | Class for 2D vectors |

`File (I/O)`
| Function | Description |
| :--- | :--- |
| File.read(path) | Returns file content as string or null. |
| File.write(path, str) | Writes (overwrites) to a file. |
| File.append(path, str) | Appends text to end of file. |
| File.exists(path) | Returns true if file exists. |
| File.delete(path) | Deletes a file. |

`System`
| Function | Description |
| :--- | :--- |
| System.clear() | Clears the console screen. |

`Time`
| Function | Description |
| :--- | :--- |
| Time.now() | Returns current timestamp (ms). |
| Time.sleep(ms) | Pauses execution. |
| Time.elapsed(start) | Returns elapsed ms since start. |

`Random`
| Function | Description |
| :--- | :--- |
| Random.int(min, max) | Random integer `[min, max[`. |
| Random.float() | Random float `0.0 - 1.0`. |

---

## 9️⃣ Global Built-ins

- `print expr`: Outputs to console.

- `input var prompt`: Reads user input.

- `len(obj)`: Length of list/string/dict.

- `str(obj)`: Convert to string.

- `to_int(str)`: Convert to integer.

- `import "file.aeg"`: Loads external script.

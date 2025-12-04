# 🛡️ Aegis Language Documentation

Aegis is a dynamic, object-oriented, interpreted scripting language written in Rust. It features a simple syntax inspired by C and JavaScript, designed to be easy to read and write.

---

## 🚀 Quick Start

To execute an Aegis script, use Cargo from the project root:
```bash
cargo run path/to/script.aeg
```

Example:
```bash
cargo run ./tests/test_full.aeg
```

## 1️⃣ Basic Syntax
### Comments

Single-line comments start with `//`.

```aegis
// This is a comment
var x = 10 // Comment at the end of a line
```

### Variables

Variables are dynamically typed. Use the var keyword to declare them.

```aegis
var a = 10         // Integer
var b = 3.14       // Float
var name = "Aegis" // String
var list = [1, 2]  // List
```

### Output (Print)

The print instruction evaluates an expression and outputs it to the console, followed by a newline.

```aegis
print "Hello"
print 10 + 20
```

### User Input (Input)

The input instruction displays a prompt message and stores the user's response (as a string) into a variable.

Syntax: input <variable_name> <prompt_message>

```aegis
input age_str "How old are you? "
// age_str now contains the user input
```

---

## 2️⃣ Operations & Types
### Arithmetic

Standard operators are supported: `+`, `-`, `*`, `/`. Operator precedence is respected (`*` and `/` before `+` and `-`).

```aegis
var res = 10 + 5 * 2 // Result: 20
```

## Concatenation (Polymorphism)

The `+` operator is context-aware. If one of the operands is a string, Aegis automatically converts the other operand to a string to concatenate them.

```aegis
var age = 25
print "I am " + age + " years old." // Output: "I am 25 years old."
```

---

## 3️⃣ Control Structures
### Conditionals (If / Else)

Parentheses are mandatory around the condition.

```aegis
var x = 50
if (x > 100) {
    print "High"
} else {
    print "Low"
}
```

### While Loop

Executes a block as long as the condition is true.

```aegis
var i = 0
while (i < 3) {
    print "Iteration " + i
    i = i + 1
}
```

### For Loop (Range)

Aegis uses a for loop optimized for numerical iterations.

Syntax: `for (variable, start, end, step)`

```aegis
// Count from 0 to 10 with a step of 2 (0, 2, 4, 6, 8)
for (j, 0, 10, 2) {
    print "j = " + j
}
```

---

## 4️⃣ Functions
### Function
Functions are declared using `func`. They support arguments, return values, and recursion.

```aegis
func square(val) {
    return val * val
}

print square(5) // Output: 25
```

### Recursion

Example with the Fibonacci sequence:

```aegis
func fib(n) {
    if (n < 2) { return n }
    return fib(n - 1) + fib(n - 2)
}
```

---

## 5️⃣ Object-Oriented Programming (OOP)

Aegis supports classes, inheritance, and encapsulation via instances.

### Class Definition

The constructor is implicit: class parameters define the instance attributes directly.

```aegis
class Animal(name) {
    speak() {
        print this.name + " makes a noise."
    }
}
```

### Instantiation

Use the `new` keyword.

```aegis
var cat = new Animal("Felix")
```

### Inheritance (extends)

A class can inherit methods and attributes from another class.

```aegis
class Dog(name) extends Animal {
    // Override or new method
    speak() {
        print this.name + " barks!"
    }
}
```

### Member Access (this and .)

Inside a class, use `this.variable` to access or modify an attribute.

Outside, use `object.variable` or `object.method()`.

```aegis
var doggo = new Dog("Rex")
doggo.speak()        // Method call
print doggo.name     // Direct attribute access

// Attribute modification
doggo.name = "Brutus"
```

---

## 6️⃣ Lists

Lists are defined using brackets `[]`.

```aegis
var my_list = [10, 20, 30]
print my_list
```

---

## 7️⃣ Standard Library (Built-ins)

These functions are native and directly available in the language.
| Function | Description | Example |
| :--- | :--- | :--- |
| len(obj) | Returns the length of a list or string. | `len([1,2]) → 2` |
| to_int(str) | Parses a string into an integer. | `to_int("42") → 42` |
| str(val) | Explicitly converts a value to a string. | `str(123) → "123"` |
| at(list, i) | (Internal) Access element at index i. | `at(list, 0)` |

---

## 8️⃣ Script Examples
### Factorial (Recursive)

```aegis
func fact(n) {
    if (n < 2) { return 1 }
    return n * fact(n - 1)
}
print fact(5) // 120
```

### Birth Year Calculation

```aegis
input age_str "How old are you? "
var year = 2025 - to_int(age_str)
print "You were born in: " + year
```

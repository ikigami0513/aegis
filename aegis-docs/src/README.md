# The Aegis Programming Language

**Aegis** is a dynamic, high-performance scripting language designed for modern systems automation, game scripting, and rapid prototyping. 

Built in **Rust**, it features a lightning-fast Bytecode Virtual Machine (VM), a gradual typing system, and a rich standard library "batteries included."

![Aegis Version](https://img.shields.io/badge/version-v0.2.0-blue.svg)
![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)

---

## ⚡ Why Aegis?

### 🚀 High Performance
Powered by a custom **Stack-Based Virtual Machine**, Aegis v0.2.0 is exponentially faster than its predecessors. It handles heavy recursion and complex algorithms with ease (benchmarked at ~250ms for Fib30, rivaling optimized dynamic runtimes).

### 🛡️ Robust & Safe
Aegis combines the flexibility of dynamic typing with the safety of **Gradual Typing**. You can prototype fast using `var x = 10`, then secure your critical code with `var x: int = 10`.
Plus, the robust `try/catch` mechanism ensures your scripts handle errors gracefully.

### 📦 Modular & Modern
* **Functional**: First-class support for Lambdas, Closures, `map`, and `filter`.
* **Object-Oriented**: Clean Class syntax with methods and state.
* **Organized**: Native support for **Namespaces** and **Imports**.

### 🔋 Batteries Included
Aegis comes with a comprehensive Standard Library out of the box:
* **HTTP Client** for web interactions.
* **JSON** parsing and serialization.
* **File System** manipulation.
* **Regex** support.
* **SQLite** integration (via plugins).

---

## A Taste of Aegis

Here is a glimpse of what Aegis code looks like:

```aegis
import "stdlib/http.aeg"
import "stdlib/json.aeg"

// A Class representing a User
class User(name, id) {
    info() {
        return "User: " + this.name + " (ID: " + this.id + ")"
    }
}

// Fetching data from an API
try {
    print "Fetching data..."
    var response = Http.get("https://jsonplaceholder.typicode.com/users/1")
    var data = Json.parse(response)

    // Creating an object
    var user = new User(data.get("name"), data.get("id"))
    
    print "✅ Success!"
    print user.info()

} catch (e) {
    print "❌ Error: " + e
}
```

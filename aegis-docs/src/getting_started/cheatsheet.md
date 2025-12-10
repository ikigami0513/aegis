# Aegis Cheat Sheet

A quick reference guide for the Aegis syntax (v0.2.0).

## Variables & Types

```aegis
var name = "Aegis"        // Dynamic
var count: int = 42       // Typed (Gradual)
var pi: float = 3.14
var is_live = true
var nothing = null
```

## Collections

```aegis
// List
var list = [1, 2, 3]
list.push(4)
var item = list.at(0)

// Dict
var user = { id: 1, name: "Admin" }
var id = user.get("id")
```

## Control Flow

```
if (x > 10) { ... } else { ... }

while (running) { ... }

for (i, 0, 10, 1) { ... } // Start (inc), End (exc), Step

switch (val) {
    case 1: print "One"
    default: print "Other"
}
```

## Functions

```aegis
// Named
func add(a, b) { return a + b }

// Lambda
var mult = func(a, b) { return a * b }

// Decorator
@logger
func action() { ... }
```

## Classes

```aegis
class Hero(name, hp) {
    heal(amount) {
        this.hp = this.hp + amount
    }
}

var h = new Hero("Link", 100)
h.heal(20)
```

## Modules

```aegis
// File: lib.aeg
namespace Lib {
    var version = "1.0"
}

// File: main.aeg
import "lib.aeg"
print Lib.version
```

## Error Handling

```aegis
try {
    throw "Oops"
} catch (e) {
    print e
}
```

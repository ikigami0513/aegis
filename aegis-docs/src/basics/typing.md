# Gradual Typing

While dynamic typing is great for prototyping, large applications often require more safety. Aegis offers **Gradual Typing**, allowing you to enforce types on specific variables or function arguments.

## Syntax

To enforce a type, add a colon `:` followed by the type name after the variable name.

```aegis
// This variable MUST be an integer
var count: int = 0

// This works fine
count = 10 

// This throws a Runtime Error:
// count = "ten" 
```

## Supported Type Keywords
| Keyword | Matches |
|--- |--- |
| `int` | Integers |
| `float` | Floating point numbers |
| `string` | Strings |
| `bool` | Booleans |
| `list` | Lists (Arrays) |
| `dict` | Dictionaries (Maps) |
| `func` | Functions |

## Typing in Functions

Gradual typing is particularly powerful in function signatures to ensure arguments and return values are correct.

```aegis
// Arguments must be floats, return value must be a float
func divide(a: float, b: float) -> float {
    return a / b
}

var res = divide(10.0, 2.0) // Works
// var err = divide(10, 2)  // Throws Error: Expected 'float', got '10' (int)
```

*Note: Type checks happen at Runtime. If a type mismatch occurs, the Virtual Machine throws an exception that can be caught with try/catch.*
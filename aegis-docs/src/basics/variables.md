# Variables &amp; Data Types

Aegis is a **dynamically typed** language. This means you do not need to specify the type of a variable when you declare it, and a variable can hold values of different types throughout its life (unless you use Gradual Typing, see the next chapter).

## Declaration

To declare a variable, use the `var` keyword:

```aegis
var username = "Admin"
var score = 100
var is_active = true
```

## Reassignment

You can change the value of a variable at any time:

```aegis
var data = 42
print data // 42

data = "Changed to string"
print data // Changed to string
```

## Primitive Types

Aegis supports the following primitive data types:

| Type | Description | Example |
|--- |--- |--- |
| Null | Represents the absence of value. | `null` |
| Boolean | Logical true or false. | `true`, `false` |
| Integer | 64-bit signed integer. | `42`, `-10`, `0` |
| Float | 64-bit floating point number. | `3.14`, `-0.01` |
| String | UTF-8 text sequence. | `"Hello World"` |

*Note: Lists and Dictionaries are complex types and are covered in the Data Structures section.*

## String Interpolation

You can inject variables directly into strings using the ${} syntax. This converts the value to a string automatically.

```aegis
var name = "Aegis"
var version = 0.2

print "Welcome to ${name} version ${version}!"
// Output: Welcome to Aegis version 0.2!
```
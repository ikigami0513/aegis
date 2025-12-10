# Strings

Strings in Aegis are immutable sequences of UTF-8 characters. While they act like primitive types, they possess several built-in methods for manipulation.

## Basic Operations

You can concatenate strings using the `+` operator.

```aegis
var first = "Hello"
var second = "World"
print first + " " + second // "Hello World"
```

## Methods
| Method | Description |
|--- |--- |
| .len() | Returns the length of the string. |
| .trim() | Removes whitespace from both ends. |
| .replace(old, new) | Replaces all occurrences of a substring. |
| .split(delimiter) | Splits the string into a List of substrings. |

### Examples
Cleaning Input

```aegis
var input = "   user@example.com   "
var clean = input.trim()
print clean // "user@example.com"
```

Replacement

```aegis
var text = "Hello World"
print text.replace("World", "Aegis") // "Hello Aegis"
```

Parsing CSV-like data

```aegis
var csv = "apple,banana,orange"
var items = csv.split(",")

print items.len()   // 3
print items.at(0)   // "apple"
```

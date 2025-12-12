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
| Method | Description | Example |
| :--- | :--- | :--- |
| `.len()` | Returns the length of the string. | `"Hi".len()` (2) |
| `.at(index)` | Returns the character at the specified index (or `null`). | `"Abc".at(1)` ("b") |
| `.trim()` | Removes whitespace from both ends. | `" a ".trim()` ("a") |
| `.upper()` | Converts the entire string to uppercase. | `"aegis".upper()` ("AEGIS") |
| `.lower()` | Converts the entire string to lowercase. | `"AEGIS".lower()` ("aegis") |
| `.contains(sub)` | Returns `true` if the string contains the substring. | `"Hello".contains("el")` |
| `.starts_with(sub)` | Returns `true` if the string starts with the substring. | `"file.txt".starts_with("file")` |
| `.ends_with(sub)` | Returns `true` if the string ends with the substring. | `"image.png".ends_with(".png")` |
| `.replace(old, new)` | Replaces all occurrences of a substring. | `"a-b-c".replace("-", ".")` |
| `.split(delim)` | Splits the string into a List of substrings. | `"a,b".split(",")` |
| `.is_empty()` | Returns `true` if the string length is 0. | `"".is_empty()` |
| `.pad_start(len, char)` | Pads the start with `char` (default " ") until length is reached. | `"1".pad_start(3, "0")` ("001") |
| `.pad_end(len, char)` | Pads the end with `char` (default " ") until length is reached. | `"Hi".pad_end(5, "!")` ("Hi!!!") |

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

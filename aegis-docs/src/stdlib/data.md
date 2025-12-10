# Data Handling

Aegis provides robust tools for data serialization and pattern matching.

## JSON

Import: `import "stdlib/json.aeg"`

| Function | Description |
| :--- | :--- |
| `Json.parse(str)` | Parses a JSON string into Aegis Lists/Dicts. |
| `Json.stringify(val)` | Converts an Aegis value into a JSON string. |

## Regex

Import: `import "stdlib/regex.aeg"`

```aegis
var re = Regex.new("^[0-9]+$")
var is_digit = Regex.test(re, "12345") // true
```

| Function | Description |
| :--- | :--- |
|Regex.new(pattern) | Compiles a regex pattern. Returns an ID. |
| Regex.test(id, str) | Returns true if the string matches. |
| Regex.replace(id, str, repl) | Replaces matches with the replacement string. |


## Crypto & Encoding

Import: `import "stdlib/crypto.aeg"`

| Function | Description |
| :--- | :--- |
|Base64.encode(str) | Encodes a string to Base64. |
| Base64.decode(str) | Decodes a Base64 string. |
| Hash.sha256(str) | Computes the SHA-256 hash (hex string). |

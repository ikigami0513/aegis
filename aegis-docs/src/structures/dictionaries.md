# Dictionaries

Dictionaries (or Maps) are collections of key-value pairs. They are useful for storing structured data or representing objects.

## Creation

Use curly braces `{}`. Keys must be strings (or identifiers which are converted to strings).

```aegis
var config = {
    host: "localhost",
    port: 8080,
    debug: true
}
```

## Operations

| Method | Description | Example |
|--- |--- |--- |
| .get(key) | Returns the value associated with the key. | dict.get("host") |
| .insert(key, val) | Adds or updates a key-value pair. | dict.insert("ssl", true) |
| .keys() | Returns a List of all keys in the dictionary. | dict.keys() |
| .len() | Returns the number of entries. | dict.len() |

### Example

```aegis
var user = {}

// Adding data
user.insert("name", "Arthur")
user.insert("level", 5)

// Retrieving data
print "User: " + user.get("name")

// Listing keys
var fields = user.keys()
print fields // ["name", "level"]
```

*Note: Accessing a non-existent key with .get() returns null.*

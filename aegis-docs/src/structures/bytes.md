# Bytes

The `Bytes` type represents a raw sequence of bytes (`u8`). It is essential for handling binary data such as images, files, or network protocols where text encoding (UTF-8) is not appropriate.

Unlike Strings, Bytes are mutable buffers.

## Creating Bytes

You can create a Bytes object in several ways:

1.  **From a String:** Using the `.to_bytes()` method.
2.  **From a File:** Using `File.read_bytes(path)`.
3.  **From a Socket:** Using `sock_read_bytes(id, size)`.

```aegis
// From String
var data = "Hello".to_bytes()

// From File (Binary mode)
var image = File.read_bytes("logo.png")
```

## Methods
| Method | Description | Example |
|--- |--- |--- |
| `.len()` | Returns the number of bytes in the buffer. | `data.len()` (5) |
| `.is_empty()` | Returns `true` if the buffer size is 0. | `data.is_empty()` |
| `.at(index)` | Returns the byte value (0-255) at the specified index. | `data.at(0)` (72 for 'H') |
| `.slice(start, end)` | Returns a new Bytes object from start to end (exclusive). | `data.slice(0, 2)` |
| `.to_string()` | Tries to convert the bytes back to a UTF-8 String. | `data.to_string()` ("Hello") |
| `.to_hex()` | Returns a hexadecimal string representation. | `data.to_hex()` ("48656C6C6F") |

## Example Usage

### Inspecting Binary Data

```aegis
var b = "Aegis".to_bytes()

print b.len()      // 5
print b.to_hex()   // "4165676973"
print b.at(0)      // 65 (ASCII for 'A')
```

### Reading an Image

When reading non-text files, always use `read_bytes` to avoid data corruption.

```aegis
try {
    var img = File.read_bytes("assets/icon.png")
    print "Image size: " + img.len() + " bytes"
    
    // You can then send this 'img' object directly via Socket.write()
} catch (e) {
    print "Error reading image: " + e
}
```

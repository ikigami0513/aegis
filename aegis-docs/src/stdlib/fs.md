# File System (IO)

The File System module allows reading from and writing to the disk.

## File Operations

Import: `import "stdlib/file.aeg"`

| Function | Description |
| :--- | :--- |
| `File.read(path)` | Reads the entire file content as a string. Throws if failed. |
| `File.write(path, content)` | Writes string content to a file (overwrites). |
| `File.exists(path)` | Returns `true` if the file or directory exists. |

### Example

```aegis
var config_path = "settings.ini"

if (File.exists(config_path)) {
    var data = File.read(config_path)
    print "Loaded: " + data
} else {
    File.write(config_path, "defaults")
}
```

## Path Manipulation

Import: `import "stdlib/path.aeg"`

It is highly recommended to use `Path` functions instead of concatenating strings manually, to ensure cross-platform compatibility (Windows vs Linux).

| Function | Description |
| :--- | :--- |
| `Path.join(a, b)` | Joins two path segments (e.g., `dir/file.txt`). |
| `Path.extension(path)` | Returns the file extension (e.g., `txt`). |
| `Path.exists(path)` | Alias for File.exists. |

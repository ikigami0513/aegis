# File Processing Recipes

This page covers common patterns for working with the file system.

**Prerequisites:**
```aegis
import "stdlib/file.aeg"
import "stdlib/path.aeg" // Optional, for path manipulation
```

## Reading a File Line-by-Line

Since `File.read` loads the entire content into memory, you can use `split` to iterate over lines.

```aegis
var content = File.read("logs/server.log")
var lines = content.split("\n")

lines.for_each(func(line) {
    // Skip empty lines
    if (line.trim().len() > 0) {
        print "Processing: " + line
    }
})
```

## Parsing a CSV String

Aegis handles string manipulation efficiently. Here is a helper function to turn CSV text into a List of Lists.

```aegis
func parse_csv(data) {
    var rows = data.split("\n")
    
    // Use map to transform ["a,b", "c,d"] into [["a","b"], ["c","d"]]
    return rows.map(func(row) {
        return row.split(",")
    })
}

// Example usage
var csv_data = File.read("data.csv") 
// Assume content is: name,age\nAlice,30\nBob,25

var table = parse_csv(csv_data)

// Accessing Bob's age (Row 2, Column 1)
// Note: Row 0 is the header
print "Age of Bob: " + table.at(2).at(1)
```

## Saving and Loading Configuration (JSON)

Combining the `File` module with the `Json` module is perfect for managing application settings.

```aegis
import "stdlib/json.aeg"

var config_path = "settings.json"

// 1. Create default config
var config = {
    theme: "dark",
    volume: 80,
    username: "Guest"
}

// 2. Save to disk
File.write(config_path, Json.stringify(config))
print "Configuration saved."

// 3. Load back
if (File.exists(config_path)) {
    var loaded_raw = File.read(config_path)
    var loaded_config = Json.parse(loaded_raw)
    
    print "Welcome back, " + loaded_config.get("username")
}
```

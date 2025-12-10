# System Automation Recipes

Aegis is great for replacing complex Bash scripts or Python glue code.

**Prerequisites:**
```aegis
import "stdlib/system.aeg"
import "stdlib/file.aeg"
import "stdlib/date.aeg"
import "stdlib/path.aeg"
```

## Daily Backup Script

This script copies a file and appends the current date to its name.

```aegis
var source = "database.db"
var timestamp = Date.format("%Y-%m-%d") // e.g., 2023-10-25
var dest = "backups/database_" + timestamp + ".db"

if (File.exists(source)) {
    print "backing up " + source + " to " + dest + "..."
    
    // Read source
    var content = File.read(source)
    
    // Ensure directory exists (basic check)
    // Ideally, use Process.run("mkdir -p backups")
    
    // Write backup
    File.write(dest, content)
    print "✅ Backup complete."
} else {
    print "❌ Source file not found!"
}
```

## Environment Checker

Check if required environment variables are set before starting an app.

```aegis
var required_vars = ["API_KEY", "DB_HOST", "PORT"]
var missing = 0

required_vars.for_each(func(key) {
    var val = System.env(key)
    if (val == null) {
        print "❌ Missing ENV: " + key
        missing = missing + 1
    } else {
        print "✅ " + key + " is set."
    }
})

if (missing > 0) {
    System.fail("Environment is not configured correctly.")
}

print "Starting Application..."
```

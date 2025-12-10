# System &amp; Process

This module provides tools to interact with the operating system, environment variables, time, and external processes.

## System

Import: `import "stdlib/system.aeg"`

| Function | Description |
| :--- | :--- |
| `System.args()` | Returns a List of command-line arguments passed to the script. |
| `System.env(key)` | Returns the value of an environment variable (or `null`). |
| `System.clear()` | Clears the console screen. |
| `System.fail(msg)` | Exits the program immediately with an error message. |

## Time

Import: `import "stdlib/time.aeg"`

| Function | Description |
| :--- | :--- |
| `Time.sleep(ms)` | Pauses execution for the specified milliseconds. |
| `Time.now()` | Returns the current system timestamp (integer). |

## Date

Import: `import "stdlib/date.aeg"`

| Function | Description |
| :--- | :--- |
| `Date.now()` | Returns the current date as an ISO 8601 string. |
| `Date.format(fmt)` | Returns the current date formatted (e.g., `"%Y-%m-%d"`). |

## Process

Import: `import "stdlib/process.aeg"`

Allows you to run shell commands.

```aegis
var output = Process.exec("git", ["status"])

print "Exit Code: " + output.get("code")
print "Stdout: " + output.get("stdout")
```

| Function | Description |
| :--- | :--- |
| `Process.exec(cmd, args)` | Runs a command and returns a Dict {code, stdout, stderr}. |
| `Process.run(cmd)` | Helper that prints stdout and returns true if successful. |
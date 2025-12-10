# Package Manager (apm)
Modern development relies heavily on reusing existing code and libraries. Aegis comes with a built-in Package Manager (often referred to as APM) to manage your project's dependencies, ensuring you can easily install, update, and publish modules.


## The Project Manifest: aegis.toml

Every Aegis project starts with a manifest file named `aegis.toml` at the root of your directory. This file describes your project and lists the external packages it needs.

Here is an example of a typical `aegis.toml` file:

```Ini, TOML
[package]
name = "my_rpg_game"
version = "0.1.0"
authors = ["You <you@example.com>"]

[dependencies]
glfw = "1.0.0"
sqlite = "0.2.0"
http = "*"
```

- `[package]`: Metadata about your project.

- `[dependencies]`: A list of packages to install from the Aegis Registry.

## Managing Dependencies

### Adding a Package

To add a new library to your project, use the `add` command. This will download the package and automatically add it to your `aegis.toml` file.

```bash
aegis add glfw
```

You can also specify a version:

```bash
aegis add sqlite 1.0.4
```

### Installation Folder

When you add a dependency, Aegis downloads the files into a `packages/` directory at the root of your project.

Project Structure:

```
my_project/
├── aegis.toml       <-- Manifest
├── src/
│   └── main.aeg     <-- Your code
└── packages/        <-- Managed by APM
    ├── glfw/
    └── sqlite/
```

*Note: You should generally add `packages/` to your `.gitignore` file, similar to `node_modules` in JavaScript or `target` in Rust.*

### Using Installed Packages

Once a package is installed, you can use it in your scripts using the `import` statement. Since packages are stored locally in the `packages` folder, the path is straightforward.

```aegis
// Importing the GLFW library installed via APM
import "packages/glfw/glfw.aeg"

// Importing SQLite
import "packages/sqlite/sqlite.aeg"

func main() {
    // Use the namespace defined in the package
    var db = Sqlite.open("game.db")
    print "Database opened!"
}

main()
```

## Publishing a Package

If you have created a library and want to share it with the world, APM makes it easy.
### 1. Authentication

First, you need to authenticate with the Aegis Registry using your token.

```bash
aegis login <your-api-token>
```

### 2. Publishing

Ensure your `aegis.toml` is correctly configured with a unique name and version, then run:

```
aegis publish
```

This will upload your code (excluding ignored files) to the registry, making it available for everyone to `aegis add`.

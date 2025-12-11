# Classes &amp; Instances

Object-Oriented Programming allows you to structure your code by bundling data (attributes) and behavior (methods) into reusable blueprints called **Classes**.

## Defining a Class

Use the `class` keyword followed by the class name and a block `{ ... }`.

Unlike the previous version, Aegis v0.2.1 uses an explicit **Initializer method** named `init`. This method is automatically called when you create a new instance.

```aegis
class User {
    // The initializer (Constructor)
    init(name, email) {
        // Assign parameters to the instance fields using 'this'
        this.name = name
        this.email = email
        this.active = true
        
        print "Creating user: " + this.name
    }
    
    // Other methods...
}
```

## Creating Instances

To create an object (an instance of a class), use the `new` keyword. You pass the arguments expected by your `init` method here.

```aegis
var admin = new User("Alice", "alice@example.com")
var guest = new User("Bob", "bob@example.com")
```

## Fields (Attributes)

Aegis objects are dynamic. You typically define your fields inside `init`, but you can access, modify, or add new fields at any time using the dot notation.

```aegis
print admin.name // "Alice"

// Modifying a field
admin.name = "SuperAlice"

// Adding a new field dynamically
admin.role = "Administrator"
print admin.role // "Administrator"
```

# Classes &amp; Instances

Object-Oriented Programming allows you to structure your code by bundling data (attributes) and behavior (methods) into reusable blueprints called **Classes**.

## Defining a Class

Use the `class` keyword. The parameters defined in the parentheses act as the **Constructor arguments**.

```aegis
// Defines a 'User' class with two properties: name and email
class User(name, email) {
    // The body of the class is executed when 'new' is called.
    
    // You can define methods here
    print "Creating user: " + name
}
```

## Creating Instances

To create an object (an instance of a class), use the `new` keyword.

```aegis
var admin = new User("Alice", "alice@example.com")
var guest = new User("Bob", "bob@example.com")
```

## Fields (Attributes)

Aegis objects are dynamic. You can access and assign fields using the dot notation. The constructor parameters are automatically available as fields.

```aegis
print admin.name // "Alice"

// Modifying a field
admin.name = "SuperAlice"

// Adding a new field dynamically
admin.role = "Administrator"
print admin.role // "Administrator"
```

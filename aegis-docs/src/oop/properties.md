# Properties

Properties allow you to expose class members like fields (variables), but define custom logic for reading and writing them using **Getters** and **Setters**.

## Defining a Property

Use the `prop` keyword. A property can have a `get` block, a `set` block, or both.

```aegis
class Circle {
    private _radius = 0

    // Public interface usually uses PascalCase or snake_case depending on style
    prop radius {
        get { 
            return this._radius 
        }
        set(value) { 
            if (value < 0) {
                print "Radius cannot be negative"
            } else {
                this._radius = value
            }
        }
    }
}

var c = new Circle()
c.radius = 10      // Calls the setter
print c.radius     // Calls the getter (10)
```

## Computed Properties (Read-Only)

If you define only a `get` block, the property becomes **Read-Only**. This is perfect for calculated values.

```aegis
class User {
    init(first, last) {
        this.first = first
        this.last = last
    }

    prop full_name {
        get { return this.first + " " + this.last }
    }
}

var u = new User("John", "Doe")
print u.full_name // "John Doe"
// u.full_name = "Jane" // Error: Property 'full_name' is write-only/read-only mismatch logic
```

## Static Properties

Properties can also be `static`. They apply to the class itself.

```aegis
class Settings {
    private static _theme = "Dark"

    static prop Theme {
        get { return Settings._theme }
        set(v) { 
            print "Changing theme to " + v
            Settings._theme = v 
        }
    }
}

Settings.Theme = "Light"
```

## Encapsulation

Properties respect visibility modifiers (`public`, `private`, `protected`).
- `private prop x`: Only accessible within the class.
- `public prop y`: Accessible from anywhere.

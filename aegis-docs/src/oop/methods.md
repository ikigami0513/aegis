# Methods &amp; this

Methods are functions defined inside a class. They define what an object can *do*.

## Defining Methods



Inside a class block, define functions as usual.

```aegis
class Rectangle(width, height) {
    
    func area() {
        return this.width * this.height
    }
    
    func scale(factor) {
        this.width = this.width * factor
        this.height = this.height * factor
    }
}
```

## The this Keyword

Inside a method, the special variable `this` refers to the current instance calling the method. It allows you to access or modify the object's properties.

*Important: To access a property inside a class, you must use `this.property`. Using just property would look for a local variable.*

Usage Example

```aegis
var rect = new Rectangle(10, 20)

print "Area: " + rect.area() // 200

rect.scale(2)
print "New Width: " + rect.width // 20
print "New Area: " + rect.area() // 800
```

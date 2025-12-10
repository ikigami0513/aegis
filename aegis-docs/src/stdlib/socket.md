# Socket (TCP Networking)

The Socket module provides low-level access to TCP networking. Unlike the `Http` module which is designed for high-level requests, `Socket` allows you to create your own servers or implement custom protocols.

**Import:** `import "stdlib/socket.aeg"`

## Server Functions

To accept incoming connections, you must first create a listener.

| Function | Description |
| :--- | :--- |
| `Socket.listen(host, port)` | Binds a TCP listener to the address. Returns a **Server ID**. |
| `Socket.accept(server_id)` | **Blocks** execution until a client connects. Returns a **Client ID**. |

## Client Functions

To connect to a remote server.

| Function | Description |
| :--- | :--- |
| `Socket.connect(host, port)` | Establishes a connection to a remote address. Returns a **Client ID**. |

## Data Transmission

These functions work with **Client IDs** (returned by `accept` or `connect`).

| Function | Description |
| :--- | :--- |
| `Socket.read(id, size)` | Reads up to `size` bytes from the stream. Returns a String. |
| `Socket.write(id, data)` | Writes the string `data` to the stream. |
| `Socket.close(id)` | Closes the connection (or the listener). |

---

## Example 1: Simple Echo Server

A basic server that listens on port 9000, reads a message, prints it, and closes.

```aegis
import "stdlib/socket.aeg"

var server = Socket.listen("127.0.0.1", 9000)
print "Listening on 9000..."

while (true) {
    // 1. Wait for connection
    var client = Socket.accept(server)
    print "Client connected!"

    // 2. Read data (max 128 bytes)
    var msg = Socket.read(client, 128)
    print "Received: " + msg

    // 3. Send response and close
    Socket.write(client, "Message received.")
    Socket.close(client)
}
```

## Example 2: Building a Web Server

Since Aegis handles strings efficiently, you can implement a basic HTTP server in just a few lines of code.

```aegis
import "stdlib/socket.aeg"
import "stdlib/time.aeg"

var HOST = "127.0.0.1"
var PORT = 8080

var server = Socket.listen(HOST, PORT)
print "HTTP Server running on http://" + HOST + ":" + PORT

while (true) {
    var client = Socket.accept(server)
    
    // Read the HTTP Request
    var request = Socket.read(client, 1024)
    
    // Simple parsing to get the path (e.g., "GET /home HTTP/1.1")
    var parts = request.split(" ")
    if (parts.len() > 1) {
        var method = parts.at(0)
        var path = parts.at(1)
        print method + " " + path
    
        // Build HTTP Response Body
        var body = "<h1>Hello from Aegis!</h1>"
        body += "<p>You requested: " + path + "</p>"
        
        // Build Headers
        var response = "HTTP/1.1 200 OK\r\n"
        response += "Content-Type: text/html; charset=utf-8\r\n"
        response += "Content-Length: " + body.len() + "\r\n"
        response += "Connection: close\r\n"
        response += "\r\n" // End of headers
        
        // Combine
        response += body
    
        Socket.write(client, response)
    }
    
    // Close the connection
    Socket.close(client)
}
```

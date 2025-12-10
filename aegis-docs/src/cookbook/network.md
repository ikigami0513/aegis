# Networking Recipes

Aegis is excellent for writing "glue code" that connects different web services. These recipes use the `Http` module.

**Prerequisites:**
```aegis
import "stdlib/http.aeg"
import "stdlib/json.aeg"
```

## Fetching JSON Data (GET)

This is the most common task: getting data from a REST API.

```aegis
var url = "https://jsonplaceholder.typicode.com/todos/1"

try {
    print "Fetching..."
    var response = Http.get(url)
    
    // Convert string response to Dict
    var todo = Json.parse(response)
    
    print "Task ID: " + todo.get("id")
    print "Title:   " + todo.get("title")
    print "Done:    " + todo.get("completed")
    
} catch (e) {
    print "Request failed: " + e
}
```

## Sending Data (POST)

To send data to a server, you typically need to stringify your payload first.

```aegis
var url = "https://jsonplaceholder.typicode.com/posts"

// 1. Prepare data
var payload = {
    title: "Aegis Language",
    body: "Aegis is a cool new language.",
    userId: 1
}

// 2. Send request
try {
    var json_payload = Json.stringify(payload)
    var response = Http.post(url, json_payload)
    
    print "Server responded: " + response
} catch (e) {
    print "Post failed: " + e
}
```

## Simple Health Check Monitor

A script to check if a website is up.

```aegis
func check_status(site) {
    try {
        var res = Http.get(site)
        // If Http.get returns without throwing, the site is reachable
        print "[OK] " + site
        return true
    } catch (e) {
        print "[DOWN] " + site + " (" + e + ")"
        return false
    }
}

var sites = [
    "https://google.com",
    "https://github.com",
    "https://invalid-url-example.com"
]

print "--- Starting Health Check ---"
sites.for_each(func(url) {
    check_status(url)
})
```
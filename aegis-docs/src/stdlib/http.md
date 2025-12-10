# Network (HTTP)

Aegis includes a lightweight HTTP client for interacting with web APIs.

Import: `import "stdlib/http.aeg"`

## Methods

### Http GET
Syntax: `Http.get(url)`

Performs a GET request.
* **Returns**: The response body as a String.
* **Throws**: An error if the connection fails or status is not 2xx.

### Http Post
Syntax: `Http.post(url, body)`

Performs a POST request.
* **body**: String (payload).

## Example: Fetching an API

```aegis
import "stdlib/http.aeg"

try {
    var response = Http.get("https://api.github.com/zen")
    print "GitHub Zen: " + response
} catch (e) {
    print "Network Error: " + e
}
```

// ExpressKS Demo

import expressks
import std.collections.dictionary

func main() {
    let app = expressks.createApp()
    
    // Basic route
    app.get(path: "/") { (req) in
        return expressks.Html("<h1>Hello Kestrel!</h1>")
    }
    
    // JSON route
    app.get(path: "/api/status") { (req) in
        var status = Dictionary[String, String]()
        status.insert(value: "ok", for: "status")
        status.insert(value: "1.0", for: "version")
        
        return expressks.Json(status)
    }
    
    // Post route
    app.post(path: "/submit") { (req) in
        return expressks.Text("Received")
    }
    
    app.listen(port: 8080)
}


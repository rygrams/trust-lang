/// `use` statements injected when `import ... from "trusty:http"` is detected.
pub fn use_statements() -> Vec<&'static str> {
    vec![
        "use std::collections::HashMap;",
        "use std::time::Duration;",
        "use std::sync::{Arc, Mutex};",
        "use std::io::Read;",
        "use serde_json::Value;",
        "use tiny_http::{Header, Response as TinyResponse, Server as TinyServer, StatusCode};",
        r#"#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct HttpRequestOptions {
    pub method: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub timeoutMs: i32,
}

#[allow(non_snake_case)]
pub fn requestOptions() -> HttpRequestOptions {
    HttpRequestOptions {
        method: "GET".to_string(),
        headers: HashMap::new(),
        body: String::new(),
        timeoutMs: 30_000,
    }
}

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct HttpResponse {
    pub status: i32,
    pub ok: bool,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub error: String,
}

#[allow(non_snake_case)]
impl HttpResponse {
    pub fn text(&self) -> String {
        self.body.clone()
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }

    pub fn jsonAs<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_str(&self.body).ok()
    }

    pub fn header(&self, name: String) -> String {
        self.headers.get(&name).cloned().unwrap_or_default()
    }
}

#[allow(non_snake_case)]
pub fn fetch(url: String) -> HttpResponse {
    fetchWith(url, requestOptions())
}

#[allow(non_snake_case)]
pub fn fetchWith(url: String, options: HttpRequestOptions) -> HttpResponse {
    let timeout_ms = if options.timeoutMs <= 0 { 30_000 } else { options.timeoutMs as u64 };
    let config = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_millis(timeout_ms)))
        .build();
    let agent: ureq::Agent = config.into();

    let method = if options.method.trim().is_empty() {
        "GET".to_string()
    } else {
        options.method.to_uppercase()
    };

    let sent: Result<ureq::http::Response<ureq::Body>, ureq::Error> = match method.as_str() {
        "GET" => {
            let mut req = agent.get(&url);
            for (k, v) in options.headers.iter() {
                req = req.header(k, v);
            }
            req.call()
        }
        "POST" => {
            let mut req = agent.post(&url);
            for (k, v) in options.headers.iter() {
                req = req.header(k, v);
            }
            if options.body.is_empty() {
                req.send_empty()
            } else {
                req.send(options.body.clone())
            }
        }
        "PUT" => {
            let mut req = agent.put(&url);
            for (k, v) in options.headers.iter() {
                req = req.header(k, v);
            }
            if options.body.is_empty() {
                req.send_empty()
            } else {
                req.send(options.body.clone())
            }
        }
        "PATCH" => {
            let mut req = agent.patch(&url);
            for (k, v) in options.headers.iter() {
                req = req.header(k, v);
            }
            if options.body.is_empty() {
                req.send_empty()
            } else {
                req.send(options.body.clone())
            }
        }
        "DELETE" => {
            let mut req = agent.delete(&url);
            for (k, v) in options.headers.iter() {
                req = req.header(k, v);
            }
            req.call()
        }
        _ => {
            return HttpResponse {
                status: 0,
                ok: false,
                body: String::new(),
                headers: HashMap::new(),
                error: format!("unsupported HTTP method: {}", method),
            };
        }
    };

    match sent {
        Ok(mut resp) => {
            let status = resp.status().as_u16() as i32;
            let ok = status >= 200 && status < 300;
            let mut headers = HashMap::new();
            for (name, value) in resp.headers().iter() {
                let header_name = name.as_str().to_string();
                let header_value = value.to_str().unwrap_or("").to_string();
                headers.insert(header_name, header_value);
            }
            let body = resp.body_mut().read_to_string().unwrap_or_default();
            HttpResponse {
                status,
                ok,
                body,
                headers,
                error: String::new(),
            }
        }
        Err(e) => HttpResponse {
            status: 0,
            ok: false,
            body: String::new(),
            headers: HashMap::new(),
            error: e.to_string(),
        },
    }
}

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Params {
    pub values: HashMap<String, String>,
}

#[allow(non_snake_case)]
impl Params {
    pub fn new() -> Params {
        Params { values: HashMap::new() }
    }

    pub fn getOr(&self, key: String, fallback: String) -> String {
        self.values.get(&key).cloned().unwrap_or(fallback)
    }
}

#[derive(Debug, Clone, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Request {
    pub method: String,
    pub path: String,
    pub query: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub params: Params,
}

#[allow(non_snake_case)]
impl Request {
    pub fn text(&self) -> String {
        self.body.clone()
    }

    pub fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }

    pub fn jsonAs<T: serde::de::DeserializeOwned>(&self) -> Option<T> {
        serde_json::from_str(&self.body).ok()
    }

    pub fn header(&self, name: String) -> String {
        self.headers.get(&name).cloned().unwrap_or_default()
    }
}

#[derive(Debug, Clone)]
pub struct Response {
    status: Arc<Mutex<i32>>,
    headers: Arc<Mutex<HashMap<String, String>>>,
    body: Arc<Mutex<String>>,
}

#[allow(non_snake_case)]
impl Response {
    pub fn new() -> Response {
        Response {
            status: Arc::new(Mutex::new(200)),
            headers: Arc::new(Mutex::new(HashMap::new())),
            body: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn status(&self, code: i32) -> Response {
        if let Ok(mut s) = self.status.lock() {
            *s = code;
        }
        self.clone()
    }

    pub fn header(&self, name: String, value: String) -> Response {
        if let Ok(mut h) = self.headers.lock() {
            h.insert(name, value);
        }
        self.clone()
    }

    pub fn send(&self, body: String) -> Response {
        if let Ok(mut b) = self.body.lock() {
            *b = body;
        }
        self.clone()
    }

    pub fn json(&self, value: String) -> Response {
        let _ = self.header("Content-Type".to_string(), "application/json".to_string());
        self.send(value)
    }

    pub fn jsonValue(&self, value: Value) -> Response {
        let body = serde_json::to_string(&value).unwrap_or("null".to_string());
        self.json(body)
    }

    pub fn jsonText(&self, json: String) -> Response {
        let _ = self.header("Content-Type".to_string(), "application/json".to_string());
        self.send(json)
    }

    fn snapshot(&self) -> (i32, HashMap<String, String>, String) {
        let status = match self.status.lock() {
            Ok(s) => *s,
            Err(_) => 500,
        };
        let headers = match self.headers.lock() {
            Ok(h) => h.clone(),
            Err(_) => HashMap::new(),
        };
        let body = match self.body.lock() {
            Ok(b) => b.clone(),
            Err(_) => String::new(),
        };
        (status, headers, body)
    }
}

type RouteHandler = Arc<dyn Fn(Request, Response) + Send + Sync>;
type Middleware = Arc<dyn Fn(Request) -> Request + Send + Sync>;

#[derive(Clone)]
struct Route {
    method: String,
    pattern: String,
    handler: RouteHandler,
}

#[derive(Clone)]
pub struct HttpServer {
    routes: Arc<Mutex<Vec<Route>>>,
    middlewares: Arc<Mutex<Vec<Middleware>>>,
    lastError: Arc<Mutex<String>>,
}

#[allow(non_snake_case)]
impl HttpServer {
    pub fn create() -> HttpServer {
        HttpServer {
            routes: Arc::new(Mutex::new(Vec::new())),
            middlewares: Arc::new(Mutex::new(Vec::new())),
            lastError: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn addMiddleware<F>(&self, middleware: F)
    where
        F: Fn(Request) -> Request + Send + Sync + 'static,
    {
        if let Ok(mut mw) = self.middlewares.lock() {
            mw.push(Arc::new(middleware));
        }
    }

    pub fn get<F>(&self, pattern: String, handler: F)
    where
        F: Fn(Request, Response) + Send + Sync + 'static,
    {
        self.add_route("GET".to_string(), pattern, handler);
    }

    pub fn post<F>(&self, pattern: String, handler: F)
    where
        F: Fn(Request, Response) + Send + Sync + 'static,
    {
        self.add_route("POST".to_string(), pattern, handler);
    }

    pub fn put<F>(&self, pattern: String, handler: F)
    where
        F: Fn(Request, Response) + Send + Sync + 'static,
    {
        self.add_route("PUT".to_string(), pattern, handler);
    }

    pub fn delete<F>(&self, pattern: String, handler: F)
    where
        F: Fn(Request, Response) + Send + Sync + 'static,
    {
        self.add_route("DELETE".to_string(), pattern, handler);
    }

    pub fn listen(&self, port: i32) -> bool {
        self.listenOn(format!("0.0.0.0:{}", port))
    }

    pub fn listenOn(&self, bind: String) -> bool {
        if let Ok(mut last) = self.lastError.lock() {
            *last = String::new();
        }
        let server = match TinyServer::http(&bind) {
            Ok(s) => s,
            Err(e) => {
                if let Ok(mut last) = self.lastError.lock() {
                    *last = e.to_string();
                }
                return false;
            }
        };

        for mut incoming in server.incoming_requests() {
            let url = incoming.url().to_string();
            let (path, query) = split_path_query(&url);
            let method = incoming.method().as_str().to_string();

            let mut headers = HashMap::new();
            for h in incoming.headers() {
                headers.insert(h.field.to_string(), h.value.to_string());
            }

            let mut body = String::new();
            let _ = incoming.as_reader().read_to_string(&mut body);

            let mut selected: Option<(RouteHandler, Params)> = None;
            let routes = match self.routes.lock() {
                Ok(r) => r.clone(),
                Err(_) => Vec::new(),
            };

            for route in routes {
                if route.method != method {
                    continue;
                }
                if let Some(params) = match_route(&route.pattern, &path) {
                    selected = Some((route.handler.clone(), params));
                    break;
                }
            }

            match selected {
                Some((handler, params)) => {
                    let mut req = Request {
                        method: method.clone(),
                        path: path.clone(),
                        query: query.clone(),
                        headers,
                        body,
                        params,
                    };

                    let middlewares = match self.middlewares.lock() {
                        Ok(m) => m.clone(),
                        Err(_) => Vec::new(),
                    };
                    for middleware in middlewares {
                        req = middleware(req);
                    }

                    let res = Response::new();
                    handler(req, res.clone());

                    let (status, out_headers, out_body) = res.snapshot();
                    let status_u16 = if status < 100 || status > 599 {
                        500
                    } else {
                        status as u16
                    };
                    let mut tiny_resp = TinyResponse::from_string(out_body)
                        .with_status_code(StatusCode(status_u16));
                    for (k, v) in out_headers {
                        if let Ok(h) = Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                            tiny_resp = tiny_resp.with_header(h);
                        }
                    }
                    let _ = incoming.respond(tiny_resp);
                }
                None => {
                    let tiny_resp = TinyResponse::from_string("Not Found".to_string())
                        .with_status_code(StatusCode(404));
                    let _ = incoming.respond(tiny_resp);
                }
            }
        }

        true
    }

    pub fn lastError(&self) -> String {
        match self.lastError.lock() {
            Ok(v) => v.clone(),
            Err(_) => "unknown server error".to_string(),
        }
    }

    fn add_route<F>(&self, method: String, pattern: String, handler: F)
    where
        F: Fn(Request, Response) + Send + Sync + 'static,
    {
        if let Ok(mut routes) = self.routes.lock() {
            routes.push(Route {
                method,
                pattern,
                handler: Arc::new(handler),
            });
        }
    }
}

fn split_path_query(url: &str) -> (String, String) {
    if let Some((path, query)) = url.split_once('?') {
        (path.to_string(), query.to_string())
    } else {
        (url.to_string(), String::new())
    }
}

fn normalize_segments(path: &str) -> Vec<&str> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect::<Vec<_>>()
    }
}

fn match_route(pattern: &str, path: &str) -> Option<Params> {
    let p = normalize_segments(pattern);
    let r = normalize_segments(path);
    if p.len() != r.len() {
        return None;
    }
    let mut params = HashMap::new();
    for (pp, rr) in p.iter().zip(r.iter()) {
        if let Some(name) = pp.strip_prefix(':') {
            params.insert(name.to_string(), rr.to_string());
            continue;
        }
        if pp != rr {
            return None;
        }
    }
    Some(Params { values: params })
}"#,
    ]
}

/// External crates needed.
pub fn required_crates() -> Vec<(&'static str, &'static str)> {
    vec![
        ("serde", "1"),
        ("serde_derive", "1"),
        ("serde_json", "1"),
        ("ureq", "3"),
        ("tiny_http", "0.12"),
    ]
}

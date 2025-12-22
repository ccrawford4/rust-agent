use std::io::{self, prelude::*};
use std::net::{TcpListener, TcpStream};

#[derive(Debug)]
enum Method {
    GET,
    POST,
}

impl Method {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "GET" => Some(Method::GET),
            "POST" => Some(Method::POST),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum Path {
    Chat,
    Root,
}

impl Path {
    fn from_str(s: &str) -> Option<Self> {
        match s {
            "/chat" => Some(Path::Chat),
            "/" => Some(Path::Root),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct Request {
    method: Method,
    path: Path,
}

impl Request {
    fn parse(request_str: &str) -> Option<Self> {
        let line = request_str.lines().next()?;
        let mut parts = line.split_whitespace();

        let method = parts.next().and_then(Method::from_str)?;
        let path = parts.next().and_then(Path::from_str)?;

        Some(Request { method, path })
    }
}

fn send_response(stream: &mut TcpStream, status: &str, body: &str) -> io::Result<()> {
    let response = format!(
        "HTTP/1.1 {}\r\nContent-Length: {}\r\n\r\n{}",
        status,
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    stream.flush()
}

fn chat_handler(stream: &mut TcpStream, method: Method) -> io::Result<()> {
    match method {
        Method::POST => send_response(stream, "200 OK", "Message received"),
        _ => send_response(stream, "405 Method Not Allowed", "Invalid method for /chat"),
    }
}

fn root_handler(stream: &mut TcpStream) -> io::Result<()> {
    send_response(stream, "200 OK", "{\"healthy\": true}")
}

fn handle_client(mut stream: TcpStream) -> io::Result<()> {
    let mut buffer = [0; 4096];
    let bytes_read = stream.read(&mut buffer)?;
    let request_str = String::from_utf8_lossy(&buffer[..bytes_read]);

    match Request::parse(&request_str) {
        Some(request) => {
            println!("Request: {:?}", request);

            match request.path {
                Path::Chat => chat_handler(&mut stream, request.method),
                Path::Root => root_handler(&mut stream),
            }
        }
        None => send_response(&mut stream, "400 Bad Request", "Invalid request"),
    }
}

pub fn listen() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080")?;
    println!("Server listening on http://127.0.0.1:8080");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                if let Err(e) = handle_client(stream) {
                    eprintln!("Error handling client: {}", e);
                }
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    Ok(())
}

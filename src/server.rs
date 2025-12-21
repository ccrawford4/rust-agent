use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};

fn handle_client(mut stream: TcpStream) {
    // Handle the client connection
    let mut buffer = [0; 1024];
    let results = stream.read(&mut buffer).unwrap();
    let request = String::from_utf8_lossy(&buffer[..results]);
    println!("Received request: {}", request);

    let line = request.lines().next().unwrap_or("");

    println!("Request line: {}", line);

    let items = line.split(" ").collect::<Vec<&str>>();
    println!(
        "Method: {}, Path: {}",
        items.get(0).unwrap_or(&""),
        items.get(1).unwrap_or(&"")
    );

    let response = "HTTP/1.1 200 OK\r\n\r\nHello, World!";
    stream.write(response.as_bytes()).unwrap();
    stream.flush().unwrap();
}

pub fn listen() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8080").expect("Could not bind to address");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                handle_client(stream);
            }
            Err(e) => {
                println!("Connection failed: {}", e);
            }
        }
    }

    Ok(())
}

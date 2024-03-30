use raft::read;
use std::thread;
use std::{
    error::Error,
    net::{Ipv4Addr, TcpListener},
};

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 5555)).unwrap();

    // accept connections and process them serially
    for wrapped_stream in listener.incoming() {
        thread::spawn(move || {
            if let Ok(mut stream) = wrapped_stream {
                loop {
                    let message = read(&mut stream).unwrap();
                    println!("{}", message);
                }
            }
        });
    }

    Ok(())
}

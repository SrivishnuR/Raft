use raft::send_message;
use std::net::Ipv4Addr;
use std::{error::Error, io, net::TcpStream};

fn main() -> Result<(), Box<dyn Error>> {
    let mut stream = TcpStream::connect((Ipv4Addr::LOCALHOST, 5555))?;

    loop {
        let mut user_input = String::new();
        let stdin = io::stdin();
        stdin.read_line(&mut user_input)?;
        send_message(&mut stream, &user_input)?;
    }
}

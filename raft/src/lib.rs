use std::io::prelude::*;
use std::str;
use std::{error::Error, net::TcpStream};

pub fn send_message(stream: &mut TcpStream, message: &str) -> Result<(), Box<dyn Error>> {
    let len = message.len();
    let padded_message = format!("{:width$}", len, width = 10) + message;
    stream.write(padded_message.as_bytes())?;
    Ok(())
}

pub fn read_exactly(stream: &mut TcpStream, size: usize) -> Result<String, Box<dyn Error>> {
    let mut buffer: Vec<u8> = vec![0; size];
    stream.read(&mut buffer)?;
    let parsed_string = str::from_utf8(&buffer)?;

    Ok(parsed_string.to_owned())
}

pub fn read(stream: &mut TcpStream) -> Result<String, Box<dyn Error>> {
    let size_str = read_exactly(stream, 10)?;
    let size = size_str.trim().parse()?;

    Ok(read_exactly(stream, size)?)
}

use std::io::prelude::*;
use std::str;
use std::{error::Error, net::TcpStream};
use tokio::{self, io};

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

pub async fn async_send_message(
    stream: &mut tokio::net::TcpStream,
    message: &str,
) -> Result<(), Box<dyn Error>> {
    let len = message.len();
    let padded_message = format!("{:width$}", len, width = 10) + message;

    loop {
        stream.writable().await?;

        match stream.try_write(padded_message.as_bytes()) {
            Ok(_) => {
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    Ok(())
}

pub async fn async_read_exactly(
    stream: &mut tokio::net::TcpStream,
    size: usize,
) -> Result<String, Box<dyn Error>> {
    let mut buffer: Vec<u8> = vec![0; size];
    loop {
        stream.readable().await?;
        match stream.try_read(&mut buffer) {
            Ok(0) => continue,
            Ok(_) => {
                break;
            }
            Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                continue;
            }
            Err(e) => {
                return Err(e.into());
            }
        }
    }

    let parsed_string = str::from_utf8(&buffer)?;

    Ok(parsed_string.to_owned())
}

pub async fn async_read(stream: &mut tokio::net::TcpStream) -> Result<String, Box<dyn Error>> {
    let size_str = async_read_exactly(stream, 10).await?;
    let size = size_str.trim().parse()?;

    Ok(async_read_exactly(stream, size).await?)
}

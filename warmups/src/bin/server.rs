use raft::{read, send_message};
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::{
    error::Error,
    net::{Ipv4Addr, TcpListener},
};

fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 5555)).unwrap();
    let data = Arc::new(Mutex::new(HashMap::new()));

    // accept connections and process them serially
    for wrapped_stream in listener.incoming() {
        let data = Arc::clone(&data);
        thread::spawn(move || {
            if let Ok(mut stream) = wrapped_stream {
                loop {
                    let message = read(&mut stream).unwrap();
                    let data = Arc::clone(&data);
                    let response = parse_message(message, data);

                    send_message(&mut stream, &response).unwrap();
                }
            }
        });
    }

    Ok(())
}

fn parse_message(message: String, data: Arc<Mutex<HashMap<String, String>>>) -> String {
    let sectioned_message: Vec<&str> = message.trim().split(' ').collect();

    if sectioned_message.len() <= 1 {
        return String::from("Invalid command");
    }

    let response: &str = match sectioned_message[0] {
        "set" => {
            if sectioned_message.len() < 3 {
                return String::from("Invalid command");
            }

            let mut data = data.lock().unwrap();
            data.insert(
                String::from(sectioned_message[1]),
                String::from(sectioned_message[2]),
            );
            return String::from("Set successfully");
        }
        "get" => {
            if sectioned_message.len() < 2 {
                return String::from("Invalid command");
            }

            let data = data.lock().unwrap();
            if let Some(entry) = data.get(sectioned_message[1]) {
                return entry.to_owned();
            }

            return String::from("No entry");
        }
        "delete" => {
            if sectioned_message.len() < 2 {
                return String::from("Invalid command");
            }

            let mut data = data.lock().unwrap();
            if let Some(_) = data.remove(sectioned_message[1]) {
                return "Deleted successfully".to_owned();
            }

            return String::from("Key not found");
        }
        _ => "Invalid command",
    };

    return String::from(response);
}

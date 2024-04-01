use raft::{async_read, async_send_message};
use std::collections::HashMap;
use std::sync::Arc;
use std::{error::Error, net::Ipv4Addr};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 5555))
        .await
        .unwrap();
    let data = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // accept connections and process them serially
    loop {
        let (mut stream, _) = listener.accept().await.unwrap();

        let data = Arc::clone(&data);
        tokio::spawn(async move {
            loop {
                let message = async_read(&mut stream).await.unwrap();
                let data = Arc::clone(&data);
                let response = parse_message(message, data).await;

                async_send_message(&mut stream, &response).await.unwrap();
            }
        });
    }
}

async fn parse_message(
    message: String,
    data: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
) -> String {
    let sectioned_message: Vec<&str> = message.trim().split(' ').collect();

    if sectioned_message.len() <= 1 {
        return String::from("Invalid command");
    }

    let response: &str = match sectioned_message[0] {
        "set" => {
            if sectioned_message.len() < 3 {
                return String::from("Invalid command");
            }

            let mut data = data.lock().await;
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

            let data = data.lock().await;
            if let Some(entry) = data.get(sectioned_message[1]) {
                return entry.to_owned();
            }

            return String::from("No entry");
        }
        "delete" => {
            if sectioned_message.len() < 2 {
                return String::from("Invalid command");
            }

            let mut data = data.lock().await;
            if let Some(_) = data.remove(sectioned_message[1]) {
                return "Deleted successfully".to_owned();
            }

            return String::from("Key not found");
        }
        _ => "Invalid command",
    };

    return String::from(response);
}

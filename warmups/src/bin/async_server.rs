use raft::{async_read, async_send_message};
use serde_json;
use std::collections::HashMap;
use std::env;
use std::str;
use std::sync::Arc;
use std::{error::Error, net::Ipv4Addr};
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    let mut data_file_path = None;
    if args.len() >= 2 {
        data_file_path = Some(args[1].to_owned());
    }

    server(data_file_path).await;
    Ok(())
}

async fn server(data_file_path: Option<String>) {
    let mut inner_data: HashMap<String, String> = HashMap::new();
    if let Some(data_file_path) = data_file_path {
        let contents = fs::read(data_file_path)
            .await
            .expect("Invalid data file path");
        inner_data =
            serde_json::from_str(str::from_utf8(&contents).unwrap()).expect("Error deserializing");
    }

    let data = Arc::new(tokio::sync::Mutex::new(inner_data));
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 5555))
        .await
        .unwrap();

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

    if sectioned_message.len() < 1 {
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
        "snapshot" => {
            let data = data.lock().await;
            let serialized_data = serde_json::to_string(&*data).unwrap();

            let uuid = Uuid::new_v4();
            let file_name = format!("kv-{}.data", &(uuid.to_string()[0..4]));
            let file_path = format!("snapshots/{}", file_name);
            let mut data_file = fs::File::create(&file_path).await.unwrap();
            data_file
                .write_all(serialized_data.as_bytes())
                .await
                .unwrap();

            return file_path;
        }
        _ => "Invalid command",
    };

    return String::from(response);
}

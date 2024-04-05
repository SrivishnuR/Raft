pub mod key_value_store {
    use std::collections::HashMap;

    use tokio::{fs, io::AsyncWriteExt};
    use uuid::Uuid;

    pub struct KeyValueStore {
        data: HashMap<String, String>,
    }

    impl KeyValueStore {
        pub async fn new(data_file_path: Option<String>) -> KeyValueStore {
            let data = if let Some(data_file_path) = data_file_path {
                let contents = fs::read(data_file_path)
                    .await
                    .expect("Invalid data file path");
                serde_json::from_str(std::str::from_utf8(&contents).unwrap())
                    .expect("Error deserializing")
            } else {
                HashMap::new()
            };

            KeyValueStore { data }
        }

        pub fn apply_command(self: &mut Self, message: &str) -> String {
            let sectioned_message: Vec<&str> = message.trim().split(' ').collect();

            if sectioned_message.len() < 1 {
                return String::from("Invalid command");
            }

            let response: &str = match sectioned_message[0] {
                "set" => {
                    if sectioned_message.len() < 3 {
                        return String::from("Invalid command");
                    }

                    self.data.insert(
                        String::from(sectioned_message[1]),
                        String::from(sectioned_message[2]),
                    );
                    return String::from("Set successfully");
                }
                "get" => {
                    if sectioned_message.len() < 2 {
                        return String::from("Invalid command");
                    }

                    if let Some(entry) = self.data.get(sectioned_message[1]) {
                        return entry.to_owned();
                    }

                    return String::from("No entry");
                }
                "delete" => {
                    if sectioned_message.len() < 2 {
                        return String::from("Invalid command");
                    }

                    if let Some(_) = self.data.remove(sectioned_message[1]) {
                        return "Deleted successfully".to_owned();
                    }

                    return String::from("Key not found");
                }
                // "snapshot" => {
                //     let serialized_data = serde_json::to_string(&self.data).unwrap();

                //     let uuid = Uuid::new_v4();
                //     let file_name = format!("kv-{}.data", &(uuid.to_string()[0..4]));
                //     let file_path = format!("snapshots/{}", file_name);
                //     let mut data_file = fs::File::create(&file_path).await.unwrap();
                //     data_file
                //         .write_all(serialized_data.as_bytes())
                //         .await
                //         .unwrap();

                //     return file_path;
                // }
                _ => "Invalid command",
            };

            return String::from(response);
        }
    }
}

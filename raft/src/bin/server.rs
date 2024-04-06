use raft::{
    key_value_store::key_value_store::KeyValueStore,
    raft::raft::{Message, Raft},
    raft_net::raft_net::{async_read, async_send_message, RaftNet, ServerNumber, SERVER_ADDRESSES},
};
use rand::Rng;
use std::{
    env,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
    time::sleep,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Server number not specified or too many arguments");
    }

    let my_server_number: usize = args[1].parse().expect("Server number is not valid");

    let (write_send, write_recv) = mpsc::channel::<(ServerNumber, String)>(100);
    let (read_send, mut read_recv) = mpsc::channel::<(ServerNumber, String)>(100);

    let key_value_store = KeyValueStore::new(None).await;

    let raft_net_read_send = read_send.clone();
    let mut raft_net = RaftNet::new(my_server_number).await;
    tokio::spawn(async move { raft_net.run(raft_net_read_send, write_recv).await });

    let mut raft = Raft::new(
        key_value_store,
        my_server_number,
        SERVER_ADDRESSES.len().try_into().unwrap(),
    );

    let random_tick_read_send = read_send.clone();
    tokio::spawn(async move {
        let random_future_time = Arc::new(tokio::sync::Mutex::new(get_random_future_time()));

        let random_future_time_pointer = random_future_time.clone();
        tokio::spawn(async move {
            loop {
                loop {
                    let wrapped_sleep_duration = {
                        let locked_random_future_time = random_future_time_pointer.lock().await;
                        (*locked_random_future_time).duration_since(SystemTime::now())
                    };

                    if let Ok(sleep_duration) = wrapped_sleep_duration {
                        sleep(sleep_duration).await;
                    } else {
                        let serialized_random_tick =
                            serde_json::to_string::<Message>(&Message::RandomTick).unwrap();
                        random_tick_read_send
                            .send((
                                ServerNumber::Server(my_server_number),
                                serialized_random_tick,
                            ))
                            .await
                            .unwrap();
                        break;
                    }
                }

                let mut timeout = random_future_time_pointer.lock().await;
                *timeout = get_random_future_time();
            }
        });

        loop {
            let (server_number, message) = read_recv.recv().await.unwrap();

            let deserialized_message =
                serde_json::from_str::<Message>(&message).expect("Invalid Message");

            let responses = raft.process_message(server_number, deserialized_message);

            for response in responses {
                match response.1 {
                    Message::RequestRandomTick => {
                        let mut locked_random_future_time = random_future_time.lock().await;
                        *locked_random_future_time = get_random_future_time();
                    }
                    _ => {
                        let serialized_response = (
                            server_number,
                            serde_json::to_string::<Message>(&response.1).unwrap(),
                        );
                        write_send.send(serialized_response).await.unwrap();
                    }
                }
            }
        }
    });

    // Creating a server console
    let server_address = SERVER_ADDRESSES
        .get(&my_server_number)
        .expect("Server number is not valid");

    let stream = TcpStream::connect(server_address).await.unwrap();
    let (mut read, mut write) = stream.into_split();
    async_send_message(&mut write, "console").await.unwrap();

    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let prompt = format!("Server {} > ", my_server_number);

    loop {
        stdout.write(prompt.as_bytes()).await.unwrap();
        stdout.flush().await.unwrap();

        let mut user_input = vec![];
        reader.read_until(b'\n', &mut user_input).await.unwrap();
        let user_input_string = std::str::from_utf8(&user_input).unwrap().to_owned();

        let request = Message::ConsoleRequest {
            message: user_input_string,
        };
        let serialized_request =
            serde_json::to_string::<Message>(&request).expect("Serialization error");

        async_send_message(&mut write, &serialized_request)
            .await
            .unwrap();

        let serialized_response = async_read(&mut read).await.unwrap();

        let deserialized_response =
            serde_json::from_str::<Message>(&serialized_response).expect("Deserialization error");

        let message = match deserialized_response {
            Message::ConsoleResponse { message } => message,
            _ => String::from("Invalid console response"),
        };

        stdout.write(message.as_bytes()).await.unwrap();
        stdout.write("\n".as_bytes()).await.unwrap();
    }
}

fn get_random_future_time() -> SystemTime {
    let time = SystemTime::now();
    let random_sleep: u64 = {
        let mut rng = rand::thread_rng();
        rng.gen_range(5000..10000)
    };
    time.checked_add(Duration::from_millis(random_sleep))
        .unwrap()
}

use raft::{
    raft::raft::{Message, Raft},
    raft_net::raft_net::{async_read, async_send_message, RaftNet, ServerNumber, SERVER_ADDRESSES},
};
use std::env;
use tokio::{
    io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    sync::mpsc,
};

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Server number not specified or too many arguments");
    }

    let server_number: u8 = args[1].parse().expect("Server number is not valid");

    let (write_send, write_recv) = mpsc::channel::<(ServerNumber, String)>(100);
    let (read_send, mut read_recv) = mpsc::channel::<(ServerNumber, String)>(100);

    let mut raft_net = RaftNet::new(server_number).await;
    tokio::spawn(async move { raft_net.run(read_send, write_recv).await });

    let mut raft = Raft::new(SERVER_ADDRESSES.len().try_into().unwrap());
    tokio::spawn(async move {
        loop {
            let (server_number, message) = read_recv.recv().await.unwrap();

            let deserialized_message =
                serde_json::from_str::<Message>(&message).expect("Invalid Message");

            let responses = raft.process_message(server_number, deserialized_message);

            let serialized_responses: Vec<(ServerNumber, String)> = responses
                .into_iter()
                .map(|(server_number, response)| {
                    (server_number, serde_json::to_string(&response).unwrap())
                })
                .collect();

            for serialized_response in serialized_responses {
                write_send.send(serialized_response).await.unwrap();
            }
        }
    });

    // Creating a server console
    let server_address = SERVER_ADDRESSES
        .get(&server_number)
        .expect("Server number is not valid");

    let stream = TcpStream::connect(server_address).await.unwrap();
    let (mut read, mut write) = stream.into_split();
    async_send_message(&mut write, "console").await.unwrap();

    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);
    let prompt = format!("Server {} > ", server_number);

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

        let response = match deserialized_response {
            Message::ConsoleResponse { message } => message,
            _ => String::from("Invalid console response"),
        };

        stdout.write(response.as_bytes()).await.unwrap();
        stdout.write("\n".as_bytes()).await.unwrap();
    }
}

use raft::{
    raft::raft::Message,
    raft_net::raft_net::{async_read, async_send_message, ServerNumber, SERVER_ADDRESSES},
};
use std::{env, error::Error};
use tokio::{
    io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

// Client
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Server number not specified or too many arguments");
    }

    let server_number: u8 = args[1].parse().expect("Server number is not valid");
    let server_address = SERVER_ADDRESSES
        .get(&server_number)
        .expect("Server number is not valid");

    let stream = TcpStream::connect(server_address).await.unwrap();
    let (mut read, mut write) = stream.into_split();
    async_send_message(&mut write, "client").await.unwrap();

    let mut stdout = io::stdout();
    let stdin = io::stdin();
    let mut reader = BufReader::new(stdin);

    loop {
        stdout.write("KV > ".as_bytes()).await.unwrap();
        stdout.flush().await.unwrap();

        let mut user_input = vec![];
        reader.read_until(b'\n', &mut user_input).await.unwrap();
        let user_input_string = std::str::from_utf8(&user_input).unwrap();

        let message = Message::ClientLogAppendRequest {
            entries: vec![user_input_string.trim().to_owned()],
        };
        let serialized_message =
            serde_json::to_string::<Message>(&message).expect("Serialization error");

        dbg!(&serialized_message);
        async_send_message(&mut write, &serialized_message)
            .await
            .unwrap();
        let response = async_read(&mut read).await.unwrap();

        stdout.write(response.as_bytes()).await.unwrap();
    }
}

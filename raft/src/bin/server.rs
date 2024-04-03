use raft::{
    raft::raft::{Message, Raft},
    raft_net::raft_net::{RaftNet, ServerNumber, SERVER_ADDRESSES},
};
use std::env;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Server number not specified or too many arguments");
    }

    let server_number: u8 = args[1].parse().expect("Server number is not valid");

    let (write_send, write_recv) = mpsc::channel::<(ServerNumber, String)>(100);
    let (read_send, mut read_recv) = mpsc::channel::<(ServerNumber, String)>(100);

    let mut raft_net = RaftNet::new(server_number);
    let raft_net_task = tokio::spawn(async move { raft_net.init(read_send, write_recv).await });

    let mut raft = Raft::new(SERVER_ADDRESSES.len().try_into().unwrap());

    let raft_task = tokio::spawn(async move {
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

    raft_net_task.await.unwrap();
    raft_task.await.unwrap();
}

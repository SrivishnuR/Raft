use raft::{async_read, async_send_message};
use std::collections::HashMap;
use std::env;
use std::net::Ipv4Addr;

use lazy_static::lazy_static;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{self, Receiver, Sender};

type ServerNumber = u8;

lazy_static! {
    static ref SERVER_ADDRESSES: HashMap<ServerNumber, (Ipv4Addr, u16)> = {
        return HashMap::from([
            (0, (Ipv4Addr::LOCALHOST, 15000)),
            (1, (Ipv4Addr::LOCALHOST, 16000)),
            (2, (Ipv4Addr::LOCALHOST, 17000)),
            (3, (Ipv4Addr::LOCALHOST, 18000)),
            (4, (Ipv4Addr::LOCALHOST, 19000)),
        ]);
    };
}

// Raftnet
#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        panic!("Server number not specified or too many arguments");
    }

    let server_number: ServerNumber = args[1].parse().expect("Server number is not valid");

    let (write_send, write_recv) = mpsc::channel::<(ServerNumber, String)>(100);
    let (read_send, read_recv) = mpsc::channel::<(ServerNumber, String)>(100);

    let mut raft_net = RaftNet::new(server_number);
    raft_net.init(read_send, write_recv).await;
}

struct RaftNet {
    server_number: ServerNumber,
}

impl RaftNet {
    // Initializes a RaftNet and reaches out to all other servers to establish a connection
    pub fn new(server_number: ServerNumber) -> RaftNet {
        RaftNet { server_number }
    }

    async fn write(
        mut write_recv: Receiver<(ServerNumber, String)>,
        mut write_stream_recv: Receiver<(ServerNumber, tokio::net::tcp::OwnedWriteHalf)>,
    ) {
        let mut stream_map = HashMap::new();

        loop {
            let (server_number, message) = write_recv.recv().await.unwrap();

            while let Ok((stream_server_number, new_write_stream)) = write_stream_recv.try_recv() {
                stream_map.insert(stream_server_number, new_write_stream);
            }

            if let Some(stream) = stream_map.get_mut(&server_number) {
                async_send_message(stream, &message).await.unwrap();
            }
        }
    }

    async fn read(
        read_send: Sender<(ServerNumber, String)>,
        mut read_stream_recv: Receiver<(ServerNumber, tokio::net::tcp::OwnedReadHalf)>,
    ) {
        loop {
            let read_send_clone = read_send.clone();

            let (server_number, read_stream) = read_stream_recv.recv().await.unwrap();
            tokio::spawn(async move {
                loop {
                    read_send_clone
                        .send((server_number, async_read(&read_stream).await.unwrap()))
                        .await
                        .unwrap();
                }
            });
        }
    }

    // Listens for other servers attempting to connect
    pub async fn init(
        self: &mut Self,
        read_send: Sender<(ServerNumber, String)>,
        write_recv: Receiver<(ServerNumber, String)>,
    ) {
        let (write_stream_send, write_stream_recv) =
            mpsc::channel::<(ServerNumber, tokio::net::tcp::OwnedWriteHalf)>(100);
        let (read_stream_send, read_stream_recv) =
            mpsc::channel::<(ServerNumber, tokio::net::tcp::OwnedReadHalf)>(100);

        tokio::spawn(RaftNet::write(write_recv, write_stream_recv));
        tokio::spawn(RaftNet::read(read_send, read_stream_recv));

        for (server_number, addr) in SERVER_ADDRESSES.iter() {
            if let Ok(stream) = TcpStream::connect(addr).await {
                let (read, write) = stream.into_split();
                read_stream_send.send((*server_number, read)).await.unwrap();
                write_stream_send
                    .send((*server_number, write))
                    .await
                    .unwrap();
            }
        }

        let listener = TcpListener::bind(
            SERVER_ADDRESSES
                .get(&self.server_number)
                .expect("Server number is not valid"),
        )
        .await
        .unwrap();

        loop {
            let (stream, socket) = listener.accept().await.unwrap();
            let port = socket.port();

            let server_number =
                SERVER_ADDRESSES.iter().find_map(
                    |(key, &val)| {
                        if val.1 == port {
                            Some(key)
                        } else {
                            None
                        }
                    },
                );

            let server_number = server_number.expect("Invalid server connected");
            let (read, write) = stream.into_split();

            read_stream_send.send((*server_number, read)).await.unwrap();
            write_stream_send
                .send((*server_number, write))
                .await
                .unwrap();
        }
    }
}

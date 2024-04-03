pub mod raft_net {
    use lazy_static::lazy_static;
    use std::collections::HashMap;
    use std::error::Error;
    use std::net::Ipv4Addr;
    use std::str;
    use tokio::io;
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::mpsc::{self, Receiver, Sender};

    pub async fn async_send_message(
        stream: &mut tokio::net::tcp::OwnedWriteHalf,
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
        stream: &tokio::net::tcp::OwnedReadHalf,
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

    pub async fn async_read(
        stream: &tokio::net::tcp::OwnedReadHalf,
    ) -> Result<String, Box<dyn Error>> {
        let size_str = async_read_exactly(stream, 10).await?;
        let size = size_str.trim().parse()?;

        Ok(async_read_exactly(stream, size).await?)
    }

    pub struct RaftNet {
        server_number: u8,
    }

    #[derive(PartialEq, Eq, Hash, Clone, Copy)]
    pub enum ServerNumber {
        Client,
        Server(u8),
    }

    lazy_static! {
        pub static ref SERVER_ADDRESSES: HashMap<ServerNumber, (Ipv4Addr, u16)> = {
            return HashMap::from([
                (ServerNumber::Server(0), (Ipv4Addr::LOCALHOST, 15000)),
                (ServerNumber::Server(1), (Ipv4Addr::LOCALHOST, 16000)),
                (ServerNumber::Server(2), (Ipv4Addr::LOCALHOST, 17000)),
                (ServerNumber::Server(3), (Ipv4Addr::LOCALHOST, 18000)),
                (ServerNumber::Server(4), (Ipv4Addr::LOCALHOST, 19000)),
            ]);
        };
    }

    impl RaftNet {
        // Initializes a RaftNet and reaches out to all other servers to establish a connection
        pub fn new(server_number: u8) -> RaftNet {
            RaftNet { server_number }
        }

        async fn write(
            mut write_recv: Receiver<(ServerNumber, String)>,
            mut write_stream_recv: Receiver<(ServerNumber, tokio::net::tcp::OwnedWriteHalf)>,
        ) {
            let mut stream_map = HashMap::new();

            loop {
                let (server_number, message) = write_recv.recv().await.unwrap();

                while let Ok((stream_server_number, new_write_stream)) =
                    write_stream_recv.try_recv()
                {
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
                    let (read, mut write) = stream.into_split();
                    async_send_message(&mut write, &self.server_number.to_string())
                        .await
                        .unwrap();

                    read_stream_send.send((*server_number, read)).await.unwrap();
                    write_stream_send
                        .send((*server_number, write))
                        .await
                        .unwrap();
                }
            }

            let listener = TcpListener::bind(
                SERVER_ADDRESSES
                    .get(&ServerNumber::Server(self.server_number))
                    .expect("Server number is not valid"),
            )
            .await
            .unwrap();

            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let (read, write) = stream.into_split();

                let server_number_string = async_read(&read).await.unwrap();
                let server_number: ServerNumber;
                if server_number_string == "client" {
                    server_number = ServerNumber::Client;
                } else {
                    server_number =
                        ServerNumber::Server(server_number_string.parse::<u8>().unwrap());
                }

                read_stream_send.send((server_number, read)).await.unwrap();
                write_stream_send
                    .send((server_number, write))
                    .await
                    .unwrap();
            }
        }
    }
}

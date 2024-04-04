pub mod raft {
    use crate::{raft_log::raft_log::RaftLog, raft_net::raft_net::ServerNumber};
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum Message {
        ConsoleRequest {
            message: String,
        },
        ConsoleResponse {
            message: String,
        },
        ClientLogAppendRequest {
            entries: Vec<String>,
        },
        AppendEntriesRequest {
            entries: Vec<(usize, String)>,
            prev_log_index: Option<usize>,
            prev_log_term: Option<usize>,
        },
        AppendEntriesResponse {
            success: bool,
        },
    }

    #[derive(PartialEq)]
    enum States {
        Follower,
        Candidate,
        Leader,
    }

    pub struct Raft {
        log: RaftLog,
        state: States,
        cluster_size: u8,
        term: usize,
    }

    impl Raft {
        pub fn new(cluster_size: u8) -> Raft {
            Raft {
                log: RaftLog::new(),
                state: States::Follower,
                cluster_size,
                term: 0,
            }
        }

        pub fn process_message(
            self: &mut Self,
            server_number: ServerNumber,
            message: Message,
        ) -> Vec<(ServerNumber, Message)> {
            match message {
                Message::ClientLogAppendRequest { entries } => {
                    self.handle_client_log_append_request(entries)
                }
                Message::AppendEntriesRequest {
                    entries,
                    prev_log_index,
                    prev_log_term,
                } => self.handle_append_entries_request(
                    server_number,
                    entries,
                    prev_log_index,
                    prev_log_term,
                ),
                Message::AppendEntriesResponse { success } => {
                    self.handle_append_entries_reponse(server_number, success)
                }
                Message::ConsoleRequest { message } => self.handle_console_request(message),
                Message::ConsoleResponse { .. } => panic!("Received a console response!"),
            }
        }

        fn handle_client_log_append_request(
            self: &Self,
            entries: Vec<String>,
        ) -> Vec<(ServerNumber, Message)> {
            if self.state != States::Leader {
                // Do nothing if node is not leader
                return vec![];
            }

            let prev_log_index = self.log.get_prev_log_index();
            let prev_log_term = self.log.get_prev_log_term();

            let term_entries: Vec<(usize, String)> =
                entries.into_iter().map(|item| (self.term, item)).collect();

            let response = Message::AppendEntriesRequest {
                entries: term_entries,
                prev_log_index,
                prev_log_term,
            };

            let mut commands = vec![];
            for server_number in 0..self.cluster_size {
                commands.push((ServerNumber::Server(server_number), response.clone()))
            }

            commands
        }

        fn handle_append_entries_request(
            self: &mut Self,
            server_number: ServerNumber,
            entries: Vec<(usize, String)>,
            prev_index: Option<usize>,
            prev_term: Option<usize>,
        ) -> Vec<(ServerNumber, Message)> {
            let success = self.log.append_entries(prev_index, prev_term, entries);
            let response_struct = Message::AppendEntriesResponse { success };

            vec![(server_number, response_struct)]
        }

        fn handle_append_entries_reponse(
            self: &Self,
            server_number: ServerNumber,
            success: bool,
        ) -> Vec<(ServerNumber, Message)> {
            vec![]
        }

        fn handle_console_request(
            self: &mut Self,
            message: String,
        ) -> Vec<(ServerNumber, Message)> {
            let response = self.parse_console_request(message);

            vec![(
                ServerNumber::Console,
                Message::ConsoleResponse { message: response },
            )]
        }

        fn parse_console_request(self: &mut Self, message: String) -> String {
            let sectioned_message: Vec<&str> = message.trim().split(' ').collect();

            if sectioned_message.len() < 1 {
                return String::from("Invalid command");
            }

            match sectioned_message[0] {
                "set" => {
                    if sectioned_message.len() != 2 {
                        return String::from("Invalid command");
                    }

                    match sectioned_message[1] {
                        "leader" => {
                            self.state = States::Leader;
                            return String::from("Set as leader");
                        }
                        "candidate" => {
                            self.state = States::Candidate;
                            return String::from("Set as candidate");
                        }
                        "follower" => {
                            self.state = States::Follower;
                            return String::from("Set as follower");
                        }
                        _ => return String::from("Invalid Command"),
                    }
                }
                "get" => {
                    if sectioned_message.len() != 2 {
                        return String::from("Invalid command");
                    }

                    match sectioned_message[1] {
                        "log" => {
                            return format!("{:?}", self.log.get_log());
                        }
                        _ => return String::from("Invalid Command"),
                    }
                }
                _ => {
                    return String::from("Invalid command");
                }
            };
        }
    }
}

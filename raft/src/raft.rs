pub mod raft {
    use crate::{raft_log::raft_log::RaftLog, raft_net::raft_net::ServerNumber};
    use serde::{Deserialize, Serialize};
    use tokio::sync::mpsc::{Receiver, Sender};

    #[derive(Serialize, Deserialize, Clone)]
    pub enum Message {
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
        cluster_size: ServerNumber,
        term: usize,
    }

    impl Raft {
        pub fn new(cluster_size: ServerNumber) -> Raft {
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
                commands.push((server_number, response.clone()))
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
    }
}

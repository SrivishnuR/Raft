pub mod raft {
    use std::cmp::{max, min};

    use crate::{
        key_value_store::key_value_store::KeyValueStore, raft_log::raft_log::RaftLog,
        raft_net::raft_net::ServerNumber,
    };
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum AppendEntriesStatus {
        AppendEntriesSuccess { match_index: usize },
        AppendEntriesFailure,
    }

    #[derive(Serialize, Deserialize, Clone, Debug)]
    pub enum Message {
        Tick,
        RandomTick,
        RequestRandomTick,
        ConsoleRequest {
            message: String,
        },
        ConsoleResponse {
            message: String,
        },
        ClientRequest {
            entries: Vec<String>,
        },
        ClientResponse {
            message: String,
        },
        AppendEntriesRequest {
            term: usize,
            entries: Vec<(Option<usize>, String)>,
            prev_log_index: usize,
            prev_log_term: Option<usize>,
            leader_commit: usize,
        },
        AppendEntriesResponse {
            term: usize,
            status: AppendEntriesStatus,
        },
        RequestVoteRequest {
            term: usize,
            candidate_id: usize,
            last_log_index: usize,
            last_log_term: Option<usize>,
        },
        RequestVoteResponse {
            term: usize,
            vote_granted: bool,
        },
    }

    #[derive(PartialEq)]
    enum States {
        Follower,
        Candidate {
            votes_vec: Vec<bool>,
        },
        Leader {
            next_index_vec: Vec<usize>,
            match_index_vec: Vec<usize>,
        },
    }

    pub struct Raft {
        key_value_store: KeyValueStore,
        log: RaftLog,
        state: States,
        cluster_size: usize,
        server_number: usize,
        term: usize,
        commit_index: usize,
        last_applied: usize,
        voted_for: Option<usize>,
        request_random_tick: bool,
    }

    impl Raft {
        pub fn new(
            key_value_store: KeyValueStore,
            server_number: usize,
            cluster_size: usize,
        ) -> Raft {
            Raft {
                log: RaftLog::new(),
                state: States::Follower,
                key_value_store,
                cluster_size,
                server_number,
                term: 0,
                // This needs to be changed later
                commit_index: 0,
                last_applied: 0,
                voted_for: None,
                request_random_tick: true,
            }
        }

        fn set_leader(self: &mut Self) {
            let next_index = self.log.get_latest_log_index() + 1;
            let mut match_index_vec = vec![0; self.cluster_size];
            match_index_vec[self.server_number] = self.log.get_latest_log_index();

            self.state = States::Leader {
                next_index_vec: vec![next_index; self.cluster_size],
                match_index_vec,
            }
        }

        fn set_candidate(self: &mut Self) {
            // Increase your term when you're a candidate
            self.term += 1;
            self.voted_for = Some(self.server_number);
            let mut votes_vec = vec![false; self.cluster_size];
            votes_vec[self.server_number] = true;
            self.state = States::Candidate { votes_vec };
            self.request_random_tick = true;
        }

        fn set_follower(self: &mut Self) {
            self.state = States::Follower;
            self.request_random_tick = true;
        }

        fn validate_term(self: &mut Self, term: usize) -> bool {
            if term < self.term {
                return false;
            }

            if term > self.term {
                self.set_follower();
                self.voted_for = None;
                self.term = term;
            }

            return true;
        }

        fn update_commit_index(
            self: &mut Self,
            new_commit_index: usize,
        ) -> Vec<(ServerNumber, Message)> {
            let mut responses = vec![];
            if new_commit_index > self.commit_index {
                dbg!("Commit");
                self.commit_index = min(new_commit_index, self.log.get_latest_log_index());

                while self.last_applied < self.commit_index {
                    let entry = self.log.get_entry(self.last_applied + 1);
                    let response = self.key_value_store.apply_command(entry);
                    self.last_applied += 1;

                    responses.push((
                        ServerNumber::Client,
                        Message::ClientResponse { message: response },
                    ));
                }
            }

            responses
        }

        pub fn process_message(
            self: &mut Self,
            server_number: ServerNumber,
            message: Message,
        ) -> Vec<(ServerNumber, Message)> {
            let mut response = match message {
                Message::Tick => self.handle_tick(),
                Message::RandomTick => self.handle_random_tick(),
                Message::ClientRequest { entries } => {
                    self.handle_client_log_append_request(entries)
                }
                Message::AppendEntriesRequest {
                    term,
                    entries,
                    prev_log_index,
                    prev_log_term,
                    leader_commit,
                } => self.handle_append_entries_request(
                    term,
                    server_number,
                    entries,
                    prev_log_index,
                    prev_log_term,
                    leader_commit,
                ),
                Message::AppendEntriesResponse { status, term } => {
                    self.handle_append_entries_reponse(term, server_number, status)
                }
                Message::RequestVoteRequest {
                    term,
                    candidate_id,
                    last_log_index,
                    last_log_term,
                } => self.handle_request_vote_request(
                    term,
                    server_number,
                    candidate_id,
                    last_log_index,
                    last_log_term,
                ),
                Message::RequestVoteResponse { term, vote_granted } => {
                    self.handle_request_vote_response(server_number, term, vote_granted)
                }
                Message::ConsoleRequest { message } => self.handle_console_request(message),
                Message::RequestRandomTick => panic!("Received a random tick request!"),
                Message::ConsoleResponse { .. } => panic!("Received a console response!"),
                Message::ClientResponse { .. } => panic!("Received a client response!"),
            };

            if !self.request_random_tick {
                self.request_random_tick = false;
                response.push((
                    ServerNumber::Server(self.server_number),
                    Message::RandomTick,
                ));
            }

            return response;
        }

        fn handle_client_log_append_request(
            self: &mut Self,
            entries: Vec<String>,
        ) -> Vec<(ServerNumber, Message)> {
            match &mut self.state {
                States::Leader {
                    next_index_vec: _,
                    match_index_vec,
                } => {
                    let prev_log_index = self.log.get_latest_log_index();
                    let prev_log_term = self.log.get_latest_log_term();

                    let term_entries: Vec<(Option<usize>, String)> = entries
                        .into_iter()
                        .map(|item| (Some(self.term), item))
                        .collect();

                    self.log
                        .append_entries(prev_log_index, prev_log_term, &term_entries);
                    match_index_vec[self.server_number] = self.log.get_latest_log_index();

                    return vec![];
                }
                _ => {
                    return vec![(
                        ServerNumber::Client,
                        Message::ClientResponse {
                            message: String::from("Not connected to a leader"),
                        },
                    )]
                }
            }
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
                            self.set_leader();
                            return String::from("Set as leader");
                        }
                        "candidate" => {
                            self.set_candidate();
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

        fn handle_tick(self: &mut Self) -> Vec<(ServerNumber, Message)> {
            match &self.state {
                States::Leader {
                    next_index_vec,
                    match_index_vec: _,
                } => {
                    let mut commands = vec![];

                    for other_server_number in 0..self.cluster_size {
                        if other_server_number == self.server_number {
                            continue;
                        }

                        let next_index = next_index_vec[other_server_number];
                        if self.log.get_latest_log_index() >= next_index {
                            let entries = self.log.get_log_from(next_index);
                            let request = Message::AppendEntriesRequest {
                                term: self.term,
                                entries,
                                prev_log_index: next_index - 1,
                                prev_log_term: self.log.get_term(next_index - 1),
                                leader_commit: self.commit_index,
                            };

                            commands.push((ServerNumber::Server(other_server_number), request))
                        } else {
                            let request = Message::AppendEntriesRequest {
                                term: self.term,
                                entries: vec![],
                                prev_log_index: self.log.get_latest_log_index(),
                                prev_log_term: self.log.get_latest_log_term(),
                                leader_commit: self.commit_index,
                            };

                            commands.push((ServerNumber::Server(other_server_number), request))
                        }
                    }

                    return commands;
                }
                States::Candidate { .. } => {
                    let mut commands = vec![];

                    for other_server_number in 0..self.cluster_size {
                        if other_server_number == self.server_number {
                            continue;
                        }

                        let request = Message::RequestVoteRequest {
                            term: self.term,
                            candidate_id: self.server_number,
                            last_log_index: self.log.get_latest_log_index(),
                            last_log_term: self.log.get_latest_log_term(),
                        };

                        commands.push((ServerNumber::Server(other_server_number), request))
                    }

                    return commands;
                }
                States::Follower => return vec![],
            };
        }

        fn handle_random_tick(self: &mut Self) -> Vec<(ServerNumber, Message)> {
            match &self.state {
                States::Leader { .. } => return vec![],
                States::Candidate { .. } => {
                    self.set_candidate();
                    return vec![];
                }
                States::Follower { .. } => {
                    self.set_candidate();
                    return vec![];
                }
            };
        }

        fn handle_append_entries_request(
            self: &mut Self,
            term: usize,
            server_number: ServerNumber,
            entries: Vec<(Option<usize>, String)>,
            prev_index: usize,
            prev_term: Option<usize>,
            leader_commit: usize,
        ) -> Vec<(ServerNumber, Message)> {
            if !self.validate_term(term) {
                return vec![];
            }

            match &mut self.state {
                States::Follower => {
                    self.update_commit_index(leader_commit);

                    let success = self.log.append_entries(prev_index, prev_term, &entries);

                    let response_status = if success {
                        AppendEntriesStatus::AppendEntriesSuccess {
                            match_index: self.log.get_latest_log_index(),
                        }
                    } else {
                        AppendEntriesStatus::AppendEntriesFailure
                    };

                    let response_struct = Message::AppendEntriesResponse {
                        status: response_status,
                        term: self.term,
                    };

                    self.request_random_tick = true;
                    return vec![(server_number, response_struct)];
                }
                States::Candidate { .. } => {
                    // Convert to follower if you get a request
                    self.set_follower();
                    return self.handle_append_entries_request(
                        term,
                        server_number,
                        entries,
                        prev_index,
                        prev_term,
                        leader_commit,
                    );
                }
                States::Leader { .. } => panic!("Two leaders detected!"),
            }
        }

        fn handle_append_entries_reponse(
            self: &mut Self,
            term: usize,
            server_number: ServerNumber,
            status: AppendEntriesStatus,
        ) -> Vec<(ServerNumber, Message)> {
            let ServerNumber::Server(server_number) = server_number else {
                panic!("Received a append entries response from not a server");
            };

            if !self.validate_term(term) {
                return vec![];
            }

            match &mut self.state {
                States::Leader {
                    next_index_vec,
                    match_index_vec,
                } => {
                    match status {
                        AppendEntriesStatus::AppendEntriesSuccess { match_index } => {
                            match_index_vec[server_number] =
                                max(match_index, match_index_vec[server_number]);
                            next_index_vec[server_number] =
                                max(match_index + 1, next_index_vec[server_number]);

                            // Calculate the median to get the commit index
                            let mut sorted_match_index = match_index_vec.clone();
                            sorted_match_index.sort();
                            let new_commit_index = if sorted_match_index.len() % 2 == 0 {
                                let index = (sorted_match_index.len() / 2) - 1;
                                sorted_match_index[index]
                            } else {
                                let index = sorted_match_index.len() / 2;
                                sorted_match_index[index]
                            };

                            return self.update_commit_index(new_commit_index);
                        }
                        AppendEntriesStatus::AppendEntriesFailure => {
                            next_index_vec[server_number] = max(next_index_vec[server_number], 1);
                            return vec![];
                        }
                    };
                }
                _ => {
                    return vec![];
                }
            }
        }

        fn handle_request_vote_request(
            self: &mut Self,
            term: usize,
            server_number: ServerNumber,
            candidate_id: usize,
            last_log_index: usize,
            last_log_term: Option<usize>,
        ) -> Vec<(ServerNumber, Message)> {
            if !self.validate_term(term) {
                return vec![(
                    server_number,
                    Message::RequestVoteResponse {
                        term: self.term,
                        vote_granted: false,
                    },
                )];
            }

            let my_last_log_term = self.log.get_latest_log_term();
            let my_last_log_index = self.log.get_latest_log_index();

            if self.voted_for == None
                && (last_log_term > my_last_log_term
                    || (last_log_term == my_last_log_term && last_log_index >= my_last_log_index))
            {
                self.voted_for = Some(candidate_id);
                return vec![(
                    server_number,
                    Message::RequestVoteResponse {
                        term: self.term,
                        vote_granted: true,
                    },
                )];
            } else {
                return vec![(
                    server_number,
                    Message::RequestVoteResponse {
                        term: self.term,
                        vote_granted: false,
                    },
                )];
            }
        }

        fn handle_request_vote_response(
            self: &mut Self,
            server_number: ServerNumber,
            term: usize,
            vote_granted: bool,
        ) -> Vec<(ServerNumber, Message)> {
            let ServerNumber::Server(server_number) = server_number else {
                panic!("Received a request vote response from not a server");
            };

            if !self.validate_term(term) {
                return vec![];
            }

            if !vote_granted {
                return vec![];
            }

            match &mut self.state {
                States::Candidate { votes_vec } => {
                    votes_vec[server_number] = true;
                    if votes_vec.iter().filter(|value| **value).count() > (self.cluster_size / 2) {
                        self.set_leader();
                    }

                    return vec![];
                }
                _ => return vec![],
            };
        }
    }
}

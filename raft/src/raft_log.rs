pub mod raft_log {
    use std::cmp::min;

    pub struct RaftLog {
        log: Vec<(usize, String)>,
    }

    impl RaftLog {
        pub fn new() -> RaftLog {
            RaftLog { log: Vec::new() }
        }

        pub fn append_entries(
            self: &mut Self,
            prev_index: Option<usize>,
            prev_term: Option<usize>,
            mut entries: Vec<(usize, String)>,
        ) -> bool {
            let index;
            if prev_index == None {
                if prev_term != None {
                    return false;
                }

                index = 0;
            } else {
                let Some(prev_index) = prev_index else {
                    return false;
                };
                let Some(prev_term) = prev_term else {
                    return false;
                };

                if self.log.len() <= prev_index {
                    return false;
                }

                if self.log[prev_index].0 != prev_term {
                    return false;
                }

                index = prev_index + 1;
            }

            if index < self.log.len() {
                let sub_log = &self.log[index..min(self.log.len(), index + entries.len())];
                if sub_log == entries {
                    // Returning true because we already contain the entry
                    return true;
                }

                self.log.drain(index..);
            }

            self.log.append(&mut entries);
            true
        }

        pub fn get_log(self: &Self) -> &Vec<(usize, String)> {
            return &self.log;
        }

        pub fn get_prev_log_index(self: &Self) -> Option<usize> {
            if self.log.len() == 0 {
                return None;
            }

            return Some(self.log.len() - 1);
        }

        pub fn get_prev_log_term(self: &Self) -> Option<usize> {
            if self.log.len() == 0 {
                return None;
            }

            return Some(self.log[self.log.len() - 1].0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_to_empty() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(!raft_log.append_entries(None, Some(0), vec![(1, String::from(""))]));
        assert!(raft_log.append_entries(None, None, vec![(1, String::from(""))]));
        assert_eq!(raft_log.get_log(), &vec![(1, String::from(""))]);
    }

    #[test]
    fn test_add_gap() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(!raft_log.append_entries(Some(1), Some(1), vec![(1, String::from(""))]));
        assert!(raft_log.append_entries(None, None, vec![(1, String::from(""))]));
        assert!(!raft_log.append_entries(Some(1), Some(1), vec![(1, String::from(""))]));
        assert_eq!(raft_log.get_log(), &vec![(1, String::from(""))]);
    }

    #[test]
    fn test_add_to_end() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(raft_log.append_entries(None, None, vec![(1, String::from(""))]));
        assert!(!raft_log.append_entries(Some(0), Some(0), vec![(2, String::from(""))]));
        assert!(raft_log.append_entries(Some(0), Some(1), vec![(2, String::from(""))]));
        assert_eq!(
            raft_log.get_log(),
            &vec![(1, String::from("")), (2, String::from(""))]
        );
    }

    #[test]
    fn test_add_to_middle() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(raft_log.append_entries(None, None, vec![(1, String::from(""))]));
        assert!(raft_log.append_entries(
            Some(0),
            Some(1),
            vec![(2, String::from("")), (2, String::from(""))]
        ));
        assert_eq!(
            raft_log.get_log(),
            &vec![
                (1, String::from("")),
                (2, String::from("")),
                (2, String::from(""))
            ]
        );

        assert!(!raft_log.append_entries(Some(0), Some(2), vec![(3, String::from(""))]));
        assert!(raft_log.append_entries(Some(0), Some(1), vec![(3, String::from(""))]));
        assert_eq!(
            raft_log.get_log(),
            &vec![(1, String::from("")), (3, String::from(""))]
        );
    }

    #[test]
    fn test_add_duplicate() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(raft_log.append_entries(None, None, vec![(1, String::from(""))]));
        assert!(raft_log.append_entries(
            Some(0),
            Some(1),
            vec![(2, String::from("")), (2, String::from(""))]
        ));

        assert!(raft_log.append_entries(Some(0), Some(1), vec![(2, String::from(""))]));

        // Shouldn't concat in this case
        assert_eq!(
            raft_log.get_log(),
            &vec![
                (1, String::from("")),
                (2, String::from("")),
                (2, String::from(""))
            ]
        );
    }
}

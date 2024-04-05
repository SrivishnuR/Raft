pub mod raft_log {
    use std::cmp::min;

    pub struct RaftLog {
        log: Vec<(Option<usize>, String)>,
    }

    impl RaftLog {
        pub fn new() -> RaftLog {
            RaftLog {
                log: vec![(None, String::from(""))],
            }
        }

        pub fn append_entries(
            self: &mut Self,
            prev_index: usize,
            prev_term: Option<usize>,
            entries: &Vec<(Option<usize>, String)>,
        ) -> bool {
            if prev_index >= self.log.len() {
                return false;
            }

            if self.log[prev_index].0 != prev_term {
                return false;
            }

            let index = prev_index + 1;

            if index < self.log.len() {
                let sub_log = &self.log[index..min(self.log.len(), index + entries.len())];
                if sub_log == entries {
                    // Returning true because we already contain the entry
                    return true;
                }

                self.log.drain(index..);
            }

            self.log.append(&mut entries.clone());
            true
        }

        pub fn get_log(self: &Self) -> &Vec<(Option<usize>, String)> {
            return &self.log;
        }

        pub fn get_log_from(self: &Self, index: usize) -> Vec<(Option<usize>, String)> {
            self.log[index..].to_vec()
        }

        pub fn get_term(self: &Self, index: usize) -> Option<usize> {
            self.log[index].0
        }

        pub fn get_entry(self: &Self, index: usize) -> &str {
            &self.log[index].1
        }

        pub fn get_latest_log_index(self: &Self) -> usize {
            self.log.len() - 1
        }

        pub fn get_latest_log_term(self: &Self) -> Option<usize> {
            self.log[self.log.len() - 1].0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_to_empty() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(!raft_log.append_entries(0, Some(0), &vec![(Some(1), String::from(""))]));
        assert!(raft_log.append_entries(0, None, &vec![(Some(1), String::from(""))]));
        assert_eq!(raft_log.get_log()[1..], vec![(Some(1), String::from(""))]);
    }

    #[test]
    fn test_add_gap() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(!raft_log.append_entries(1, Some(1), &vec![(Some(1), String::from(""))]));
        assert!(raft_log.append_entries(0, None, &vec![(Some(1), String::from(""))]));
        assert!(!raft_log.append_entries(2, Some(1), &vec![(Some(1), String::from(""))]));
        assert_eq!(raft_log.get_log()[1..], vec![(Some(1), String::from(""))]);
    }

    #[test]
    fn test_add_to_end() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(raft_log.append_entries(0, None, &vec![(Some(1), String::from(""))]));
        assert!(!raft_log.append_entries(1, Some(0), &vec![(Some(2), String::from(""))]));
        assert!(raft_log.append_entries(1, Some(1), &vec![(Some(2), String::from(""))]));
        assert_eq!(
            raft_log.get_log()[1..],
            vec![(Some(1), String::from("")), (Some(2), String::from(""))]
        );
    }

    #[test]
    fn test_add_to_middle() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(raft_log.append_entries(0, None, &vec![(Some(1), String::from(""))]));
        assert!(raft_log.append_entries(
            1,
            Some(1),
            &vec![(Some(2), String::from("")), (Some(2), String::from(""))]
        ));
        assert_eq!(
            raft_log.get_log()[1..],
            vec![
                (Some(1), String::from("")),
                (Some(2), String::from("")),
                (Some(2), String::from(""))
            ]
        );

        assert!(!raft_log.append_entries(1, Some(2), &vec![(Some(3), String::from(""))]));
        assert!(raft_log.append_entries(1, Some(1), &vec![(Some(3), String::from(""))]));
        assert_eq!(
            raft_log.get_log()[1..],
            vec![(Some(1), String::from("")), (Some(3), String::from(""))]
        );
    }

    #[test]
    fn test_add_duplicate() {
        let mut raft_log = raft_log::RaftLog::new();
        assert!(raft_log.append_entries(0, None, &vec![(Some(1), String::from(""))]));
        assert!(raft_log.append_entries(
            1,
            Some(1),
            &vec![(Some(2), String::from("")), (Some(2), String::from(""))]
        ));

        assert!(raft_log.append_entries(1, Some(1), &vec![(Some(2), String::from(""))]));

        // Shouldn't concat in this case
        assert_eq!(
            raft_log.get_log()[1..],
            vec![
                (Some(1), String::from("")),
                (Some(2), String::from("")),
                (Some(2), String::from(""))
            ]
        );
    }
}

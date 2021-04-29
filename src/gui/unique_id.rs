use crate::gui::storage2::Wrapper;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fmt;

#[derive(Clone, Debug, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
pub struct UniqueId(usize);

impl fmt::Display for UniqueId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default, Serialize, Deserialize)]
pub struct UniqueIds {
    available: VecDeque<UniqueId>,
    max: usize,
}

impl Wrapper for () {
    fn wrap(_: UniqueId) -> Self {
        ()
    }
    fn un_wrap(self) -> UniqueId {
        UniqueId(0)
    }
}

impl UniqueIds {
    pub fn get_unique(&mut self) -> UniqueId {
        if let Some(result) = self.available.pop_front() {
            result
        } else {
            let result = UniqueId(self.max);
            self.max += 1;
            result
        }
    }

    pub fn remove_existing(&mut self, id: UniqueId) {
        self.available.push_back(id);
        self.available.make_contiguous().sort();
        while self
            .available
            .back()
            .map(|x| x.0 == self.max - 1)
            .unwrap_or(false)
        {
            self.max -= 1;
            self.available.pop_back().unwrap();
        }
    }
}

#[cfg(test)]
mod id_test {
    use super::*;

    #[test]
    fn test() {
        let mut ids = UniqueIds::default();
        assert_eq!(ids.get_unique().0, 0);
        assert_eq!(ids.get_unique().0, 1);
        assert_eq!(ids.get_unique().0, 2);
        assert_eq!(ids.get_unique().0, 3);
        ids.remove_existing(UniqueId(2));
        assert_eq!(
            ids,
            UniqueIds {
                available: vec![UniqueId(2)].into_iter().collect(),
                max: 4,
            }
        );
        ids.remove_existing(UniqueId(3));
        assert_eq!(
            ids,
            UniqueIds {
                available: vec![].into_iter().collect(),
                max: 2,
            }
        );
        ids.remove_existing(UniqueId(1));
        assert_eq!(
            ids,
            UniqueIds {
                available: vec![].into_iter().collect(),
                max: 1,
            }
        );
        assert_eq!(ids.get_unique().0, 1);
        ids.remove_existing(UniqueId(0));
        assert_eq!(ids.get_unique().0, 0);
    }
}

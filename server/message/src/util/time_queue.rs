#![feature(map_first_last)]
use std::collections::BTreeMap;
use ahash::AHashMap;

pub(crate) struct TimeQueue {
    tree: BTreeMap<u128, u64>,
    id_time: AHashMap<u64, u128>,
}

impl TimeQueue {
    pub fn new() -> Self {
        TimeQueue {
            tree: BTreeMap::new(),
            id_time: AHashMap::new(),
        }
    }

    pub fn push(&mut self, id: u64) {
        let time = common::util::nanos_time();
        match self.id_time.get(&id) {
            None => {
                self.tree.insert(time, id);
                self.id_time.insert(id, time);
            },
            Some(old_time) => {
                self.tree.remove(old_time);
                self.tree.insert(time, id);
                self.id_time.insert(id, time);
            }
        }
    }

    pub fn peek(&self) -> Option<(u128, u64)> {
        match self.tree.first_key_value() {
            None => None,
            Some(entry) => {
                Some((*entry.0, *entry.1))
            }
        }
    }

    pub fn pop(&mut self) -> Option<(u128, u64)> {
        match self.tree.first_entry() {
            None => None,
            Some(entry) => {
                let time = *entry.key();
                let id = *entry.get();
                self.tree.remove(&time);
                self.id_time.remove(&id);
                Some((time, id))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::util::time_queue::TimeQueue;

    #[test]
    fn test() {
        let mut queue = TimeQueue::new();
        queue.push(1);
        queue.push(2);
        queue.push(3);
        queue.push(1);
        queue.push(4);
        queue.push(5);
        queue.push(2);
        println!("{:?}", queue.pop());
        println!("{:?}", queue.pop());
        println!("{:?}", queue.pop());
        println!("{:?}", queue.pop());
        println!("{:?}", queue.pop());
    }
}
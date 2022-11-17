mod client;
mod handler;

use std::cell::RefCell;

use lazy_static::lazy_static;
use lib::net::OuterSender;

lazy_static! {
    static ref SCHEDULER_SENDER: RefCell<Option<OuterSender>> = RefCell::new(None);
}
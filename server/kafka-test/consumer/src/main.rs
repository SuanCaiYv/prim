use std::time::Duration;

use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::util::Timeout;
use rdkafka::{ClientConfig, Message};

#[tokio::main]
async fn main() {
    let consumer1: BaseConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set(
            "bootstrap.servers",
            "localhost:9092,localhost:9093,localhost:9094",
        )
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "700")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "700")
        .create()
        .expect("Consumer creation failed");
    consumer1
        .subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    let consumer2: BaseConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set(
            "bootstrap.servers",
            "localhost:9092,localhost:9093,localhost:9094",
        )
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "700")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "700")
        .create()
        .expect("Consumer creation failed");
    consumer2
        .subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    let consumer3: BaseConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set(
            "bootstrap.servers",
            "localhost:9092,localhost:9093,localhost:9094",
        )
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "700")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "700")
        .create()
        .expect("Consumer creation failed");
    consumer3
        .subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    let consumer4: BaseConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set(
            "bootstrap.servers",
            "localhost:9092,localhost:9093,localhost:9094",
        )
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "700")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "700")
        .create()
        .expect("Consumer creation failed");
    consumer4
        .subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    tokio::spawn(async move {
        match consumer2.poll(Timeout::After(Duration::from_millis(0))) {
            Some(res) => match res {
                Err(e) => {
                    println!("error: {}", e.to_string())
                }
                Ok(msg) => {
                    println!(
                        "2 topic: {}, payload: {}",
                        msg.topic(),
                        String::from_utf8_lossy(msg.payload().unwrap())
                    );
                    // consumer1.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
                }
            },
            None => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    });
    tokio::spawn(async move {
        match consumer3.poll(Timeout::After(Duration::from_millis(0))) {
            Some(res) => match res {
                Err(e) => {
                    println!("error: {}", e.to_string())
                }
                Ok(msg) => {
                    println!(
                        "3 topic: {}, payload: {}",
                        msg.topic(),
                        String::from_utf8_lossy(msg.payload().unwrap())
                    );
                    // consumer1.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
                }
            },
            None => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    });
    tokio::spawn(async move {
        match consumer4.poll(Timeout::After(Duration::from_millis(0))) {
            Some(res) => match res {
                Err(e) => {
                    println!("error: {}", e.to_string())
                }
                Ok(msg) => {
                    println!(
                        "4 topic: {}, payload: {}",
                        msg.topic(),
                        String::from_utf8_lossy(msg.payload().unwrap())
                    );
                    // consumer1.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
                }
            },
            None => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    });
    loop {
        match consumer1.poll(Timeout::After(Duration::from_millis(0))) {
            Some(res) => match res {
                Err(e) => {
                    println!("error: {}", e.to_string())
                }
                Ok(msg) => {
                    println!(
                        "1 topic: {}, payload: {}",
                        msg.topic(),
                        String::from_utf8_lossy(msg.payload().unwrap())
                    );
                    // consumer1.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
                }
            },
            None => {
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    }
}

use rdkafka::{ClientConfig, Message};
use rdkafka::consumer::{Consumer, StreamConsumer};

#[tokio::main]
async fn main() {
    let consumer1: StreamConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set("bootstrap.servers", "localhost:9092,localhost:9093,localhost:9094")
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "1000")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "1000")
        .create()
        .expect("Consumer creation failed");
    consumer1.subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    let consumer2: StreamConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set("bootstrap.servers", "localhost:9092,localhost:9093,localhost:9094")
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "1000")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "1000")
        .create()
        .expect("Consumer creation failed");
    consumer2.subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    let consumer3: StreamConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set("bootstrap.servers", "localhost:9092,localhost:9093,localhost:9094")
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "1000")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "1000")
        .create()
        .expect("Consumer creation failed");
    consumer3.subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    let consumer4: StreamConsumer = ClientConfig::new()
        .set("group.id", "msg-test-1")
        .set("bootstrap.servers", "localhost:9092,localhost:9093,localhost:9094")
        .set("enable.partition.eof", "false")
        .set("enable.auto.commit", "false")
        .set("session.timeout.ms", "1000")
        .set("heartbeat.interval.ms", "200")
        .set("max.poll.interval.ms", "1000")
        .create()
        .expect("Consumer creation failed");
    consumer4.subscribe(&["msg-test"])
        .expect("Can't subscribe to specified topics");
    tokio::spawn(async move {
        loop {
            match consumer2.recv().await {
                Err(e) => {
                    println!("error: {}", e.to_string())
                }
                Ok(msg) => {
                    println!("4 topic: {}, payload: {}", msg.topic(), String::from_utf8_lossy(msg.payload().unwrap()));
                    // consumer4.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
                }
            }
        }
    });
    tokio::spawn(async move {
        loop {
            match consumer3.recv().await {
                Err(e) => {
                    println!("error: {}", e.to_string())
                }
                Ok(msg) => {
                    println!("4 topic: {}, payload: {}", msg.topic(), String::from_utf8_lossy(msg.payload().unwrap()));
                    // consumer4.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
                }
            }
        }
    });
    tokio::spawn(async move {
        loop {
            match consumer4.recv().await {
                Err(e) => {
                    println!("error: {}", e.to_string())
                }
                Ok(msg) => {
                    println!("4 topic: {}, payload: {}", msg.topic(), String::from_utf8_lossy(msg.payload().unwrap()));
                    // consumer4.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
                }
            }
        }
    });
    loop {
        match consumer1.recv().await {
            Err(e) => {
                println!("error: {}", e.to_string())
            }
            Ok(msg) => {
                println!("4 topic: {}, payload: {}", msg.topic(), String::from_utf8_lossy(msg.payload().unwrap()));
                // consumer4.commit_message(&msg, rdkafka::consumer::CommitMode::Sync).unwrap();
            }
        }
    }
}

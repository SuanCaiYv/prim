use lib::util::timestamp;
use rdkafka::{producer::{FutureProducer, FutureRecord}, ClientConfig, util::Timeout};

#[tokio::main]
async fn main() {
    let producer: FutureProducer = ClientConfig::new()
        .set("bootstrap.servers", "localhost:9092,localhost:9093,localhost:9094")
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");
    for i in 0..30 {
        let key = timestamp().to_string();
        let msg = format!("msg-{:03}", i);
        let res = producer.send(
            FutureRecord::to("msg-test")
                .key(&key)
                .payload(msg.as_bytes()),
            Timeout::Never,
        );
        match res.await {
            Ok(res) => {
                println!("partition: {}, offset: {}", res.0, res.1)
            }
            Err(e) => {
                println!("error: {}", e.0.to_string())
            }
        }
    }
}
use rdkafka::ClientConfig;
use rdkafka::admin::{AdminClient, AdminOptions, ResourceSpecifier};
use rdkafka::client::DefaultClientContext;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::util::Timeout;
use lib::util::timestamp;

#[tokio::main]
async fn main() {
    let mut client_config = ClientConfig::new();
    client_config.set("bootstrap.servers", "localhost:9092,localhost:9093,localhost:9094");

    let admin_client: AdminClient<DefaultClientContext> = client_config.create().unwrap();
    let admin_options = AdminOptions::new().operation_timeout(Some(std::time::Duration::from_secs(5)));

    let topic_metadata = admin_client.describe_configs(vec![&ResourceSpecifier::Topic("msg-test1")], &admin_options).await.unwrap();
    let item = &topic_metadata[0];
    println!("topic: {:?}", item.unwrap().entries);
    // let producer: FutureProducer = ClientConfig::new()
    //     .set("bootstrap.servers", "localhost:9092,localhost:9093,localhost:9094")
    //     .set("message.timeout.ms", "5000")
    //     .create()
    //     .expect("Producer creation error");
    // for i in 90..120 {
    //     let key = timestamp().to_string();
    //     let msg = format!("msg-{:3}", i);
    //     let res = producer.send(
    //         FutureRecord::to("msg-test")
    //             .key(&key)
    //             .payload(msg.as_bytes()),
    //         Timeout::Never,
    //     );
    //     match res.await {
    //         Ok(res) => {
    //             println!("partition: {}, offset: {}", res.0, res.1)
    //         }
    //         Err(e) => {
    //             println!("error: {}", e.0.to_string())
    //         }
    //     }
    // }
}
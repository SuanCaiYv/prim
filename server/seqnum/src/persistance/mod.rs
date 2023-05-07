use lib::Result;
use tokio::sync::mpsc;

pub(self) static mut APPENDER_SENDER: Option<mpsc::Sender<(u64, u64, u64)>> = None;

pub(crate) async fn new_seq_num(user_id: u64, peer_id: u64, seq_num: u64) -> Result<()> {
    unsafe {
        if let Some(sender) = &APPENDER_SENDER {
            sender.send((user_id, peer_id, seq_num)).await?;
        }
    }
    Ok(())
}

pub(crate) fn persistance_seq_num_start() -> Result<()> {
    let (tx, mut rx) = mpsc::channel(16384);
    unsafe {
        APPENDER_SENDER = Some(tx);
    };
    tokio::spawn(async move {
        let mut counter: u64 = 0;
        while let Some((user_id, peer_id, seq_num)) = rx.recv().await {
            counter += 1;
            if counter % 1000 == 0 {}
        };
    });
    Ok(())
}
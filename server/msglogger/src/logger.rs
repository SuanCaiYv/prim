use std::ops::Sub;

use lib::{entity::Msg, Result};

#[inline(always)]
pub(crate) async fn logger(msg: Msg, file: &mut monoio::fs::File) -> Result<()> {
    let (res, _) = file.write_all_at(msg.0, 0).await;
    res?;
    Ok(())
}

/// clear log file of the day before 7 days
pub(crate) fn clear_log(id: usize) -> Result<()> {
    let prefix = chrono::Local::now()
        .sub(chrono::Duration::days(7))
        .date_naive()
        .format("%Y-%m-%d")
        .to_string();
    let path = format!("./msglog/{}-{}.log", prefix, id);
    _ = std::fs::remove_file(path);
    Ok(())
}

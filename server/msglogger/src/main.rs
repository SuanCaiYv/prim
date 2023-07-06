use sysinfo::SystemExt;

mod logger;
mod recv;

fn main() {
    _ = std::fs::create_dir_all("./msglog");
    let sys = sysinfo::System::new_all();
    for id in 1..sys.cpus().len() {
        std::thread::spawn(move || {
            #[cfg(target_os = "linux")]
            let _ = monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
                .with_entries(16384)
                .enable_timer()
                .build()
                .unwrap()
                .block_on(recv::start(id));
            #[cfg(target_os = "macos")]
            let _ = monoio::RuntimeBuilder::<monoio::LegacyDriver>::new()
                .enable_timer()
                .build()
                .unwrap()
                .block_on(recv::start(id));
        });
    }
    #[cfg(target_os = "linux")]
    let _ = monoio::RuntimeBuilder::<monoio::IoUringDriver>::new()
        .with_entries(16384)
        .enable_timer()
        .build()
        .unwrap()
        .block_on(recv::start(0));
    #[cfg(target_os = "macos")]
    let _ = monoio::RuntimeBuilder::<monoio::LegacyDriver>::new()
        .enable_timer()
        .build()
        .unwrap()
        .block_on(recv::start(0));
}

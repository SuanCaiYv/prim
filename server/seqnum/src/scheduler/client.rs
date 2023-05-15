use lib::Result;

pub(super) struct Client {}

impl Client {
    pub(super) async fn run() -> Result<()> {
        let addresses = &CONFIG.scheduler.addresses;
        let index = my_id() as usize % addresses.len();
        let scheduler_address = addresses[index].clone();
        let mut config_builder = ClientConfigBuilder::default();
        config_builder
            .with_remote_address(scheduler_address)
            .with_ipv4_type(CONFIG.server.cluster_address.is_ipv4())
            .with_domain(CONFIG.scheduler.domain.clone())
            .with_cert(CONFIG.scheduler.cert.clone())
            .with_keep_alive_interval(CONFIG.transport.keep_alive_interval)
            .with_max_bi_streams(CONFIG.transport.max_bi_streams);
        let client_config = config_builder.build().unwrap();
        let mut client = ClientTimeout::new(client_config, std::time::Duration::from_millis(3000));
        client.run().await?;

        let mut service_address = CONFIG.server.service_address;
        service_address.set_ip(CONFIG.server.service_ip.parse().unwrap());
        let mut cluster_address = CONFIG.server.cluster_address;
        cluster_address.set_ip(CONFIG.server.cluster_ip.parse().unwrap());
        let server_info = ServerInfo {
            id: my_id(),
            service_address,
            cluster_address: Some(cluster_address),
            connection_id: 0,
            status: ServerStatus::Online,
            typ: ServerType::SchedulerClient,
            load: None,
        };
        // todo: multi net streams open, now only one.
        let (sender, receiver, timeout) = client.io_channel_server_info(&server_info, 0).await?;
        unsafe {
            SCHEDULER_SENDER = Some(lib::net::MsgSender::Client(sender.clone()));
        }

        let mut handler_list: Vec<Box<dyn Handler>> = Vec::new();
        handler_list.push(Box::new(logic::ClientAuth {}));
        handler_list.push(Box::new(NodeRegister {}));
        handler_list.push(Box::new(NodeUnregister {}));
        let handler_list = HandlerList::new(handler_list);
        let io_task_sender = get_io_task_sender().clone();
        let mut inner_states = AHashMap::new();

        if let Err(e) = super::handler::handler_func(
            lib::net::MsgSender::Client(sender),
            receiver,
            timeout,
            io_task_sender,
            &handler_list,
            &mut inner_states,
        )
        .await
        {
            error!("handler_func error: {}", e);
        }
        Ok(())
    }
}

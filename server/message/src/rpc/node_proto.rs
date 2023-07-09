#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CurrNodeGroupIdUserListReq {
    #[prost(uint32, tag = "1")]
    pub node_id: u32,
    #[prost(uint64, tag = "2")]
    pub group_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct CurrNodeGroupIdUserListResp {
    #[prost(uint64, repeated, tag = "1")]
    pub user_list: ::prost::alloc::vec::Vec<u64>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WhichNodeReq {
    #[prost(uint64, tag = "1")]
    pub user_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WhichNodeResp {
    #[prost(uint32, tag = "1")]
    pub node_id: u32,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AllGroupNodeListReq {
    #[prost(uint64, tag = "1")]
    pub group_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AllGroupNodeListResp {
    #[prost(uint32, repeated, tag = "1")]
    pub node_list: ::prost::alloc::vec::Vec<u32>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushMsgReq {
    #[prost(uint64, tag = "1")]
    pub sender: u64,
    #[prost(uint64, tag = "2")]
    pub receiver: u64,
    #[prost(uint64, tag = "3")]
    pub timestamp: u64,
    #[prost(uint32, tag = "4")]
    pub version: u32,
    #[prost(uint32, tag = "5")]
    pub r#type: u32,
    #[prost(string, tag = "6")]
    pub payload: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub extension: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushMsgResp {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(string, tag = "2")]
    pub err_msg: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WhichToConnectReq {
    #[prost(uint64, tag = "1")]
    pub user_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct WhichToConnectResp {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SeqnumNodeAddressReq {
    #[prost(uint64, tag = "1")]
    pub node_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SeqnumNodeAddressResp {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
    #[prost(uint64, tag = "2")]
    pub node_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SeqnumNodeUserSelectReq {
    #[prost(uint64, tag = "1")]
    pub user_id1: u64,
    #[prost(uint64, tag = "2")]
    pub user_id2: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SeqnumNodeUserSelectResp {
    #[prost(uint64, tag = "1")]
    pub node_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MessageNodeAliveReq {
    #[prost(uint64, tag = "1")]
    pub node_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MessageNodeAliveResp {
    #[prost(bool, tag = "1")]
    pub alive: bool,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GroupUserListReq {
    #[prost(uint64, tag = "1")]
    pub group_id: u64,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GroupUserListResp {
    #[prost(uint64, repeated, tag = "1")]
    pub user_list: ::prost::alloc::vec::Vec<u64>,
}
/// Generated client implementations.
pub mod scheduler_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct SchedulerClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl SchedulerClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> SchedulerClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> SchedulerClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            SchedulerClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn curr_node_group_id_user_list(
            &mut self,
            request: impl tonic::IntoRequest<super::CurrNodeGroupIdUserListReq>,
        ) -> std::result::Result<
            tonic::Response<super::CurrNodeGroupIdUserListResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/CurrNodeGroupIdUserList",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(
                    GrpcMethod::new("node_proto.Scheduler", "CurrNodeGroupIdUserList"),
                );
            self.inner.unary(req, path, codec).await
        }
        pub async fn which_node(
            &mut self,
            request: impl tonic::IntoRequest<super::WhichNodeReq>,
        ) -> std::result::Result<tonic::Response<super::WhichNodeResp>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/WhichNode",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.Scheduler", "WhichNode"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn all_group_node_list(
            &mut self,
            request: impl tonic::IntoRequest<super::AllGroupNodeListReq>,
        ) -> std::result::Result<
            tonic::Response<super::AllGroupNodeListResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/AllGroupNodeList",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.Scheduler", "AllGroupNodeList"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn push_msg(
            &mut self,
            request: impl tonic::IntoRequest<super::PushMsgReq>,
        ) -> std::result::Result<tonic::Response<super::PushMsgResp>, tonic::Status> {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/PushMsg",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.Scheduler", "PushMsg"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn which_to_connect(
            &mut self,
            request: impl tonic::IntoRequest<super::WhichToConnectReq>,
        ) -> std::result::Result<
            tonic::Response<super::WhichToConnectResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/WhichToConnect",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.Scheduler", "WhichToConnect"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn seqnum_node_address(
            &mut self,
            request: impl tonic::IntoRequest<super::SeqnumNodeAddressReq>,
        ) -> std::result::Result<
            tonic::Response<super::SeqnumNodeAddressResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/SeqnumNodeAddress",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.Scheduler", "SeqnumNodeAddress"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn seqnum_node_user_select(
            &mut self,
            request: impl tonic::IntoRequest<super::SeqnumNodeUserSelectReq>,
        ) -> std::result::Result<
            tonic::Response<super::SeqnumNodeUserSelectResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/SeqnumNodeUserSelect",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.Scheduler", "SeqnumNodeUserSelect"));
            self.inner.unary(req, path, codec).await
        }
        pub async fn message_node_alive(
            &mut self,
            request: impl tonic::IntoRequest<super::MessageNodeAliveReq>,
        ) -> std::result::Result<
            tonic::Response<super::MessageNodeAliveResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.Scheduler/MessageNodeAlive",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.Scheduler", "MessageNodeAlive"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated client implementations.
pub mod api_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    #[derive(Debug, Clone)]
    pub struct ApiClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ApiClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ApiClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ApiClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + Send + Sync,
        {
            ApiClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        pub async fn group_user_list(
            &mut self,
            request: impl tonic::IntoRequest<super::GroupUserListReq>,
        ) -> std::result::Result<
            tonic::Response<super::GroupUserListResp>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::new(
                        tonic::Code::Unknown,
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/node_proto.API/GroupUserList",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("node_proto.API", "GroupUserList"));
            self.inner.unary(req, path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod scheduler_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with SchedulerServer.
    #[async_trait]
    pub trait Scheduler: Send + Sync + 'static {
        async fn curr_node_group_id_user_list(
            &self,
            request: tonic::Request<super::CurrNodeGroupIdUserListReq>,
        ) -> std::result::Result<
            tonic::Response<super::CurrNodeGroupIdUserListResp>,
            tonic::Status,
        >;
        async fn which_node(
            &self,
            request: tonic::Request<super::WhichNodeReq>,
        ) -> std::result::Result<tonic::Response<super::WhichNodeResp>, tonic::Status>;
        async fn all_group_node_list(
            &self,
            request: tonic::Request<super::AllGroupNodeListReq>,
        ) -> std::result::Result<
            tonic::Response<super::AllGroupNodeListResp>,
            tonic::Status,
        >;
        async fn push_msg(
            &self,
            request: tonic::Request<super::PushMsgReq>,
        ) -> std::result::Result<tonic::Response<super::PushMsgResp>, tonic::Status>;
        async fn which_to_connect(
            &self,
            request: tonic::Request<super::WhichToConnectReq>,
        ) -> std::result::Result<
            tonic::Response<super::WhichToConnectResp>,
            tonic::Status,
        >;
        async fn seqnum_node_address(
            &self,
            request: tonic::Request<super::SeqnumNodeAddressReq>,
        ) -> std::result::Result<
            tonic::Response<super::SeqnumNodeAddressResp>,
            tonic::Status,
        >;
        async fn seqnum_node_user_select(
            &self,
            request: tonic::Request<super::SeqnumNodeUserSelectReq>,
        ) -> std::result::Result<
            tonic::Response<super::SeqnumNodeUserSelectResp>,
            tonic::Status,
        >;
        async fn message_node_alive(
            &self,
            request: tonic::Request<super::MessageNodeAliveReq>,
        ) -> std::result::Result<
            tonic::Response<super::MessageNodeAliveResp>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct SchedulerServer<T: Scheduler> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Scheduler> SchedulerServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for SchedulerServer<T>
    where
        T: Scheduler,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/node_proto.Scheduler/CurrNodeGroupIdUserList" => {
                    #[allow(non_camel_case_types)]
                    struct CurrNodeGroupIdUserListSvc<T: Scheduler>(pub Arc<T>);
                    impl<
                        T: Scheduler,
                    > tonic::server::UnaryService<super::CurrNodeGroupIdUserListReq>
                    for CurrNodeGroupIdUserListSvc<T> {
                        type Response = super::CurrNodeGroupIdUserListResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::CurrNodeGroupIdUserListReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).curr_node_group_id_user_list(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = CurrNodeGroupIdUserListSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/node_proto.Scheduler/WhichNode" => {
                    #[allow(non_camel_case_types)]
                    struct WhichNodeSvc<T: Scheduler>(pub Arc<T>);
                    impl<T: Scheduler> tonic::server::UnaryService<super::WhichNodeReq>
                    for WhichNodeSvc<T> {
                        type Response = super::WhichNodeResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::WhichNodeReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move { (*inner).which_node(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = WhichNodeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/node_proto.Scheduler/AllGroupNodeList" => {
                    #[allow(non_camel_case_types)]
                    struct AllGroupNodeListSvc<T: Scheduler>(pub Arc<T>);
                    impl<
                        T: Scheduler,
                    > tonic::server::UnaryService<super::AllGroupNodeListReq>
                    for AllGroupNodeListSvc<T> {
                        type Response = super::AllGroupNodeListResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::AllGroupNodeListReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).all_group_node_list(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = AllGroupNodeListSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/node_proto.Scheduler/PushMsg" => {
                    #[allow(non_camel_case_types)]
                    struct PushMsgSvc<T: Scheduler>(pub Arc<T>);
                    impl<T: Scheduler> tonic::server::UnaryService<super::PushMsgReq>
                    for PushMsgSvc<T> {
                        type Response = super::PushMsgResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PushMsgReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move { (*inner).push_msg(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PushMsgSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/node_proto.Scheduler/WhichToConnect" => {
                    #[allow(non_camel_case_types)]
                    struct WhichToConnectSvc<T: Scheduler>(pub Arc<T>);
                    impl<
                        T: Scheduler,
                    > tonic::server::UnaryService<super::WhichToConnectReq>
                    for WhichToConnectSvc<T> {
                        type Response = super::WhichToConnectResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::WhichToConnectReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).which_to_connect(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = WhichToConnectSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/node_proto.Scheduler/SeqnumNodeAddress" => {
                    #[allow(non_camel_case_types)]
                    struct SeqnumNodeAddressSvc<T: Scheduler>(pub Arc<T>);
                    impl<
                        T: Scheduler,
                    > tonic::server::UnaryService<super::SeqnumNodeAddressReq>
                    for SeqnumNodeAddressSvc<T> {
                        type Response = super::SeqnumNodeAddressResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SeqnumNodeAddressReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).seqnum_node_address(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SeqnumNodeAddressSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/node_proto.Scheduler/SeqnumNodeUserSelect" => {
                    #[allow(non_camel_case_types)]
                    struct SeqnumNodeUserSelectSvc<T: Scheduler>(pub Arc<T>);
                    impl<
                        T: Scheduler,
                    > tonic::server::UnaryService<super::SeqnumNodeUserSelectReq>
                    for SeqnumNodeUserSelectSvc<T> {
                        type Response = super::SeqnumNodeUserSelectResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::SeqnumNodeUserSelectReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).seqnum_node_user_select(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SeqnumNodeUserSelectSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/node_proto.Scheduler/MessageNodeAlive" => {
                    #[allow(non_camel_case_types)]
                    struct MessageNodeAliveSvc<T: Scheduler>(pub Arc<T>);
                    impl<
                        T: Scheduler,
                    > tonic::server::UnaryService<super::MessageNodeAliveReq>
                    for MessageNodeAliveSvc<T> {
                        type Response = super::MessageNodeAliveResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MessageNodeAliveReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).message_node_alive(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = MessageNodeAliveSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Scheduler> Clone for SchedulerServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: Scheduler> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Scheduler> tonic::server::NamedService for SchedulerServer<T> {
        const NAME: &'static str = "node_proto.Scheduler";
    }
}
/// Generated server implementations.
pub mod api_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    /// Generated trait containing gRPC methods that should be implemented for use with ApiServer.
    #[async_trait]
    pub trait Api: Send + Sync + 'static {
        async fn group_user_list(
            &self,
            request: tonic::Request<super::GroupUserListReq>,
        ) -> std::result::Result<
            tonic::Response<super::GroupUserListResp>,
            tonic::Status,
        >;
    }
    #[derive(Debug)]
    pub struct ApiServer<T: Api> {
        inner: _Inner<T>,
        accept_compression_encodings: EnabledCompressionEncodings,
        send_compression_encodings: EnabledCompressionEncodings,
        max_decoding_message_size: Option<usize>,
        max_encoding_message_size: Option<usize>,
    }
    struct _Inner<T>(Arc<T>);
    impl<T: Api> ApiServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
                max_decoding_message_size: None,
                max_encoding_message_size: None,
            }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
        /// Enable decompressing requests with the given encoding.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.accept_compression_encodings.enable(encoding);
            self
        }
        /// Compress responses with the given encoding, if the client supports it.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.send_compression_encodings.enable(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.max_decoding_message_size = Some(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.max_encoding_message_size = Some(limit);
            self
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ApiServer<T>
    where
        T: Api,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(
            &mut self,
            _cx: &mut Context<'_>,
        ) -> Poll<std::result::Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/node_proto.API/GroupUserList" => {
                    #[allow(non_camel_case_types)]
                    struct GroupUserListSvc<T: Api>(pub Arc<T>);
                    impl<T: Api> tonic::server::UnaryService<super::GroupUserListReq>
                    for GroupUserListSvc<T> {
                        type Response = super::GroupUserListResp;
                        type Future = BoxFuture<
                            tonic::Response<Self::Response>,
                            tonic::Status,
                        >;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GroupUserListReq>,
                        ) -> Self::Future {
                            let inner = Arc::clone(&self.0);
                            let fut = async move {
                                (*inner).group_user_list(request).await
                            };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let max_decoding_message_size = self.max_decoding_message_size;
                    let max_encoding_message_size = self.max_encoding_message_size;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GroupUserListSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec)
                            .apply_compression_config(
                                accept_compression_encodings,
                                send_compression_encodings,
                            )
                            .apply_max_message_size_config(
                                max_decoding_message_size,
                                max_encoding_message_size,
                            );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => {
                    Box::pin(async move {
                        Ok(
                            http::Response::builder()
                                .status(200)
                                .header("grpc-status", "12")
                                .header("content-type", "application/grpc")
                                .body(empty_body())
                                .unwrap(),
                        )
                    })
                }
            }
        }
    }
    impl<T: Api> Clone for ApiServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
                max_decoding_message_size: self.max_decoding_message_size,
                max_encoding_message_size: self.max_encoding_message_size,
            }
        }
    }
    impl<T: Api> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(Arc::clone(&self.0))
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: Api> tonic::server::NamedService for ApiServer<T> {
        const NAME: &'static str = "node_proto.API";
    }
}

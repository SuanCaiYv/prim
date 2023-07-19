## Pure Rust Instant Message(PRIM)

### 概述

使用纯Rust实现的即时通讯系统。

- 服务端：Rust
- 后端：Rust
- 客户端：Rust+TS

关于客户端，原本钦定了Electron+Vue3，后面看到Tauri(一个使用Rust实现的类似Electron的跨平台框架)；考虑到和服务端契合度以及代码可复用(偷懒了属于是)，所以切换到了这里。

#### 依赖

- ##### Database：PostGreSQL

- ##### NoSQL：Redis

- **Web: Salvo**

- **RPC: Tonic**

- **HttpClient: Reqwest**

- **Core: QUIC**

- **Runtime: Tokio+Monoio**

### [系统架构](./doc/1.md)

### [细节实现](./doc/2.md)

### [客户端实现](./doc/3.md)

### 效果

#### 登录注册

![image-20230719213947927](./doc/image-20230719213947927.png)

#### 主页面

![image-20230719235035721](./doc/image-20230719235035721.png)

#### 添加好友

![image-20230719235158717](./doc/image-20230719235158717.png)

#### 消息



#### 好友列表

![image-20230719234712822](./doc/image-20230719234712822.png)



### 待办

 - [ ] 撤回
 - [ ] 发送失败回执
 - [ ] 群聊
 - [ ] 发送文件，表情，图片，视频，音频
 - [ ] 

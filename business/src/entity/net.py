from enum import Enum

HEAD_LENGTH = 37


class Type(Enum):
    NA = 0
    # 消息部分
    Text = 1
    Meme = 2
    File = 3
    Image = 4
    Video = 5
    Audio = 6
    # 逻辑部分
    Ack = 7
    Box = 8
    Auth = 9
    Sync = 10
    Error = 11
    Offline = 12
    Heartbeat = 13
    UnderReview = 14
    InternalError = 15
    # 业务部分
    AddFriend = 16
    SysNotification = 17


class Head:
    length: int
    type: Type
    sender: int
    receiver: int
    timestamp: int
    seq_num: int
    version: int

    @staticmethod
    def from_bytes(content: bytes):
        pass

    def to_bytes(self):
        pass


class Msg:
    head: Head
    body: bytes

    @staticmethod
    def from_bytes(content: bytes):
        pass

    def to_bytes(self):
        pass

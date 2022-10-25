from enum import Enum

from numpy import uint16, uint64

import src.util.base

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
    FriendRelationship = 16
    SysNotification = 17


class Head:
    length: uint16
    type: Type
    sender: uint64
    receiver: uint64
    timestamp: uint64
    seq_num: uint64
    version: uint16

    def __init__(self, length: uint16, type: Type, sender: uint64, receiver: uint64,
                 timestamp: uint64, seq_num: uint64, version: uint16):
        self.length = length
        self.type = type
        self.sender = sender
        self.receiver = receiver
        self.timestamp = timestamp
        self.seq_num = seq_num
        self.version = version

    @staticmethod
    def from_bytes(content: bytes):
        length = int.from_bytes(content[0:2], byteorder='big')
        type = Type(int.from_bytes(content[2:3], byteorder='big'))
        sender = int.from_bytes(content[3:11], byteorder='big')
        receiver = int.from_bytes(content[11:19], byteorder='big')
        timestamp = int.from_bytes(content[19:27], byteorder='big')
        seq_num = int.from_bytes(content[27:35], byteorder='big')
        version = int.from_bytes(content[35:37], byteorder='big')
        return Head(uint16(length), type, uint64(sender), uint64(receiver), uint64(timestamp),
                    uint64(seq_num), uint16(version))

    def to_bytes(self) -> bytes:
        length = int(self.length)
        type = int(self.type.value)
        sender = int(self.sender)
        receiver = int(self.receiver)
        timestamp = int(self.timestamp)
        seq_num = int(self.seq_num)
        version = int(self.version)
        array = bytearray(HEAD_LENGTH)
        array[0:2] = int(length).to_bytes(2, byteorder='big')
        array[2:3] = int(type).to_bytes(1, byteorder='big')
        array[3:11] = int(sender).to_bytes(8, byteorder='big')
        array[11:19] = int(receiver).to_bytes(8, byteorder='big')
        array[19:27] = int(timestamp).to_bytes(8, byteorder='big')
        array[27:35] = int(seq_num).to_bytes(8, byteorder='big')
        array[35:37] = int(version).to_bytes(2, byteorder='big')
        return bytes(array)

    def __str__(self):
        return 'length: {}, type: {}, sender: {}, receiver: {}, timestamp: {}, seq_num: {}, version: {}'.format(
            self.length, self.type, self.sender, self.receiver, self.timestamp, self.seq_num, self.version)


class Msg:
    head: Head
    body: bytes

    def __init__(self, head: Head, body: bytes):
        self.head = head
        self.body = body

    @staticmethod
    def from_bytes(content: bytes):
        head = Head.from_bytes(content[0:HEAD_LENGTH])
        body = content[HEAD_LENGTH:]
        return Msg(head, body)

    def to_bytes(self) -> bytes:
        array = bytearray(HEAD_LENGTH + len(self.body))
        array[0:HEAD_LENGTH] = self.head.to_bytes()
        array[HEAD_LENGTH:] = self.body
        return array

    @staticmethod
    def friend_relationship(sender, receiver: int, info: str):
        payload = bytes(info, encoding='utf-8')
        head = Head(uint16(len(payload)), Type.FriendRelationship, uint64(sender), uint64(receiver),
                    uint64(src.util.base.timestamp()), uint64(0), uint16(0))
        return Msg(head, payload)

    def __str__(self):
        return 'head: {}, body: {}'.format(self.head, self.body)

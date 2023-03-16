import { timestamp } from "../util/base";

const HEAD_LEN = 32;
const EXTENSION_THRESHOLD = 1 << 6 - 1;
const PAYLOAD_THRESHOLD = 1 << 14 - 1;

enum Type {
    NA = 0,
    /// this type can only be used for acknowledging certain msg.
    /// it's so special that we put it on the top of the enum.
    Ack = 1,

    /// the below types are used for user's communication.
    ///
    /// pure message part
    Text = 32,
    Meme = 33,
    File = 34,
    Image = 35,
    Video = 36,
    Audio = 37,
    /// control message part
    Edit = 64,
    Withdraw = 65,

    /// the below types are used for user and server's communication.
    ///
    /// logic part
    Auth = 96,
    Ping = 97,
    Pong = 98,
    Echo = 99,
    Error = 100,
    BeOffline = 101,
    InternalError = 102,
    /// business part
    /// some types may derived by user but send between server, those types are also viewed as business type.
    SystemMessage = 128,
    AddFriend = 129,
    RemoveFriend = 130,
    JoinGroup = 131,
    LeaveGroup = 132,
    RemoteInvoke = 133,
    SetRelationship = 134,

    /// the below types are used for server's communication.
    ///
    /// internal part
    /// this part should never be visible to the user end.
    Noop = 160,
    InterruptSignal = 161,
    UserNodeMapChange = 162,
    MessageNodeRegister = 163,
    MessageNodeUnregister = 164,
    RecorderNodeRegister = 165,
    RecorderNodeUnregister = 166,
    SchedulerNodeRegister = 167,
    SchedulerNodeUnregister = 168,
}

class Head {
    version: number;
    sender: bigint;
    nodeId: number;
    receiver: bigint;
    type: Type;
    extensionLength: number;
    timestamp: bigint;
    payloadLength: number;
    seqNum: bigint;

    constructor(
        version: number,
        sender: bigint,
        nodeId: number,
        receiver: bigint,
        type: Type,
        extensionLength: number,
        timestamp: bigint,
        payloadLength: number,
        seqNum: bigint,
    ) {
        this.version = version;
        this.sender = sender;
        this.nodeId = nodeId;
        this.receiver = receiver;
        this.type = type;
        this.extensionLength = extensionLength;
        this.timestamp = timestamp;
        this.payloadLength = payloadLength;
        this.seqNum = seqNum;
    }

    static fromArrayBuffer = (buffer: ArrayBuffer): Head => {
        let view = new DataView(buffer);
        let versionSender = view.getBigUint64(0, false);
        let nodeIdReceiver = view.getBigUint64(8, false);
        let typeExtensionLengthTimestamp = view.getBigUint64(16, false);
        let payloadLengthWithSeqNum = view.getBigUint64(24, false);
        let version = Number(versionSender >> 46n);
        let sender = BigInt(versionSender & 0x3ffffffffffn);
        let nodeId = Number(nodeIdReceiver >> 46n);
        let receiver = BigInt(nodeIdReceiver & 0x3ffffffffffn);
        let type = Number(typeExtensionLengthTimestamp >> 52n) as Type;
        let extensionLength = Number(typeExtensionLengthTimestamp >> 46n & 0x3fn);
        let timestamp = BigInt(typeExtensionLengthTimestamp & 0x3ffffffffffn);
        let payloadLength = Number(payloadLengthWithSeqNum >> 50n);
        let seqNum = BigInt(payloadLengthWithSeqNum & 0x3fffffffffffn);
        return new Head(
            version,
            sender,
            nodeId,
            receiver,
            type,
            extensionLength,
            timestamp,
            payloadLength,
            seqNum,
        );
    }

    toArrayBuffer(): ArrayBuffer {
        let buffer = new ArrayBuffer(HEAD_LEN);
        let view = new DataView(buffer);
        let versionSender = BigInt(this.version) << 46n | BigInt(this.sender);
        let nodeIdReceiver = BigInt(this.nodeId) << 46n | BigInt(this.receiver)
        let typeExtensionLengthTimestamp = BigInt(this.type) << 52n
            | BigInt(this.extensionLength) << 46n
            | BigInt(this.timestamp);
        let payloadLengthWithSeqNum = BigInt(this.payloadLength) << 50n | BigInt(this.seqNum);
        view.setBigUint64(0, BigInt(versionSender), false);
        view.setBigUint64(8, BigInt(nodeIdReceiver), false);
        view.setBigUint64(16, BigInt(typeExtensionLengthTimestamp), false);
        view.setBigUint64(24, BigInt(payloadLengthWithSeqNum), false);
        return buffer;
    }
}

class Msg {
    head: Head;
    payload: ArrayBuffer;
    extension: ArrayBuffer;

    constructor(head: Head, payload: ArrayBuffer, extension: ArrayBuffer) {
        this.head = head;
        this.payload = payload;
        this.extension = extension;
    }

    static fromArrayBuffer = (buffer: ArrayBuffer): Msg => {
        let head = Head.fromArrayBuffer(buffer.slice(0, HEAD_LEN));
        let payload = buffer.slice(HEAD_LEN, HEAD_LEN + head.payloadLength);
        let extension = buffer.slice(HEAD_LEN + head.payloadLength);
        return new Msg(head, payload, extension);
    }

    toArrayBuffer = (): ArrayBuffer => {
        let buffer = new ArrayBuffer(HEAD_LEN + this.head.payloadLength + this.head.extensionLength);
        let head = new Uint8Array(this.head.toArrayBuffer());
        let view = new DataView(buffer);
        for (let i = 0; i < HEAD_LEN; i++) {
            view.setUint8(i, head[i]);
        }
        let payload = new Uint8Array(this.payload);
        for (let i = 0; i < this.head.payloadLength; i++) {
            view.setUint8(HEAD_LEN + i, payload[i]);
        }
        let extension = new Uint8Array(this.extension);
        for (let i = 0; i < this.head.extensionLength; i++) {
            view.setUint8(HEAD_LEN + this.head.payloadLength + i, extension[i]);
        }
        return buffer;
    }

    payloadText = (): string => {
        return new TextDecoder().decode(this.payload);
    }

    static text = (sender: bigint, receiver: bigint, nodeId: number, text: string): Msg => {
        let payload = new TextEncoder().encode(text);
        let head = new Head(0, sender, nodeId, receiver, Type.Text, 0, timestamp(), payload.length, 0n);
        return new Msg(head, payload, new ArrayBuffer(0));
    }

    static text0 = (sender: bigint, receiver: bigint, nodeId: number, text: string, timestamp: bigint): Msg => {
        let payload = new TextEncoder().encode(text);
        let head = new Head(0, sender, nodeId, receiver, Type.Text, 0, timestamp, payload.length, 0n);
        return new Msg(head, payload, new ArrayBuffer(0));
    }
}

export { Type, Head, Msg };
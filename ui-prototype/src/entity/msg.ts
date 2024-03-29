import { timestamp } from "../util/base";

// @ts-ignore
const BIT_MASK_LEFT_46: bigint = 0xFFFF_C000_0000_0000n;
const BIT_MASK_RIGHT_46: bigint = 0x0000_3FFF_FFFF_FFFFn;
// @ts-ignore
const BIT_MASK_LEFT_50: bigint = 0xFFFC_0000_0000_0000n;
const BIT_MASK_RIGHT_50: bigint = 0x0003_FFFF_FFFF_FFFFn;
// @ts-ignore
const BIT_MASK_LEFT_12: bigint = 0xFFF0_0000_0000_0000n;
const BIT_MASK_RIGHT_12: bigint = 0x000F_FFFF_FFFF_FFFFn;

const HEAD_LEN = 32;
// @ts-ignore
const EXTENSION_THRESHOLD = 1 << 6 - 1;
// @ts-ignore
const PAYLOAD_THRESHOLD = 1 << 14 - 1;
// user_id lager than(also equal) this value is considered as a group
const GROUP_ID_THRESHOLD: bigint = BigInt(1 << 36);

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
    seqnum: bigint;

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
        this.seqnum = seqNum;
    }

    static fromArrayBuffer = (buffer: ArrayBuffer): Head => {
        let view = new DataView(buffer);
        let versionSender = view.getBigUint64(0, false);
        let nodeIdReceiver = view.getBigUint64(8, false);
        let typeExtensionLengthTimestamp = view.getBigUint64(16, false);
        let payloadLengthWithSeqNum = view.getBigUint64(24, false);
        let version = Number(versionSender >> 46n);
        let sender = BigInt(versionSender & BIT_MASK_RIGHT_46);
        let nodeId = Number(nodeIdReceiver >> 46n);
        let receiver = BigInt(nodeIdReceiver & BIT_MASK_RIGHT_46);
        let type = Number(typeExtensionLengthTimestamp >> 52n) as Type;
        let extensionLength = Number((typeExtensionLengthTimestamp & BIT_MASK_RIGHT_12) >> 46n);
        let timestamp = BigInt(typeExtensionLengthTimestamp & BIT_MASK_RIGHT_46);
        let payloadLength = Number(payloadLengthWithSeqNum >> 50n);
        let seqNum = BigInt(payloadLengthWithSeqNum & BIT_MASK_RIGHT_50);
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
        let payloadLengthWithSeqNum = BigInt(this.payloadLength) << 50n | BigInt(this.seqnum);
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

    extensionText = (): string => {
        return new TextDecoder().decode(this.extension);
    }

    static text = (sender: bigint, receiver: bigint, nodeId: number, text: string): Msg => {
        let payload = new TextEncoder().encode(text);
        let head = new Head(0, sender, nodeId, receiver, Type.Text, 0, timestamp(), payload.length, 0n);
        return new Msg(head, payload, new ArrayBuffer(0));
    }

    static text2 = (sender: bigint, receiver: bigint, nodeId: number, text: string, extension: string): Msg => {
        let payload = new TextEncoder().encode(text);
        let extensionArrayBuffer = new TextEncoder().encode(extension);
        let head = new Head(0, sender, nodeId, receiver, Type.Text, extension.length, timestamp(), payload.length, 0n);
        return new Msg(head, payload, extensionArrayBuffer);
    }

    static text0 = (sender: bigint, receiver: bigint, nodeId: number, text: string, timestamp: bigint): Msg => {
        let payload = new TextEncoder().encode(text);
        let head = new Head(0, sender, nodeId, receiver, Type.Text, 0, timestamp, payload.length, 0n);
        return new Msg(head, payload, new ArrayBuffer(0));
    }
}

export { Type, Head, Msg, GROUP_ID_THRESHOLD };
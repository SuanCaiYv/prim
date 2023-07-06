package msg

import (
	"encoding/binary"
	"time"
)

type Type = uint16

const (
	HeadLen            int    = 32
	ExtensionThreshold int    = 1<<6 - 1
	PayloadThreshold   int    = 1<<14 - 1
	GroupIdThreshold   uint64 = 1 << 36
)

const (
	BitMaskLeft46  uint64 = 0xFFFF_C000_0000_0000
	BitMaskRight46 uint64 = 0x00003FFFFFFFFF
	BitMaskLeft50  uint64 = 0xFFFC000000000000
	BitMaskRight50 uint64 = 0x0003FFFFFFFFFFFF
	BitMaskLeft12  uint64 = 0xFFF0000000000000
	BitMaskRight12 uint64 = 0x000FFFFFFFFFFFFF
)

const (
	NA              Type = 0
	Ack             Type = 1
	Text            Type = 32
	Meme            Type = 33
	File            Type = 34
	Image           Type = 35
	Video           Type = 36
	Audio           Type = 37
	Edit            Type = 64
	Withdraw        Type = 65
	Auth            Type = 96
	Ping            Type = 97
	Pong            Type = 98
	Echo            Type = 99
	Error           Type = 100
	BeOffline       Type = 101
	InternalError   Type = 102
	SystemMessage   Type = 128
	AddFriend       Type = 129
	RemoveFriend    Type = 130
	JoinGroup       Type = 131
	LeaveGroup      Type = 132
	RemoteInvoke    Type = 133
	SetRelationship Type = 134
	Noop            Type = 160
	Close           Type = 161
)

type Msg struct {
	inner []byte
}

type Head struct {
	versionSender          uint64
	nodeIdReceiver         uint64
	typeExtensionTimestamp uint64
	payloadSeqnum          uint64
}

type innerHead struct {
	version         uint32
	sender          uint64
	nodeId          uint32
	receiver        uint64
	typ             Type
	extensionLength uint8
	timestamp       uint64
	payloadLength   uint16
	seqnum          uint64
}

func (h *Head) toInnerHead() innerHead {
	version := uint32(h.versionSender >> 46)
	sender := h.versionSender & BitMaskRight46
	nodeId := uint32(h.nodeIdReceiver >> 46)
	receiver := h.nodeIdReceiver & BitMaskRight46
	typ := Type(h.typeExtensionTimestamp >> 52)
	extensionLength := uint8((h.typeExtensionTimestamp & BitMaskRight12) >> 46)
	timestamp := h.typeExtensionTimestamp & BitMaskRight46
	payloadLength := uint16(h.payloadSeqnum >> 50)
	seqnum := h.payloadSeqnum & BitMaskRight50
	return innerHead{
		version:         version,
		sender:          sender,
		nodeId:          nodeId,
		receiver:        receiver,
		typ:             typ,
		extensionLength: extensionLength,
		timestamp:       timestamp,
		payloadLength:   payloadLength,
		seqnum:          seqnum,
	}
}

func (i *innerHead) toHead() Head {
	versionSender := uint64(i.version) << 46
	versionSender |= i.sender
	nodeIdReceiver := uint64(i.nodeId) << 46
	nodeIdReceiver |= i.receiver
	typeExtensionTimestamp := uint64(i.typ) << 52
	typeExtensionTimestamp |= uint64(i.extensionLength) << 46
	typeExtensionTimestamp |= i.timestamp
	payloadSeqnum := uint64(i.payloadLength) << 50
	payloadSeqnum |= i.seqnum
	return Head{
		versionSender:          versionSender,
		nodeIdReceiver:         nodeIdReceiver,
		typeExtensionTimestamp: typeExtensionTimestamp,
		payloadSeqnum:          payloadSeqnum,
	}
}

func (i *innerHead) toBytes() []byte {
	versionSender := uint64(i.version) << 46
	versionSender |= i.sender
	nodeIdReceiver := uint64(i.nodeId) << 46
	nodeIdReceiver |= i.receiver
	typeExtensionTimestamp := uint64(i.typ) << 52
	typeExtensionTimestamp |= uint64(i.extensionLength) << 46
	typeExtensionTimestamp |= i.timestamp
	payloadSeqnum := uint64(i.payloadLength) << 50
	payloadSeqnum |= i.seqnum
	bytes := make([]byte, HeadLen, HeadLen)
	binary.BigEndian.PutUint64(bytes[0:8], versionSender)
	binary.BigEndian.PutUint64(bytes[8:16], nodeIdReceiver)
	binary.BigEndian.PutUint64(bytes[16:24], typeExtensionTimestamp)
	binary.BigEndian.PutUint64(bytes[24:32], payloadSeqnum)
	return bytes
}

func Bytes2Head(bytes []byte) Head {
	versionSender := binary.BigEndian.Uint64(bytes[0:8])
	nodeIdReceiver := binary.BigEndian.Uint64(bytes[8:16])
	typeExtensionTimestamp := binary.BigEndian.Uint64(bytes[16:24])
	payloadSeqnum := binary.BigEndian.Uint64(bytes[24:32])
	return Head{
		versionSender:          versionSender,
		nodeIdReceiver:         nodeIdReceiver,
		typeExtensionTimestamp: typeExtensionTimestamp,
		payloadSeqnum:          payloadSeqnum,
	}
}

func PreAlloc(head *Head) Msg {
	extensionLength := int((head.typeExtensionTimestamp & BitMaskRight12) >> 46)
	payloadLength := int(head.payloadSeqnum >> 50)
	inner := make([]byte, HeadLen+extensionLength+payloadLength, HeadLen+extensionLength+payloadLength)
	binary.BigEndian.PutUint64(inner[0:8], head.versionSender)
	binary.BigEndian.PutUint64(inner[8:16], head.nodeIdReceiver)
	binary.BigEndian.PutUint64(inner[16:24], head.typeExtensionTimestamp)
	binary.BigEndian.PutUint64(inner[24:32], head.payloadSeqnum)
	return Msg{inner: inner}
}

func (m *Msg) AsBytes() []byte {
	return m.inner
}

func TextMsg(sender uint64, receiver uint64, nodeId uint32, text string) Msg {
	innerHead := innerHead{
		version:         1,
		sender:          sender,
		nodeId:          nodeId,
		receiver:        receiver,
		typ:             Text,
		extensionLength: 0,
		timestamp:       uint64(time.Now().UnixMilli()),
		payloadLength:   uint16(len(text)),
		seqnum:          0,
	}
	inner := make([]byte, HeadLen+len(text), HeadLen+len(text))
	copy(inner[0:HeadLen], innerHead.toBytes())
	copy(inner[HeadLen:], text)
	return Msg{inner: inner}
}

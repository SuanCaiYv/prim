package reqwest

import "encoding/binary"

type ReqwestResourceID = uint16

const (
	Noop ReqwestResourceID = iota
	Ping
	Pong
	Seqnum
	NodeAuth
	MessageForward
	InterruptSignal
	ConnectionTimeout
	SeqnumNodeRegister
	MessageNodeRegister
	SeqnumNodeUnregister
	MessageNodeUnregister
	SchedulerNodeRegister
	SchedulerNodeUnregister
	MsgprocessorNodeRegister
	MsgprocessorNodeUnregister
	MessageConfigHotReload
	AssignMQProcessor
	UnassignMQProcessor
)

type ReqwestMsg struct {
	inner []byte
}

func PreAlloc(length uint16) ReqwestMsg {
	r := make([]byte, length+2, length+2)
	binary.BigEndian.PutUint16(r, length)
	return ReqwestMsg{
		inner: r,
	}
}

func Raw(inner []byte) ReqwestMsg {
	return ReqwestMsg{inner: inner}
}

func (r *ReqwestMsg) AsSlice() []byte {
	return r.inner
}

func (r *ReqwestMsg) Length() uint16 {
	return binary.BigEndian.Uint16((r.inner)[:2])
}

func (r *ReqwestMsg) ReqId() uint64 {
	return binary.BigEndian.Uint64((r.inner)[2:10])
}

func (r *ReqwestMsg) SetReqId(reqId uint64) {
	binary.BigEndian.PutUint64((r.inner)[2:10], reqId)
}

func (r *ReqwestMsg) ResourceId() ReqwestResourceID {
	return binary.BigEndian.Uint16((r.inner)[10:12])
}

func (r *ReqwestMsg) SetResourceId(resourceId ReqwestResourceID) {
	binary.BigEndian.PutUint16((r.inner)[10:12], resourceId)
}

func (r *ReqwestMsg) Payload() []byte {
	return (r.inner)[12:]
}

func (r *ReqwestMsg) Body() []byte {
	return (r.inner)[2:]
}

func WithResourceIdPayload(resourceId ReqwestResourceID, payload []byte) ReqwestMsg {
	inner := make([]byte, len(payload)+12, len(payload)+12)
	binary.BigEndian.PutUint16(inner[0:2], uint16(len(payload)+10))
	binary.BigEndian.PutUint16(inner[2:10], 0)
	binary.BigEndian.PutUint16(inner[10:12], resourceId)
	copy(inner[12:], payload)
	return ReqwestMsg{
		inner: inner,
	}
}

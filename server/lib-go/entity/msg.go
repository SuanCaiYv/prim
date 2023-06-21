package entity

import "encoding/binary"

type ReqwestMsg = []byte

func PreAlloc(length uint16) ReqwestMsg {
	r := make([]byte, length+2, length+2)
	binary.BigEndian.PutUint16(r, length)
	return r
}

func (r *ReqwestMsg) AsSlice() []byte {
	return *r
}

func (r *ReqwestMsg) Length() uint16 {
	return binary.BigEndian.Uint16((*r)[:2])
}

func (r *ReqwestMsg) ReqId() uint64 {
	return binary.BigEndian.Uint64((*r)[2:10])
}

func (r *ReqwestMsg) SetReqId(reqId uint64) {
	binary.BigEndian.PutUint64((*r)[2:10], reqId)
}

func (r *ReqwestMsg) ResourceId() uint16 {
	return binary.BigEndian.Uint16((*r)[10:12])
}

func (r *ReqwestMsg) SetResourceId(resourceId uint16) {
	binary.BigEndian.PutUint16((*r)[10:12], resourceId)
}

func (r *ReqwestMsg) Payload() []byte {
	return (*r)[12:]
}

func (r *ReqwestMsg) Body() []byte {
	return (*r)[2:]
}

func WithResourceIdPayload(resourceId uint16, payload []byte) ReqwestMsg {
	r := PreAlloc(uint16(len(payload)) + 12)
	binary.BigEndian.PutUint16(r[0:2], uint16(len(payload)+10))
	binary.BigEndian.PutUint16(r[2:10], 0)
	binary.BigEndian.PutUint16(r[10:12], resourceId)
	copy(r[12:], payload)
	return r
}

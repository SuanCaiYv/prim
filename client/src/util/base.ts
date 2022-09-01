import {get} from "idb-keyval";
import {Constant} from "../system/constant";

const startTimeMillSeconds = performance.now()
const startTimeUnixTimestamp = Date.now()

const i64ToByteArray = (long: number): Uint8Array => {
    let byteArray = new Uint8Array(8)

    for (let index = byteArray.length - 1; index >= 0; index--) {
        let byte = long & 0xff;
        byteArray [index] = byte;
        long = (long - byte) / 256;
    }

    return byteArray;
}

const i32ToByteArray = (long: number): Uint8Array => {
    let byteArray = new Uint8Array(4)

    for (let index = byteArray.length - 1; index >= 0; index--) {
        let byte = long & 0xff;
        byteArray [index] = byte;
        long = (long - byte) / 256;
    }

    return byteArray;
}

const i16ToByteArray = (long: number): Uint8Array => {
    let byteArray = new Uint8Array(2)

    for (let index = byteArray.length - 1; index >= 0; index--) {
        let byte = long & 0xff;
        byteArray [index] = byte;
        long = (long - byte) / 256;
    }

    return byteArray;
}

const byteArrayToI64 = (byteArray: Uint8Array): number => {
    let value = 0;
    for (let i = 0; i < byteArray.length; i++) {
        value = (value * 256) + byteArray[i];
    }

    return value;
}

const byteArrayToI32 = (byteArray: Uint8Array): number => {
    let value = 0;
    for (let i = 0; i < byteArray.length; i++) {
        value = (value * 256) + byteArray[i];
    }

    return value;
}

const byteArrayToI16 = (byteArray: Uint8Array): number => {
    let value = 0;
    for (let i = 0; i < byteArray.length; i++) {
        value = (value * 256) + byteArray[i];
    }

    return value;
}

const whoWhereAre = (id1: number, id2: number): string => {
    if (id1 < id2) {
        return id1 + "-" + id2;
    } else {
        return id2 + "-" + id1;
    }
}

const timestamp = (): number => {
    return performance.now() - startTimeMillSeconds + startTimeUnixTimestamp;
}

const checkNull = (value: any): boolean => {
    return value === null || value === undefined || value === 0 || value === '';
}

const whoWeAre = (id1: number, id2: number): string => {
    return id1 < id2 ? (id1 + '-' + id2) : (id2 + '-' + id1)
}

const whichIsNotMe = async (id1: number, id2: number): Promise<number> => {
    const accountId = await get(Constant.AccountId)
    if (accountId === id1) {
        return id2
    } else {
        return id1
    }
}

export {i64ToByteArray, byteArrayToI64, i32ToByteArray, byteArrayToI32,
    i16ToByteArray, byteArrayToI16, whoWeAre, whichIsNotMe, timestamp, checkNull};
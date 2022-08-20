function i64ToByteArray(long: number): Uint8Array {
    let byteArray = new Uint8Array(8)

    for (let index = byteArray.length - 1; index >= 0; index--) {
        let byte = long & 0xff;
        byteArray [index] = byte;
        long = (long - byte) / 256;
    }

    return byteArray;
}

function i32ToByteArray(long: number): Uint8Array {
    let byteArray = new Uint8Array(4)

    for (let index = byteArray.length - 1; index >= 0; index--) {
        let byte = long & 0xff;
        byteArray [index] = byte;
        long = (long - byte) / 256;
    }

    return byteArray;
}

function i16ToByteArray(long: number): Uint8Array {
    let byteArray = new Uint8Array(2)

    for (let index = byteArray.length - 1; index >= 0; index--) {
        let byte = long & 0xff;
        byteArray [index] = byte;
        long = (long - byte) / 256;
    }

    return byteArray;
}

function byteArrayToI64(byteArray: Uint8Array): number {
    let value = 0;
    for (let i = 0; i < byteArray.length; i++) {
        value = (value * 256) + byteArray[i];
    }

    return value;
}

function byteArrayToI32(byteArray: Uint8Array): number {
    let value = 0;
    for (let i = 0; i < byteArray.length; i++) {
        value = (value * 256) + byteArray[i];
    }

    return value;
}

function byteArrayToI16(byteArray: Uint8Array): number {
    let value = 0;
    for (let i = 0; i < byteArray.length; i++) {
        value = (value * 256) + byteArray[i];
    }

    return value;
}

function whoWhereAre(id1: number, id2: number): string {
    if (id1 < id2) {
        return id1 + "-" + id2;
    } else {
        return id2 + "-" + id1;
    }
}

function timestamp(): number {
    return Date.now();
}

export {i64ToByteArray, byteArrayToI64, i32ToByteArray, byteArrayToI32,
    i16ToByteArray, byteArrayToI16, whoWhereAre, timestamp};
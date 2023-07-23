const StartTimeMillSeconds = performance.now()
const StartTimeUnixTimestamp = Date.now()

const timestamp = (): bigint => {
    let ts = performance.now() - StartTimeMillSeconds + StartTimeUnixTimestamp;
    return BigInt(Math.floor(ts));
}

const checkNull = (value: any): boolean => {
    return value === null || value === undefined || value === 0 || value === '';
}

const buffer2Array = (buffer: ArrayBuffer): Array<number> => {
    let arr = new Array<number>(buffer.byteLength);
    let buf = new Uint8Array(buffer);
    for (let i = 0; i < buffer.byteLength; ++ i) {
        arr[i] = Number(buf.at(i))
    }
    return arr;
}

const array2Buffer = (array: Array<number>): ArrayBuffer => {
    let buffer = new ArrayBuffer(array.length);
    let view = new DataView(buffer);
    for (let i = 0; i < array.length; ++ i) {
        view.setInt8(i, array[i])
    }
    return buffer;
}

export { timestamp, checkNull, buffer2Array, array2Buffer}
const StartTimeMillSeconds = performance.now()
const StartTimeUnixTimestamp = Date.now()

const timestamp = (): bigint => {
    let ts = performance.now() - StartTimeMillSeconds + StartTimeUnixTimestamp;
    return BigInt(Math.floor(ts));
}

const checkNull = (value: any): boolean => {
    return value === null || value === undefined || value === 0 || value === '';
}

export { timestamp, checkNull}
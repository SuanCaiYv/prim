class BlockQueue<T> {
    producer: Array<Promise<T>>;
    consumer: Array<Promise<T>>;

    constructor() {
        this.producer = new Array<Promise<T>>();
        this.consumer = new Array<Promise<T>>();
    }

    _push() {
        this.consumer.push(new Promise((resolve) => {
            // @ts-ignore
            this.producer.push(resolve)
        }));
    }

    push(item: T) {
        if (!this.producer.length) {
            this._push();
        }
        let resolve = this.producer.shift();
        if (resolve === undefined) {
            throw new Error("BlockQueue.push: unexpected error");
        } else {
            // @ts-ignore
            resolve(item);
        }
    }

    pop(): Promise<T> {
        if (!this.consumer.length) {
            this._push();
        }
        let res = this.consumer.shift();
        if (res === undefined) {
            throw new Error("BlockQueue.pop: unexpected error");
        } else {
            return res;
        }
    }
}

export { BlockQueue }
export interface Response {
    ok: boolean
    errCode: number
    errMsg: string
    data: object
    timestamp: Date
}

export interface ListResult {
    total: number,
    count: number,
    pageNum: number,
    pageSize: number,
    nextPageNum: number,
    endPage: boolean,
    list: Array<any>,
}

class ListResultClass implements ListResult {
    count: number;
    endPage: boolean;
    list: Array<object>;
    nextPageNum: number;
    pageNum: number;
    pageSize: number;
    total: number;

    constructor(count: number, endPage: boolean, list: Array<object>, nextPageNum: number, pageNum: number, pageSize: number, total: number) {
        this.count = count;
        this.endPage = endPage;
        this.list = list;
        this.nextPageNum = nextPageNum;
        this.pageNum = pageNum;
        this.pageSize = pageSize;
        this.total = total;
    }
}

export class KV<K, V> {
    key: K
    value: V

    constructor(key: K, value: V) {
        this.key = key;
        this.value = value;
    }
}
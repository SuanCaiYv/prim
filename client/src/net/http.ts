import axios, { AxiosResponse } from "axios"
import Response from "../entity/http"
import { KVDB } from "../service/database"

export const BASE_URL = 'https://127.0.0.1:11130'

axios.defaults.timeout = 2000

class ResponseClass implements Response {
    ok: boolean
    errCode: number
    errMsg: string
    data: any
    timestamp: Date

    constructor() {
        this.ok = false
        this.errCode = 0
        this.errMsg = ""
        this.data = {}
        this.timestamp = new Date()
    }
}

class HttpClient {
    static get = async (uri: string, query: any, auth: boolean): Promise<Response> => {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length - 1)
        }
        if (auth) {
            const token = await KVDB.get('access-token') as string
            return await axios.get(url, {
                headers: {
                    'Authorization': token,
                }
            }).then(dealResp).finally(() => { dealResp(undefined) });
        } else {
            return axios.get(url).then(dealResp);
        }
    }
    static post = async (uri: string, query: any, params: any, auth: boolean): Promise<Response> => {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length - 1)
        }
        if (auth) {
            const token = await KVDB.get('access-token') as string
            return await axios.post(url, params, {
                headers: {
                    'Authorization': token,
                    'Content-Type': "application/json"
                }
            }).then(dealResp).catch(() => { return dealResp(undefined) });
        } else {
            return axios.post(url, params, {
                headers: {
                    "Content-Type": "application/json"
                }
            }).then(dealResp).catch(() => { return dealResp(undefined) });
        }
    }
    static put = async (uri: string, query: any, params: any, auth: boolean): Promise<Response> => {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length - 1)
        }
        if (auth) {
            const token = await KVDB.get('access-token') as string
            return await axios.put(url, params, {
                headers: {
                    'Authorization': token,
                    'Content-Type': 'application/json'
                }
            }).then(dealResp).catch(() => { return dealResp(undefined) });
        } else {
            return await axios.put(url, params, {
                headers: {
                    'Content-Type': 'application/json'
                }
            }).then(dealResp).catch(() => { return dealResp(undefined) });
        }
    }
    static delete = async (uri: string, query: any, auth: boolean): Promise<Response> => {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length - 1)
        }
        if (auth) {
            const token = await KVDB.get('access-token') as string
            return await axios.delete(url, {
                headers: {
                    "Authorization": token,
                }
            }).then(dealResp).catch(() => { return dealResp(undefined) });
        } else {
            return axios.delete(url).then(dealResp).catch(() => { return dealResp(undefined) });
        }
    }
}

const dealResp = (resp: AxiosResponse | undefined): ResponseClass => {
    let r = new ResponseClass()
    if (resp === null || resp === undefined) {
        r.ok = false
        r.errCode = 500
        r.errMsg = "Server Error!"
        r.data = undefined
        r.timestamp = new Date()
        return r
    }
    if (resp.status === 200) {
        let rawData = resp.data
        r.ok = rawData.code === 200
        r.errCode = rawData.code
        r.errMsg = rawData.msg
        r.data = rawData.data
        r.timestamp = new Date(rawData.timestamp)
    } else {
        r.ok = false
        r.errCode = 500
        r.errMsg = "Server Error!"
        r.data = undefined
        r.timestamp = new Date()
    }
    return r
}

export { ResponseClass, HttpClient }
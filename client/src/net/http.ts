import { invoke } from "@tauri-apps/api"
import axios, { AxiosResponse } from "axios"
import Response from "../entity/http"
import { KVDB } from "../service/database"

export const BASE_URL = '127.0.0.1:11130'

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
        let headers = {}
        if (auth) {
            const token = await KVDB.get('access-token');
            headers = {
                'Authorization': token,
            }
        }
        return await invoke<any>('http_get', {
            params: {
                host: BASE_URL,
                uri: uri,
                query: query,
                headers: headers
            }
        }).then(dealResp).catch((e) => { return dealResp(e) });
    }
    static post = async (uri: string, query: any, params: any, auth: boolean): Promise<Response> => {
        let headers;
        if (auth) {
            const token = await KVDB.get('access-token');
            headers = {
                'Authorization': token,
                'Content-Type': "application/json"
            }
        } else {
            headers = {
                "Content-Type": "application/json"
            }
        }
        return await invoke<any>('http_post', {
            params: {
                host: BASE_URL,
                uri: uri,
                query: query,
                headers: headers,
                body: params,
            }
        }).then(dealResp).catch((e) => { return dealResp(e) });
    }
    static put = async (uri: string, query: any, params: any, auth: boolean): Promise<Response> => {
        let headers;
        if (auth) {
            const token = await KVDB.get('access-token');
            headers = {
                'Authorization': token,
                'Content-Type': "application/json"
            }
        } else {
            headers = {
                "Content-Type": "application/json"
            }
        }
        return await invoke<any>('http_put', {
            params: {
                host: BASE_URL,
                uri: uri,
                query: query,
                headers: headers,
                body: params,
            }
        }).then(dealResp).catch((e) => { return dealResp(e) });
    }
    static delete = async (uri: string, query: any, auth: boolean): Promise<Response> => {
        let headers = {}
        if (auth) {
            const token = await KVDB.get('access-token')
            headers = {
                'Authorization': token,
            }
        }
        return await invoke<any>('http_delete', {
            params: {
                host: BASE_URL,
                uri: uri,
                query: query,
                headers: headers
            }
        }).then(dealResp).catch((e) => { return dealResp(e) });
    }
}

const dealResp = (resp: any | string): ResponseClass => {
    let r = new ResponseClass()
    if (typeof resp === 'string') {
        console.log(resp as string);
        r.ok = false
        r.errCode = 500
        r.errMsg = "Server Error!"
        r.data = undefined
        r.timestamp = new Date()
        return r
    }
    r.ok = resp.code === 200
    r.errCode = resp.code
    r.errMsg = resp.message
    r.data = resp.data
    r.timestamp = new Date(resp.timestamp)
    return r
}

export { ResponseClass, HttpClient }
import axios, {AxiosResponse} from "axios";
import {Response} from "./interface";
import {get} from "idb-keyval";

export const BASE_URL = "http://127.0.0.1:8290"

axios.defaults.timeout = 2000

class ResponseClass implements Response {
    ok: boolean
    errCode: number
    errMsg: string
    data: object
    timestamp: Date

    constructor() {
        this.ok = false
        this.errCode = 0
        this.errMsg = ""
        this.data = {}
        this.timestamp = new Date()
    }
}

const httpClient = {
    get: async function <T extends object>(uri: string, query: T, auth: boolean): Promise<Response> {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length-1)
        }
        if (auth) {
            const token = await get('Token')
            return await axios.get(url, {
                headers: {
                    'Authorization': token,
                }
            }).then(dealResp)
        } else {
            return axios.get(url).then(dealResp);
        }
    },
    post: async function <T extends object>(uri: string, query: T, params: T, auth: boolean): Promise<Response> {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length-1)
        }
        if (auth) {
            const token = await get('Token')
            return await axios.post(url, params, {
                headers: {
                    'Authorization': token,
                    'Content-Type': "application/json"
                }
            }).then(dealResp);
        } else {
            return axios.post(url, params, {
                headers: {
                    "Content-Type": "application/json"
                }
            }).then(dealResp);
        }
    },
    put: async function <T extends object>(uri: string, query: T, params: T, auth: boolean): Promise<Response> {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length-1)
        }
        if (auth) {
            const token = await get('Token')
            return await axios.put(url, params, {
                headers: {
                    'Authorization': token,
                    'Content-Type': 'application/json'
                }
            }).then(dealResp);
        } else {
            return await axios.put(url, params, {
                headers: {
                    'Content-Type': 'application/json'
                }
            }).then(dealResp)
        }
    },
    delete: async function <T extends object>(uri: string, query: T, auth: boolean) {
        let str = ""
        for (let field in query) {
            str += (field + "=" + query[field] + "&")
        }
        let url = BASE_URL + uri
        if (str.length > 0) {
            url += "?" + str.substring(0, str.length-1)
        }
        if (auth) {
            const token = await get('Token')
            return await axios.delete(url, {
                headers: {
                    "Authorization": token,
                }
            }).then(dealResp)
        } else {
            return axios.delete(url).then(dealResp)
        }
    },
}

const dealResp = function (resp: AxiosResponse): ResponseClass {
    let r = new ResponseClass()
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
        r.data = {}
        r.timestamp = new Date()
    }
    return r
}

export {httpClient, ResponseClass}
export interface ElectronApi {
    connect: Function
}

declare global {
    interface Window {
        electronApi: ElectronApi
    }
}
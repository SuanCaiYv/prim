const storage = {
    set: function (k: string, v: string) {
        localStorage.setItem(k, v)
    },
    setOnce: function (k: string, v: string) {
        if (localStorage.getItem(k) !== null) {
            return
        }
        this.set(k, v)
    },
    get: function (k: string): string {
        const v = localStorage.getItem(k)
        if (v === null) {
            return ""
        } else {
            return v
        }
    },
    remove: function (k: string) {
        localStorage.removeItem(k)
    },
    removeAll: function () {
        localStorage.clear()
    }
}

export default storage
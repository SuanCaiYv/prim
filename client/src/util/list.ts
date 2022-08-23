export class Node<T> {
    value: any
    score: number
    prev: Node<T> | null
    next: Node<T> | null

    constructor(value: T, score: number = 0) {
        this.value = value;
        this.score = score;
        this.prev = null;
        this.next = null;
    }
}

export class List<T> {
    head: Node<T> | null
    tail: Node<T> | null
    size: number

    constructor() {
        this.head = null;
        this.tail = null;
        this.size = 0;
    }

    public push_back(value: T, score: number = 0) {
        // @ts-ignore
        let node = new Node(value, score);
        if (this.size === 0) {
            this.head = node;
            this.tail = node;
        } else {
            // @ts-ignore
            this.tail.next = node;
            // @ts-ignore
            node.prev = this.tail;
            this.tail = node;
        }
        this.size ++;
    }

    public push_front(value: T, score: number = 0) {
        // @ts-ignore
        let node = new Node(value, score);
        if (this.size === 0) {
            this.head = node;
            this.tail = node;
        } else {
            // @ts-ignore
            this.head.prev = node;
            // @ts-ignore
            node.next = this.head;
            this.head = node;
        }
        this.size ++;
    }

    public push_front_suit(value: T, score: number = 0): void {
        let node = new Node(value);
        let head = this.head;
        while (head != null) {
            if (head.score === score) {
                break;
            }
            if (head.value > value) {
                this.insert_before(head, value, score);
                break;
            }
            head = head.next;
        }
    }

    public pop_back(): Node<T> | null {
        if (this.size === 0) {
            return null;
        }
        // @ts-ignore
        let value = this.tail;
        if (this.size === 1) {
            this.head = null;
            this.tail = null;
        } else {
            let old_tail = this.tail;
            // @ts-ignore
            this.tail = this.tail.prev;
            // @ts-ignore
            this.tail.next = null;
            // @ts-ignore
            old_tail.prev = null;
        }
        this.size --;
        return value;
    }

    public pop_front(): Node<T> | null {
        if (this.size === 0) {
            return null;
        }
        // @ts-ignore
        let value = this.head;
        if (this.size === 1) {
            this.head = null;
            this.tail = null;
        } else {
            let old_head = this.head;
            // @ts-ignore
            this.head = this.head.next;
            // @ts-ignore
            this.head.prev = null;
            // @ts-ignore
            old_head.next = null;
        }
        this.size --;
        return value;
    }

    public front(): T | null {
        if (this.size === 0) {
            return null;
        }
        // @ts-ignore
        return this.head.value;
    }

    public back(): T | null {
        if (this.size === 0) {
            return null;
        }
        // @ts-ignore
        return this.tail.value;
    }

    public empty(): boolean {
        return this.size === 0;
    }

    public insert_before(pos: Node<T>, value: any, score: number = 0): void {
        if (pos == null) {
            return;
        }
        // @ts-ignore
        let node = new Node(value);
        // @ts-ignore
        if (pos.prev === null) {
            this.head = node;
            // @ts-ignore
            node.next = pos;
        } else {
            // @ts-ignore
            pos.prev.next = node;
            // @ts-ignore
            node.prev = pos.prev;
            // @ts-ignore
            pos.prev = node;
            // @ts-ignore
            node.next = pos;
        }
        this.size ++;
    }

    public insert_after(pos: Node<T>, value: any, score: number = 0): void {
        if (pos == null) {
            return;
        }
        // @ts-ignore
        let node = new Node(value);
        // @ts-ignore
        if (pos.next === null) {
            this.tail = node;
            // @ts-ignore
            node.prev = pos;
        } else {
            // @ts-ignore
            pos.next.prev = node;
            // @ts-ignore
            node.next = pos.next;
            // @ts-ignore
            pos.next = node;
            // @ts-ignore
            node.prev = pos;
        }
        this.size ++;
    }
}
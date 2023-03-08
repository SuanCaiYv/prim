import {Msg} from '../entity/msg'
import { timestamp } from '../util/base'

let TextSet = [
    'Hello, world!',
    'Hi, there!',
    'Hello',
    'Hi',
    'How are you?',
    'I\'m fine, thank you.',
    'I\'m fine, thanks.',
    'I\'m fine.',
    'I\'m fine, and you?',
    'What\'s up?',
    'What\'s new?',
    'What\'s going on?',
    'What\'s happening?',
    'Fine, thanks.',
    'Fine, thank you.',
    'All good.',
    'All good, thanks.',
    'All good, thank you.',
    'Long time no see.',
    'Quite a long time.',
    'So long.',
    'Zzz...',
    'Pardon?',
    'Lol',
    'Meh',
    'Haha',
    'Rofl',
]

let senderSet = [
    2n,
    3n,
    4n,
    5n,
    6n,
]

let MsgList = [
    Msg.text0(1n, 2n, 1, TextSet[0], timestamp() - 12000n),
    Msg.text0(1n, 2n, 1, TextSet[1], timestamp() - 11100n),
    Msg.text0(1n, 3n, 1, TextSet[2], timestamp() - 11000n),
    Msg.text0(2n, 1n, 1, TextSet[3], timestamp() - 10000n),
    Msg.text0(2n, 1n, 1, TextSet[4], timestamp() - 9000n),
    Msg.text0(2n, 1n, 1, TextSet[5], timestamp() - 8000n),
    Msg.text0(4n, 1n, 1, TextSet[6], timestamp() - 8500n),
    Msg.text0(5n, 1n, 1, TextSet[7], timestamp() - 8000n),
    Msg.text0(4n, 1n, 1, TextSet[8], timestamp() - 7000n),
    Msg.text0(6n, 1n, 1, TextSet[9], timestamp() - 6000n),
    Msg.text0(1n, 2n, 1, TextSet[10], timestamp() - 5000n),
    Msg.text0(1n, 3n, 1, TextSet[11], timestamp() - 4000n),
    Msg.text0(1n, 4n, 1, TextSet[12], timestamp() - 3000n),
    Msg.text0(1n, 5n, 1, TextSet[13], timestamp() - 2000n),
    Msg.text0(1n, 6n, 1, TextSet[14], timestamp() - 1500n),
    Msg.text0(5n, 1n, 1, TextSet[15], timestamp() - 1000n),
    Msg.text0(6n, 1n, 1, TextSet[16], timestamp() - 500n),
    Msg.text0(1n, 2n, 1, TextSet[17], timestamp() - 400n),
    Msg.text0(1n, 3n, 1, TextSet[18], timestamp() - 300n),
    Msg.text0(1n, 4n, 1, TextSet[19], timestamp() - 200n),
    Msg.text0(1n, 5n, 1, TextSet[20], timestamp() - 100n),
    Msg.text0(1n, 6n, 1, TextSet[21], timestamp() - 50n),
]

let index = 0;

const randomMsg = (): Msg => {
    let msg = MsgList[index % MsgList.length]
    ++ index
    return msg
}

export {MsgList, randomMsg}
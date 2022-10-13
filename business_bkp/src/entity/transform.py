import datetime


class Resp:
    code: int
    msg: str
    timestamp: datetime.datetime
    data: any

    def __init__(self, code: int, msg: str, data=None):
        self.code = code
        self.msg = msg
        self.data = data
        self.timestamp = datetime.datetime.now()

    def to_dict(self) -> dict:
        return {
            'code': self.code,
            'msg': self.msg,
            'timestamp': self.timestamp,
            'data': self.data
        }

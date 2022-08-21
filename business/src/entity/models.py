import datetime
import time

from sqlalchemy import Column, BigInteger, String, LargeBinary, JSON, CHAR, ARRAY, TIMESTAMP

from db.pgsql import Base


class User(Base):
    __table_args__ = {'schema': 'business'}
    __tablename__ = 'user'
    id = Column(BigInteger, primary_key=True, autoincrement=True, nullable=False)
    account_id = Column(BigInteger, nullable=False)
    nickname = Column(CHAR(64), nullable=False)
    credential = Column(String(128), nullable=False)
    salt = Column(CHAR(12), nullable=False)
    role = Column(ARRAY(CHAR(12)), nullable=False)
    delete_at = Column(TIMESTAMP(timezone=True), nullable=True)
    create_at = Column(TIMESTAMP(timezone=True), nullable=False)
    update_at = Column(TIMESTAMP(timezone=True), nullable=False)

    def __init__(self, account_id: int, nickname: str, credential: str, salt: str,
                 role: list, delete_at: time = None, create_at: datetime.datetime = datetime.datetime.now(),
                 update_at: datetime.datetime = datetime.datetime.now()):
        self.account_id = account_id
        self.nickname = nickname
        self.credential = credential
        self.salt = salt
        self.role = role
        self.delete_at = delete_at
        self.create_at = create_at
        self.update_at = update_at


class UserInfo(Base):
    __table_args__ = {'schema': 'business'}
    __tablename__ = 'user_info'
    user_d = Column(BigInteger, primary_key=True, nullable=False)
    avatar = Column(LargeBinary, nullable=False)
    email = Column(String(32), nullable=True)
    phone = Column(BigInteger, nullable=True)
    signature = Column(String(128), nullable=True)

    def __init__(self, user_d: int, avatar: bytes, email: str, phone: int, signature: str):
        self.user_d = user_d
        self.avatar = avatar
        self.email = email
        self.phone = phone
        self.signature = signature


class UserRelationship(Base):
    __table_args__ = {'schema': 'business'}
    __tablename__ = 'user_relationship'
    id = Column(BigInteger, primary_key=True, autoincrement=True, nullable=False)
    user_id_l = Column(BigInteger, nullable=False)
    user_id_r = Column(BigInteger, nullable=False)
    remark_l = Column(String(64), nullable=False)
    remark_r = Column(String(64), nullable=False)
    extension_l = Column(JSON, nullable=True)
    extension_r = Column(JSON, nullable=True)
    delete_at = Column(TIMESTAMP(timezone=True), nullable=True)
    create_at = Column(TIMESTAMP(timezone=True), nullable=False)
    update_at = Column(TIMESTAMP(timezone=True), nullable=False)

    def __init__(self, user_id_l: int, user_id_r: int, remark_l: str, remark_r: str, extension_l: dict,
                 extension_r: dict, delete_at: str, create_at: str, update_at: str):
        self.user_id_l = user_id_l
        self.user_id_r = user_id_r
        self.remark_l = remark_l
        self.remark_r = remark_r
        self.extension_l = extension_l
        self.extension_r = extension_r
        self.delete_at = delete_at
        self.create_at = create_at
        self.update_at = update_at

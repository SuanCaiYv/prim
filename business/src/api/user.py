import hashlib

from flask import request, Blueprint, jsonify
from sqlalchemy.future import select

from db.pgsql import async_session
from entity import transform
from entity.models import User
from nosql.ops import get_instance
from util import base

user_router = Blueprint('user', __name__, template_folder=None)


@user_router.get('/account_id')
async def new_account_id():
    async with async_session() as session:
        while True:
            account_id = base.generate_account_id()
            query = await session.execute(select(User).where(User.account_id == account_id))
            temp = query.scalar()
            if temp is None:
                return str(account_id)


@user_router.post('/user')
async def sign():
    json = request.json
    account_id = json['account_id']
    credential = json['credential']
    async with async_session() as session:
        query = await session.execute(select(User).where(User.account_id == account_id))
        temp = query.scalar()
        if temp is not None:
            resp = transform.Resp(code=400, msg='account_id already exists')
            return jsonify(resp.to_dict())
    salt = base.salt()
    md5 = hashlib.md5()
    md5.update(credential.encode('utf-8') + salt.encode('utf-8'))
    password = md5.hexdigest()
    user = User(account_id=account_id, nickname=str(account_id), credential=password, salt=salt, role=['user'])
    async with async_session() as session:
        session.add(user)
        await session.commit()
    resp = transform.Resp(code=200, msg='ok')
    return jsonify(resp.to_dict())


@user_router.put('/user')
async def login():
    redis_ops = get_instance()
    json = request.json
    account_id = json['account_id']
    credential = json['credential']
    async with async_session() as session:
        user = (await session.execute(select(User).where(User.account_id == account_id))).scalar()
        if user is None:
            resp = transform.Resp(code=400, msg='account_id not exists')
            return jsonify(resp.to_dict())
    salt = user.salt
    md5 = hashlib.md5()
    md5.update(credential.encode('utf-8') + salt.encode('utf-8'))
    password = md5.hexdigest()
    if password != user.credential:
        resp = transform.Resp(code=400, msg='credential error')
        return jsonify(resp.to_dict())
    resp = transform.Resp(code=200, msg='ok', data=base.salt())
    await redis_ops.set('auth-' + str(account_id), salt)
    return jsonify(resp.to_dict())


@user_router.post('/friend')
async def add_friend():
    pass


@user_router.get('/friend/list/<int:account_id>')
async def friend_list(account_id: int):
    pass


@user_router.delete('/friend/<int:account_id>/<int:friend_account_id>')
async def delete_friend():
    pass

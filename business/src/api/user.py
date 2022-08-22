import datetime
import hashlib

from flask import request, Blueprint, jsonify
from sqlalchemy import update
from sqlalchemy.future import select

from db.pgsql import async_session
from entity import transform
from entity.models import User, UserRelationship, UserInfo
from entity.net import Msg
from net.api import get_net_api
from nosql.ops import get_instance
from util import base

user_router = Blueprint('user', __name__, template_folder=None)


@user_router.get('/account_id')
async def new_account_id():
    async with async_session() as session:
        async with session.begin():
            while True:
                account_id = base.generate_account_id()
                temp = (await session.execute(select(User).where(User.account_id == account_id))).first()
                if temp is None:
                    return str(account_id)


@user_router.post('/user')
async def sign():
    json = request.json
    account_id = json['account_id']
    credential = json['credential']
    async with async_session() as session:
        user = (await session.execute(select(User).where(User.account_id == account_id))).first()
        if user is not None:
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
    info = UserInfo(user_id=user.id, avatar='/static/default-avatar.jpg', email='', phone=0, signature='')
    async with async_session() as session:
        session.add(info)
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
        user = (await session.execute(select(User).where(User.account_id == account_id))).first()
        async_session.remove()
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


@user_router.get('/user/info/<int:account_id>')
async def user_info(account_id: int):
    token = request.headers['token']
    if not await check_user(account_id, token):
        resp = transform.Resp(code=400, msg='token error')
        return jsonify(resp.to_dict())
    async with async_session() as session:
        user = (await session.execute(select(User).where(User.account_id == account_id))).first()
        if user is None:
            resp = transform.Resp(code=400, msg='account_id not exists')
            return jsonify(resp.to_dict())
        info = (await session.execute(select(UserInfo).where(UserInfo.user_id == user.id))).first()
    resp = transform.Resp(code=200, msg='ok', data=dict({
        'account_id': user.account_id,
        'nickname': user.nickname,
        'avatar': info.avatar,
        'email': info.email,
        'phone': info.phone,
        'signature': info.signature
    }))
    return jsonify(resp.to_dict())


async def check_user(account_id: int, token: str) -> bool:
    redis_ops = get_instance()
    return token == await redis_ops.get('auth-' + str(account_id))


@user_router.post('/friend')
async def add_friend():
    token = request.headers['token']
    json = request.json
    account_id = json['account_id']
    friend_id = json['friend_account_id']
    remark = json['remark']
    if not await check_user(account_id, token):
        resp = transform.Resp(code=400, msg='you are not the owner of this account')
        return jsonify(resp.to_dict())
    net_api = await get_net_api()
    is_friend = False
    if account_id < friend_id:
        id1 = account_id
        id2 = friend_id
    else:
        id1 = friend_id
        id2 = account_id
        is_friend = True
    async with async_session() as session:
        user_relationship = (await session.execute(select(UserRelationship)
                                                   .where(UserRelationship.user_id_l == id1 and
                                                          UserRelationship.user_id_r == id2 and
                                                          UserRelationship.delete_at is None)
                                                   .order(UserRelationship.create_at)
                                                   .limit(1))
                             ).first()
        if user_relationship is None:
            user_relationship = UserRelationship(user_id_l=id1, user_id_r=id2)
            if is_friend:
                user_relationship.remark_l = str(friend_id)
            else:
                user_relationship.remark_r = str(friend_id)
            session.add(user_relationship)
            await session.commit()
            await net_api.send(Msg.friend_relationship(account_id, friend_id, "ADD_" + remark))
        else:
            if is_friend:
                user_relationship.remark_l = str(friend_id)
            else:
                user_relationship.remark_r = str(friend_id)
            await session.execute(update(UserRelationship)
                                  .where(UserRelationship.id == user_relationship.id)
                                  .values(remark_l=user_relationship.remark_l,
                                          remark_r=user_relationship.remark_r))
        if user_relationship.remark_l != "" and user_relationship.remark_r != "":
            await net_api.send(Msg.friend_relationship(account_id, friend_id, "COMPLETE"))
    resp = transform.Resp(code=200, msg='ok')
    return jsonify(resp.to_dict())


@user_router.get('/friend/list/<int:account_id>')
async def friend_list(account_id: int):
    token = request.headers['token']
    if not await check_user(account_id, token):
        resp = transform.Resp(code=400, msg='you are not the owner of this account')
        return jsonify(resp.to_dict())
    async with async_session() as session:
        query = await session.execute(select(UserRelationship)
                                      .where((UserRelationship.user_id_l == account_id or
                                              UserRelationship.user_id_r == account_id) and
                                             UserRelationship.delete_at is None)
                                      .order(UserRelationship.create_at))
        user_relationships = query.fetchall()
        resp = transform.Resp(code=200, msg='ok', data=user_relationships)
        return jsonify(resp.to_dict())


@user_router.delete('/friend/<int:account_id>/<int:friend_account_id>')
async def delete_friend(account_id: int, friend_account_id: int):
    token = request.headers['token']
    if not await check_user(account_id, token):
        resp = transform.Resp(code=400, msg='you are not the owner of this account')
        return jsonify(resp.to_dict())
    if account_id < friend_account_id:
        id1 = account_id
        id2 = friend_account_id
    else:
        id1 = friend_account_id
        id2 = account_id
    async with async_session() as session:
        user_relationship = (await session.execute(select(UserRelationship)
                                                   .where(UserRelationship.user_id_l == id1 and
                                                          UserRelationship.user_id_r == id2 and
                                                          UserRelationship.delete_at is None)
                                                   .order(UserRelationship.create_at)
                                                   .limit(1))
                             ).first()
        if user_relationship is None:
            resp = transform.Resp(code=400, msg='not exists')
            return jsonify(resp.to_dict())
        await session.execute(update(UserRelationship)
                              .where(UserRelationship.id == user_relationship.id)
                              .values(delete_at=datetime.datetime.now()))
        await session.commit()
        net_api = await get_net_api()
        await net_api.send(Msg.friend_relationship(account_id, friend_account_id, "DELETE"))
    resp = transform.Resp(code=200, msg='ok')
    return jsonify(resp.to_dict())

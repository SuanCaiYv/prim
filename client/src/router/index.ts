import {createRouter, createWebHistory, Router, RouteRecordRaw} from 'vue-router';

const routes: Array<RouteRecordRaw> = [
    {
        path: "/home",
        alias: "/",
        name: "home",
        meta: {
            title: "QM"
        },
        component: () => import("../components/home/Home.vue")
    },
    {
        path: "/sign",
        meta: {
            title: "QM"
        },
        component: () => import("../components/mutual/Home.vue")
    },
    {
        path: "/friends",
        name: "sign",
        meta: {
            title: "QM"
        },
        component: () => import("../components/home/friends/Home.vue")
    },
    {
        path: "/t",
        alias: "/test",
        name: "test",
        component: () => import("../components/Test.vue")
    }
]

const router: Router = createRouter({
    history: createWebHistory(),
    routes: routes
})

export default router
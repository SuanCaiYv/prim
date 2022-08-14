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
        alias: "/sign",
        name: "sign",
        meta: {
            title: "QM"
        },
        component: () => import("../components/mutual/Home.vue")
    }
]

const router: Router = createRouter({
    history: createWebHistory(),
    routes: routes
})

export default router
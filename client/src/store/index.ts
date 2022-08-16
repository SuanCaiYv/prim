import {createStore, Store} from "vuex";

const store: Store<any> = createStore({
    state() {
        return {
            draftArticleId: String,
            operation: String,
            articleList1: Array,
            articleList2: Array,
            sort1: String,
            sort2: String,
            desc1: Boolean,
            desc2: Boolean,
            page1: Number,
            page2: Number,
        }
    },
    mutations: {
        updateDraftArticleId(state, draftArticleId) {
            state.draftArticleId = draftArticleId
        },
        updateOperation(state, operation) {
            state.operation = operation
        },
        updateArticleList1(state, articleList1) {
            state.articleList1 = articleList1
        },
        updateArticleList2(state, articleList2) {
            state.articleList2 = articleList2
        },
        updateSort1(state, sort1) {
            state.sort1 = sort1
        },
        updateSort2(state, sort2) {
            state.sort2 = sort2
        },
        updateDesc1(state, desc1) {
            state.desc1 = desc1
        },
        updateDesc2(state, desc2) {
            state.desc2 = desc2
        },
        updatePage1(state, page1) {
            state.page1 = page1
        },
        updatePage2(state, page2) {
            state.page2 = page2
        },
    },
    getters: {
        draftArticleId: (state) => {
            return state.draftArticleId
        },
        operation: (state) => {
            return state.operation
        },
        articleList1: (state) => {
            return state.articleList1
        },
        articleList2: (state) => {
            return state.articleList2
        },
        sort1: (state) => {
            return state.sort1
        },
        sort2: (state) => {
            return state.sort2
        },
        desc1: (state) => {
            return state.desc1
        },
        desc2: (state) => {
            return state.desc2
        },
        page1: (state) => {
            return state.page1
        },
        page2: (state) => {
            return state.page2
        },
    }
})

export default store
var ctx = null;
var memory;

params_set_mem = function (wasm_memory, _wasm_exports) {
    memory = wasm_memory;
    ctx = {};
}

function set_url(params, hash) {
    let result = window.location.origin + window.location.pathname;
    if (params != "") {
        if (params !== undefined && params !== null) {
            result += '?' + params;    
        } else {
            result += window.location.search;
        }
    }
    if (hash != "") {
        if (hash !== undefined && hash !== null) {
            result += '#' + hash;
        } else {
            result += window.location.hash;
        }
    }
    window.history.pushState({path:result},'',result); // https://stackoverflow.com/questions/10970078/modifying-a-query-string-without-reloading-the-page
}

params_register_js_plugin = function (importObject) {
    importObject.env.quad_url_path = function (full) {
        if (full == 1) {
            return js_object(window.location.href);    
        } else {
            return js_object(window.location.origin + window.location.pathname);    
        }
    }
    importObject.env.quad_url_param_count = function () {
        ctx.entries = [];
        var some = new URLSearchParams(window.location.search);
        for (let i of some.entries()) {
            ctx.entries.push(i);
        }
        return ctx.entries.length;
    }
    importObject.env.quad_url_get_key = function (i) {
        return js_object(ctx.entries[i][0])
    }
    importObject.env.quad_url_get_value = function (i) {
        return js_object(ctx.entries[i][1])
    }
    importObject.env.quad_url_link_open = function (url_rs, new_tab) {
        let url = get_js_object(url_rs);
        if (new_tab == 0) {
            window.open(url, "_self"); // https://stackoverflow.com/questions/8454510/open-url-in-same-window-and-in-same-tab
        } else {
            window.open(url);
        }
    }
    importObject.env.quad_url_set_program_parameter = function (name_rs, value_rs) {
        let name = get_js_object(name_rs);
        let value = get_js_object(value_rs);
        let params = new URLSearchParams(window.location.search);
        params.set(name, value);
        // todo убрать вопрос если пусто
        set_url(params.toString(), null);
    }
    importObject.env.quad_url_delete_program_parameter = function (name_rs) {
        let name = get_js_object(name_rs);
        let params = new URLSearchParams(window.location.search);
        params.delete(name);
        set_url(params.toString(), null);
    }
    importObject.env.quad_url_get_hash = function () {
        return js_object(window.location.hash);    
    }
    importObject.env.quad_url_set_hash = function (hash) {
        set_url(null, get_js_object(hash));
    }
}

miniquad_add_plugin({
    register_plugin: params_register_js_plugin,
    on_init: params_set_mem,
    name: "quad_url",
    version: "0.1.0"
});

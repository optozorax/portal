var ctx = null;
var memory;

params_set_mem = function (wasm_memory, _wasm_exports) {
    memory = wasm_memory;
    ctx = {};
    ctx.entries = [];
    var some = new URLSearchParams(window.location.search);
    for (i of some.entries()) {
        ctx.entries.push(i);
    }
}

params_register_js_plugin = function (importObject) {
    importObject.env.miniquad_parameters_param_count = function () {
        return ctx.entries.length;
    }
    importObject.env.miniquad_parameters_get_key = function (i) {
        return js_object(ctx.entries[i][0])
    }
    importObject.env.miniquad_parameters_get_value = function (i) {
        return js_object(ctx.entries[i][1])
    }
}

miniquad_add_plugin({
    register_plugin: params_register_js_plugin,
    on_init: params_set_mem
});
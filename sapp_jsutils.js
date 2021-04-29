var ctx = null;

js_objects = {}
unique_js_id = 0

register_plugin = function (importObject) {
    importObject.env.js_create_string = function (buf, max_len) {
        var string = UTF8ToString(buf, max_len);
        return js_object(string);
    }

    // Copy given bytes into newly allocated Uint8Array
    importObject.env.js_create_buffer = function (buf, max_len) {
        var src = new Uint8Array(wasm_memory.buffer, buf, max_len);
        var new_buffer = new Uint8Array(new ArrayBuffer(src.byteLength));
        new_buffer.set(new Uint8Array(src));
        return js_object(new_buffer);
    }

    importObject.env.js_create_object = function () {
        var object = {};
        return js_object(object);
    }

    importObject.env.js_set_field_f32 = function (js_object, buf, max_len, data) {
        var field = UTF8ToString(buf, max_len);

        js_objects[js_object][field] = data;
    }

    importObject.env.js_set_field_string = function (js_object, buf, max_len, data_buf, data_len) {
        var field = UTF8ToString(buf, max_len);
        var data = UTF8ToString(data_buf, data_len);

        js_objects[js_object][field] = data;
    }

    importObject.env.js_unwrap_to_str = function (js_object, buf, max_len) {
        var str = js_objects[js_object];
        var utf8array = toUTF8Array(str);
        var length = utf8array.length;
        var dest = new Uint8Array(wasm_memory.buffer, buf, max_len); // with max_len in case of buffer overflow we will panic (I BELIEVE) in js, no UB in rust
        for (var i = 0; i < length; i++) {
            dest[i] = utf8array[i];
        }
    }

    importObject.env.js_unwrap_to_buf = function (js_object, buf, max_len) {
        var src = js_objects[js_object];
        var length = src.length;
        var dest = new Uint8Array(wasm_memory.buffer, buf, max_len); 
        for (var i = 0; i < length; i++) {
            dest[i] = src[i];
        }
    }

    // measure length of the string. This function allocates because there is no way
    // go get string byte length in JS 
    importObject.env.js_string_length = function (js_object) {
        var str = js_objects[js_object];
        return toUTF8Array(str).length;
    }

    // similar to .length call on Uint8Array in javascript.
    importObject.env.js_buf_length = function (js_object) {
        var buf = js_objects[js_object];
        return buf.length;
    }

    importObject.env.js_free_object = function (js_object) {
        delete js_objects[js_object];
    }

    importObject.env.js_have_field = function (js_object, buf, length) {
        var field_name = UTF8ToString(buf, length);

        return js_objects[js_object][field_name] == undefined;
    }

    importObject.env.js_field_num = function (js_object, buf, length) {
        var field_name = UTF8ToString(buf, length);

        return js_objects[js_object][field_name];
    }

    importObject.env.js_field = function (js_object, buf, length) {
        // UTF8ToString is from gl.js wich should be in the scope now
        var field_name = UTF8ToString(buf, length);

        // apparently .field and ["field"] is the same thing in js
        var field = js_objects[js_object][field_name];

        var id = unique_js_id
        js_objects[id] = field

        unique_js_id += 1;

        return id;
    }
}
miniquad_add_plugin({ register_plugin, version: "0.1.5", name: "sapp_jsutils" });

// Its like https://developer.mozilla.org/en-US/docs/Web/API/TextEncoder, 
// but works on more browsers
function toUTF8Array(str) {
    var utf8 = [];
    for (var i = 0; i < str.length; i++) {
        var charcode = str.charCodeAt(i);
        if (charcode < 0x80) utf8.push(charcode);
        else if (charcode < 0x800) {
            utf8.push(0xc0 | (charcode >> 6),
                0x80 | (charcode & 0x3f));
        }
        else if (charcode < 0xd800 || charcode >= 0xe000) {
            utf8.push(0xe0 | (charcode >> 12),
                0x80 | ((charcode >> 6) & 0x3f),
                0x80 | (charcode & 0x3f));
        }
        // surrogate pair
        else {
            i++;
            // UTF-16 encodes 0x10000-0x10FFFF by
            // subtracting 0x10000 and splitting the
            // 20 bits of 0x0-0xFFFFF into two halves
            charcode = 0x10000 + (((charcode & 0x3ff) << 10)
                | (str.charCodeAt(i) & 0x3ff))
            utf8.push(0xf0 | (charcode >> 18),
                0x80 | ((charcode >> 12) & 0x3f),
                0x80 | ((charcode >> 6) & 0x3f),
                0x80 | (charcode & 0x3f));
        }
    }
    return utf8;
}

// Store js object reference to prevent JS garbage collector on destroying it
// And let Rust keep ownership of this reference
// There is no guarantees on JS side of this reference uniqueness, its good idea to use this only on rust functions arguments
function js_object(obj) {
    var id = unique_js_id;

    js_objects[id] = obj;
    unique_js_id += 1;
    return id;
}

/// Consume the JsObject returned from rust
/// Rust gives us ownership on the object. This method consume ownership from rust to normal JS garbage collector.
function consume_js_object(id) {
    var object = js_objects[id];
    // in JS delete operator does not delete (JS!), the intention here is to remove the value from hashmap, like "js_objects.remove(id)"
    delete js_objects[id];
    return object;
}

/// Get the real object from JsObject returned from rust 
/// Acts like borrowing in rust, but without any checks
/// Be carefull, for most use cases "consume_js_object" is usually better option
function get_js_object(id) {
    return js_objects[id];
}

//! Lotus Signer · wasm 绑定层
//!
//! **本层不得包含任何解析、交易事实判定或五态逻辑** —— 它只做一件事：
//! 把 JS 传来的字节交给 `lscore`，把 `Review` 序列化成 JSON 回传。
//! 所有权威语义在 `lscore` 里，网页与命令行共用同一份实现（宪法第 8 条）。

use std::alloc::{alloc as rust_alloc, dealloc as rust_dealloc, Layout};

mod json;
mod serialize;

/// 分配一块线性内存给 JS 写入输入数据。
/// # Safety
/// 调用方须保证 `size > 0`，并在使用后调用 `ls_dealloc` 归还。
#[no_mangle]
pub extern "C" fn ls_alloc(size: usize) -> *mut u8 {
    if size == 0 {
        return std::ptr::null_mut();
    }
    unsafe {
        match Layout::from_size_align(size, 1) {
            Ok(layout) => rust_alloc(layout),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

/// 归还 `ls_alloc` 分配的内存。
/// # Safety
/// `ptr` 必须来自 `ls_alloc`，`size` 必须与分配时一致。
#[no_mangle]
pub unsafe extern "C" fn ls_dealloc(ptr: *mut u8, size: usize) {
    if ptr.is_null() || size == 0 {
        return;
    }
    if let Ok(layout) = Layout::from_size_align(size, 1) {
        rust_dealloc(ptr, layout);
    }
}

/// 审阅入口。
///
/// 输入：`ptr`/`len` 指向 UTF-8 的 base64 PSBT 文本。
/// 输出：指向一块**长度前缀缓冲区**的指针 —— 头 4 字节为小端 u32 长度，其后为 JSON 字节。
/// 返回空指针表示分配失败。调用方读完须用 `ls_dealloc(ptr, len + 4)` 释放。
///
/// # Safety
/// `ptr` 必须指向 `len` 字节的有效可读内存。
#[no_mangle]
pub unsafe extern "C" fn ls_review(ptr: *const u8, len: usize) -> *mut u8 {
    let input = std::slice::from_raw_parts(ptr, len);
    let text = match std::str::from_utf8(input) {
        Ok(t) => t,
        Err(e) => {
            let body = format!("{{\"ok\":false,\"error\":\"输入不是合法的 UTF-8：{}\"}}", json::escape(&e.to_string()));
            return pack(&body);
        }
    };

    let body = match lscore::review_psbt_base64(text) {
        Ok(review) => serialize::review_to_json(&review),
        Err(e) => format!("{{\"ok\":false,\"error\":{}}}", json::string(&e.to_string())),
    };

    pack(&body)
}

/// 把字符串打包成「4 字节小端长度 + 内容」并返回指针。
fn pack(s: &str) -> *mut u8 {
    let bytes = s.as_bytes();
    let total = bytes.len() + 4;
    let out = unsafe {
        match Layout::from_size_align(total, 1) {
            Ok(layout) => rust_alloc(layout),
            Err(_) => return std::ptr::null_mut(),
        }
    };
    if out.is_null() {
        return out;
    }
    unsafe {
        let len_le = (bytes.len() as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_le.as_ptr(), out, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.add(4), bytes.len());
    }
    out
}

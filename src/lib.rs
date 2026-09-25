//! Compiler-only C ABI. No database handles or result data cross this boundary.
use std::{
    ffi::{CStr, CString, c_char},
    panic::{AssertUnwindSafe, catch_unwind},
    sync::OnceLock,
};

#[unsafe(no_mangle)]
pub extern "C" fn orchiddb_abi_version() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn orchiddb_version() -> *const c_char {
    concat!(env!("CARGO_PKG_VERSION"), "\0").as_ptr().cast()
}

#[unsafe(no_mangle)]
pub extern "C" fn orchiddb_core_revision() -> *const c_char {
    concat!(env!("ORCHIDDB_BUILT_CORE_REVISION"), "\0")
        .as_ptr()
        .cast()
}

fn compile(input: String) -> Result<serde_json::Value, String> {
    static RUNTIME: OnceLock<Result<tokio::runtime::Runtime, String>> = OnceLock::new();
    let runtime = RUNTIME
        .get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .thread_stack_size(16 * 1024 * 1024)
                .enable_all()
                .build()
                .map_err(|e| e.to_string())
        })
        .as_ref()
        .map_err(Clone::clone)?;
    // Avoid recursively planning on small foreign-language/NIF stacks. No database is opened.
    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    runtime.spawn(async move {
        let result = orchiddb::compiler::compile_json(&input).await;
        let _ = sender.send(result);
    });
    let output = receiver
        .recv()
        .map_err(|_| "compiler worker failed".to_string())??;
    serde_json::from_str(&output).map_err(|e| e.to_string())
}

/// Compile a version-1 JSON request. The returned UTF-8 JSON is caller-owned.
///
/// # Safety
/// `input` is null or points to a valid NUL-terminated string for this call.
/// The returned pointer must be freed exactly once with `orchiddb_string_free`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn orchiddb_compile_json(input: *const c_char) -> *mut c_char {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if input.is_null() {
            return Err("input must not be null".to_string());
        }
        let text = unsafe { CStr::from_ptr(input) }
            .to_str()
            .map_err(|_| "input must be UTF-8".to_string())?;
        compile(text.to_owned())
    }));
    let response = match result {
        Ok(Ok(result)) => serde_json::json!({"ok": true, "result": result}),
        Ok(Err(error)) => serde_json::json!({"ok": false, "error": error}),
        Err(_) => {
            serde_json::json!({"ok": false, "error": "compiler panicked; no SQL was executed"})
        }
    };
    // JSON escapes embedded NULs; CString cannot fail for the serializer's UTF-8 output.
    CString::new(response.to_string())
        .expect("JSON has no raw NUL")
        .into_raw()
}

/// Release one returned response; null is accepted. Never free version/revision pointers.
///
/// # Safety
/// `response` is null or an unfreed pointer returned by this library's compile function.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn orchiddb_string_free(response: *mut c_char) {
    if !response.is_null() {
        drop(unsafe { CString::from_raw(response) });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    unsafe fn invoke(ptr: *const c_char) -> serde_json::Value {
        let out = unsafe { orchiddb_compile_json(ptr) };
        assert!(!out.is_null());
        let response =
            serde_json::from_str(unsafe { CStr::from_ptr(out) }.to_str().unwrap()).unwrap();
        unsafe { orchiddb_string_free(out) };
        response
    }
    #[test]
    fn version_and_error_ownership() {
        assert_eq!(orchiddb_abi_version(), 1);
        assert_eq!(
            unsafe { CStr::from_ptr(orchiddb_version()) }
                .to_str()
                .unwrap(),
            "0.1.0"
        );
        assert_eq!(unsafe { invoke(std::ptr::null()) }["ok"], false);
        assert_eq!(unsafe { invoke(c"not-json".as_ptr()) }["ok"], false);
        let invalid = [255u8, 0];
        assert_eq!(unsafe { invoke(invalid.as_ptr().cast()) }["ok"], false);
        unsafe { orchiddb_string_free(std::ptr::null_mut()) };
    }
    #[test]
    fn actual_compiler_and_parallel_foreign_calls() {
        let threads: Vec<_> = (0..4).map(|_| std::thread::spawn(|| {
            let input = CString::new(r#"{"version":1,"dialect":"duckdb","language":"cypher","query":"RETURN 42 AS answer","tables":[],"nodes":[]}"#).unwrap();
            let result = unsafe { invoke(input.as_ptr()) };
            assert_eq!(result["ok"], true, "{result}");
            assert_eq!(result["result"]["fields"], serde_json::json!(["answer"]));
            assert!(result["result"]["sql"].as_str().unwrap().contains("42"));
        })).collect();
        for thread in threads {
            thread.join().unwrap();
        }
    }
}

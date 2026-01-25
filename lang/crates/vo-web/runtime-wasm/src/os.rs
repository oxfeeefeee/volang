//! os package WASM stub implementation.
//!
//! Most os functions are not supported in WASM and return ErrNotSupported.
//! Some functions like Getenv/Getpid return reasonable defaults.

use vo_runtime::bytecode::ExternDef;
use vo_runtime::ffi::{ExternCallContext, ExternRegistry, ExternResult};
use vo_runtime::objects::{array, slice, string};
use vo_runtime::core_types::{ValueKind, ValueMeta};

const ERR_NOT_SUPPORTED: &str = "operation not supported on wasm";

fn write_error(call: &mut ExternCallContext, slot: u16, msg: &str) {
    let gc = call.gc();
    let str_ref = string::from_rust_str(gc, msg);
    // Error is interface{} - 2 slots: [meta, data]
    call.ret_u64(slot, ValueKind::String as u64);
    call.ret_ref(slot + 1, str_ref);
}

fn write_nil_error(call: &mut ExternCallContext, slot: u16) {
    call.ret_u64(slot, 0);
    call.ret_u64(slot + 1, 0);
}

fn write_not_supported_error(call: &mut ExternCallContext, slot: u16) {
    write_error(call, slot, ERR_NOT_SUPPORTED);
}

// OS errors - return pre-created error values
fn os_get_errors(call: &mut ExternCallContext) -> ExternResult {
    let errors = [
        "file does not exist",
        "file already exists",
        "permission denied",
        "invalid argument",
        "i/o timeout",
        "file already closed",
        "not a directory",
        "is a directory",
    ];
    
    for (i, msg) in errors.iter().enumerate() {
        let str_ref = string::from_rust_str(call.gc(), msg);
        call.ret_u64((i * 2) as u16, ValueKind::String as u64);
        call.ret_ref((i * 2 + 1) as u16, str_ref);
    }
    ExternResult::Ok
}

// OS constants
fn os_get_consts(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);    // O_RDONLY
    call.ret_i64(1, 1);    // O_WRONLY
    call.ret_i64(2, 2);    // O_RDWR
    call.ret_i64(3, 1024); // O_APPEND
    call.ret_i64(4, 64);   // O_CREATE
    call.ret_i64(5, 128);  // O_EXCL
    call.ret_i64(6, 4096); // O_SYNC
    call.ret_i64(7, 512);  // O_TRUNC
    ExternResult::Ok
}

// File operations - all return error
fn file_read(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn file_write(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn file_read_at(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn file_write_at(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn file_seek(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn file_close(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn file_sync(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn file_stat(call: &mut ExternCallContext) -> ExternResult {
    for i in 0..10 {
        call.ret_u64(i, 0);
    }
    write_not_supported_error(call, 10);
    ExternResult::Ok
}

fn file_truncate(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn open_file(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, -1);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

// Directory operations
fn native_mkdir(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_mkdir_all(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_remove(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_remove_all(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_rename(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_stat(call: &mut ExternCallContext) -> ExternResult {
    for i in 0..10 {
        call.ret_u64(i, 0);
    }
    write_not_supported_error(call, 10);
    ExternResult::Ok
}

fn native_lstat(call: &mut ExternCallContext) -> ExternResult {
    for i in 0..10 {
        call.ret_u64(i, 0);
    }
    write_not_supported_error(call, 10);
    ExternResult::Ok
}

fn native_read_dir(call: &mut ExternCallContext) -> ExternResult {
    let gc = call.gc();
    let elem_meta = ValueMeta::new(0, ValueKind::Struct);
    let arr = array::create(gc, elem_meta, 24, 0);
    let slice_ref = slice::from_array(gc, arr);
    call.ret_ref(0, slice_ref);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn native_chmod(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_chown(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_symlink(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_readlink(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "");
    call.ret_ref(0, str_ref);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn native_link(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_truncate(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_read_file(call: &mut ExternCallContext) -> ExternResult {
    let gc = call.gc();
    let elem_meta = ValueMeta::new(0, ValueKind::Uint8);
    let arr = array::create(gc, elem_meta, 1, 0);
    let slice_ref = slice::from_array(gc, arr);
    call.ret_ref(0, slice_ref);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn native_write_file(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

// Environment - return empty/defaults
fn native_getenv(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "");
    call.ret_ref(0, str_ref);
    ExternResult::Ok
}

fn native_setenv(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_unsetenv(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_environ(call: &mut ExternCallContext) -> ExternResult {
    let gc = call.gc();
    let elem_meta = ValueMeta::new(0, ValueKind::String);
    let arr = array::create(gc, elem_meta, 8, 0);
    let slice_ref = slice::from_array(gc, arr);
    call.ret_ref(0, slice_ref);
    ExternResult::Ok
}

fn native_lookup_env(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "");
    call.ret_ref(0, str_ref);
    call.ret_bool(1, false);
    ExternResult::Ok
}

fn native_clearenv(_call: &mut ExternCallContext) -> ExternResult {
    ExternResult::Ok
}

fn native_expand_env(call: &mut ExternCallContext) -> ExternResult {
    let s = call.arg_str(0).to_string();
    let str_ref = string::from_rust_str(call.gc(), &s);
    call.ret_ref(0, str_ref);
    ExternResult::Ok
}

// Working directory
fn native_getwd(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "/");
    call.ret_ref(0, str_ref);
    write_nil_error(call, 1);
    ExternResult::Ok
}

fn native_chdir(call: &mut ExternCallContext) -> ExternResult {
    write_not_supported_error(call, 0);
    ExternResult::Ok
}

fn native_user_home_dir(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "/");
    call.ret_ref(0, str_ref);
    write_nil_error(call, 1);
    ExternResult::Ok
}

fn native_user_cache_dir(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "/tmp");
    call.ret_ref(0, str_ref);
    write_nil_error(call, 1);
    ExternResult::Ok
}

fn native_user_config_dir(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "/");
    call.ret_ref(0, str_ref);
    write_nil_error(call, 1);
    ExternResult::Ok
}

fn native_temp_dir(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "/tmp");
    call.ret_ref(0, str_ref);
    ExternResult::Ok
}

// Process info - return defaults
fn native_getpid(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 1);
    ExternResult::Ok
}

fn native_getppid(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    ExternResult::Ok
}

fn native_getuid(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    ExternResult::Ok
}

fn native_geteuid(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    ExternResult::Ok
}

fn native_getgid(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    ExternResult::Ok
}

fn native_getegid(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, 0);
    ExternResult::Ok
}

fn native_exit(_call: &mut ExternCallContext) -> ExternResult {
    ExternResult::Ok
}

fn native_get_args(call: &mut ExternCallContext) -> ExternResult {
    let gc = call.gc();
    let elem_meta = ValueMeta::new(0, ValueKind::String);
    let arr = array::create(gc, elem_meta, 8, 1);
    let arg0 = string::from_rust_str(gc, "wasm");
    array::set(arr, 0, arg0 as u64, 8);
    let slice_ref = slice::from_array(gc, arr);
    call.ret_ref(0, slice_ref);
    ExternResult::Ok
}

fn native_hostname(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "wasm");
    call.ret_ref(0, str_ref);
    write_nil_error(call, 1);
    ExternResult::Ok
}

fn native_executable(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "");
    call.ret_ref(0, str_ref);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

fn native_create_temp(call: &mut ExternCallContext) -> ExternResult {
    call.ret_i64(0, -1);
    let str_ref = string::from_rust_str(call.gc(), "");
    call.ret_ref(1, str_ref);
    write_not_supported_error(call, 2);
    ExternResult::Ok
}

fn native_mkdir_temp(call: &mut ExternCallContext) -> ExternResult {
    let str_ref = string::from_rust_str(call.gc(), "");
    call.ret_ref(0, str_ref);
    write_not_supported_error(call, 1);
    ExternResult::Ok
}

pub fn register_externs(registry: &mut ExternRegistry, externs: &[ExternDef]) {
    for (id, def) in externs.iter().enumerate() {
        match def.name.as_str() {
            "os_getOsErrors" => registry.register_with_context(id as u32, os_get_errors),
            "os_getOsConsts" => registry.register_with_context(id as u32, os_get_consts),
            "os_fileRead" => registry.register_with_context(id as u32, file_read),
            "os_fileWrite" => registry.register_with_context(id as u32, file_write),
            "os_fileReadAt" => registry.register_with_context(id as u32, file_read_at),
            "os_fileWriteAt" => registry.register_with_context(id as u32, file_write_at),
            "os_fileSeek" => registry.register_with_context(id as u32, file_seek),
            "os_fileClose" => registry.register_with_context(id as u32, file_close),
            "os_fileSync" => registry.register_with_context(id as u32, file_sync),
            "os_fileStat" => registry.register_with_context(id as u32, file_stat),
            "os_fileTruncate" => registry.register_with_context(id as u32, file_truncate),
            "os_openFile" => registry.register_with_context(id as u32, open_file),
            "os_nativeMkdir" => registry.register_with_context(id as u32, native_mkdir),
            "os_nativeMkdirAll" => registry.register_with_context(id as u32, native_mkdir_all),
            "os_nativeRemove" => registry.register_with_context(id as u32, native_remove),
            "os_nativeRemoveAll" => registry.register_with_context(id as u32, native_remove_all),
            "os_nativeRename" => registry.register_with_context(id as u32, native_rename),
            "os_nativeStat" => registry.register_with_context(id as u32, native_stat),
            "os_nativeLstat" => registry.register_with_context(id as u32, native_lstat),
            "os_nativeReadDir" => registry.register_with_context(id as u32, native_read_dir),
            "os_nativeChmod" => registry.register_with_context(id as u32, native_chmod),
            "os_nativeChown" => registry.register_with_context(id as u32, native_chown),
            "os_nativeSymlink" => registry.register_with_context(id as u32, native_symlink),
            "os_nativeReadlink" => registry.register_with_context(id as u32, native_readlink),
            "os_nativeLink" => registry.register_with_context(id as u32, native_link),
            "os_nativeTruncate" => registry.register_with_context(id as u32, native_truncate),
            "os_nativeReadFile" => registry.register_with_context(id as u32, native_read_file),
            "os_nativeWriteFile" => registry.register_with_context(id as u32, native_write_file),
            "os_nativeGetenv" => registry.register_with_context(id as u32, native_getenv),
            "os_nativeSetenv" => registry.register_with_context(id as u32, native_setenv),
            "os_nativeUnsetenv" => registry.register_with_context(id as u32, native_unsetenv),
            "os_nativeEnviron" => registry.register_with_context(id as u32, native_environ),
            "os_nativeLookupEnv" => registry.register_with_context(id as u32, native_lookup_env),
            "os_nativeClearenv" => registry.register_with_context(id as u32, native_clearenv),
            "os_nativeExpandEnv" => registry.register_with_context(id as u32, native_expand_env),
            "os_nativeGetwd" => registry.register_with_context(id as u32, native_getwd),
            "os_nativeChdir" => registry.register_with_context(id as u32, native_chdir),
            "os_nativeUserHomeDir" => registry.register_with_context(id as u32, native_user_home_dir),
            "os_nativeUserCacheDir" => registry.register_with_context(id as u32, native_user_cache_dir),
            "os_nativeUserConfigDir" => registry.register_with_context(id as u32, native_user_config_dir),
            "os_nativeTempDir" => registry.register_with_context(id as u32, native_temp_dir),
            "os_nativeGetpid" => registry.register_with_context(id as u32, native_getpid),
            "os_nativeGetppid" => registry.register_with_context(id as u32, native_getppid),
            "os_nativeGetuid" => registry.register_with_context(id as u32, native_getuid),
            "os_nativeGeteuid" => registry.register_with_context(id as u32, native_geteuid),
            "os_nativeGetgid" => registry.register_with_context(id as u32, native_getgid),
            "os_nativeGetegid" => registry.register_with_context(id as u32, native_getegid),
            "os_nativeExit" => registry.register_with_context(id as u32, native_exit),
            "os_nativeGetArgs" => registry.register_with_context(id as u32, native_get_args),
            "os_nativeHostname" => registry.register_with_context(id as u32, native_hostname),
            "os_nativeExecutable" => registry.register_with_context(id as u32, native_executable),
            "os_nativeCreateTemp" => registry.register_with_context(id as u32, native_create_temp),
            "os_nativeMkdirTemp" => registry.register_with_context(id as u32, native_mkdir_temp),
            _ => {}
        }
    }
}

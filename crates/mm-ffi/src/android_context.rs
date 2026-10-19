use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject};
use jni::sys::{JNI_FALSE, JNI_TRUE, jboolean, jstring};
use std::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::OnceLock;

static ANDROID_APPLICATION_CONTEXT: OnceLock<GlobalRef> = OnceLock::new();

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_mirrormesh_1flutter_MainActivity_initRustAndroidContext(
    env: JNIEnv,
    _activity: JObject,
    application_context: JObject,
) -> jboolean {
    let java_vm = match env.get_java_vm() {
        Ok(java_vm) => java_vm,
        Err(_) => return JNI_FALSE,
    };
    let global_context = match env.new_global_ref(&application_context) {
        Ok(global_context) => global_context,
        Err(_) => return JNI_FALSE,
    };

    ANDROID_APPLICATION_CONTEXT.get_or_init(|| {
        // The VM pointer and global reference stay valid for the process lifetime. The
        // OnceLock ensures ndk-context's initialize function is called exactly once.
        unsafe {
            ndk_context::initialize_android_context(
                java_vm.get_java_vm_pointer().cast::<c_void>(),
                global_context.as_obj().as_raw().cast::<c_void>(),
            );
        }
        global_context
    });

    JNI_TRUE
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_mirrormesh_mirrormesh_1flutter_MainActivity_prepareAndroidPairingRuntime(
    env: JNIEnv,
    _activity: JObject,
) -> jstring {
    let initialized = catch_unwind(AssertUnwindSafe(
        mm_adapter_android::initialize_pairing_runtime,
    ));
    match initialized {
        Ok(Ok(device_id)) => match env.new_string(device_id.to_string()) {
            Ok(value) => value.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        Ok(Err(_)) | Err(_) => std::ptr::null_mut(),
    }
}

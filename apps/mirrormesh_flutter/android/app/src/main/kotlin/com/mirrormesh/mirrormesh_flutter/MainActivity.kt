package com.mirrormesh.mirrormesh_flutter

import android.content.Context
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel

class MainActivity : FlutterActivity() {
    private var rustLibraryLoaded = false

    init {
        try {
            System.loadLibrary("mm_ffi")
            rustLibraryLoaded = true
        } catch (_: UnsatisfiedLinkError) {
            // The Flutter shell can still show its existing Rust-unavailable state.
        }
    }

    private external fun initRustAndroidContext(applicationContext: Context): Boolean
    private external fun prepareAndroidPairingRuntime(): String?

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        if (rustLibraryLoaded) {
            check(initRustAndroidContext(applicationContext)) {
                "MirrorMesh Rust bridge could not initialize its Android context"
            }
        }
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, PAIRING_CHANNEL)
            .setMethodCallHandler { call, result ->
                if (call.method != PREPARE_PAIRING_RUNTIME) {
                    result.notImplemented()
                } else if (!rustLibraryLoaded) {
                    result.error(
                        "rust_unavailable",
                        "MirrorMesh Rust pairing support is unavailable in this build",
                        null,
                    )
                } else {
                    Thread({
                        val deviceId = try {
                            prepareAndroidPairingRuntime()
                        } catch (_: UnsatisfiedLinkError) {
                            null
                        }
                        runOnUiThread {
                            if (deviceId == null) {
                                result.error(
                                    "pairing_runtime_unavailable",
                                    "Could not prepare local Android pairing support",
                                    null,
                                )
                            } else {
                                result.success(deviceId)
                            }
                        }
                    }, "MirrorMeshPairingRuntimeInit").start()
                }
            }
    }

    private companion object {
        const val PAIRING_CHANNEL = "com.mirrormesh/android_pairing"
        const val PREPARE_PAIRING_RUNTIME = "preparePairingRuntime"
    }
}

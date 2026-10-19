package com.mirrormesh.mirrormesh_flutter

import android.content.Context
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine

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

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        if (rustLibraryLoaded) {
            check(initRustAndroidContext(applicationContext)) {
                "MirrorMesh Rust bridge could not initialize its Android context"
            }
        }
        super.configureFlutterEngine(flutterEngine)
    }
}

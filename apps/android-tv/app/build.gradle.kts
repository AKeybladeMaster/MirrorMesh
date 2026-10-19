import java.io.File
import org.gradle.api.tasks.Exec

plugins {
    id("com.android.application")
}

val androidNdkVersion = "28.2.13676358"
val rustTarget = "aarch64-linux-android"
val repositoryRoot = rootProject.projectDir.resolve("../..").canonicalFile
val generatedJniLibs = layout.buildDirectory.dir("generated/jniLibs")

android {
    namespace = "com.mirrormesh.tv"
    compileSdk = 37
    ndkVersion = androidNdkVersion

    defaultConfig {
        applicationId = "com.mirrormesh.tv"
        minSdk = 21
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += "arm64-v8a" }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets.getByName("main").jniLibs.srcDir(generatedJniLibs)
}

val buildRustAndroid by tasks.registering(Exec::class) {
    val sdkDirectory = providers.environmentVariable("ANDROID_SDK_ROOT")
        .orElse(providers.environmentVariable("ANDROID_HOME"))
        .getOrElse("")
    require(sdkDirectory.isNotBlank()) { "ANDROID_SDK_ROOT or ANDROID_HOME is required" }
    val hostTag = if (System.getProperty("os.name").startsWith("Windows")) {
        "windows-x86_64"
    } else {
        "linux-x86_64"
    }
    val linkerName = if (hostTag.startsWith("windows")) {
        "aarch64-linux-android21-clang.cmd"
    } else {
        "aarch64-linux-android21-clang"
    }
    val ndkBin = file("$sdkDirectory/ndk/$androidNdkVersion/toolchains/llvm/prebuilt/$hostTag/bin")
    val linker = ndkBin.resolve(linkerName)
    val rustLibrary = repositoryRoot.resolve("target/$rustTarget/release/libmm_ffi.so")

    workingDir(repositoryRoot)
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", linker.absolutePath)
    environment("CC_aarch64-linux-android", linker.absolutePath)
    environment("CC_aarch64_linux_android", linker.absolutePath)
    environment("PATH", "${ndkBin.absolutePath}${File.pathSeparator}${System.getenv("PATH")}")
    commandLine("cargo", "build", "--manifest-path", "Cargo.toml", "--package", "mm-ffi",
        "--target", rustTarget, "--release", "--locked")
    inputs.file(repositoryRoot.resolve("Cargo.lock"))
    inputs.dir(repositoryRoot.resolve("crates"))
    outputs.file(generatedJniLibs.map { it.file("arm64-v8a/libmm_ffi.so") })
    doLast {
        require(rustLibrary.isFile) { "Rust Android library was not produced: $rustLibrary" }
        project.copy {
            from(rustLibrary)
            into(generatedJniLibs.get().dir("arm64-v8a"))
        }
    }
}

tasks.named("preBuild").configure {
    dependsOn(buildRustAndroid)
}

dependencies {
    implementation("io.github.webrtc-sdk:android:150.7871.01")
    implementation("com.google.zxing:core:3.5.4")
}

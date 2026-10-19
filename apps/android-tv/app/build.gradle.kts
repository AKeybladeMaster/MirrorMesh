import java.io.File
import org.gradle.api.tasks.Exec

plugins {
    id("com.android.application")
}

val androidNdkVersion = "28.2.13676358"
val androidAbi = providers.gradleProperty("mmAndroidAbi").orElse("arm64-v8a").get()
val rustTarget = when (androidAbi) {
    "arm64-v8a" -> "aarch64-linux-android"
    "x86_64" -> "x86_64-linux-android"
    else -> throw GradleException("Unsupported MirrorMesh Android ABI: $androidAbi")
}
val repositoryRoot = rootProject.projectDir.resolve("../..").canonicalFile
val generatedJniLibs = layout.buildDirectory.dir("generated/jniLibs")

android {
    namespace = "com.mirrormesh.tv"
    compileSdk = 37
    ndkVersion = androidNdkVersion
    buildFeatures { resValues = true }

    defaultConfig {
        applicationId = "com.mirrormesh.tv"
        minSdk = 21
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += androidAbi }
    }

    flavorDimensions += "device"
    productFlavors {
        create("tv") {
            dimension = "device"
            applicationId = "com.mirrormesh.tv"
            manifestPlaceholders["leanbackRequired"] = "true"
            resValue("string", "app_name", "MirrorMesh TV")
        }
        create("phone") {
            dimension = "device"
            applicationId = "com.mirrormesh.receiver"
            manifestPlaceholders["leanbackRequired"] = "false"
            resValue("string", "app_name", "MirrorMesh Receiver")
        }
    }

    // Signing material is supplied locally; it is never stored in this repository.
    val signingStore = providers.environmentVariable("MM_ANDROID_KEYSTORE").orNull
    if (signingStore != null) {
        signingConfigs.create("distribution") {
            storeFile = file(signingStore)
            storePassword = providers.environmentVariable("MM_ANDROID_STORE_PASSWORD").get()
            keyAlias = providers.environmentVariable("MM_ANDROID_KEY_ALIAS").get()
            keyPassword = providers.environmentVariable("MM_ANDROID_KEY_PASSWORD").get()
        }
        buildTypes.getByName("release").signingConfig = signingConfigs.getByName("distribution")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets.getByName("main").jniLibs.srcDir(generatedJniLibs.get().asFile)
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
        "$rustTarget".replace("-linux-android", "-linux-android21-clang") + ".cmd"
    } else {
        "$rustTarget".replace("-linux-android", "-linux-android21-clang")
    }
    val ndkBin = file("$sdkDirectory/ndk/$androidNdkVersion/toolchains/llvm/prebuilt/$hostTag/bin")
    val linker = ndkBin.resolve(linkerName)
    val rustLibrary = repositoryRoot.resolve("target/$rustTarget/release/libmm_ffi.so")
    val linkerEnvironment = "CARGO_TARGET_${rustTarget.uppercase().replace('-', '_')}_LINKER"
    val ccEnvironment = "CC_$rustTarget"
    val ccUnderscoreEnvironment = "CC_${rustTarget.replace('-', '_')}"

    workingDir(repositoryRoot)
    environment(linkerEnvironment, linker.absolutePath)
    environment(ccEnvironment, linker.absolutePath)
    environment(ccUnderscoreEnvironment, linker.absolutePath)
    environment("PATH", "${ndkBin.absolutePath}${File.pathSeparator}${System.getenv("PATH")}")
    commandLine("cargo", "build", "--manifest-path", "Cargo.toml", "--package", "mm-ffi",
        "--target", rustTarget, "--release", "--locked")
    inputs.file(repositoryRoot.resolve("Cargo.lock"))
    inputs.dir(repositoryRoot.resolve("crates"))
    outputs.file(generatedJniLibs.map { it.file("$androidAbi/libmm_ffi.so") })
    doLast {
        require(rustLibrary.isFile) { "Rust Android library was not produced: $rustLibrary" }
        project.copy {
            from(rustLibrary)
            into(generatedJniLibs.get().dir(androidAbi))
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

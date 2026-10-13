import org.jetbrains.kotlin.gradle.dsl.JvmTarget

/**
 * The version comes from one place (DECISIONS D79): `cargo android-build` passes it from the
 * workspace manifest and the store's schema (xtask/src/version.rs); there is no default here.
 */
fun passed(name: String): String = providers.gradleProperty(name).orNull
    ?: error("$name is not set: build with `cargo android-build`, which passes the version")

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.openpaint"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.openpaint"
        // Must equal MIN_SDK in xtask/src/android.rs.
        minSdk = 28
        targetSdk = 35
        versionCode = passed("openpaint.versionCode").toInt()
        versionName = passed("openpaint.versionName")
        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    sourceSets {
        named("main") {
            // cargo-ndk writes libopenpaint_android.so here (xtask/src/android.rs).
            jniLibs.srcDir(rootProject.file("../../target/android-jni"))
        }
    }

    buildTypes {
        debug {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    packaging {
        jniLibs {
            keepDebugSymbols += "**/libopenpaint_android.so"
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

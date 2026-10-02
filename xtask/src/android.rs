//! `cargo android-build` and `cargo android-install`: the tablet APK. cargo-ndk builds the Rust
//! library into `target/android-jni/` (which `apps/android/app/build.gradle.kts` packages),
//! the Gradle wrapper assembles the APK, adb installs and launches it.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::process::{cargo_command, root, run};

/// The tablet's ABI.
const ABI: &str = "arm64-v8a";
/// Must equal `minSdk` in `apps/android/app/build.gradle.kts`.
const MIN_SDK: &str = "28";
const ACTIVITY: &str = "dev.openpaint/.MainActivity";
const APK: &str = "apps/android/app/build/outputs/apk/debug/app-debug.apk";
const WRAPPER_JAR: &str = "apps/android/gradle/wrapper/gradle-wrapper.jar";

pub fn build() -> Result<(), String> {
    let sdk = sdk_dir()?;
    let ndk = ndk_dir(&sdk)?;
    println!(
        "Android SDK {}\nAndroid NDK {}",
        sdk.display(),
        ndk.display()
    );
    let jni_libs = root().join("target").join("android-jni");
    run(cargo_command()
        .env("ANDROID_NDK_HOME", &ndk)
        .args(["ndk", "-t", ABI, "-P", MIN_SDK, "-o"])
        .arg(&jni_libs)
        .args(["build", "--release", "-p", "openpaint-android"]))?;
    // The wrapper jar is launched directly: no gradlew scripts in the repository (D05).
    run(Command::new(java())
        .env("ANDROID_HOME", &sdk)
        .args(["-cp", WRAPPER_JAR, "org.gradle.wrapper.GradleWrapperMain"])
        .args(["-p", "apps/android", "assembleDebug"]))?;
    println!("APK {APK}");
    Ok(())
}

pub fn install() -> Result<(), String> {
    build()?;
    let adb = sdk_dir()?
        .join("platform-tools")
        .join(format!("adb{}", std::env::consts::EXE_SUFFIX));
    run(Command::new(&adb).args(["install", "-r", APK]))?;
    run(Command::new(&adb).args(["shell", "am", "start", "-n", ACTIVITY]))?;
    println!("logs: adb logcat -s openpaint");
    Ok(())
}

/// `ANDROID_HOME`, `ANDROID_SDK_ROOT`, then Android Studio's default location.
fn sdk_dir() -> Result<PathBuf, String> {
    let from_env = ["ANDROID_HOME", "ANDROID_SDK_ROOT"]
        .iter()
        .find_map(std::env::var_os);
    let default = if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("Android/Sdk"))
    } else if cfg!(target_os = "macos") {
        home().map(|dir| dir.join("Library/Android/sdk"))
    } else {
        home().map(|dir| dir.join("Android/Sdk"))
    };
    from_env
        .map(PathBuf::from)
        .or(default)
        .filter(|dir| dir.is_dir())
        .ok_or_else(|| "Android SDK not found: set ANDROID_HOME".to_string())
}

/// `ANDROID_NDK_HOME`, else the newest side-by-side NDK under `<sdk>/ndk/`.
fn ndk_dir(sdk: &Path) -> Result<PathBuf, String> {
    if let Some(dir) = std::env::var_os("ANDROID_NDK_HOME") {
        return Ok(PathBuf::from(dir));
    }
    let names: Vec<OsString> = std::fs::read_dir(sdk.join("ndk"))
        .map(|entries| entries.flatten().map(|entry| entry.file_name()).collect())
        .unwrap_or_default();
    let names: Vec<String> = names
        .iter()
        .map(|name| name.to_string_lossy().into_owned())
        .collect();
    newest_version(&names)
        .map(|name| sdk.join("ndk").join(name))
        .ok_or_else(|| "Android NDK not found: install one with sdkmanager \"ndk;<version>\" or set ANDROID_NDK_HOME".to_string())
}

/// The name with the highest dotted numeric version, ignoring names that are not versions.
fn newest_version(names: &[String]) -> Option<&String> {
    let parse = |name: &str| -> Option<Vec<u32>> {
        name.split('.').map(|part| part.parse().ok()).collect()
    };
    names
        .iter()
        .filter_map(|name| parse(name).map(|version| (version, name)))
        .max()
        .map(|(_, name)| name)
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// `$JAVA_HOME/bin/java`, else `java` from `PATH`. The Android Gradle plugin needs JDK 17+.
fn java() -> PathBuf {
    let name = format!("java{}", std::env::consts::EXE_SUFFIX);
    std::env::var_os("JAVA_HOME")
        .map(|home| PathBuf::from(home).join("bin").join(&name))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

#[cfg(test)]
mod tests {
    use super::newest_version;

    #[test]
    fn newest_ndk_compares_numerically_and_skips_other_names() {
        let names = [
            "27.0.12077973",
            "29.0.14206865",
            "9.9.9",
            "latest",
            ".DS_Store",
        ]
        .map(String::from);
        assert_eq!(
            newest_version(&names).map(String::as_str),
            Some("29.0.14206865")
        );
        assert_eq!(newest_version(&[]), None);
    }
}

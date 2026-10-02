// The APK project. Build it with `cargo android-build` (xtask), which builds the Rust library
// first and then runs the Gradle wrapper jar; there are no gradlew scripts in the repository.
pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "OpenPaint"
include(":app")

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "org.meshchat.app"
    compileSdk = 36
    buildToolsVersion = "35.0.0"

    signingConfigs {
        getByName("debug") {
            storeFile = rootProject.file("../../.work/android/debug.keystore")
        }
    }

    defaultConfig {
        applicationId = "org.meshchat.app"
        minSdk = 29
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    lint {
        warningsAsErrors = true
        abortOnError = true
        // Update suggestions must not turn pinned builds into a moving version gate.
        // API compatibility and correctness checks remain enabled.
        disable += setOf("GradleDependency", "AndroidGradlePluginVersion")
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
        allWarningsAsErrors.set(true)
    }
}

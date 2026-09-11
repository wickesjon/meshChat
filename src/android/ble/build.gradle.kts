plugins {
    id("com.android.application") version "8.11.1"
    id("org.jetbrains.kotlin.android") version "2.2.0"
}
android {
    namespace = "org.meshchat.bleprobe"
    compileSdk = 36
    buildToolsVersion = "35.0.0"
    defaultConfig {
        applicationId = "org.meshchat.bleprobe"
        minSdk = 29
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0-probe"
    }
    signingConfigs.getByName("debug") {
        storeFile = file("../../../.work/android/debug.keystore")
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    lint {
        warningsAsErrors = true
        abortOnError = true
        disable += setOf("GradleDependency", "AndroidGradlePluginVersion")
    }
}
kotlin.compilerOptions {
    jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    allWarningsAsErrors.set(true)
}

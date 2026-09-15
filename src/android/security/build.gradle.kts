plugins {
    id("com.android.application") version "8.11.1"
    id("org.jetbrains.kotlin.android") version "2.2.0"
}
android {
    namespace = "org.meshchat.securityprobe"
    compileSdk = 36
    buildToolsVersion = "35.0.0"
    defaultConfig {
        applicationId = "org.meshchat.securityprobe"
        minSdk = 29
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0-probe"
        testInstrumentationRunner = "org.meshchat.securityprobe.SecurityInstrumentation"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    signingConfigs.getByName("debug") { storeFile = file("../../../.work/android/debug.keystore") }
    sourceSets.getByName("main") {
        java.srcDir("../../../.work/security-ffi/kotlin")
        jniLibs.srcDir("../../../.work/security-ffi/android")
    }
    sourceSets.getByName("androidTest").java.srcDir("../../../tests/bench/security/android")
    sourceSets.getByName("androidTest").java.srcDir("../../../tests/integration/storage/android")
    sourceSets.getByName("androidTest").manifest.srcFile("../../../tests/integration/storage/android/AndroidManifest.xml")
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    lint { warningsAsErrors = true; abortOnError = true; disable += setOf("GradleDependency", "AndroidGradlePluginVersion") }
}
kotlin.compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17); allWarningsAsErrors.set(true) }
dependencies {
    implementation("androidx.annotation:annotation:1.9.1")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
    implementation(files("../../../.work/security-sqlcipher/sqlcipher-4.17.0-aligned.aar"))
    implementation("androidx.sqlite:sqlite:2.5.2")
}
tasks.matching { it.name == "packageDebug" || it.name == "packageRelease" }.configureEach {
    doFirst {
        for (abi in listOf("arm64-v8a", "x86_64")) {
            check(file("../../../.work/security-ffi/android/$abi/libmeshchat_core.so").isFile) {
                "Run src/core/build_bindings.py android --security-probe before packaging."
            }
        }
    }
}

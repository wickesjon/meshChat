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
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    signingConfigs.getByName("debug") {
        storeFile = file("../../../.work/android/debug.keystore")
    }
    sourceSets.getByName("main") {
        java.srcDir("../../../.work/ffi/kotlin")
        jniLibs.srcDir("../../../.work/ffi/android")
    }
    sourceSets.getByName("test").java.srcDirs("../../../tests/bench/android/kotlin", "../../../tests/integration/ffi/kotlin")
    testOptions.unitTests.all {
        it.systemProperty("jna.library.path", file("../../../target/debug").absolutePath)
        it.systemProperty("jna.tmpdir", file("../../../.work/tmp").absolutePath)
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
dependencies {
    implementation("androidx.annotation:annotation:1.9.1")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
    testImplementation("net.java.dev.jna:jna:5.17.0")
    testImplementation("junit:junit:4.13.2")
}
// The existing BLE CI entry point runs lint; make callback tests part of it.
tasks.matching { it.name == "lintDebug" }.configureEach { dependsOn("testDebugUnitTest") }
val verifyTransportApks = tasks.register<Exec>("verifyTransportApks") {
    dependsOn("assembleDebug", "assembleRelease")
    doFirst {
        val debugApk = fileTree("build/outputs/apk/debug") { include("*.apk") }.singleFile
        val releaseApk = fileTree("build/outputs/apk/release") { include("*.apk") }.singleFile
        val python = if (System.getProperty("os.name").startsWith("Windows")) "python" else "python3"
        commandLine(python, "-B", file("../../../tests/integration/ffi/check_android_apk.py"), debugApk, releaseApk)
    }
}
tasks.matching { it.name == "lintDebug" }.configureEach { dependsOn(verifyTransportApks) }
kotlin.compilerOptions {
    jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    allWarningsAsErrors.set(true)
}

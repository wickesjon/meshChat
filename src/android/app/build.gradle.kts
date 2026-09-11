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
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    sourceSets {
        getByName("main") {
            java.srcDir(rootProject.file("../../.work/ffi/kotlin"))
            jniLibs.srcDir(rootProject.file("../../.work/ffi/android"))
        }
        getByName("test").java.srcDir(rootProject.file("../../tests/integration/ffi/kotlin"))
    }

    testOptions.unitTests.all {
        it.systemProperty("jna.library.path", rootProject.file("../../target/debug").absolutePath)
        it.systemProperty("jna.tmpdir", rootProject.file("../../.work/tmp").absolutePath)
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

dependencies {
    implementation("androidx.annotation:annotation:1.9.1")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
    testImplementation("net.java.dev.jna:jna:5.17.0")
    testImplementation("junit:junit:4.13.2")
}

tasks.matching { it.name == "packageDebug" || it.name == "packageRelease" }.configureEach {
    doFirst {
        for (abi in listOf("arm64-v8a", "x86_64")) {
            check(rootProject.file("../../.work/ffi/android/$abi/libmeshchat_core.so").isFile) {
                "Run python3 -B src/core/build_bindings.py android from the repository root before packaging."
            }
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
        allWarningsAsErrors.set(true)
    }
}

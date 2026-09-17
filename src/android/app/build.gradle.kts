plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "org.meshchat.app"
    compileSdk = 36
    buildToolsVersion = "35.0.0"
    buildFeatures { compose = true }

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
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }

    sourceSets {
        getByName("main") {
            java.srcDir(rootProject.file("../../.work/ffi/kotlin"))
            java.srcDirs(rootProject.file("ui/src/main/kotlin"),
                rootProject.file("ble/src/main/java/org/meshchat/transport"),
                rootProject.file("security/src/main/java/org/meshchat/identity"),
                rootProject.file("security/src/main/java/org/meshchat/storage"))
            jniLibs.srcDir(rootProject.file("../../.work/ffi/android"))
        }
        getByName("test").java.srcDir(rootProject.file("../../tests/integration/ffi/kotlin"))
        getByName("test").java.srcDir(rootProject.file("../../tests/integration/android-ui/kotlin"))
        getByName("androidTest").java.srcDir(rootProject.file("../../tests/integration/android-ui/android"))
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
    // The upstream JNI has 4 KB RELRO; package the source-rebuilt pinned AAR.
    configurations.configureEach { exclude(group = "androidx.graphics", module = "graphics-path") }
    implementation(files(rootProject.file("../../.work/ui-graphics/graphics-path-1.0.1-aligned.aar")))
    implementation("androidx.annotation:annotation:1.9.1")
    implementation("net.java.dev.jna:jna:5.17.0@aar")
    implementation(files(rootProject.file("../../.work/security-sqlcipher/sqlcipher-4.17.0-aligned.aar")))
    implementation("androidx.sqlite:sqlite:2.5.2")
    implementation(platform("androidx.compose:compose-bom:2025.05.01"))
    implementation("androidx.activity:activity-compose:1.10.1")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui")
    androidTestImplementation(platform("androidx.compose:compose-bom:2025.05.01"))
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    testImplementation("net.java.dev.jna:jna:5.17.0")
    testImplementation("junit:junit:4.13.2")
}

tasks.matching { it.name == "packageDebug" || it.name == "packageRelease" }.configureEach {
    doFirst {
        check(rootProject.file("../../.work/ui-graphics/graphics-path-1.0.1-aligned.aar").isFile) {
            "Run src/android/ui/build_graphics.py before packaging."
        }
        check(rootProject.file("../../.work/security-sqlcipher/sqlcipher-4.17.0-aligned.aar").isFile) {
            "Build the pinned aligned SQLCipher AAR with src/android/security/build_sqlcipher.py before packaging."
        }
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

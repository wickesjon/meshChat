plugins { kotlin("jvm") version "2.2.0" }

val repository = rootDir.resolve("../../..").canonicalFile
kotlin {
    jvmToolchain(17)
    sourceSets.test {
        kotlin.srcDir("kotlin")
        kotlin.srcDir(repository.resolve(".work/mc022/bindings/uniffi"))
    }
    compilerOptions { allWarningsAsErrors.set(true) }
}
dependencies {
    testImplementation("net.java.dev.jna:jna:5.17.0")
    testImplementation("junit:junit:4.13.2")
}
tasks.test {
    systemProperty("meshchat.root", repository.absolutePath)
    systemProperty("jna.library.path", repository.resolve(".work/mc022/target/debug").absolutePath)
    systemProperty("jna.tmpdir", repository.resolve(".work/tmp").absolutePath)
    outputs.upToDateWhen { false }
}

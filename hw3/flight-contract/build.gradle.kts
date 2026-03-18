import com.google.protobuf.gradle.*
import org.gradle.api.tasks.compile.JavaCompile
import org.jetbrains.kotlin.gradle.dsl.JvmTarget
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

plugins {
    kotlin("jvm")
    `java-library`
    id("com.google.protobuf")
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(25))
    }
    sourceSets["main"].java.srcDirs(
        "build/generated/source/proto/main/java",
        "build/generated/source/proto/main/grpc",
    )
}

tasks.withType<JavaCompile>().configureEach {
    options.release.set(25)
}

tasks.withType<KotlinCompile>().configureEach {
    compilerOptions.jvmTarget.set(JvmTarget.JVM_25)
}

dependencies {
    api("com.google.protobuf:protobuf-java-util:4.31.1")
    api("io.grpc:grpc-protobuf:1.79.0")
    api("io.grpc:grpc-stub:1.79.0")
    compileOnly("javax.annotation:javax.annotation-api:1.3.2")
}

protobuf {
    protoc {
        artifact = "com.google.protobuf:protoc:4.31.1"
    }

    plugins {
        register("grpc") {
            artifact = "io.grpc:protoc-gen-grpc-java:1.79.0"
        }
    }

    generateProtoTasks {
        all().forEach {
            it.plugins {
                register("grpc")
            }
        }
    }
}

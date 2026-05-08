plugins {
    kotlin("jvm") version "2.3.10"
    id("com.github.davidmc24.gradle.plugin.avro") version "1.9.1"
    application
}

java {
    toolchain {
        languageVersion.set(JavaLanguageVersion.of(25))
    }
}

repositories {
    mavenCentral()
    maven("https://packages.confluent.io/maven/")
}

application {
    mainClass.set("warehouse.MainKt")
}

dependencies {
    implementation("org.apache.kafka:kafka-clients:3.9.0")
    implementation("io.confluent:kafka-avro-serializer:7.9.0")
    implementation("org.apache.avro:avro:1.12.0")

    implementation("com.datastax.oss:java-driver-core:4.17.0")

    implementation("com.fasterxml.jackson.module:jackson-module-kotlin:2.18.2")

    implementation("io.prometheus:simpleclient:0.16.0")
    implementation("io.prometheus:simpleclient_common:0.16.0")

    implementation("ch.qos.logback:logback-classic:1.5.18")
    implementation("org.slf4j:slf4j-api:2.0.16")
}

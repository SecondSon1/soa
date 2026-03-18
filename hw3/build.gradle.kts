plugins {
    kotlin("jvm") version "2.3.10" apply false
    id("com.google.protobuf") version "0.9.5" apply false
}

allprojects {
    group = "com.example"
    version = "0.1.0"

    repositories {
        mavenCentral()
    }
}

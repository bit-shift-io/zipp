import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val keystoreProperties = Properties().apply {
    val file = rootProject.file("keystore.properties")
    if (file.exists()) file.inputStream().use { load(it) }
}
val hasSharedKey = keystoreProperties.getProperty("storeFile") != null

android {
    namespace = "io.bitshift.zipp"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    sourceSets.getByName("main").java.setSrcDirs(listOf(
        "src/main/kotlin", providers.gradleProperty("icedAndroidJava").get()
    ))

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        applicationId = "io.bitshift.zipp"
        minSdk = 34
        targetSdk = 36
        versionCode = providers.gradleProperty("zippVersionCode").getOrElse("1").toInt()
        versionName = providers.gradleProperty("zippVersionName").getOrElse("0.0.0")
        ndk { abiFilters += "arm64-v8a" }
    }

    signingConfigs {
        if (hasSharedKey) {
            create("shared") {
                storeFile = rootProject.file(keystoreProperties.getProperty("storeFile"))
                storePassword = keystoreProperties.getProperty("storePassword")
                keyAlias = keystoreProperties.getProperty("keyAlias")
                keyPassword = keystoreProperties.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        debug {
            signingConfig = if (hasSharedKey) {
                signingConfigs.getByName("shared")
            } else {
                signingConfigs.getByName("debug")
            }
        }
        release {
            signingConfig = if (hasSharedKey) {
                signingConfigs.getByName("shared")
            } else {
                signingConfigs.getByName("debug")
            }
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

// ScolIA — appli Android unique (D-003). Deux modules :
//  - `:core` : Kotlin pur (modèles, client API, droits → écrans, rattachement, passkey,
//    cache hors ligne). Compile et se teste sur n'importe quelle JVM.
//  - `:app`  : appli Jetpack Compose (Android SDK + dépôt Google Maven nécessaires).
// Sans accès au dépôt Google : `./gradlew -Pscolia.coreSeulement=true :core:test`.

pluginManagement {
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
        gradlePluginPortal()
    }
    plugins {
        id("com.android.application") version "8.11.1"
        id("org.jetbrains.kotlin.android") version "2.2.21"
        id("org.jetbrains.kotlin.jvm") version "2.2.21"
        id("org.jetbrains.kotlin.plugin.serialization") version "2.2.21"
        id("org.jetbrains.kotlin.plugin.compose") version "2.2.21"
    }
}

dependencyResolutionManagement {
    repositoriesMode.set(RepositoriesMode.FAIL_ON_PROJECT_REPOS)
    repositories {
        google {
            content {
                includeGroupByRegex("com\\.android.*")
                includeGroupByRegex("com\\.google.*")
                includeGroupByRegex("androidx.*")
            }
        }
        mavenCentral()
    }
}

rootProject.name = "scolia-android"

include(":core")
if (providers.gradleProperty("scolia.coreSeulement").orNull != "true") {
    include(":app")
}

import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

// Images Android instrumentées de Robolectric (DefaultSdkProvider de Robolectric 4.14.1) :
// - SDK d'exécution des tests, @Config(sdk = [34]) de EcransComposeTest : 14-robolectric-10818077 ;
// - « compileSdk » de Robolectric = plus haut SDK qu'il connaît (35, ressources système) :
//   15-robolectric-12650502 ; il est chargé quel que soit le SDK des tests.
// « -i7 » = PREINSTRUMENTED_VERSION de Robolectric 4.14.1. Monter Robolectric ou changer @Config
// impose de mettre cette liste à jour : en mode hors ligne, une image absente fait échouer les tests.
val versionsImageRobolectric = listOf("14-robolectric-10818077-i7", "15-robolectric-12650502-i7")
val imageRobolectric: Configuration by configurations.creating { isTransitive = false }
val dossierImageRobolectric = layout.buildDirectory.dir("robolectric-sdk")
val preparerImageRobolectric = tasks.register<Sync>("preparerImageRobolectric") {
    from(imageRobolectric)
    into(dossierImageRobolectric)
}

android {
    namespace = "fr.scolia.mobile.app"
    compileSdk = 36

    defaultConfig {
        applicationId = "fr.scolia.mobile"
        minSdk = 28 // Android 9 : passkeys via Credential Manager + services Play.
        targetSdk = 36 // DA2 : Google Play exige l'API 36 depuis le 31/08/2026.
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        // Domaine des passkeys (D-013) et des établissements (D-012).
        buildConfigField("String", "DOMAINE_SCOLIA", "\"scolia.fr\"")
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
            // Labo local (`http://10.0.2.2:3001`) accepté uniquement en debug.
            buildConfigField("boolean", "MODE_LABO", "true")
        }
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            buildConfigField("boolean", "MODE_LABO", "false")
        }
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    testOptions {
        // Robolectric (tests Compose sans appareil) a besoin des ressources fusionnées.
        unitTests.isIncludeAndroidResources = true
        // Robolectric hors ligne : l'image Android est résolue par Gradle (cache Gradle de la CI,
        // nouvelles tentatives, miroirs de ~/.gradle/init.d) au lieu d'être téléchargée en direct
        // par Robolectric pendant les tests (échecs réseau HttpURLConnection sur les runners).
        unitTests.all { t ->
            t.dependsOn(preparerImageRobolectric)
            val dossier = dossierImageRobolectric.get().asFile
            t.systemProperty("robolectric.offline", "true")
            t.systemProperty("robolectric.dependency.dir", dossier.absolutePath)
            // Échec lisible plutôt que l'IllegalArgumentException muette de LocalDependencyResolver.
            t.doFirst {
                for (v in versionsImageRobolectric) {
                    val attendu = dossier.resolve("android-all-instrumented-$v.jar")
                    check(attendu.isFile) {
                        "Image Robolectric absente : $attendu (contenu : ${dossier.list()?.toList()})"
                    }
                }
            }
        }
    }

    packaging {
        resources.excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }
}

kotlin {
    compilerOptions { jvmTarget.set(JvmTarget.JVM_17) }
}

dependencies {
    implementation(project(":core"))

    val composeBom = platform("androidx.compose:compose-bom:2024.12.01")
    implementation(composeBom)
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-core")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")

    implementation("androidx.core:core-ktx:1.13.1")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.fragment:fragment-ktx:1.8.5")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")

    // Passkeys natives (Credential Manager ; services Play pour Android 9 à 13).
    implementation("androidx.credentials:credentials:1.3.0")
    implementation("androidx.credentials:credentials-play-services-auth:1.3.0")
    // Déverrouillage biométrique (empreinte / visage / code de l'appareil).
    implementation("androidx.biometric:biometric:1.1.0")
    // Lecture du QR code de rattachement (Apache-2.0, sans service Google).
    implementation("com.journeyapps:zxing-android-embedded:4.3.0")

    testImplementation("junit:junit:4.13.2")
    testImplementation(kotlin("test"))
    // Tests Compose sans appareil ni émulateur (JVM + Robolectric).
    testImplementation("org.robolectric:robolectric:4.14.1")
    versionsImageRobolectric.forEach { imageRobolectric("org.robolectric:android-all-instrumented:$it") }
    testImplementation("androidx.compose.ui:ui-test-junit4")
    testImplementation("androidx.test:core-ktx:1.6.1")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

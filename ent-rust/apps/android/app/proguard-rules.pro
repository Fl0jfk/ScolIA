# kotlinx.serialization : conserver les sérialiseurs générés.
-keepattributes *Annotation*, InnerClasses
-keepclassmembers class fr.scolia.mobile.core.** {
    *** Companion;
    kotlinx.serialization.KSerializer serializer(...);
}
-keep,includedescriptorclasses class fr.scolia.mobile.core.**$$serializer { *; }
-dontwarn okhttp3.internal.platform.**
-dontwarn org.conscrypt.**
-dontwarn org.bouncycastle.**
-dontwarn org.openjsse.**

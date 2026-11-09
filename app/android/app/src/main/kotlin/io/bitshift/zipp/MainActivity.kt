package io.bitshift.zipp

import android.content.pm.PackageManager
import android.graphics.PixelFormat
import android.os.Bundle
import io.github.santuzius.icedandroid.IcedActivity
import java.util.Locale

class MainActivity : IcedActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        // Install the bridge and the core before NativeActivity starts
        // android_main. Loading on the Java thread lets JNI_OnLoad cache the
        // ZippBridge class for every later callback thread.
        System.loadLibrary("zipp")
        ZippBridge.install(this, applicationContext)
        ZippBridge.nativeInit(Locale.getDefault().language)
        super.onCreate(savedInstanceState)
        // NativeActivity.onCreate resets this to RGB_565. Set it afterwards,
        // before the surface is created, or Vulkan's inherited alpha is opaque.
        window.setFormat(PixelFormat.RGBA_8888)
        ZippBridge.requestPermissions()
        ZippBridge.registerWifi()
        ZippBridge.startGps()
    }

    override fun onRequestPermissionsResult(
        requestCode: Int,
        permissions: Array<String>,
        grantResults: IntArray,
    ) {
        super.onRequestPermissionsResult(requestCode, permissions, grantResults)
        ZippBridge.onPermissionResult(
            grantResults.isNotEmpty() &&
                grantResults.all { it == PackageManager.PERMISSION_GRANTED }
        )
    }

    // Older back dispatch; IcedActivity handles predictive back through iced.
    @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
    override fun onBackPressed() {
        ZippBridge.nativeBack()
    }

    override fun onDestroy() {
        ZippBridge.shutdown()
        super.onDestroy()
    }
}

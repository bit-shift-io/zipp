package io.bitshift.zipp

import android.Manifest
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothGatt
import android.bluetooth.BluetoothGattCallback
import android.bluetooth.BluetoothGattCharacteristic
import android.bluetooth.BluetoothGattDescriptor
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.bluetooth.le.BluetoothLeScanner
import android.bluetooth.le.ScanCallback
import android.bluetooth.le.ScanFilter
import android.bluetooth.le.ScanResult
import android.bluetooth.le.ScanSettings
import android.content.Context
import android.content.pm.PackageManager
import android.location.Location
import android.location.LocationListener
import android.location.LocationManager
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.os.Handler
import android.os.Looper
import java.lang.ref.WeakReference
import java.util.UUID

// Everything the Rust core asks the OS for: BLE scan and GATT, GPS and Wi-Fi.
// Rust owns all data and logic; this object only drives the Android APIs and
// reports every change through the native callbacks. The `external fun`s are
// instance methods on the singleton, which is why the Rust `Java_...` exports
// take a `JObject`; the commands Rust calls back with `CallStaticVoidMethod`
// are `@JvmStatic` so they resolve as static methods on
// `io/bitshift/zipp/ZippBridge`.
object ZippBridge {
    /// The Client Characteristic Configuration descriptor shared by every GATT
    /// server; writing it turns notifications or indications on.
    private const val CCCD = "00002902-0000-1000-8000-00805f9b34fb"
    private const val REQUEST_PERMISSIONS = 1
    private const val SCAN_TIMEOUT_MS = 15_000L
    private const val CONNECT_TIMEOUT_MS = 10_000L

    private var activity: WeakReference<MainActivity>? = null
    private lateinit var context: Context
    private var bluetoothManager: BluetoothManager? = null
    private var adapter: BluetoothAdapter? = null
    private var scanner: BluetoothLeScanner? = null
    private var gatt: BluetoothGatt? = null
    private var characteristics = mutableMapOf<UUID, BluetoothGattCharacteristic>()
    private var targetName = "BScooter"
    private var pendingScanTarget: String? = null
    private var scanTimeout: Runnable? = null
    private var connectTimeout: Runnable? = null
    private var locationManager: LocationManager? = null
    private var locationListener: LocationListener? = null
    private var connectivity: ConnectivityManager? = null
    private var networkCallback: ConnectivityManager.NetworkCallback? = null
    private val handler = Handler(Looper.getMainLooper())

    // Native entry points called from Rust. Declared inside the object, so they
    // are instance methods on the singleton and the Rust exports receive the
    // object as their `JObject` receiver.
    external fun nativeInit(locale: String)
    external fun nativeBack()
    external fun onScanResult(address: String, name: String, rssi: Int)
    external fun onScanState(scanning: Boolean)
    external fun onAdapterState(on: Boolean)
    external fun onConnectionState(address: String, connected: Boolean)
    external fun onNotification(uuid: String, bytes: ByteArray)
    external fun onRead(uuid: String, bytes: ByteArray)
    external fun onWifiState(connected: Boolean)
    external fun onGpsStatus(status: String)
    external fun onGpsSpeed(mps: Double)
    external fun onError(message: String)

    /// Remember the owning Activity weakly, keep the application context, and
    /// look up the Bluetooth and connectivity services. Called once from
    /// `MainActivity.onCreate` before `nativeInit`.
    @JvmStatic
    fun install(activity: MainActivity, appContext: Context) {
        this.activity = WeakReference(activity)
        context = appContext.applicationContext
        bluetoothManager = context.getSystemService(Context.BLUETOOTH_SERVICE) as BluetoothManager
        adapter = bluetoothManager?.adapter
        scanner = adapter?.bluetoothLeScanner
        connectivity = context.getSystemService(Context.CONNECTIVITY_SERVICE) as ConnectivityManager
    }

    /// Ask for the runtime permissions scanning needs, skipping the granted
    /// ones. The result arrives in `MainActivity.onRequestPermissionsResult`.
    @JvmStatic
    fun requestPermissions() {
        val activity = activity?.get() ?: return
        val needed = mutableListOf<String>()
        for (permission in listOf(
            Manifest.permission.BLUETOOTH_SCAN,
            Manifest.permission.BLUETOOTH_CONNECT,
            Manifest.permission.ACCESS_FINE_LOCATION,
        )) {
            if (activity.checkSelfPermission(permission) != PackageManager.PERMISSION_GRANTED) {
                needed += permission
            }
        }
        if (needed.isNotEmpty()) {
            activity.requestPermissions(needed.toTypedArray(), REQUEST_PERMISSIONS)
        }
    }

    /// From `MainActivity.onRequestPermissionsResult`: retry the scan that was
    /// waiting on permissions and start GPS, or report the refusal.
    fun onPermissionResult(granted: Boolean) {
        if (!granted) {
            onError("Permissions denied for scanning")
            return
        }
        pendingScanTarget?.let { target ->
            pendingScanTarget = null
            startScan(target)
        }
        startGps()
    }

    private fun hasScanPermissions(): Boolean =
        context.checkSelfPermission(Manifest.permission.BLUETOOTH_SCAN) ==
            PackageManager.PERMISSION_GRANTED &&
            context.checkSelfPermission(Manifest.permission.BLUETOOTH_CONNECT) ==
            PackageManager.PERMISSION_GRANTED &&
            context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) ==
            PackageManager.PERMISSION_GRANTED

    @JvmStatic
    fun startScan(target: String) {
        targetName = target
        if (!hasScanPermissions()) {
            pendingScanTarget = target
            requestPermissions()
            return
        }
        // `onAdapterState(false)` already makes the core log the single
        // "Bluetooth is off" line (as Flutter does); a second `onError` would
        // duplicate it on every reconnect tick.
        val bleAdapter = adapter ?: run {
            onAdapterState(false)
            return
        }
        if (!bleAdapter.isEnabled) {
            onAdapterState(false)
            return
        }
        onAdapterState(true)
        stopScan()
        try {
            val filter = ScanFilter.Builder().setDeviceName(target).build()
            val settings = ScanSettings.Builder()
                .setScanMode(ScanSettings.SCAN_MODE_LOW_LATENCY)
                .build()
            scanner?.startScan(listOf(filter), settings, scanCallback)
            onScanState(true)
            scanTimeout = Runnable { stopScan() }.also {
                handler.postDelayed(it, SCAN_TIMEOUT_MS)
            }
        } catch (error: Exception) {
            onError("Scan Error: $error")
        }
    }

    @JvmStatic
    fun stopScan() {
        scanTimeout?.let { handler.removeCallbacks(it) }
        scanTimeout = null
        try {
            scanner?.stopScan(scanCallback)
        } catch (_: Exception) {
        }
        onScanState(false)
    }

    /// Reconnect to a bonded device when there is one, otherwise scan for it.
    @JvmStatic
    fun reconnect(target: String) {
        val bonded = adapter?.bondedDevices?.firstOrNull { it.name == target }
        if (bonded != null) {
            connect(bonded.address)
        } else {
            startScan(target)
        }
    }

    @JvmStatic
    fun connect(address: String) {
        val device = adapter?.getRemoteDevice(address) ?: return
        stopScan()
        disconnect()
        gatt = device.connectGatt(context, false, gattCallback, BluetoothDevice.TRANSPORT_LE)
        connectTimeout = Runnable {
            onError("Connection error: timeout")
            disconnect()
        }.also { handler.postDelayed(it, CONNECT_TIMEOUT_MS) }
    }

    @JvmStatic
    fun disconnect() {
        connectTimeout?.let { handler.removeCallbacks(it) }
        connectTimeout = null
        characteristics.clear()
        gatt?.disconnect()
        gatt?.close()
        gatt = null
    }

    /// Write to a characteristic, without a response when it supports one.
    @JvmStatic
    fun write(service: String, characteristic: String, bytes: ByteArray) {
        val gatt = gatt ?: return
        val char = characteristics[UUID.fromString(characteristic)] ?: return
        val withoutResponse =
            char.properties and BluetoothGattCharacteristic.PROPERTY_WRITE_NO_RESPONSE != 0
        val writeType = if (withoutResponse) {
            BluetoothGattCharacteristic.WRITE_TYPE_NO_RESPONSE
        } else {
            BluetoothGattCharacteristic.WRITE_TYPE_DEFAULT
        }
        try {
            gatt.writeCharacteristic(char, bytes, writeType)
        } catch (error: Exception) {
            onError("Command Failed: $error")
        }
    }

    @JvmStatic
    fun read(service: String, characteristic: String) {
        val gatt = gatt ?: return
        val char = characteristics[UUID.fromString(characteristic)] ?: return
        try {
            gatt.readCharacteristic(char)
        } catch (error: Exception) {
            onError("Command Failed: $error")
        }
    }

    // The scan filter accepts the target name, so every result is that device.
    private val scanCallback = object : ScanCallback() {
        override fun onScanResult(callbackType: Int, result: ScanResult) {
            val name = result.scanRecord?.deviceName
                ?: result.device.name
                ?: ""
            ZippBridge.onScanResult(result.device.address, name, result.rssi)
        }

        override fun onScanFailed(errorCode: Int) {
            ZippBridge.onError("Scan Error: $errorCode")
            ZippBridge.onScanState(false)
        }
    }

    private val gattCallback = object : BluetoothGattCallback() {
        override fun onConnectionStateChange(gatt: BluetoothGatt, status: Int, newState: Int) {
            val device = gatt.device
            if (status == BluetoothGatt.GATT_SUCCESS &&
                newState == BluetoothProfile.STATE_CONNECTED
            ) {
                cancelConnectTimeout()
                gatt.discoverServices()
                ZippBridge.onConnectionState(device.address, true)
            } else if (newState == BluetoothProfile.STATE_DISCONNECTED) {
                cancelConnectTimeout()
                characteristics.clear()
                gatt.close()
                if (this@ZippBridge.gatt === gatt) {
                    this@ZippBridge.gatt = null
                }
                ZippBridge.onConnectionState(device.address, false)
            } else {
                ZippBridge.onError("Connection error: status $status")
            }
        }

        override fun onServicesDiscovered(gatt: BluetoothGatt, status: Int) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onError("Connection error: status $status")
                return
            }
            for (service in gatt.services) {
                for (characteristic in service.characteristics) {
                    characteristics[characteristic.uuid] = characteristic
                    val properties = characteristic.properties
                    val notify =
                        (properties and BluetoothGattCharacteristic.PROPERTY_NOTIFY) != 0
                    val indicate =
                        (properties and BluetoothGattCharacteristic.PROPERTY_INDICATE) != 0
                    if (notify || indicate) {
                        gatt.setCharacteristicNotification(characteristic, true)
                        characteristic.getDescriptor(UUID.fromString(CCCD))?.let { descriptor ->
                            val value = if (notify) {
                                BluetoothGattDescriptor.ENABLE_NOTIFICATION_VALUE
                            } else {
                                BluetoothGattDescriptor.ENABLE_INDICATION_VALUE
                            }
                            gatt.writeDescriptor(descriptor, value)
                        }
                    }
                    if ((properties and BluetoothGattCharacteristic.PROPERTY_READ) != 0) {
                        gatt.readCharacteristic(characteristic)
                    }
                }
            }
        }

        @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
        ) {
            ZippBridge.onNotification(characteristic.uuid.toString(), characteristic.value)
        }

        override fun onCharacteristicChanged(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray,
        ) {
            ZippBridge.onNotification(characteristic.uuid.toString(), value)
        }

        @Suppress("DEPRECATION", "OVERRIDE_DEPRECATION")
        override fun onCharacteristicRead(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int,
        ) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onRead(characteristic.uuid.toString(), characteristic.value)
            } else {
                ZippBridge.onError("Command Failed: status $status")
            }
        }

        override fun onCharacteristicRead(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            value: ByteArray,
            status: Int,
        ) {
            if (status == BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onRead(characteristic.uuid.toString(), value)
            } else {
                ZippBridge.onError("Command Failed: status $status")
            }
        }

        override fun onCharacteristicWrite(
            gatt: BluetoothGatt,
            characteristic: BluetoothGattCharacteristic,
            status: Int,
        ) {
            if (status != BluetoothGatt.GATT_SUCCESS) {
                ZippBridge.onError("Command Failed: status $status")
            }
        }
    }

    private fun cancelConnectTimeout() {
        connectTimeout?.let { handler.removeCallbacks(it) }
        connectTimeout = null
    }

    // GPS speed only; the status strings ("denied", "disabled", "searching",
    // "active") are the ones the core maps in `platform::android`.
    @JvmStatic
    fun startGps() {
        if (context.checkSelfPermission(Manifest.permission.ACCESS_FINE_LOCATION) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            onGpsStatus("denied")
            return
        }
        val manager = context.getSystemService(Context.LOCATION_SERVICE) as LocationManager
        locationManager = manager
        if (!manager.isProviderEnabled(LocationManager.GPS_PROVIDER)) {
            onGpsStatus("disabled")
            return
        }
        val listener = object : LocationListener {
            override fun onLocationChanged(location: Location) {
                ZippBridge.onGpsStatus("active")
                ZippBridge.onGpsSpeed(location.speed.toDouble())
            }

            override fun onProviderEnabled(provider: String) {}

            override fun onProviderDisabled(provider: String) {
                ZippBridge.onGpsStatus("disabled")
            }

            @Suppress("OVERRIDE_DEPRECATION")
            @Deprecated("Deprecated in Java")
            override fun onStatusChanged(
                provider: String?,
                status: Int,
                extras: android.os.Bundle?,
            ) {
            }
        }
        locationListener = listener
        try {
            manager.requestLocationUpdates(
                LocationManager.GPS_PROVIDER,
                0L,
                0f,
                listener,
                Looper.getMainLooper(),
            )
            onGpsStatus("searching")
        } catch (error: Exception) {
            onGpsStatus("disabled")
            onError("GPS Error: $error")
        }
    }

    @JvmStatic
    fun stopGps() {
        locationListener?.let { locationManager?.removeUpdates(it) }
        locationListener = null
        locationManager = null
    }

    /// Follow the default network and report whether it is Wi-Fi.
    @JvmStatic
    fun registerWifi() {
        val manager = connectivity ?: return
        networkCallback?.let { manager.unregisterNetworkCallback(it) }
        val callback = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) = report(manager, network)

            override fun onCapabilitiesChanged(
                network: Network,
                caps: NetworkCapabilities,
            ) = report(manager, network)

            override fun onLost(network: Network) {
                ZippBridge.onWifiState(false)
            }
        }
        networkCallback = callback
        manager.registerDefaultNetworkCallback(callback)
        val current = manager.activeNetwork
        ZippBridge.onWifiState(
            current != null &&
                manager.getNetworkCapabilities(current)
                    ?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
        )
    }

    private fun report(manager: ConnectivityManager, network: Network) {
        val wifi = manager.getNetworkCapabilities(network)
            ?.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) == true
        onWifiState(wifi)
    }

    /// Release every OS registration; called from `MainActivity.onDestroy`.
    @JvmStatic
    fun shutdown() {
        stopScan()
        disconnect()
        stopGps()
        networkCallback?.let { connectivity?.unregisterNetworkCallback(it) }
        networkCallback = null
        activity = null
    }
}
